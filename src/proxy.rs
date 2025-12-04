use anyhow::{anyhow, Context};
use hyper::client::HttpConnector;
use hyper::service::{make_service_fn, service_fn};
use hyper::{
    server::conn::AddrStream, Body, Client, Method, Request, Response, Server, StatusCode,
};
use tokio::io::{copy_bidirectional, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::task::JoinHandle;
use tracing::{debug, error, info, instrument};

use crate::settings::ProxySettings;

#[derive(Clone)]
pub struct ProxyServer {
    settings: ProxySettings,
    client: Client<HttpConnector>,
}

impl ProxyServer {
    pub fn new(settings: ProxySettings) -> Self {
        let mut connector = HttpConnector::new();
        connector.enforce_http(false);
        let client = Client::builder().build::<_, Body>(connector);

        Self { settings, client }
    }

    pub fn address(&self) -> String {
        self.settings.listen().to_string()
    }

    pub async fn run(&self) -> anyhow::Result<()> {
        let addr = self.settings.listen();
        let client = self.client.clone();
        let make_service = make_service_fn(move |conn: &AddrStream| {
            let client = client.clone();
            let remote_addr = conn.remote_addr();
            async move {
                Ok::<_, hyper::Error>(service_fn(move |req| {
                    let client = client.clone();
                    handle_request(client, remote_addr, req)
                }))
            }
        });

        Server::bind(&addr)
            .serve(make_service)
            .await
            .context("Error al iniciar el servidor")
    }
}

#[instrument(skip_all, fields(remote = %remote_addr))]
async fn handle_request(
    client: Client<HttpConnector>,
    remote_addr: std::net::SocketAddr,
    req: Request<Body>,
) -> Result<Response<Body>, hyper::Error> {
    match *req.method() {
        Method::CONNECT => handle_connect(remote_addr, req).await,
        _ => handle_http(client, remote_addr, req).await,
    }
}

async fn handle_http(
    client: Client<HttpConnector>,
    remote_addr: std::net::SocketAddr,
    mut req: Request<Body>,
) -> Result<Response<Body>, hyper::Error> {
    let uri = req.uri().clone();
    debug!(%remote_addr, %uri, method = %req.method(), "HTTP proxy request");

    // Hyper proxy requests must have absolute URI; fail otherwise.
    if uri.scheme().is_none() || uri.host().is_none() {
        return Ok(Response::builder()
            .status(StatusCode::BAD_REQUEST)
            .body(Body::from("URI debe ser absoluta (incluya esquema y host)"))
            .expect("respuesta bad request"));
    }

    // Remove hop-by-hop headers
    sanitize_headers(req.headers_mut());

    client.request(req).await
}

async fn handle_connect(
    remote_addr: std::net::SocketAddr,
    req: Request<Body>,
) -> Result<Response<Body>, hyper::Error> {
    let authority = req.uri().authority().map(|a| a.to_string());
    let host = match authority {
        Some(authority) => authority,
        None => {
            return Ok(Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .body(Body::from("CONNECT requiere autoridad host:puerto"))
                .expect("respuesta CONNECT bad request"))
        }
    };

    info!(%remote_addr, %host, "Estableciendo tunel CONNECT");

    // Establish TCP tunnel
    let on_upgrade = hyper::upgrade::on(req);
    let stream = match TcpStream::connect(&host).await {
        Ok(stream) => stream,
        Err(e) => {
            error!(%host, error = %e, "Fallo al conectar con destino");
            return Ok(Response::builder()
                .status(StatusCode::BAD_GATEWAY)
                .body(Body::from("No se pudo conectar con el destino"))
                .expect("respuesta BAD_GATEWAY"));
        }
    };

    let mut response = Response::new(Body::empty());
    *response.status_mut() = StatusCode::OK;
    tokio::task::spawn(async move {
        match tunnel(on_upgrade, stream).await {
            Ok(_) => debug!(%remote_addr, %host, "Tunel cerrado"),
            Err(e) => error!(%remote_addr, %host, error = %e, "Tunel fallido"),
        }
    });

    Ok(response)
}

fn sanitize_headers(headers: &mut hyper::HeaderMap) {
    const HOP_BY_HOP: [&str; 8] = [
        "connection",
        "keep-alive",
        "proxy-authenticate",
        "proxy-authorization",
        "te",
        "trailers",
        "transfer-encoding",
        "upgrade",
    ];

    for name in HOP_BY_HOP.iter() {
        headers.remove(*name);
    }
}

async fn tunnel(
    on_upgrade: hyper::upgrade::OnUpgrade,
    mut stream: TcpStream,
) -> anyhow::Result<()> {
    let mut upgraded = on_upgrade.await.context("Upgrade HTTP falló")?;
    let bytes_copied = copy_bidirectional(&mut upgraded, &mut stream).await?;
    debug!(
        "Tunel bytes enviados: client->server={} server->client={}",
        bytes_copied.0, bytes_copied.1
    );
    upgraded.shutdown().await.ok();
    stream.shutdown().await.ok();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use hyper::body::to_bytes;
    use hyper::server::conn::Http;
    use hyper::service::service_fn as hyper_service_fn;
    use hyper::{Response as HyperResponse, StatusCode};
    use std::convert::Infallible;
    use std::net::TcpListener as StdTcpListener;
    use tokio::net::TcpListener;

    #[tokio::test]
    async fn test_handle_http_rejects_relative_uri() {
        let client = Client::new();
        let addr = "127.0.0.1:3000".parse().unwrap();
        let req = Request::builder()
            .method(Method::GET)
            .uri("/solo-relativo")
            .body(Body::empty())
            .unwrap();
        let res = handle_http(client, addr, req).await.unwrap();
        assert_eq!(res.status(), StatusCode::BAD_REQUEST);
        let body = to_bytes(res.into_body()).await.unwrap();
        assert!(std::str::from_utf8(&body)
            .unwrap()
            .contains("URI debe ser absoluta"));
    }

    #[tokio::test]
    async fn test_tunnel_and_forward() {
        // Target server that echoes path
        let target_listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let target_addr = target_listener.local_addr().unwrap();
        tokio::spawn(async move {
            loop {
                let (stream, _) = target_listener.accept().await.unwrap();
                let service = hyper_service_fn(|req| async move {
                    let body = format!("echo:{}", req.uri().path());
                    Ok::<_, Infallible>(HyperResponse::new(Body::from(body)))
                });
                tokio::spawn(async move {
                    if let Err(e) = Http::new().serve_connection(stream, service).await {
                        eprintln!("target server error: {e}");
                    }
                });
            }
        });

        // Proxy server
        let proxy_settings = ProxySettings::new("127.0.0.1:0".parse().unwrap());
        let addr = proxy_settings.listen();
        let listener = TcpListener::bind(addr).await.unwrap();
        let proxy_task: JoinHandle<anyhow::Result<()>> = tokio::spawn(async move {
            let client = Client::new();
            let make_service = make_service_fn(move |conn: &AddrStream| {
                let client = client.clone();
                let remote_addr = conn.remote_addr();
                async move {
                    Ok::<_, hyper::Error>(service_fn(move |req| {
                        let client = client.clone();
                        handle_request(client, remote_addr, req)
                    }))
                }
            });

            let std_listener: StdTcpListener = listener.into_std().unwrap();
            std_listener.set_nonblocking(true).unwrap();

            Server::from_tcp(std_listener)
                .unwrap()
                .serve(make_service)
                .await
                .map_err(|e| anyhow!(e))
        });

        // Client request through proxy using absolute URI
        let url = format!("http://{}/hello", target_addr);
        let request = Request::builder()
            .method(Method::GET)
            .uri(url)
            .body(Body::empty())
            .unwrap();

        let client = Client::builder().build::<_, Body>(HttpConnector::new());
        let response = client.request(request).await.expect("proxy request failed");
        let status = response.status();
        let body = to_bytes(response.into_body()).await.unwrap();
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body, hyper::body::Bytes::from("echo:/hello"));

        proxy_task.abort();
    }
}
