//! Proxies the connections from clients to the data_server

use std::error::Error;
use std::net::{IpAddr, SocketAddr, Ipv6Addr};
use anyhow::Context;
use tokio::net::{TcpListener, TcpStream};
use tokio::io::{AsyncBufReadExt, BufReader, BufWriter, self};
use tunneled_reverse_proxy::messages;
use std::str::FromStr;
use tokio::io::AsyncWriteExt;
use tokio::sync::mpsc;


#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
	env_logger::init();

	let control_addr = SocketAddr::new(IpAddr::from_str("0.0.0.0")?, 5533);
	let inner_socket = TcpListener::bind(control_addr).await?;
	log::info!("Listening on control channel {}", control_addr);



	// A data server has connected
	let (mut control_channel, control_channel_addr) = inner_socket.accept().await.context("Accepting server")?;
	log::info!("Data server connected from {:?}", control_channel_addr);

	let (control_channel_reader, control_channel_writer) = control_channel.split();
	let mut control_channel_reader = BufReader::new(control_channel_reader);
	let mut control_channel_writer = BufWriter::new(control_channel_writer);

	// Data server sends us the port it wants us to listen on and forward traffic to it
	log::debug!("Waiting for the preferred outside port");
	let mut preferred_outside_port = String::new();
	control_channel_reader.read_line(&mut preferred_outside_port).await.context("Receiving preferred outside port from server")?;
	let preferred_outside_port = preferred_outside_port.trim().parse::<u16>().context(format!("Parsing preferred outside port from server {} as u16", preferred_outside_port))?;

	let (send_inside_sockets, mut recv_inside_sockets) = mpsc::channel(1);

	tokio::spawn(async move {
		loop {
			let (inside_socket, _) = inner_socket.accept().await.context("Accepting inside socket").unwrap();
			send_inside_sockets.send(inside_socket).await.context("Send inside socket to other thread").unwrap();
		}
	});

	log::debug!("Listening on outside port: {}", preferred_outside_port);
	let outside = TcpListener::bind(SocketAddr::new(IpAddr::V6(Ipv6Addr::UNSPECIFIED), preferred_outside_port)).await.context(format!("Open outside port {}", preferred_outside_port))?;
	loop {
		let (mut outside_client, _) = outside.accept().await.context("Accepting outside client")?;
		log::info!("Outside client connected from {:?}", outside_client.peer_addr()?);

		// Send connection address to server. This indicates that a new client wants to connect.
		// The server will now create a new separate TCP connection to this proxy.
		control_channel_writer.write_all(format!("{:?}\n", outside_client.peer_addr()?).as_bytes()).await.context("Sending outside client addr to server")?;
		control_channel_writer.flush().await.context("Sending outside client addr to server")?;
		
		let mut inside_server = recv_inside_sockets.recv().await.ok_or(anyhow::anyhow!("No inside socket available"))?;
		log::info!("Inside server connected from {:?}", inside_server.peer_addr()?);
		
		tokio::spawn(async move {
			log::debug!("Copying {:?} - {:?}", outside_client.peer_addr()?, inside_server.peer_addr()?);
			io::copy_bidirectional(&mut inside_server, &mut outside_client).await.context("Copy between outside client and server")
		});
	}
}
