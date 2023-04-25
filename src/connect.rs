use std::fs::File;
use std::io::{self, BufReader};
use std::path::Path;
use std::sync::Arc;
use std::{net::SocketAddr, path::PathBuf};
use tokio_rustls::rustls::{self, OwnedTrustAnchor};

use rustls_pemfile::{certs, rsa_private_keys};
use tokio::{
    io::split,
    io::{AsyncReadExt, AsyncWriteExt},
};
use tokio_rustls::rustls::{Certificate, PrivateKey};
use tokio_rustls::{webpki, TlsAcceptor, TlsConnector};

use tokio::net::{TcpListener, TcpStream};

pub async fn connect(address: SocketAddr, tls_cert: Option<PathBuf>) -> io::Result<()> {
    let cafile = tls_cert;

    println!("connecting ...");

    let mut root_cert_store = rustls::RootCertStore::empty();
    if let Some(cafile) = cafile {
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
        println!("ta: {:?}", trust_anchors);
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

    println!("TA: {:?}", root_cert_store);

    let config = rustls::ClientConfig::builder()
        .with_safe_defaults()
        .with_root_certificates(root_cert_store)
        .with_no_client_auth();

    let connector = TlsConnector::from(Arc::new(config));

    let stream = TcpStream::connect(&address).await?;
    println!("tcp connection is ok");

    let domain = rustls::ServerName::try_from(&*address.ip().to_string())
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "invalid dnsname"))?;

    println!("{:?}", domain);

    let mut stream = connector.connect(domain, stream).await.unwrap_or_else(|e| {
        println!("error: {:?}", e);
        panic!("{}", e)
    });

    println!("TLS is established!");

    let data = "Hello";

    stream.write_all(data.as_bytes()).await?;

    let mut buf = [0 as u8; 5];

    stream.read_exact(&mut buf).await?;

    println!("{:?}", String::from_utf8(buf.to_vec()));
    Ok(())
}
