//! The sockets the server takes its connections on.

use std::io;
use std::net::{Ipv4Addr, Ipv6Addr, SocketAddr};

use futures::future::select_all;
use tokio::net::{TcpListener, TcpStream};

/// The machines the server takes connections from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reach {
    /// Any machine: a client on another PC can reach the server.
    Everywhere,
    /// This machine only. Windows asks the user to let a program through the firewall when it
    /// listens for other machines; it does not ask for this. A throw-away server has a new
    /// program path in each worktree, so it would ask again and again.
    ThisMachine,
}

/// The sockets of the server, listened to as one.
pub struct Listeners(Vec<TcpListener>);

impl Listeners {
    /// The addresses the server listens on.
    pub fn local_addrs(&self) -> io::Result<Vec<SocketAddr>> {
        self.0.iter().map(TcpListener::local_addr).collect()
    }
}

impl From<TcpListener> for Listeners {
    fn from(listener: TcpListener) -> Listeners {
        Listeners(vec![listener])
    }
}

/// Listens on the port of this machine over both IPv6 and IPv4. Port 0 takes a free port, the
/// same for both. The listening for every machine is in main.rs: no test can run it (see the
/// tests below).
///
/// On Windows, `localhost` is `::1` first. A client then waits about 2s for a refused IPv6
/// connect before it tries IPv4, each time it opens a new connection.
pub async fn this_machine(port: u16) -> io::Result<Listeners> {
    // `::1` is not `127.0.0.1` mapped to IPv6, so one socket cannot take both
    let v4 = TcpListener::bind((Ipv4Addr::LOCALHOST, port)).await?;
    let port = v4.local_addr()?.port();
    match TcpListener::bind((Ipv6Addr::LOCALHOST, port)).await {
        Ok(v6) => Ok(Listeners(vec![v6, v4])),
        Err(e) => {
            // A machine with IPv6 turned off must still run the server
            tracing::warn!("IPv6 is not available, listening on IPv4 only: {e}");
            Ok(Listeners(vec![v4]))
        }
    }
}

impl axum::serve::Listener for Listeners {
    type Io = TcpStream;
    type Addr = SocketAddr;

    async fn accept(&mut self) -> (TcpStream, SocketAddr) {
        // The accept of axum retries after an error. Dropping the others is safe: a connection
        // stays in the queue of its socket until the next call
        let accepts = self
            .0
            .iter_mut()
            .map(|listener| Box::pin(axum::serve::Listener::accept(listener)));
        select_all(accepts).await.0
    }

    fn local_addr(&self) -> io::Result<SocketAddr> {
        self.0[0].local_addr()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::serve::Listener;

    /// Connects to the address and checks that the listeners take the connection.
    async fn taken(listeners: &mut Listeners, address: SocketAddr) -> bool {
        let connect = TcpStream::connect(address);
        let accept = tokio::time::timeout(std::time::Duration::from_secs(5), listeners.accept());
        let (connected, accepted) = tokio::join!(connect, accept);
        connected.is_ok() && accepted.is_ok()
    }

    // この機械だけから受けるサーバーは、IPv6とIPv4のループバックのどちらからの接続も受ける。
    #[tokio::test]
    async fn a_server_for_this_machine_takes_connections_to_both_loopbacks() {
        let mut listeners = this_machine(0).await.unwrap();
        let port = listeners.local_addr().unwrap().port();

        assert!(taken(&mut listeners, (Ipv6Addr::LOCALHOST, port).into()).await);
        assert!(taken(&mut listeners, (Ipv4Addr::LOCALHOST, port).into()).await);
    }

    // この機械だけから受けるサーバーは、ループバックのアドレスでだけ待ち受ける。
    #[tokio::test]
    async fn a_server_for_this_machine_listens_on_the_loopbacks_only() {
        let listeners = this_machine(0).await.unwrap();

        let addresses = listeners.local_addrs().unwrap();
        assert_eq!(addresses.len(), 2);
        assert!(
            addresses.iter().all(|a| a.ip().is_loopback()),
            "{addresses:?}"
        );
    }

    // No test listens for other machines (`Reach::Everywhere`): Windows would ask the developer
    // to let each new test program through the firewall, and the dialog takes the front from
    // the tests of the screen.
}
