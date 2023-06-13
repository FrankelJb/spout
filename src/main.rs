//! A proxy that forwards data to another server and forwards that server's
//! responses back to clients.
//!
//! Because the Tokio runtime uses a thread pool, each TCP connection is
//! processed concurrently with all other TCP connections across multiple
//! threads.
//!
//! You can showcase this by running this in one terminal:
//!
//!     cargo run --example proxy
//!
//! This in another terminal
//!
//!     cargo run --example echo
//!
//! And finally this in another terminal
//!
//!     cargo run --example connect 127.0.0.1:8081
//!
//! This final terminal will connect to our proxy, which will in turn connect to
//! the echo server, and you'll be able to see data flowing, between them.

#![warn(rust_2018_idioms)]

use clap::Parser;
use futures::future::try_join_all;
use tokio::io;
use tokio::io::AsyncWriteExt;
use tokio::net::{TcpListener, TcpStream};

use futures::FutureExt;
use std::error::Error;

use crate::args::Args;

mod args;

#[derive(Debug)]
struct TcpPair {
    local_listener: String,
    remote_stream: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();
    let local_host = args.local_host;
    let remote_host = args.remote_host;
    let tcp_pairs: Vec<TcpPair> = args
        .ports
        .iter()
        .map(|port| TcpPair {
            local_listener: host_port(&local_host, port),
            remote_stream: host_port(&remote_host, port),
        })
        .collect();

    // for tcp_pair in tcp_pairs {
    let connection = tcp_pairs
        .into_iter()
        .map(|tcp_pair| {
            println!("tcp_pair {:?}", tcp_pair);
            let run_pair = run_proxy_for_pair(tcp_pair).map(|r| {
                if let Err(e) = r {
                    println!("Failed to run_pair; error={}", e);
                }
            });
            tokio::spawn(run_pair)
        })
        .collect::<Vec<_>>();

    try_join_all(connection).await?;

    Ok(())
}

async fn run_proxy_for_pair(tcp_pair: TcpPair) -> Result<(), Box<dyn Error>> {
    println!("trying to connect: {:?}", tcp_pair);
    let local_listener = tcp_pair.local_listener;
    let listener = TcpListener::bind(local_listener.clone()).await?;
    println!("listener bound: {}", local_listener);

    while let Ok((inbound, _)) = listener.accept().await {
        println!("peer address: {}", inbound.peer_addr()?);
        let transfer = transfer(inbound, tcp_pair.remote_stream.clone()).map(|r| {
            if let Err(e) = r {
                println!("Failed to transfer; error={}", e);
            }
        });

        tokio::spawn(transfer);
    }

    Ok(())
}

async fn transfer(mut inbound: TcpStream, proxy_addr: String) -> Result<(), Box<dyn Error>> {
    let mut outbound = TcpStream::connect(proxy_addr).await?;

    let (mut ri, mut wi) = inbound.split();
    let (mut ro, mut wo) = outbound.split();

    let client_to_server = async {
        io::copy(&mut ri, &mut wo).await?;
        wo.shutdown().await
    };

    let server_to_client = async {
        io::copy(&mut ro, &mut wi).await?;
        wi.shutdown().await
    };

    tokio::try_join!(client_to_server, server_to_client)?;

    Ok(())
}

fn host_port<'a>(host: &'a str, port: &u16) -> String {
    format!("{}:{}", host, port)
}
