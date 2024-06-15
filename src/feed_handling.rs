use std::sync::Arc;
use url::Url;
use reqwest::Client;
use article_scraper::ArticleScraper;
use feed_rs::model::Feed;
use rss::{Channel, Item};
use feed_rs::parser;
use tokio::sync::mpsc::UnboundedSender;
use tokio::sync::oneshot;
use crate::cache::cache_message::{CacheGetMessage, CacheMessage, CacheSetMessage};
use crate::error::AppError;

pub async fn qualify_rss(url: Url, cache_sender: UnboundedSender<CacheMessage>) -> Result<String, AppError> {
    let content = reqwest::get(url).await?.bytes().await?;

    let feed = match parser::parse(&content[..]) {
        Ok(feed) => feed,
        Err(_) => return Err(AppError::ScrapeError("Failed to parse feed".to_string())),
    };

    let mut channel = convert_feed_to_channel(feed);

    let tasks: Vec<_> = channel.items.iter().
        filter(|item|item.link().is_some()).
        map(|item| tokio::spawn(fetch_html_or_use_cache(item.link().unwrap().to_string(), cache_sender.clone()))).
        collect();

    for task in tasks {
        match task.await {
            Ok(htmlResult) =>
                match htmlResult {
                    Ok(response) =>
                        if let Some(item) = channel.items_mut().iter_mut().find(|i| i.link() == Some(&response.url)) {
                            item.set_content(response.content.clone());
                            let _ = cache_sender.send(CacheMessage::Set(CacheSetMessage {url: response.url, content: Box::new(response.content)}));
                        }
                    Err(err) => eprintln!("Fetch html failed: {}", err)
                }
            Err(joinErr) => eprintln!("Task failed: {}", joinErr)
        }
    }
    Ok(channel.to_string())
}

pub struct FetchResponse {
    pub url: String,
    pub content: String,
}

pub async fn fetch_html(url: String) -> Result<FetchResponse, AppError> {
    let parsedUrl = Url::parse(&url).map_err(|e| AppError::UrlParseError(e))?;
    let client = Client::new();
    let scraper = ArticleScraper::new(None).await;
    let article = scraper.parse(&parsedUrl,false,&client,None).await.map_err(|e| AppError::ScrapeError(e.to_string()))?;
    if let Some(content) = article.html {
        Ok( FetchResponse {url, content })
    }
    else {
        Err(AppError::ScrapeError("missing scraped html response".to_string()))
    }
}
async fn fetch_html_or_use_cache(url: String, cache_sender: UnboundedSender<CacheMessage>) -> Result<FetchResponse, AppError> {
    let (response_sender, response_receiver) = oneshot::channel::<Option<Arc<Box<String>>>>();
    if let Ok(_) = cache_sender.send(CacheMessage::Get(CacheGetMessage{url: url.clone(), response_channel: response_sender})){
        if let Ok(response) = response_receiver.await {
            if let Some(html) = response {
                return Ok(FetchResponse { url, content: (**html).clone() });
            }
        }
    }
    fetch_html(url).await
}

fn convert_feed_to_channel(feed: Feed) -> Channel {
    let items: Vec<Item> = feed.entries.into_iter().map(|entry| {
        let mut item = Item::default();
        item.set_title(entry.title.map(|t| t.content).unwrap_or_else(|| "".to_string()));
        item.set_link(entry.links.first().map(|l| l.href.clone()).unwrap_or_else(|| "".to_string()));
        item.set_description(entry.summary.map(|s| s.content).unwrap_or_else(|| "".to_string()));
        item.set_author(entry.authors.first().map(|person| person.email.to_owned()).unwrap_or_else(|| None));
        if let Some(publishing_date) = entry.published {
            item.set_pub_date(Some(publishing_date.to_rfc3339()));
        }
        if let Some(updated) = entry.updated {
            item.set_pub_date(Some(updated.to_rfc3339()));
        }
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
