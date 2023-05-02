
use tokio::{select, sync::mpsc};

extern crate async_socket;

use std::{io, path::Path};

use async_socket::accept::Server;

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

    let (tx, rx) = mpsc::channel(20);

    let serve_loop = tokio::spawn(server.run_server(tx));


    loop {
        select! {

            _ = tokio::time::sleep(std::time::Duration::from_secs(1)) => {
                // do something after waiting for 1 second
            }
        }
    }

    // Ok(())
}
