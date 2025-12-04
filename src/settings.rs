use std::net::SocketAddr;

#[derive(Debug, Clone)]
pub struct ProxySettings {
    listen: SocketAddr,
}

impl ProxySettings {
    pub fn new(listen: SocketAddr) -> Self {
        Self { listen }
    }

    pub fn listen(&self) -> SocketAddr {
        self.listen
    }
}
