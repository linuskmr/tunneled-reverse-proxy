//! Proxies the connections from clients to the data_server

use std::error::Error;

use anyhow::Context;
use clap::Parser;
use tokio::{
	io::{self, AsyncWriteExt, BufWriter},
	net::TcpListener,
	sync::mpsc,
};

#[derive(clap::Parser, Debug, Clone)]
struct Cli {
	/// Address of the control channel of this remote proxy, e.g. `localhost:5533`.
	#[arg(long)]
	control_addr: String,
	#[arg(long)]
	outside_addr: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
	env_logger::init();
	let cli_args = Cli::parse();

	let control_socket = TcpListener::bind(&cli_args.control_addr).await?;
	log::info!("Listening on control channel {}", &cli_args.control_addr);

	// An origin proxy has connected
	let (mut origin_proxy, origin_proxy_addr) = control_socket.accept().await.context("Accepting origin proxy")?;
	log::info!("Origin proxy connected from {:?}", origin_proxy_addr);

	let (_origin_proxy_reader, origin_proxy_writer) = origin_proxy.split();
	let mut origin_proxy_writer = BufWriter::new(origin_proxy_writer);

	// Accept connections from the origin proxy and send them to the other thread
	let (origin_proxy_client_send_channel, mut origin_proxy_client_recv_channel) = mpsc::channel(1);
	tokio::spawn(async move {
		loop {
			let (origin_proxy_client, _) = control_socket.accept().await.context("Accepting inside socket").unwrap();
			origin_proxy_client_send_channel
				.send(origin_proxy_client)
				.await
				.context("Send inside socket to other thread")
				.unwrap();
		}
	});

	let remote_client = TcpListener::bind(&cli_args.outside_addr)
		.await
		.context(format!("Listening on {} for remote clients", &cli_args.outside_addr))?;
	log::info!("Listening on outside addr: {}", cli_args.outside_addr);

	loop {
		let (mut remote_client, _) = remote_client.accept().await.context("Accepting remote client")?;
		log::info!("Remote client connected from {:?}", remote_client.peer_addr()?);

		// Send connection address to server. This indicates that a new client wants to connect.
		// The origin proxy will thus now create a new separate TCP connection to this program (the remote proxy).
		origin_proxy_writer
			.write_all(format!("{:?}\n", remote_client.peer_addr()?).as_bytes())
			.await
			.context("Sending outside client addr to server")?;
		origin_proxy_writer.flush().await.context("Sending outside client addr to server")?;

		let mut origin_proxy_client =
			origin_proxy_client_recv_channel.recv().await.ok_or(anyhow::anyhow!("No origin proxy client available"))?;
		log::info!("Origin proxy client connected from {:?}", origin_proxy_client.peer_addr()?);

		tokio::spawn(async move {
			log::debug!("Copying {:?} - {:?}", remote_client.peer_addr()?, origin_proxy_client.peer_addr()?);
			io::copy_bidirectional(&mut origin_proxy_client, &mut remote_client)
				.await
				.context("Copying between remote client and origin proxy client")
		});
	}
}
