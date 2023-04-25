use std::io;
use std::sync::Arc;
use std::{net::SocketAddr, path::PathBuf};
use tokio_rustls::rustls;

use tokio::{
    io::split,
    io::{AsyncReadExt, AsyncWriteExt},
};

use tokio_rustls::TlsAcceptor;

use tokio::net::TcpListener;

use crate::utils::{load_certs, load_keys};

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
