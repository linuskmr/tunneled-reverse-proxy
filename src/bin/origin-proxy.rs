//! Running on the computer that should be reachable from outside

use std::{error};
use anyhow::Context;
use clap::Parser;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::net::TcpStream;
use tokio::io;

#[derive(clap::Parser, Debug, Clone)]
struct Cli {
	/// Address of the remote proxy server controller, e.g. `example.com:5533`.
	#[arg(long)]
	remote_proxy_addr: String,
	/// Address of the local server that should be proxied, e.g. `localhost:8080`.
	#[arg(long)]
	origin_server_addr: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn error::Error>> {
	env_logger::init();
	let cli_args = Cli::parse();

	let remote_proxy_control_channel = TcpStream::connect(&cli_args.remote_proxy_addr).await.context(format!("Connecting to proxy at {}", cli_args.remote_proxy_addr))?;
	let mut remote_proxy_control_channel = BufReader::new(remote_proxy_control_channel);

	loop {
		log::debug!("Waiting for remote proxy server to announce that a new client wants to connect");

		let mut remote_client_addr = String::new();
		remote_proxy_control_channel.read_line(&mut remote_client_addr).await.context("Read next proxied client addr from proxy")?;
		let is_eof = remote_client_addr.is_empty();
		if is_eof {
			log::error!("Proxy server closed connection");
			break;
		}

		let remote_client_addr = remote_client_addr.strip_suffix('\n').context("Illegal proxied client addr supplied by proxy").unwrap();
		log::info!("Connection request from {} forwarded from remote proxy", remote_client_addr);

		tokio::spawn({
			let cli_args = cli_args.clone();
			async move {
				log::info!("Creating socket to remote proxy at {}", &cli_args.remote_proxy_addr);
				let mut remote_proxy = TcpStream::connect(&cli_args.remote_proxy_addr).await.context(format!("Creating connection to proxy for proxied client at {}", &cli_args.remote_proxy_addr))?;

				log::debug!("Creating socket to origin server at {}", &cli_args.origin_server_addr);
				let mut origin_server = TcpStream::connect(&cli_args.origin_server_addr).await.context(format!("Creating connection to local server at {}", &cli_args.origin_server_addr))?;

				log::debug!("Copying {:?} <-> {:?}", remote_proxy.peer_addr()?, origin_server.peer_addr()?);
				io::copy_bidirectional(&mut origin_server, &mut remote_proxy).await.context("Copying between proxied client and local server")
			}
		});
	}
	Ok(())
}