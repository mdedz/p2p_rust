use std::sync::Arc;
use anyhow::Result;
use rcgen::{generate_simple_self_signed, CertifiedKey, KeyPair};
use rustls::{
    pki_types::{
        CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer, ServerName,
    },
    ClientConfig, RootCertStore, ServerConfig,
};
use tokio_rustls::{TlsAcceptor, TlsConnector};

pub struct TlsCert {
    pub certs: Vec<CertificateDer<'static>>,
    pub key: PrivateKeyDer<'static>,
}


pub fn generate_self_signed_cert() -> Result<TlsCert> {

    let subject_alt_names = vec!["localhost".to_string(), "127.0.0.1".to_string()];

    let cert: CertifiedKey<KeyPair> = generate_simple_self_signed(subject_alt_names)?;

    let cert_der = vec![CertificateDer::from(cert.cert.der().to_vec())];

    let pkcs8_der = PrivatePkcs8KeyDer::from(cert.signing_key.serialize_der());
    let key_der = PrivateKeyDer::from(pkcs8_der);

    Ok(TlsCert { certs: cert_der, key: key_der })

}

pub fn make_server_config(tls_cert: &TlsCert) -> Result<Arc<ServerConfig>> {
    let cfg = ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(tls_cert.certs.clone(), tls_cert.key.clone_key())?;

    Ok(Arc::new(cfg))
}

pub fn make_client_config(server_cert: &CertificateDer<'static>) -> Result<Arc<ClientConfig>> {
    let mut roots = RootCertStore::empty();
    roots.add(server_cert.clone())?;

    let cfg = ClientConfig::builder()
        .with_root_certificates(roots)
        .with_no_client_auth();

    Ok(Arc::new(cfg))
}

pub fn server_name(host: &str) -> Result<ServerName<'static>> {
    Ok(ServerName::try_from(host.to_string())?)
}

pub fn make_acceptor(cfg: Arc<ServerConfig>) -> TlsAcceptor {
    TlsAcceptor::from(cfg)
}

pub fn make_connector(cfg: Arc<ClientConfig>) -> TlsConnector {
    TlsConnector::from(cfg)
}