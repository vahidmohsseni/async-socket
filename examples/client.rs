extern crate async_socket;

use std::{io, path::PathBuf, time::Duration};

use async_socket::connect::Client;
use bytes::BytesMut;
use tokio::{select, sync::mpsc};

#[tokio::main]
async fn main() -> io::Result<()> {

    // Create a logger instance with a specific configuration
    let config = simplelog::Config::default();
    let level = simplelog::LevelFilter::Debug;
    let logger = simplelog::SimpleLogger::new(level, config);

    // Set the logger as the default for the application
    log::set_boxed_logger(Box::new(logger)).unwrap();
    log::set_max_level(level);

    log::info!("Hello from client!");
    let host_address = "127.0.0.1".to_string();
    let host_port = 5000;

    let cert_file = Some(PathBuf::from("keys/rootCA.crt"));
    // let cert_file = None;
    let client = Client::from_args(host_address, host_port, cert_file);

    let (tx, mut rx) = mpsc::channel(2);

    let connect_loop = tokio::spawn(client.run_client(tx));

    let (mut recv, mut send) = rx.recv().await.unwrap();

    let mut retrying = false;
    loop {

        select! {

            _ = tokio::time::sleep(std::time::Duration::from_secs(1)), if retrying => {

                (recv, send) = rx.recv().await.unwrap();
                retrying = false;
            
            }

            _ = tokio::time::sleep(std::time::Duration::from_secs(2)), if !retrying => {

                match recv.try_recv(){
                    Ok(d) => log::info!("data: {:?}", d),
                    Err(e) => log::info!("error in cl: {:?}", e),
                }
                if send.send(BytesMut::from("Got it working!")).await.is_err() {
                    retrying = true;
                }
            }
        }

        // break;
    }


    // loop {
    //     select! {
    //         _ = tokio::time::sleep(Duration::from_secs(1)), if connect.is_err() => {
    //             // do something after waiting for 1 second
    //         }
    //     }
    // }
    // Ok(())
}
