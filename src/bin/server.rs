extern crate async_socket;

use std::{net::ToSocketAddrs, path::{PathBuf, Path}};

use async_socket::accept::accpet_connection;

fn main() -> () {
    println!("Hello from server");

    accpet_connection("0.0.0.0:5000".to_socket_addrs().unwrap().next(),
        true, 
        Path::new("keys/key.pem"), 
        Path::new("keys/cert.pem"));
    
    return;
}
