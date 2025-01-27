# tunneled-reverse-proxy

Exposes a TCP server application running on a computer not exposed to the internet to the internet via a remote proxy. This was inspired by [Cloudflare Tunnel](https://developers.cloudflare.com/cloudflare-one/connections/connect-networks/) and [ngrok](https://ngrok.com).


## Motivation

Let's say you have a TCP server application running on a local computer in a local network, so it is not accessible from the internet. You now want to show your newly developed TCP server application to a colleague or friend who is not in the local network.
You don't want to give them access to your local network via some sort of VPN because this also exposes other ports and devices in your local network.
You also don't want to send the application to them nor to a remote server like a VPS because the setup might be complicated or involves sensitive data.
You also cannot expose the application to the internet via port forwarding because this is not supported by your router or you are not the admin of the router. Furthermore, you don't want to leak your IP address.

This is where `tunneled-reverse-proxy` comes in. It is a reverse proxy that tunnels the TCP server application running on the local computer to the open internet.


## Related Work

This project was inspired by [Cloudflare Tunnel](https://developers.cloudflare.com/cloudflare-one/connections/connect-networks/) and [ngrok](https://ngrok.com), which offer a similar service. An open source project is [bore](https://github.com/ekzhang/bore). However, I wanted to implement my own solution to understand the concept and have less dependencies on external service providers/maintainers.


## Approach

First, the TCP server application running in the local network, called **origin server**, is started. Then, the **remote proxy** software is started on a internet-accessible server (e.g. a VPS). Then, the **origin proxy** software is started on a computer in the local network. It then establishes a control connection to the remote proxy. This is not blocked by the firewall/NAT because is in outbound connection from the local network to the internet. The remote proxy then starts to listen for incoming connections from **remote clients** on the internet. When a remote client connects to the remote proxy, the remote proxy uses the control connection to the origin proxy to inform it that a new remote client connected, sends the remote client's IP address and port to the origin proxy, and asks the origin proxy to open a new connection to the origin server. As this is also an outbound connection, it again conforms with the NAT/firewall. The remote proxy then forwards the data between the remote client and the origin proxy while the origin proxy forwards the data between the remote proxy and the origin server.


## Setup

On both computers (the remote proxy server as well as the local computer), clone this repository and [install Rust](https://rustup.rs).

On the remote, internet-accessible server (e.g. a VPS), start the remote proxy software:

```bash
$ cargo run --release --bin remote-proxy -- --control-addr 0.0.0.0:9001 --outside-addr 0.0.0.0:9000
```

On the local computer, run some origin server application:

```bash
$ python3 -m http.server 8000
```

Then, start the origin proxy software:

```bash
cargo run --release --bin origin-proxy -- --remote-proxy-addr example.com:9001 --origin-server-addr localhost:8000
```

Now visit `http://example.com:9000` in a browser. The HTTP server running on the local computer should be accessible.

You can now also use a normal reverse proxy like Caddy or nginx on your remote server to add HTTPS support, e.g. by proxying `https://tunnel.example.com` to `http://localhost:9000`.