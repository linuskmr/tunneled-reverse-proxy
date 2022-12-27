//! Proxies the connections from clients to the data_server

use std::error::Error;
use std::net::{IpAddr, SocketAddr};
use tokio::net::{TcpListener, TcpStream};
use tokio::io::{AsyncBufReadExt, BufReader, BufWriter};
use tunneled_reverse_proxy::messages;
use std::str::FromStr;
use tokio::io::AsyncWriteExt;


#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
	env_logger::init();

	let controls_addr = SocketAddr::new(IpAddr::from_str("0.0.0.0")?, 5533);
	let controls = TcpListener::bind(controls_addr).await?;
	log::info!("Listening on control channel {}", controls_addr);
	loop {
		// A data server has connected
		let (control, _) = controls.accept().await?;
		log::info!("Handshake established with {:?}", control.peer_addr()?);
		tokio::spawn(async move {
			handle_control(control).await
		});
	}
}

async fn handle_control(control_channel: TcpStream) {
	if let Err(err) = handle_control_inner(control_channel).await {
		log::error!("{}", err);
	}
}

async fn handle_control_inner(mut control_channel: TcpStream) -> Result<(), Box<dyn Error>> {
	let (control_channel_reader, control_channel_writer) = control_channel.split();
	let mut control_channel_reader = BufReader::new(control_channel_reader);
	let mut control_channel_writer = BufWriter::new(control_channel_writer);

	// Data server sends us the port it want us to listen on and forward traffic to it
	log::debug!("Waiting for preferred port");
	let mut preferred_port = String::new();
	control_channel_reader.read_line(&mut preferred_port).await?;
	let messages::PreferredPort(preferred_port) = serde_json::from_str(&preferred_port).unwrap();
	log::debug!("Preferred port: {}", preferred_port);

	// Open a listener for clients to connect to
	let clients_addr = SocketAddr::new(IpAddr::from_str("0.0.0.0")?, preferred_port);
	let clients = TcpListener::bind(clients_addr).await?;
	log::info!("Listening on {} for clients", clients_addr);

	loop {
		let (client, _) = clients.accept().await?;
		log::info!("Client connection from {:?}. Asking server to create connection to me", client.peer_addr()?);

		// Send connection address to server. This indicates that a new client wants to connect.
		// The server will now create a new separate TCP connection to this proxy.
		let connect_to_port = 
		control_channel_writer.write_all(format!("{:?}\n", client.peer_addr()?).as_bytes()).await?;
		control_channel_writer.flush().await?;
	}
}