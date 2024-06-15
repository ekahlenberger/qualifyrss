#![allow(non_snake_case)]

use clap::Parser;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper_util::rt::TokioIo;
use std::net::SocketAddr;
use std::str;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::signal;
use tokio::signal::unix::{signal, SignalKind};
use tokio::sync::oneshot;
use tokio::sync::oneshot::Sender;

mod error;
mod request_handling;
mod feed_handling;

use crate::error::AppError;
use crate::request_handling::handle_request;

/// Simple HTTP server serving files with configurable port.
#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Config {
    /// Port to bind the server to
    #[arg(short = 'p', long = "port", default_value_t = 8080)]
    port: u16,
}

async fn wait_for_shutdown_signals(shutdown_tx: Sender<()>) {
    println!("Waiting for shutdown signals...");

    let ctrl_c = async {
        signal::ctrl_c().await.expect("Failed to listen for SIGINT");
        println!("Received SIGINT");
    };

    let sigterm = async {
        let mut sigterm = signal(SignalKind::terminate()).expect("Failed to listen for SIGTERM");
        sigterm.recv().await;
        println!("Received SIGTERM");
    };

    // Wait for either SIGINT or SIGTERM
    tokio::select! {
        _ = ctrl_c => { println!("Ctrl+C pressed (SIGINT)"); },
        _ = sigterm => { println!("Terminate signal received (SIGTERM)"); },
    }

    println!("Sending shutdown signal...");
    // Send the shutdown signal
    let _ = shutdown_tx.send(());
}

#[tokio::main]
async fn main() -> Result<(), AppError> {
    let config = Arc::new(Config::parse());

    let addr = SocketAddr::from(([0, 0, 0, 0], config.port));

    let (shutdown_tx, mut shutdown_rx) = oneshot::channel::<()>();

    tokio::task::spawn(wait_for_shutdown_signals(shutdown_tx));

    println!("server is waiting for incoming connections on {}", addr);
    let listener = TcpListener::bind(addr).await?;

    loop {
        tokio::select! {
            _ = &mut shutdown_rx => {
                println!("Received shutdown signal. Shutting down...");
                break;
            }
            Ok((stream, _)) = listener.accept() => {
                let config = config.clone();
                println!("incoming connection accepted");
                let io = TokioIo::new(stream);
                tokio::task::spawn(async move {
                    println!("spawning connection worker");
                    // Finally, we bind the incoming connection to our `hello` service
                    if let Err(err) = http1::Builder::new()
                        // `service_fn` converts our function in a `Service`
                        .serve_connection(io, service_fn(move |req| handle_request(req, config.clone())))
                        .await
                    {
                        eprintln!("Error serving connection: {:?}", err);
                    }
                });
            }
        }
    }
    println!("Server has been gracefully shut down.");
    Ok(())
}
