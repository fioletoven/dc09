use rustls::ServerConfig;
use rustls_pki_types::pem::PemObject;
use rustls_pki_types::{CertificateDer, PrivateKeyDer};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use thiserror::Error;
use tokio_rustls::TlsAcceptor;

#[derive(Debug, Error)]
pub enum TlsConfigError {
    #[error("failed to read TLS certificate '{path}'")]
    ReadCertificate {
        path: PathBuf,
        #[source]
        source: rustls_pki_types::pem::Error,
    },

    #[error("failed to read TLS private key '{path}'")]
    ReadPrivateKey {
        path: PathBuf,
        #[source]
        source: rustls_pki_types::pem::Error,
    },

    #[error("TLS certificate '{path}' does not contain any PEM certificates")]
    EmptyCertificateChain { path: PathBuf },

    #[error("failed to build TLS server configuration")]
    BuildServerConfig(#[from] rustls::Error),
}

pub fn build_tls_acceptor(cert_path: &Path, key_path: &Path) -> Result<TlsAcceptor, TlsConfigError> {
    let cert_chain = load_cert_chain(cert_path)?;
    let private_key = load_private_key(key_path)?;
    let config = ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(cert_chain, private_key)?;

    Ok(TlsAcceptor::from(Arc::new(config)))
}

fn load_cert_chain(cert_path: &Path) -> Result<Vec<CertificateDer<'static>>, TlsConfigError> {
    let certs = CertificateDer::pem_file_iter(cert_path)
        .map_err(|source| TlsConfigError::ReadCertificate {
            path: cert_path.to_path_buf(),
            source,
        })?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|source| TlsConfigError::ReadCertificate {
            path: cert_path.to_path_buf(),
            source,
        })?;

    if certs.is_empty() {
        return Err(TlsConfigError::EmptyCertificateChain {
            path: cert_path.to_path_buf(),
        });
    }

    Ok(certs)
}

fn load_private_key(key_path: &Path) -> Result<PrivateKeyDer<'static>, TlsConfigError> {
    PrivateKeyDer::from_pem_file(key_path).map_err(|source| TlsConfigError::ReadPrivateKey {
        path: key_path.to_path_buf(),
        source,
    })
}
