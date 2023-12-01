//! a small ping clone, sending a ping to a peer, expecting a pong as a response.
//!
//! reference: https://docs.rs/libp2p/latest/libp2p/tutorials/ping/index.html
use futures::prelude::*;
use libp2p::swarm::SwarmEvent;
use libp2p::{ping, tcp, tls, yamux, Multiaddr};
use std::error::Error;
use std::time::Duration;
use tracing::level_filters::LevelFilter;
use tracing_subscriber::EnvFilter;

// https://docs.libp2p.io/concepts/appendix/glossary/ can be handy to read along with this code
//
// async_std provides an asynchronous runtime similat to Tokio, but with less features
#[async_std::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // it is an utility the for implementing and composing tracing subscribers
    // in this case traces are filtered using the `RUST_LOG` environment variable
    tracing_subscriber::fmt()
        // The added directive will be used in addition to any previously set
        // directives, either added using this method or provided when the filter is constructed.
        .with_env_filter(EnvFilter::from_default_env().add_directive(LevelFilter::INFO.into()))
        .init();

    tracing::info!("Starting ping program...");

    // Called also "switch", see documentation https://docs.libp2p.io/concepts/multiplex/switch
    // and also `libp2p::swarm` docs. The swarm contains the state of the network as a whole
    //
    // with_new_identity creates a new identity for the
    // local node generating a peer id
    let mut swarm = libp2p::SwarmBuilder::with_new_identity()
        .with_async_std()
        // Next up we need to construct a transport. Each transport in libp2p provides encrypted streams.
        // E.g. combining TCP to establish connections, TLS to encrypt these connections and Yamux
        // to run one or more streams on a connection. Another libp2p transport is QUIC,
        // providing encrypted streams out-of-the-box. We will stick to TCP for now.
        // Each of these implement the Transport trait.
        .with_tcp(
            tcp::Config::default(),
            tls::Config::new,
            // the multiplexer protocol used for the tcp connection
            yamux::Config::default,
        )?
        // a `NetworkBehaviour` defines what bytes and to whom to send on the network.
        .with_behaviour(|_| ping::Behaviour::default())?
        // Allows us to observe pings for 30 seconds. How long to keep a connection alive once it is idling
        .with_swarm_config(|cfg| cfg.with_idle_connection_timeout(Duration::from_secs(30)))
        .build();

    // this can be seen with RUST_LOG=debug set
    tracing::debug!("Built libp2p swarm/switch");

    // Tell the swarm to listen on all interfaces and a random, OS-assigned port.
    //
    // the P2P network node will bind and listen for incoming connections on
    // any available network interface on the machine where the code is running
    //
    // * Using 0.0.0.0 as the address means the node will listen on all
    //   available IPv4 addresses assigned to the machine (my laptop).
    //   It's a wildcard that indicates any IP address.
    // * Using 0 as a port means that is randomly assigned by the OS.
    swarm.listen_on("/ip4/0.0.0.0/tcp/0".parse()?)?;

    // Dial/Connect to the peer identified by the multi-address
    // given as the second command-line argument, if any.
    if let Some(addr) = std::env::args().nth(1) {
        let remote: Multiaddr = addr.parse()?;
        swarm.dial(remote)?;
        println!("Dialed {addr}")
    }

    loop {
        // Returns a `Future` that resolves when the next item
        // in this (TCP in this example) stream is ready.
        match swarm.select_next_some().await {
            SwarmEvent::NewListenAddr { address, .. } => tracing::info!("Listening on {address:?}"),
            SwarmEvent::ConnectionEstablished {
                peer_id,
                connection_id,
                endpoint,
                num_established,
                concurrent_dial_errors,
                established_in,
            } => {
                tracing::info!("Connection established: {peer_id:?}, {connection_id:?}, {endpoint:?}, {num_established:?}, {concurrent_dial_errors:?}, {established_in:?}")
            }
            // Once we dial the peer we'll see two events:
            // * there has been a TCP connection to the peer, from another peer (ping)
            // * the local node answers to the other peer (pong)
            SwarmEvent::Behaviour(event) => tracing::info!("{event:?}"),
            SwarmEvent::ConnectionClosed {
                peer_id,
                connection_id,
                endpoint,
                num_established,
                cause,
            } => {
                tracing::info!("Connection closed: {peer_id:?}, {connection_id:?}, {endpoint:?}, {num_established:?}, {cause:?}")
            }
            _ => {}
        }
    }
}
