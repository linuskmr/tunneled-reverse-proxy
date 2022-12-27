//! Running on the computer that should be reachable from outside

use std::{error};
use std::error::Error;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;
use tokio::{
	select,
	io,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn error::Error>> {
	env_logger::init();

	// Create connection to proxy server, because proxy server is publicly reachable,
	// and the router allows *outgoing* connections to the proxy server, but not incoming.
	let mut proxy = TcpStream::connect("0.0.0.0:5533").await?;
	// Write port on which the proxy should listen for connections that it will forward to me
	log::debug!("Write preferred port");
	proxy.write_all("3344\n".as_bytes()).await?;

	let mut proxy = BufReader::new(proxy);

	loop {
		// Wait for proxy server to announce that a new client wants to connect
		log::debug!("Waiting for proxy server to announce that a new client wants to connect");
		let mut target_addr = String::new();
		proxy.read_line(&mut target_addr).await?;
		let target_addr = target_addr.strip_suffix('\n').unwrap();
		log::info!("Connection request to {} forwarded from proxy", target_addr);

		log::info!("Creating separate socket to proxy server to handle the new client");
		let data_source = TcpStream::connect(target_addr).await?;
		handle_client(data_source).await;
	}
}

async fn handle_client(target: TcpStream) {
	if let Err(err) = handle_client_inner(target).await {
		log::error!("{}", err);
	}
}

async fn handle_client_inner(mut target: TcpStream) -> Result<(), Box<dyn Error>> {
	let (mut target_read, mut target_write) = target.split();


	let mut outgoing = TcpStream::connect("0.0.0.0:80").await?;
	let (mut outgoing_read, mut outgoing_write) = outgoing.split();
	select! {
		_ = io::copy(&mut target_read, &mut outgoing_write) => (),
		_ = io::copy(&mut outgoing_read, &mut target_write) => (),
	};
	Ok(())
}