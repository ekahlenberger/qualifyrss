use url::Url;
use hyper::{Request, Response, StatusCode};
use std::sync::Arc;
use http_body_util::Full;
use hyper::body::Bytes;
use std::convert::Infallible;
use base64::engine::general_purpose;
use std::str;
use base64::Engine;
use crate::{Config};
use crate::feed_handling::qualify_rss;

pub async fn handle_request(req: Request<hyper::body::Incoming>, _config: Arc<Config>) -> Result<Response<Full<Bytes>>, Infallible> {
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
