#![allow(non_snake_case)]

mod error;

use clap::Parser;
use std::convert::Infallible;
use std::io::Read;
use std::net::SocketAddr;

use http_body_util::Full;
use hyper::body::Bytes;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{Request, Response, StatusCode};
use hyper_util::rt::TokioIo;
use tokio::net::TcpListener;
use base64::{decode, Engine};
use reqwest::Client;
use std::str;
use std::sync::Arc;
use base64::engine::general_purpose;
use hyper::client::conn::http2;
use rss::Channel;
use tokio::fs;
use url::Url;

use crate::error::AppError;

/// Simple HTTP server serving files with configurable port.
#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Config {
    /// Port to bind the server to
    #[arg(short = 'p', long = "port", default_value_t = 8080)]
    port: u16,
    #[arg(short = 'c', long = "cachedir", default_value = "")]
    cache_dir: String,
}



#[tokio::main]
async fn main() -> Result<(), AppError> {
    let config = Arc::new(Config::parse());

    let addr = SocketAddr::from(([127, 0, 0, 1], config.port));
    let listener = TcpListener::bind(addr).await?;

    loop {
        let config = config.clone();
        let (stream, _) = listener.accept().await?;
        let io = TokioIo::new(stream);
        tokio::task::spawn(async move {
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

    Ok(())
}

async fn handle_request(req: Request<hyper::body::Incoming>, config: Arc<Config>) -> Result<Response<Full<Bytes>>, Infallible> {
    let path = req.uri().path().trim_start_matches('/');

    match fs::read(&path).await {
        Ok(contents) => Ok(Response::new(Full::new(Bytes::from(contents)))),
        Err(_) => {
            match general_purpose::STANDARD.decode(path) {
                Ok(decoded_bytes) => {
                    let raw_url = match str::from_utf8(&decoded_bytes) {
                        Ok(url) => url,
                        Err(_) => return Ok(Response::builder()
                            .status(StatusCode::BAD_REQUEST)
                            .body(Full::new(Bytes::from("Invalid UTF-8 sequence in Base64 decoded URL")))
                            .unwrap())
                    };
                    match Url::parse(raw_url) {
                        Ok(url) => match qualify_rss(url).await {
                            Ok(qualified_rss) => Ok(Response::new(Full::new(Bytes::from(qualified_rss)))),
                            Err(error) => Ok(Response::builder()
                                .status(StatusCode::INTERNAL_SERVER_ERROR)
                                .body(Full::new(Bytes::from(format!("Could not qualify RSS: {}", error.to_string()))))
                                .unwrap())
                        },
                        Err(_) => Ok(Response::builder()
                            .status(StatusCode::BAD_REQUEST)
                            .body(Full::new(Bytes::from("Invalid URL in Base64 decoded path")))
                            .unwrap())
                    }
                },
                Err(_) => Ok(Response::builder()
                    .status(StatusCode::BAD_REQUEST)
                    .body(Full::new(Bytes::from("Failed to decode Base64 path")))
                    .unwrap())
            }
        }
    }
}

async fn qualify_rss(url: Url) -> Result<String, AppError> {
    let content = reqwest::get(url).await?.bytes().await?;
    let channel = Channel::read_from(&content[..])?;

    // for newsItem in channel.items {
    //
    // }

    Ok(channel.to_string())
}
