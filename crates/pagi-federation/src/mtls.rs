//! mTLS (Mutual TLS) configuration for Jamey <-> Satellite links.
//! Only bare-metal nodes with valid certificates can handshake.

use rustls::pki_types::{CertificateDer, PrivateKeyDer};
use rustls::server::WebPkiClientVerifier;
use rustls::{ClientConfig, RootCertStore, ServerConfig};
use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use std::sync::Arc;

/// Load PEM certificates from a file (server cert chain or CA certs).
fn load_certs(path: &Path) -> Result<Vec<CertificateDer<'static>>, std::io::Error> {
    let f = File::open(path)?;
    let mut reader = BufReader::new(f);
    let certs = rustls_pemfile::certs(&mut reader)
        .filter_map(|r| r.ok())
        .map(|c| c.into_owned())
        .collect::<Vec<_>>();
    Ok(certs)
}

/// Load a single private key from PEM (server or client key).
fn load_private_key(path: &Path) -> Result<PrivateKeyDer<'static>, std::io::Error> {
    let f = File::open(path)?;
    let mut reader = BufReader::new(f);
    let key = rustls_pemfile::private_key(&mut reader)?
        .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotFound, "no private key"))?;
    // rustls_pemfile returns owned key; pass through (no into_owned on PrivateKeyDer in rustls 0.23)
    Ok(key)
}

/// Build server TLS config for Jamey (Master): present server cert, require client cert (mTLS).
/// - `server_cert_path`: PEM chain (server cert + optional intermediates).
/// - `server_key_path`: PEM private key for the server.
/// - `client_ca_path`: PEM of CA that signed client (Satellite) certs; only those clients are accepted.
pub fn server_tls_config(
    server_cert_path: &Path,
    server_key_path: &Path,
    client_ca_path: &Path,
) -> Result<Arc<ServerConfig>, String> {
    let certs = load_certs(server_cert_path).map_err(|e| e.to_string())?;
    let key = load_private_key(server_key_path).map_err(|e| e.to_string())?;
    let client_ca_certs = load_certs(client_ca_path).map_err(|e| e.to_string())?;

    let mut client_ca_store = RootCertStore::empty();
    for cert in client_ca_certs {
        client_ca_store.add(cert).map_err(|e| e.to_string())?;
    }
    let verifier = WebPkiClientVerifier::builder(Arc::new(client_ca_store))
        .build()
        .map_err(|e| e.to_string())?;

    let config = ServerConfig::builder()
        .with_client_cert_verifier(verifier)
        .with_single_cert(certs, key)
        .map_err(|e| e.to_string())?;

    Ok(Arc::new(config))
}

/// Build client TLS config for a Satellite: present client cert, trust Jamey's server via `ca_path`.
/// - `client_cert_path`: PEM chain for the client.
/// - `client_key_path`: PEM private key for the client.
/// - `ca_path`: PEM of CA that signed the server (Jamey) cert.
pub fn client_tls_config(
    client_cert_path: &Path,
    client_key_path: &Path,
    ca_path: &Path,
) -> Result<Arc<ClientConfig>, String> {
    let certs = load_certs(client_cert_path).map_err(|e| e.to_string())?;
    let key = load_private_key(client_key_path).map_err(|e| e.to_string())?;
    let ca_certs = load_certs(ca_path).map_err(|e| e.to_string())?;

    let mut root_store = RootCertStore::empty();
    for cert in ca_certs {
        root_store.add(cert).map_err(|e| e.to_string())?;
    }

    let config = ClientConfig::builder()
        .with_root_certificates(root_store)
        .with_client_auth_cert(certs, key)
        .map_err(|e| e.to_string())?;

    Ok(Arc::new(config))
}
