#![allow(non_snake_case)]

use article_scraper::ArticleScraper;
use base64::{Engine};
use base64::engine::general_purpose;
use clap::Parser;
use http_body_util::Full;
use hyper::body::Bytes;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{Request, Response, StatusCode};
use hyper_util::rt::TokioIo;
use reqwest::Client;
use rss::{Channel, Item};
use std::convert::Infallible;
use std::net::SocketAddr;
use std::str;
use std::sync::{Arc};
use tokio::net::TcpListener;
use url::Url;
use feed_rs;
use feed_rs::model::Feed;
use feed_rs::parser;
use tokio::{signal};
use tokio::signal::unix::{signal, SignalKind};
use tokio::sync::oneshot;
use tokio::sync::oneshot::Sender;

mod error;
use crate::error::AppError;

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

async fn handle_request(req: Request<hyper::body::Incoming>, _config: Arc<Config>) -> Result<Response<Full<Bytes>>, Infallible> {
    let path = req.uri().path().trim_start_matches('/');
    println!("handling request for {}", path);

    match general_purpose::STANDARD.decode(path) {
        Ok(decoded_bytes) => {
            let raw_url = match str::from_utf8(&decoded_bytes) {
                Ok(url) => url,
                Err(_) => {
                    println!("Invalid UTF-8 sequence in Base64 decoded URL");
                    return Ok(Response::builder()
                        .status(StatusCode::BAD_REQUEST)
                        .body(Full::new(Bytes::from("Invalid UTF-8 sequence in Base64 decoded URL")))
                        .unwrap())
                }
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


async fn qualify_rss(url: Url) -> Result<String, AppError> {
    let content = reqwest::get(url).await?.bytes().await?;

    let feed = match parser::parse(&content[..]) {
        Ok(feed) => feed,
        Err(_) => return Err(AppError::ScrapeError("Failed to parse feed".to_string())),
    };

    let mut channel = convert_feed_to_channel(feed);

    let mut tasks = vec![];

    for item in channel.items() {
        if let Some(link) = item.link() {
            let link = link.to_string();
            tasks.push(tokio::spawn(fetch_html(link)));
        }
    }

    for task in tasks {
        match task.await {
            Ok(htmlResult) =>
                match htmlResult {
                    Ok((html, link)) =>
                        if let Some(item) = channel.items_mut().iter_mut().find(|i| i.link() == Some(&link)) {
                            item.set_content(html);
                        }
                    Err(err) => eprintln!("Fetch html failed: {}", err)
                }
            Err(joinErr) => eprintln!("Task failed: {}", joinErr)
        }
    }
    Ok(channel.to_string())
}

async fn fetch_html(url: String) -> Result<(String, String), AppError> {
    let parsedUrl = Url::parse(&url).map_err(|e| AppError::UrlParseError(e))?;
    let client = Client::new();
    let scraper = ArticleScraper::new(None).await;
    let article = scraper.parse(&parsedUrl,false,&client,None).await.map_err(|e| AppError::ScrapeError(e.to_string()))?;
    if let Some(html) = article.html {
        Ok((html, url))
    }
    else {
        Err(AppError::ScrapeError("missing scraped html response".to_string()))
    }
}

fn convert_feed_to_channel(feed: Feed) -> Channel {
    let items: Vec<Item> = feed.entries.into_iter().map(|entry| {
        let mut item = Item::default();
        item.set_title(entry.title.map(|t| t.content).unwrap_or_else(|| "".to_string()));
        item.set_link(entry.links.first().map(|l| l.href.clone()).unwrap_or_else(|| "".to_string()));
        item.set_description(entry.summary.map(|s| s.content).unwrap_or_else(|| "".to_string()));
        if let Some(content) = entry.content {
            item.set_content(content.body.unwrap_or_else(|| "".to_string()));
        }
        item
    }).collect();

    let mut channel = Channel::default();
    channel.set_title(feed.title.map(|t| t.content).unwrap_or_else(|| "".to_string()));
    channel.set_link(feed.links.first().map(|l| l.href.clone()).unwrap_or_else(|| "".to_string()));
    channel.set_description(feed.description.map(|d| d.content).unwrap_or_else(|| "".to_string()));
    channel.set_items(items);
    channel
}
