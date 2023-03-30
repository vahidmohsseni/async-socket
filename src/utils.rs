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

fn load_certs(path: &Path) -> io::Result<Vec<Certificate>> {
    certs(&mut BufReader::new(File::open(path)?))
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "invalid cert"))
        .map(|mut certs| certs.drain(..).map(Certificate).collect())
}

fn load_keys(path: &Path) -> io::Result<Vec<PrivateKey>> {
    rsa_private_keys(&mut BufReader::new(File::open(path)?))
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "invalid key"))
        .map(|mut keys| keys.drain(..).map(PrivateKey).collect())
}

pub async fn accpet_connection(
    address: SocketAddr,
    tls_enabled: bool,
    tls_key: Option<PathBuf>,
    tls_cert: Option<PathBuf>,
) -> io::Result<()> {
    println!("running server .... ... .....");
    let listener = TcpListener::bind(address).await?;
    
    if tls_enabled {
        let certs = load_certs(&tls_cert.expect("Error: A valid cert is required!"))?;

        let mut keys = load_keys(&tls_key.expect("Error: A valid key is required!"))?;

        println!("len keys: {:?} \n len certs: {:?}", keys, certs);

        let config = rustls::ServerConfig::builder()
            .with_safe_defaults()
            .with_no_client_auth()
            .with_single_cert(certs, keys.remove(0))
            .map_err(|err| io::Error::new(io::ErrorKind::InvalidInput, err))?;
        let acceptor = TlsAcceptor::from(Arc::new(config));
        println!("waiting ... ");

        loop {
            let (stream, address) = listener.accept().await?;
            println!("Accepting connection from: {}", address);
            let acceptor = acceptor.clone();

            let fut = async move {
                
                let mut stream = acceptor.accept(stream).await?;
                println!("TLS established!");
                let (mut reader, mut writer) = split(stream);

                let mut buffer = [0 as u8; 16];
                reader.read_exact(&mut buffer).await?;
                println!("Data: {:?}", buffer);
                writer.write(&buffer).await?;
                writer.flush().await?;

                Ok(()) as io::Result<()>
            };
        }
    }

    Ok(())
}

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

    let mut stream = connector.connect(domain, stream).await.unwrap_or_else(|e | {println!("error: {:?}", e); panic!("{}", e)} );

    println!("TLS is established!");

    let data = "Hello";

    stream.write_all(data.as_bytes()).await?;

    let mut buf = [0 as u8; 5];

    stream.read_exact(&mut buf).await?;

    println!("{:?}", String::from_utf8(buf.to_vec()));
    Ok(())
}
