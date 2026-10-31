use rustls::client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier};
use rustls::crypto::{CryptoProvider, verify_tls12_signature, verify_tls13_signature};
use rustls::server::danger::{ClientCertVerified, ClientCertVerifier};
use rustls::server::{VerifierBuilderError, WebPkiClientVerifier};
use rustls::{ClientConfig, DigitallySignedStruct, DistinguishedName, RootCertStore, ServerConfig, SignatureScheme};
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

    #[error("failed to build TLS client configuration")]
    BuildClientConfig(#[source] rustls::Error),

    #[error("failed to build TLS client certificate verifier")]
    BuildClientVerifier(#[from] VerifierBuilderError),
}

/// Client certificate verification mode used by the TLS server (mTLS).
#[derive(Debug, Clone, Default, PartialEq)]
pub enum ClientAuth {
    /// Clients are not asked for a certificate (plain server-side TLS).
    #[default]
    Disabled,

    /// Client certificates must chain to one of the default webpki root certificates.
    WebPkiRoots,

    /// Client certificates must chain to one of the certificates from the given PEM file.
    Certificate(PathBuf),

    /// Client certificate is required, but it is accepted without verification.
    Insecure,
}

/// Builds a TLS connector with optional certificate verification.
pub fn build_tls_connector(
    force_tls: bool,
    cert_path: Option<&Path>,
    client_identity: Option<(&Path, &Path)>,
    insecure: bool,
) -> Result<Option<TlsConnector>, TlsConfigError> {
    if !force_tls && cert_path.is_none() && client_identity.is_none() && !insecure {
        return Ok(None);
    }

    let builder = ClientConfig::builder();
    let builder = if insecure {
        builder
            .dangerous()
            .with_custom_certificate_verifier(Arc::new(NoCertificateVerification))
    } else {
        let mut roots = webpki_roots_store();
        if let Some(cert_path) = cert_path {
            add_certificates(&mut roots, cert_path)?;
        }
        builder.with_root_certificates(roots)
    };

    let config = match client_identity {
        Some((cert_path, key_path)) => builder
            .with_client_auth_cert(load_cert_chain(cert_path)?, load_private_key(key_path)?)
            .map_err(TlsConfigError::BuildClientConfig)?,
        None => builder.with_no_client_auth(),
    };

    Ok(Some(TlsConnector::from(Arc::new(config))))
}

/// Builds a TLS acceptor from the given certificate and private key paths.
pub fn build_tls_acceptor(cert_path: &Path, key_path: &Path, client_auth: &ClientAuth) -> Result<TlsAcceptor, TlsConfigError> {
    let cert_chain = load_cert_chain(cert_path)?;
    let private_key = load_private_key(key_path)?;
    let builder = ServerConfig::builder();
    let provider = Arc::clone(builder.crypto_provider());

    let builder = match client_auth {
        ClientAuth::Disabled => builder.with_no_client_auth(),
        ClientAuth::WebPkiRoots => builder.with_client_cert_verifier(
            WebPkiClientVerifier::builder_with_provider(Arc::new(webpki_roots_store()), provider).build()?,
        ),
        ClientAuth::Certificate(ca_path) => {
            let mut roots = RootCertStore::empty();
            add_certificates(&mut roots, ca_path)?;
            builder.with_client_cert_verifier(WebPkiClientVerifier::builder_with_provider(Arc::new(roots), provider).build()?)
        },
        ClientAuth::Insecure => builder.with_client_cert_verifier(Arc::new(AnyClientCertificate { provider })),
    };

    let config = builder.with_single_cert(cert_chain, private_key)?;

    Ok(TlsAcceptor::from(Arc::new(config)))
}

fn webpki_roots_store() -> RootCertStore {
    RootCertStore::from_iter(webpki_roots::TLS_SERVER_ROOTS.iter().cloned())
}

fn add_certificates(roots: &mut RootCertStore, cert_path: &Path) -> Result<(), TlsConfigError> {
    let certs = load_cert_chain(cert_path)?;
    if roots.add_parsable_certificates(certs).0 == 0 {
        return Err(TlsConfigError::EmptyCertificateChain {
            path: cert_path.to_path_buf(),
        });
    }

    Ok(())
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

#[derive(Debug)]
struct AnyClientCertificate {
    provider: Arc<CryptoProvider>,
}

impl ClientCertVerifier for AnyClientCertificate {
    fn root_hint_subjects(&self) -> &[DistinguishedName] {
        &[]
    }

    fn verify_client_cert(
        &self,
        _end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _now: UnixTime,
    ) -> Result<ClientCertVerified, rustls::Error> {
        Ok(ClientCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        verify_tls12_signature(message, cert, dss, &self.provider.signature_verification_algorithms)
    }

    fn verify_tls13_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        verify_tls13_signature(message, cert, dss, &self.provider.signature_verification_algorithms)
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        self.provider.signature_verification_algorithms.supported_schemes()
    }
}
