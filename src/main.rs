#![allow(non_snake_case)]

use clap::Parser;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper_util::rt::TokioIo;
use std::net::SocketAddr;
use std::str;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::{mpsc, oneshot};

mod error;
mod request_handling;
mod feed_handling;
mod cache;
mod fluent;
mod shutdown_handling;

use crate::cache::cache_manager_task;
use crate::cache::cache_message::CacheMessage;
use crate::error::AppError;
use crate::request_handling::handle_request;
use crate::shutdown_handling::wait_for_shutdown_signals;

/// Simple HTTP server serving files with configurable port.
#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Config {
    /// Port to bind the server to
    #[arg(short = 'p', long = "port", default_value_t = 8080)]
    port: u16,
}

#[tokio::main]
async fn main() -> Result<(), AppError> {
    let config = Arc::new(Config::parse());

    let addr = SocketAddr::from(([0, 0, 0, 0], config.port));

    let (shutdown_sender, mut shutdown_receiver) = oneshot::channel::<()>();
    tokio::spawn(wait_for_shutdown_signals(shutdown_sender));

    let (cache_sender, cache_receiver) = mpsc::unbounded_channel::<CacheMessage>();
    tokio::spawn(cache_manager_task(cache_sender.clone(), cache_receiver));

    println!("server is waiting for incoming connections on {}", addr);
    let listener = TcpListener::bind(addr).await?;

    loop {
        tokio::select! {
            _ = &mut shutdown_receiver => {
                println!("Received shutdown signal. Shutting down...");
                break;
            }
            Ok((stream, _)) = listener.accept() => {
                let config = config.clone();
                let cache_sender = cache_sender.clone();
                println!("incoming connection accepted");
                let io = TokioIo::new(stream);
                tokio::spawn(async move {
                    println!("spawning connection worker");
                    // Finally, we bind the incoming connection to our `hello` service
                    if let Err(err) = http1::Builder::new()
                        // `service_fn` converts our function in a `Service`
                        .serve_connection(io, service_fn(move |req| handle_request(req, config.clone(), cache_sender.clone())))
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
