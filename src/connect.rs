use bytes::BytesMut;
use std::io;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::net::TcpStream;
use tokio::select;
use tokio::sync::mpsc;
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

    pub async fn run_client(
        self,
        send_back: mpsc::Sender<(mpsc::Receiver<BytesMut>, mpsc::Sender<BytesMut>)>,
    ) -> io::Result<()> {
        let connect_fut = connect(&self.config, send_back.clone());

        tokio::pin!(connect_fut);

        let mut recovery = Recovery::None;
        let mut last_error = None;

        let mut number_of_retries = 0;

        loop {
            select! {
                connect_result = &mut connect_fut, if recovery == Recovery::None => {
                    match connect_result {
                        Ok(res) => {
                            log::info!("Connect returned Ok!: {:?}", res);

                        },
                        Err(error) => {
                            log::warn!("IO Error: {:?}, kind: {:?}", error, error.kind());
                            match error.kind() {
                                io::ErrorKind::NotFound => todo!(),
                                io::ErrorKind::PermissionDenied => todo!(),
                                io::ErrorKind::ConnectionRefused => {
                                    recovery = Recovery::Retry;
                                    if last_error != Some(io::ErrorKind::ConnectionRefused) {number_of_retries = 0};
                                    last_error = Some(io::ErrorKind::ConnectionRefused);
                                },
                                io::ErrorKind::ConnectionReset => {
                                    recovery = Recovery::Retry;
                                    if last_error != Some(io::ErrorKind::ConnectionReset) {number_of_retries = 0};
                                    last_error = Some(io::ErrorKind::ConnectionReset);
                                },
                                io::ErrorKind::ConnectionAborted => todo!(),
                                io::ErrorKind::NotConnected => todo!(),
                                io::ErrorKind::AddrInUse => todo!(),
                                io::ErrorKind::AddrNotAvailable => todo!(),
                                io::ErrorKind::BrokenPipe => {
                                    recovery = Recovery::Retry;
                                    if last_error != Some(io::ErrorKind::BrokenPipe) {number_of_retries = 0};
                                    last_error = Some(io::ErrorKind::BrokenPipe);
                                },
                                io::ErrorKind::AlreadyExists => todo!(),
                                io::ErrorKind::WouldBlock => todo!(),
                                io::ErrorKind::InvalidInput => {
                                    return Err(error);
                                },
                                io::ErrorKind::InvalidData => todo!(),
                                io::ErrorKind::TimedOut => todo!(),
                                io::ErrorKind::WriteZero => todo!(),
                                io::ErrorKind::Interrupted => todo!(),
                                io::ErrorKind::Unsupported => todo!(),
                                io::ErrorKind::UnexpectedEof => {
                                    return Err(error);
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
                    log::warn!("Retyting: {number_of_retries} ...");
                    connect_fut.  set(connect(&self.config, send_back.clone()));
                    tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                    recovery = Recovery::None;
                }
            }
        }
    }
}

async fn connect(
    config: &ClientConfig,
    send_back: mpsc::Sender<(mpsc::Receiver<BytesMut>, mpsc::Sender<BytesMut>)>,
) -> io::Result<()> {
    log::info!("Connecting ...");

    let address = config.get_address()?;
    let root_cert_store = config.get_root_cert_store()?;

    tokio::time::sleep(std::time::Duration::from_secs(1)).await;

    // return Err(io::Error::new(io::ErrorKind::Other, "Deliberate error!"));

    // TODO: due to tls configuration, the domain name must be passed to the function
    // OR the server ip address must be seen in the signed certificate
    let domain = address.ip().to_string();

    let tls_config = rustls::ClientConfig::builder()
        .with_safe_defaults()
        .with_root_certificates(root_cert_store)
        .with_no_client_auth();

    let connector = TlsConnector::from(Arc::new(tls_config));

    let stream = TcpStream::connect(&address).await?;
    log::debug!("tcp connection is ok");

    let domain = rustls::ServerName::try_from(domain.as_str())
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "invalid dnsname"))?;

    let stream = connector.connect(domain, stream).await?;

    log::debug!("TLS is established!");

    // let (mut reader, mut writer) = split(stream);
    let (_t, r) = tokio::sync::oneshot::channel();
    control_loop(stream, true, send_back, r).await?;

    Ok(())
}
