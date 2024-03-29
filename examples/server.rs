
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

    let mut senders:Vec<mpsc::Sender<BytesMut>> = Vec::with_capacity(10);
    let mut senders_disconnect = Vec::with_capacity(10);
    let mut nodes = Vec::with_capacity(10);
    let mut send = 0;
    let mut shutdown = Vec::with_capacity(10);
    loop {
        select! {

            _ = tokio::time::sleep(std::time::Duration::from_millis(100)) => {
                // do something after waiting for 1 second
                let d = _rx.recv().await.unwrap();
                
                match d {
                    NodeMsg::Event(addr, data) => { if data == BytesMut::from("bit") {continue;} log::info!("addr {} sent: {:?}",addr,  data);},
                    NodeMsg::Connected(addr) => log::info!("addr {addr} is connected!"),
                    NodeMsg::Disconnected(addr) => {
                        log::warn!("addr {addr} is disconnected!");
                        send -= 1;
                        let indx = nodes.iter().position(|&x| x == addr).unwrap();
                        nodes.remove(indx);
                        senders.remove(indx);
                        shutdown.remove(indx);
                    },
                    NodeMsg::Sender(addr, sen, close_tx) => {
                        log::info!("addr {addr} is added!");
                        senders_disconnect.push(close_tx);
                        shutdown.push(0);
                        senders.push(sen); nodes.push(addr); send += 1;
                    },
                    NodeMsg::MasterDisconnected(_) => {
                        
                    }
                }

                if send > 0{
                    for i in 0..send{
                        shutdown[i] += 1;
                        if senders[i].send(BytesMut::from("Dummy data!")).await.is_err(){
                            log::debug!("Error! address {:?}", nodes);
                        };
                        // shut down the node when 10 message is sent
                        if shutdown[i] > 10 {
                            // comment these lines if you do not want to close the channel
                            let t = senders_disconnect.remove(i);
                            t.send(()).unwrap();

                        }
                    }
                }
            },

        }
    }

    // Ok(())
}
