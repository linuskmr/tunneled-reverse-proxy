//! Running on the computer that should be reachable from outside

use std::net::{IpAddr, SocketAddr};
use std::{error};
use std::error::Error;
use anyhow::Context;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;
use tokio::{
	select,
	io,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn error::Error>> {
	env_logger::init();

	let proxy_control_addr = "0.0.0.0:5533";
	let preferred_outside_port: u16 = 3344;
	let local_server_addr = "127.0.0.1:5000"; // The port of the local server that should be proxied

	let mut proxy = TcpStream::connect(&proxy_control_addr).await.context(format!("Connect to proxy at {}", proxy_control_addr))?;
	proxy.write_all(format!("{}\n", preferred_outside_port).as_bytes()).await.context(format!("Writing preferred outside port {}", preferred_outside_port))?;
	let mut proxy = BufReader::new(proxy);

	loop {
		log::debug!("Waiting for proxy server to announce that a new client wants to connect");
		let mut proxied_client_addr = String::new();
		proxy.read_line(&mut proxied_client_addr).await.context("Read next proxied client addr from proxy")?;
		let is_eof = proxied_client_addr.is_empty();
		if is_eof {
			log::error!("Proxy server closed connection");
			break;
		}
		let outside_client_addr = proxied_client_addr.strip_suffix('\n').context("Illegal proxied client addr supplied by proxy").unwrap();
		log::info!("Connection request to {} forwarded from proxy", outside_client_addr);

		tokio::spawn(async move {
			log::info!("Creating separate socket to proxy server to handle the new client");
			let mut proxied_client = TcpStream::connect(&proxy_control_addr).await.context("Creating connection to proxy for proxied client")?;
			let mut local_server = TcpStream::connect(local_server_addr).await.context(format!("Creating connection to local server at {}", local_server_addr))?;

			log::debug!("Copying {:?} - {:?}", proxied_client.peer_addr()?, local_server.peer_addr()?);
			io::copy_bidirectional(&mut local_server, &mut proxied_client).await.context("Copying between proxied client and local server")
		});
	}
	Ok(())
}