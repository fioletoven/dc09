use rustls::client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier};
use rustls::{ClientConfig, DigitallySignedStruct, RootCertStore, ServerConfig, SignatureScheme};
use rustls_pki_types::pem::PemObject;
use rustls_pki_types::{CertificateDer, PrivateKeyDer, ServerName, UnixTime};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use thiserror::Error;
use tokio_rustls::{TlsAcceptor, TlsConnector};

/// Errors that can occur while configuring TLS.
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

/// Builds a TLS connector with optional certificate verification.
pub fn build_tls_connector(cert_path: Option<&Path>, insecure: bool) -> Result<Option<TlsConnector>, TlsConfigError> {
    if insecure {
        let config = ClientConfig::builder()
            .dangerous()
            .with_custom_certificate_verifier(Arc::new(NoCertificateVerification))
            .with_no_client_auth();
        Ok(Some(TlsConnector::from(Arc::new(config))))
    } else {
        if let Some(cert_path) = cert_path {
            let certs = load_cert_chain(cert_path)?;
            let mut roots = RootCertStore::empty();
            if roots.add_parsable_certificates(certs).0 == 0 {
                return Err(TlsConfigError::EmptyCertificateChain {
                    path: cert_path.to_path_buf(),
                });
            }
            let config = ClientConfig::builder().with_root_certificates(roots).with_no_client_auth();
            Ok(Some(TlsConnector::from(Arc::new(config))))
        } else {
            Ok(None)
        }
    }
}

/// Builds a TLS acceptor from the given certificate and private key paths.
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

#[derive(Debug)]
struct NoCertificateVerification;

impl ServerCertVerifier for NoCertificateVerification {
    fn verify_server_cert(
        &self,
        _end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _server_name: &ServerName<'_>,
        _ocsp_response: &[u8],
        _now: UnixTime,
    ) -> Result<ServerCertVerified, rustls::Error> {
        Ok(ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        _message: &[u8],
        _cert: &CertificateDer<'_>,
        _dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        Ok(HandshakeSignatureValid::assertion())
    }

    fn verify_tls13_signature(
        &self,
        _message: &[u8],
        _cert: &CertificateDer<'_>,
        _dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        Ok(HandshakeSignatureValid::assertion())
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        vec![
            SignatureScheme::ECDSA_NISTP256_SHA256,
            SignatureScheme::ECDSA_NISTP384_SHA384,
            SignatureScheme::ED25519,
            SignatureScheme::RSA_PSS_SHA256,
            SignatureScheme::RSA_PSS_SHA384,
            SignatureScheme::RSA_PSS_SHA512,
            SignatureScheme::RSA_PKCS1_SHA256,
            SignatureScheme::RSA_PKCS1_SHA384,
            SignatureScheme::RSA_PKCS1_SHA512,
        ]
    }
}
