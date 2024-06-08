use std::io;
use thiserror::Error;
use tokio::task::JoinError;
use url::ParseError;

#[derive(Error, Debug)]
pub enum AppError{
    #[error("IO error: {0}")]
    IO(#[from] io::Error),
    #[error("HTTP request error: {0}")]
    Reqwest(#[from] reqwest::Error),
    #[error("Url error: {0}")]
    UrlParseError(#[from] ParseError),
    #[error("ScraperError: {0}")]
    ScrapeError(String),
    #[error("RssError: {0}")]
    RssError(#[from] rss::Error),
    #[error("JoinError: {0}")]
    JoinError(#[from] JoinError)
}