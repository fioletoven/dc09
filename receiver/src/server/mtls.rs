use aws_lc_rs::digest::{SHA256, digest};
use std::cell::Cell;
use std::net::SocketAddr;
use tokio::net::TcpStream;
use tokio_rustls::rustls::pki_types::CertificateDer;
use tokio_rustls::server::TlsStream;
use tokio_rustls::{TlsAcceptor, rustls::ServerConnection};
use x509_cert::{Certificate, der::Decode};

use crate::metrics::ClientCertificate;

// We need to preserve certificate as rustls do not expose rejected certificates on the stream.
tokio::task_local! {
    static OBSERVED_CERTIFICATE: Cell<Option<ClientCertificate>>;
}

pub fn observe_client_certificate(cert: &CertificateDer<'_>) {
    let _ = OBSERVED_CERTIFICATE.try_with(|slot| slot.set(Some(certificate_details(cert))));
}

pub async fn accept(
    acceptor: TlsAcceptor,
    stream: TcpStream,
    addr: SocketAddr,
) -> Option<(TlsStream<TcpStream>, Option<ClientCertificate>)> {
    let (result, cert) = OBSERVED_CERTIFICATE
        .scope(Cell::new(None), async move {
            let result = acceptor.accept(stream).await;
            (result, OBSERVED_CERTIFICATE.with(Cell::take))
        })
        .await;

    match result {
        Ok(stream) => {
            // A resumed session may supply its peer certificate without calling the verifier.
            let cert = cert.or_else(|| get_certificate_details(stream.get_ref().1));

            if let Some(cert) = &cert {
                let subject = cert.subject.as_deref().unwrap_or("<unavailable>");
                log::info!(
                    "mTLS handshake succeeded for {addr}: subject={subject:?}, thumbprint={}",
                    cert.thumbprint
                );
            } else {
                log::info!("mTLS handshake succeeded for {addr}");
            }

            Some((stream, cert))
        },
        Err(error) => {
            if let Some(cert) = cert {
                let subject = cert.subject.as_deref().unwrap_or("<unavailable>");
                log::warn!(
                    "mTLS handshake failed for {addr}: {error}; subject={subject:?}, thumbprint={}",
                    cert.thumbprint
                );
            } else {
                log::warn!("mTLS handshake failed for {addr}: {error}");
            }

            None
        },
    }
}

fn get_certificate_details(stream: &ServerConnection) -> Option<ClientCertificate> {
    stream
        .peer_certificates()
        .and_then(|chain| chain.first())
        .map(certificate_details)
}

fn certificate_details(cert: &CertificateDer<'_>) -> ClientCertificate {
    ClientCertificate {
        subject: Certificate::from_der(cert.as_ref())
            .ok()
            .map(|cert| cert.tbs_certificate().subject().to_string()),
        thumbprint: hex::encode(digest(&SHA256, cert).as_ref()),
    }
}
