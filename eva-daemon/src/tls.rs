use rustls::{ClientConfig, RootCertStore};
use std::sync::Arc;
use tokio::net::TcpStream;
use tokio_rustls::{TlsConnector, client::TlsStream};

pub struct TlsManager {
    connector: TlsConnector,
}

impl TlsManager {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        // Carregar certificados raiz (CA certificates)
        let mut root_store = RootCertStore::empty();
        
        // Usar certificados do sistema
        for cert in rustls_native_certs::load_native_certs()? {
            root_store.add(cert).ok();
        }
        
        // Fallback: usar certificados embutidos do webpki
        root_store.extend(
            webpki_roots::TLS_SERVER_ROOTS
                .iter()
                .cloned()
        );

        // Configurar cliente TLS
        let config = ClientConfig::builder()
            .with_root_certificates(root_store)
            .with_no_client_auth();

        let connector = TlsConnector::from(Arc::new(config));

        Ok(Self { connector })
    }

    pub async fn connect(
        &self,
        domain: &str,
        port: u16,
    ) -> Result<TlsStream<TcpStream>, Box<dyn std::error::Error>> {
        // Conectar TCP primeiro
        let addr = format!("{}:{}", domain, port);
        let tcp_stream = TcpStream::connect(&addr).await?;

        // Fazer handshake TLS
        let server_name = rustls::pki_types::ServerName::try_from(domain.to_string())?;
        let tls_stream = self.connector.connect(server_name, tcp_stream).await?;

        Ok(tls_stream)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_tls_connection() {
        let tls = TlsManager::new().expect("Failed to create TLS manager");
        
        let result = tls.connect("google.com", 443).await;
        assert!(result.is_ok(), "TLS connection should succeed");
    }
}
