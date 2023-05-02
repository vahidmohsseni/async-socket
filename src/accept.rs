use std::net::SocketAddr;
use std::path::Path;
use std::sync::Arc;
use std::{io, path::PathBuf};
use tokio_rustls::{rustls, TlsAcceptor};

use tokio::{
    net::{TcpListener, TcpStream},
    select,
};

use crate::manager::control_loop;
use crate::utils::server_helper::ServerConfig;

pub struct Server {
    config: ServerConfig,
}

impl Server {
    pub fn from_conf_file(path: &Path) -> io::Result<Server> {
        Ok(Server {
            config: ServerConfig::from_json_file(path)?,
        })
    }

    pub fn from_args(
        host: String,
        port: u16,
        tls_enabled: bool,
        cert_file: PathBuf,
        key_file: PathBuf,
    ) -> io::Result<Server> {
        Ok(Server {
            config: ServerConfig::from_args(host, port, tls_enabled, cert_file, key_file),
        })
    }

    pub async fn run_server(self) -> io::Result<()> {
        let accept_fut = accpet_connection(&self.config);

        tokio::pin!(accept_fut);

        loop {
            select! {
                accept_result = &mut accept_fut => {
                    match accept_result {
                        Ok(result) => println!("This is not possible: {:?}", result),
                        Err(error) => {
                            match error.kind() {
                                io::ErrorKind::NotFound => todo!(),
                                io::ErrorKind::PermissionDenied => todo!(),
                                io::ErrorKind::ConnectionRefused => todo!(),
                                io::ErrorKind::ConnectionReset => todo!(),
                                io::ErrorKind::ConnectionAborted => todo!(),
                                io::ErrorKind::NotConnected => todo!(),
                                io::ErrorKind::AddrInUse => todo!(),
                                io::ErrorKind::AddrNotAvailable => todo!(),
                                io::ErrorKind::BrokenPipe => todo!(),
                                io::ErrorKind::AlreadyExists => todo!(),
                                io::ErrorKind::WouldBlock => todo!(),
                                io::ErrorKind::InvalidInput => todo!(),
                                io::ErrorKind::InvalidData => todo!(),
                                io::ErrorKind::TimedOut => todo!(),
                                io::ErrorKind::WriteZero => todo!(),
                                io::ErrorKind::Interrupted => todo!(),
                                io::ErrorKind::Unsupported => todo!(),
                                io::ErrorKind::UnexpectedEof => todo!(),
                                io::ErrorKind::OutOfMemory => todo!(),
                                io::ErrorKind::Other => todo!(),
                                _ => todo!(),
                            }
                        },
                    }
                }
            }
        }
    }
}

async fn accpet_connection(config: &ServerConfig) -> io::Result<()> {
    let address = config.get_address()?;
    let (tls_cert, tls_key) = config.load_cert_and_key()?;
    let tls_enabled = config.is_tls_enabled();

    println!("running server ............");
    let listener = TcpListener::bind(address).await?;

    if tls_enabled {
        let tls_config = rustls::ServerConfig::builder()
            .with_safe_defaults()
            .with_no_client_auth()
            .with_single_cert(tls_cert, tls_key)
            .map_err(|err| io::Error::new(io::ErrorKind::InvalidInput, err))?;
        let acceptor = TlsAcceptor::from(Arc::new(tls_config));
        println!("Waiting for a client... ");

        loop {
            let (stream, address) = listener.accept().await?;
            println!("Accepting connection from: {}", address);
            tokio::spawn(establish_connection(acceptor.clone(), stream, address));
        }
    }

    Ok(())
}

async fn establish_connection(
    acceptor: TlsAcceptor,
    stream: TcpStream,
    address: SocketAddr,
) -> io::Result<()> {
    let stream = acceptor.accept(stream).await?;
    println!("TLS established from address: {address}");

    // run a macro to handle
    // let a = manage!(reader, writer);

    control_loop(stream, false).await;

    Ok(())
}
