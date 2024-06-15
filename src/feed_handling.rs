use url::Url;
use reqwest::Client;
use article_scraper::ArticleScraper;
use feed_rs::model::Feed;
use rss::{Channel, Item};
use feed_rs::parser;
use crate::error::AppError;

pub async fn qualify_rss(url: Url) -> Result<String, AppError> {
    let content = reqwest::get(url).await?.bytes().await?;

    let feed = match parser::parse(&content[..]) {
        Ok(feed) => feed,
        Err(_) => return Err(AppError::ScrapeError("Failed to parse feed".to_string())),
    };

    let mut channel = convert_feed_to_channel(feed);

    let tasks: Vec<_> = channel.items.iter().
        filter(|item|item.link().is_some()).
        map(|item| tokio::spawn(fetch_html(item.link().unwrap().to_string()))).
        collect();

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
