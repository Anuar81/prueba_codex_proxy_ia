use clap::{ArgAction, Parser};
use std::net::SocketAddr;
use tracing::{info, Level};
use tracing_subscriber::EnvFilter;

mod proxy;
mod settings;

use crate::proxy::ProxyServer;
use crate::settings::ProxySettings;

#[derive(Parser, Debug)]
#[command(name = "proxy-ia", version, about = "Proxy HTTP(S) con IA en Rust", long_about = None)]
struct Cli {
    /// Dirección de escucha en formato host:puerto (ej. 0.0.0.0:8888)
    #[arg(short, long, default_value = "0.0.0.0:8888")]
    listen: SocketAddr,

    /// Nivel de logs (trace, debug, info, warn, error)
    #[arg(long, default_value = "info")]
    log_level: String,

    /// Activa el modo silencioso (solo errores)
    #[arg(short, long, action = ArgAction::SetTrue)]
    quiet: bool,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    init_tracing(&cli);
    let settings = ProxySettings::new(cli.listen);
    let server = ProxyServer::new(settings);

    info!(address = %server.address(), "Iniciando proxy");
    server.run().await
}

fn init_tracing(cli: &Cli) {
    let level = if cli.quiet {
        Level::ERROR
    } else {
        cli.log_level
            .parse::<Level>()
            .unwrap_or_else(|_| Level::INFO)
    };

    let env_filter = EnvFilter::from_default_env().add_directive(level.into());

    tracing_subscriber::fmt()
        .with_env_filter(env_filter)
        .with_target(false)
        .init();
}
