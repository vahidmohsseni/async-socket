use rustls_pemfile::{certs, read_one, Item};
use serde::Deserialize;
use std::fs::File;
use std::io::{self, BufReader};
use std::net::{SocketAddr, ToSocketAddrs};
use std::path::{Path, PathBuf};
use tokio_rustls::rustls::{Certificate, PrivateKey};

#[derive(Debug, Deserialize)]
pub(crate) struct ServerConfig {
    host: String,
    port: u16,
    tls_enabled: bool,
    cert_file: PathBuf,
    key_file: PathBuf,
}

impl ServerConfig {
    pub(crate) fn from_json_file(path: &Path) -> Result<ServerConfig, io::Error> {
        let file = File::open(path)?;
        let reader = BufReader::new(file);
        serde_json::from_reader(reader).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
    }

    pub(crate) fn from_args(
        host: String,
        port: u16,
        tls_enabled: bool,
        cert_file: PathBuf,
        key_file: PathBuf,
    ) -> ServerConfig {
        ServerConfig {
            host,
            port,
            tls_enabled,
            cert_file,
            key_file,
        }
    }

    pub(crate) fn is_tls_enabled(&self) -> bool {
        self.tls_enabled
    }

    pub(crate) fn load_cert_and_key(&self) -> io::Result<(Vec<Certificate>, PrivateKey)> {
        let mut cert_file = BufReader::new(File::open(&self.cert_file)?);
        let certs = certs(&mut cert_file)
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "invalid cert"))
            .map(|mut certs| certs.drain(..).map(Certificate).collect())?;
        let mut key_file = BufReader::new(File::open(&self.key_file)?);
        if let Item::PKCS8Key(key) = read_one(&mut key_file)?.ok_or(io::Error::new(
            io::ErrorKind::NotFound,
            "Key does not exist",
        ))? {
            return Ok((certs, PrivateKey(key)));
        }
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "The key must be formatted as PKCS8Key",
        ));
    }

    pub(crate) fn get_address(&self) -> io::Result<SocketAddr> {
        let addr = (self.host.as_str(), self.port);
        addr.to_socket_addrs()?.next().ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                "Unable to calculate the address",
            )
        })
    }
}
