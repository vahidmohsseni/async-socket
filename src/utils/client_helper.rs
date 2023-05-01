use std::{
    fs::File,
    io::{self, BufReader},
    net::{SocketAddr, ToSocketAddrs},
    path::PathBuf,
};

use tokio_rustls::rustls::{self, OwnedTrustAnchor, RootCertStore};

pub struct ClientConfig {
    host_address: String,
    host_port: u16,
    cert_file: Option<PathBuf>,
}

impl ClientConfig {
    pub fn from_args(
        host_address: String,
        host_port: u16,
        cert_file: Option<PathBuf>,
    ) -> ClientConfig {
        ClientConfig {
            host_address,
            host_port,
            cert_file,
        }
    }

    pub fn get_address(&self) -> io::Result<SocketAddr> {
        let addr = (self.host_address.as_str(), self.host_port);
        addr.to_socket_addrs()?.next().ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                "Unable to calculate the address",
            )
        })
    }

    pub fn get_root_cert_store(&self) -> io::Result<RootCertStore> {
        let mut root_cert_store = rustls::RootCertStore::empty();
        if let Some(cafile) = &self.cert_file {
            let mut pem = BufReader::new(File::open(cafile)?);
            let certs = rustls_pemfile::certs(&mut pem)?;
            let trust_anchors = certs.iter().map(|cert| {
                let ta = webpki::TrustAnchor::try_from_cert_der(&cert[..]).unwrap();
                OwnedTrustAnchor::from_subject_spki_name_constraints(
                    ta.subject,
                    ta.spki,
                    ta.name_constraints,
                )
            });
            root_cert_store.add_server_trust_anchors(trust_anchors);
        } else {
            root_cert_store.add_server_trust_anchors(webpki_roots::TLS_SERVER_ROOTS.0.iter().map(
                |ta| {
                    OwnedTrustAnchor::from_subject_spki_name_constraints(
                        ta.subject,
                        ta.spki,
                        ta.name_constraints,
                    )
                },
            ));
        }
        Ok(root_cert_store)
    }
}
