
pub enum DataServerToProxy {
    /// Data Server -> Proxy; control channel.
    /// The port the data server wants the proxy to listen on for clients.
    PreferredPort(u16),
}

pub enum ProxyToDataServer {
    /// Proxy -> Data Server; control channel.
    /// The data server should create a connection to the proxy on specified port.
    ConnectTo(u16),
}