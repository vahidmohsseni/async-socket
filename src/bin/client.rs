extern crate async_socket;

use std::{io, path::PathBuf, time::Duration};

use async_socket::connect::Client;
use tokio::{select, sync::mpsc};

#[tokio::main]
async fn main() -> io::Result<()> {
    println!("Hello from client!");
    let host_address = "127.0.0.1".to_string();
    let host_port = 5000;

    let cert_file = Some(PathBuf::from("keys/rootCA.crt"));
    // let cert_file = None;
    let client = Client::from_args(host_address, host_port, cert_file);

    let (tx, rx) = mpsc::channel(2);

    let connect = client.run_client(tx).await;

    loop {
        select! {
            _ = tokio::time::sleep(Duration::from_secs(1)), if connect.is_err() => {
                // do something after waiting for 1 second
            }
        }
    }
    // Ok(())
}
