
use bytes::BytesMut;
use tokio::{select, sync::mpsc};

extern crate async_socket;

use std::{io, path::Path};

use async_socket::accept::Server;

use async_socket::accept::NodeMsg;

#[tokio::main]
async fn main() -> io::Result<()> {

    // Create a logger instance with a specific configuration
    let config = simplelog::Config::default();
    let level = simplelog::LevelFilter::Debug;
    let logger = simplelog::SimpleLogger::new(level, config);

    // Set the logger as the default for the application
    log::set_boxed_logger(Box::new(logger)).unwrap();
    log::set_max_level(level);

    log::info!("Hello from server!");

    let path = Path::new("config.json");

    let server = Server::from_conf_file(path)?;

    let (tx, mut _rx) = mpsc::channel(20);

    let _serve_loop = tokio::spawn(server.run_server(tx));

    let mut senders = Vec::with_capacity(10);

    let mut send = 0;
    loop {
        select! {

            _ = tokio::time::sleep(std::time::Duration::from_secs(1)) => {
                // do something after waiting for 1 second
                let d = _rx.recv().await.unwrap();
                
                match d {
                    NodeMsg::Event(addr, data) => log::info!("addr {} sent: {:?}",addr,  data),
                    NodeMsg::Connected(addr) => log::info!("addr {addr} is connected!"),
                    NodeMsg::Disconnected(addr) => {
                        log::warn!("addr {addr} is disconnected!");
                        send -= 1;
                        senders.pop();
                    },
                    NodeMsg::Sender(_, sen) => {senders.push(sen); send += 1;},
                }

                if send > 0{
                    senders[0].send(BytesMut::from("Por roo gharine meshki!")).await.unwrap();
                }
            }
        }
    }

    // Ok(())
}
