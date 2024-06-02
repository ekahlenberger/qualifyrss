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
use rss::Channel;
use std::convert::Infallible;
use std::net::SocketAddr;
use std::str;
use std::sync::{Arc, Mutex};
use tokio::net::TcpListener;
use url::Url;

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
}

async fn handle_request(req: Request<hyper::body::Incoming>, _config: Arc<Config>) -> Result<Response<Full<Bytes>>, Infallible> {
    let path = req.uri().path().trim_start_matches('/');


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


async fn qualify_rss(url: Url) -> Result<String, AppError> {
    let content = reqwest::get(url).await?.bytes().await?;
    let channel = Channel::read_from(&content[..])?;
    let channel = Arc::new(Mutex::new(channel));

    let mut tasks = vec![];

    for item in channel.lock().unwrap().items_mut() {
        if let Some(link) = item.link() {
            let link = link.to_string();
            let channel = Arc::clone(&channel);
            let task = tokio::spawn(async move {
                match fetch_html(&link).await {
                    Ok(html) => {
                        let mut channel = channel.lock().unwrap();
                        if let Some(item) = channel.items_mut().iter_mut().find(|i| i.link() == Some(&link)) {
                            item.set_content(html);
                        }
                    },
                    Err(e) => {
                        eprintln!("Failed to fetch HTML: {}", e);
                    }
                }
            });
            tasks.push(task);
        }
    }

    for task in tasks {
        if let Err(e) = task.await {
            eprintln!("Task failed: {}", e);
        }
    }

    let channel = Arc::try_unwrap(channel).expect("Failed to unwrap Arc").into_inner().expect("Failed to get Mutex guard");

    Ok(channel.to_string())
}

async fn fetch_html(url: &str) -> Result<String, AppError> {
    let parsedUrl = Url::parse(url).map_err(|e| AppError::UrlParseError(e))?;
    let client = Client::new();
    let scraper = ArticleScraper::new(None).await;
    let article = scraper.parse(&parsedUrl,false,&client,None).await.map_err(|e| AppError::ScrapeError(e.to_string()))?;
    if let Some(html) = article.html {
        return Ok(html);
    }
    else {
        return Err(AppError::ScrapeError("missing scraped html response".to_string()))
    }
}
