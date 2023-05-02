use std::io;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::net::TcpStream;
use tokio::select;
use tokio_rustls::rustls;
use tokio_rustls::TlsConnector;

use crate::manager::control_loop;
use crate::utils::client_helper::ClientConfig;
use crate::utils::Recovery;

pub struct Client {
    config: ClientConfig,
}

impl Client {
    pub fn from_args(host_address: String, host_port: u16, cert_file: Option<PathBuf>) -> Client {
        Client {
            config: ClientConfig::from_args(host_address, host_port, cert_file),
        }
    }

    pub async fn run_client(&self) -> io::Result<()> {
        let connect_fut = connect(&self.config);

        tokio::pin!(connect_fut);

        let mut recovery = Recovery::None;
        let mut last_error = None;

        let mut number_of_retries = 0;

        loop {
            select! {
                connect_result = &mut connect_fut, if recovery == Recovery::None => {
                    match connect_result {
                        Ok(res) => {
                            println!("res: {:?}", res);

                        },
                        Err(error) => {
                            println!("error: {error}");
                            println!("kind: {:?}", error.kind());
                            match error.kind() {
                                io::ErrorKind::NotFound => todo!(),
                                io::ErrorKind::PermissionDenied => todo!(),
                                io::ErrorKind::ConnectionRefused => {
                                    recovery = Recovery::Retry;
                                    if last_error != Some(io::ErrorKind::ConnectionRefused) {number_of_retries = 0};
                                    last_error = Some(io::ErrorKind::ConnectionRefused);
                                },
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
                                io::ErrorKind::UnexpectedEof => {
                                    recovery = Recovery::Retry;
                                    if last_error != Some(io::ErrorKind::UnexpectedEof) {number_of_retries = 0};
                                    last_error = Some(io::ErrorKind::UnexpectedEof);
                                },
                                io::ErrorKind::OutOfMemory => todo!(),
                                io::ErrorKind::Other => return Err(error),
                                _ => todo!(),
                            }
                        }
                    }
                }

                _ = async move {}, if recovery == Recovery::Retry => {
                    number_of_retries += 1;
                    println!("Retyting: {number_of_retries} ...");
                    connect_fut.  set(connect(&self.config));
                    tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                    recovery = Recovery::None;
                }
            }
        }
    }
}

async fn connect(config: &ClientConfig) -> io::Result<()> {
    println!("Connecting ...");

    let address = config.get_address()?;
    let root_cert_store = config.get_root_cert_store()?;

    tokio::time::sleep(std::time::Duration::from_secs(1)).await;

    // return Err(io::Error::new(io::ErrorKind::Other, "Deliberate error!"));

    // TODO: due to tls configuration, the domain name must be passed to the function
    // OR the server ip address must be seen in the signed certificate
    let domain = "127.0.0.1".to_string();

    let tls_config = rustls::ClientConfig::builder()
        .with_safe_defaults()
        .with_root_certificates(root_cert_store)
        .with_no_client_auth();

    let connector = TlsConnector::from(Arc::new(tls_config));

    let stream = TcpStream::connect(&address).await?;
    println!("tcp connection is ok");

    let domain = rustls::ServerName::try_from(domain.as_str())
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "invalid dnsname"))?;

    let stream = connector.connect(domain, stream).await?;

    println!("TLS is established!");

    // let (mut reader, mut writer) = split(stream);

    control_loop(stream, true).await?;

    Ok(())
}
