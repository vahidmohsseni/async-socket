use tokio::{select, sync::mpsc};

extern crate async_socket;

use std::{io, path::Path};

use async_socket::accept::Server;

#[tokio::main]
async fn main() -> io::Result<()> {
    println!("Hello from server!");

    let path = Path::new("config.json");

    let server = Server::from_conf_file(path)?;

    let (tx, rx) = mpsc::channel(20);

    let serve_loop = server.run_server(tx).await;

    loop {
        select! {

            _ = tokio::time::sleep(std::time::Duration::from_secs(1)), if serve_loop.is_err() => {
                // do something after waiting for 1 second
            }
        }
    }

    // Ok(())
}
