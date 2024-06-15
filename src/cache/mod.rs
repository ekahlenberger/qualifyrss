use crate::cache::cache_message::CacheSetMessage;
use crate::feed_handling::fetch_html;
use crate::fluent::FluentDuration;
use cache_data::CacheData;
use cache_message::CacheMessage;
use std::collections::HashMap;
use std::ops::Sub;
use std::sync::Arc;
use std::time::SystemTime;
use tokio::sync::mpsc::error::TryRecvError;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use tokio::time::sleep;

mod cache_data;
pub mod cache_message;

pub async fn cache_manager_task(
    sender: UnboundedSender<CacheMessage>,
    mut receiver: UnboundedReceiver<CacheMessage>,
) {
    let mut cache: HashMap<String, CacheData> = HashMap::new();
    let mut current_update_url: Option<String> = None;
    loop {
        match receiver.try_recv() {
            Ok(message) => match message {
                CacheMessage::Get(get_msg) => {
                    let _ = get_msg
                        .response_channel
                        .send(match cache.get_mut(&get_msg.url) {
                            None => None,
                            Some(data) => {
                                data.last_access = SystemTime::now();
                                Some(data.get_content())
                            }
                        });
                }
                CacheMessage::Set(set_msg) => {
                    let url = set_msg.url.clone();
                    if set_msg.content.is_some() {
                        let content = Arc::new(set_msg.content.unwrap());
                        let data = cache
                            .entry(url)
                            .or_insert(CacheData::new(content.clone()));
                        data.last_update = SystemTime::now();
                        data.set_content(content);
                        if current_update_url.is_some() && current_update_url.clone().unwrap().eq(&set_msg.url) {
                            current_update_url = None;
                        }
                        println!("cached: {}", &set_msg.url);
                    }
                    else if current_update_url.is_some() && current_update_url.clone().unwrap().eq(&set_msg.url) {
                        current_update_url = None;
                        eprintln!("cache update failed for: {}", &set_msg.url);
                    }
                }
            },
            Err(e) => match e {
                TryRecvError::Empty => {
                    delete_not_accessed(&mut cache);
                    if current_update_url.is_none() {
                        current_update_url = schedule_next_update(sender.clone(), &mut cache);
                    }
                    sleep(100.milli_seconds()).await
                }
                TryRecvError::Disconnected => break,
            },
        }
    }
}

fn delete_not_accessed(cache: &mut HashMap<String, CacheData>) {
    let delete_limit = SystemTime::now().sub(6.hours());
    let deletables: Vec<_> = cache.iter().filter_map(|(url, data)| if data.last_access < delete_limit {Some(url.clone())} else {None}).collect();
    for del in deletables {
        cache.remove(&del);
        println!("removed from cache: {}", &del);
    }
}

fn schedule_next_update(sender: UnboundedSender<CacheMessage>, cache: &mut HashMap<String, CacheData>) -> Option<String> {
    let update_limit = SystemTime::now().sub(211.minutes());
    let updateable = cache.iter().find_map(|(key, value)| {
        if value.last_update < update_limit {
            Some(key.to_owned())
        } else {
            None
        }
    });
    if let Some(url) = updateable {
        let sender = sender.clone();
        println!("scheduled cache update for: {}", &url);
        let response_url = url.clone();
        tokio::spawn(async move{
            let failure_url = url.clone();
            if let Ok(fetch_response) = fetch_html(url).await {
                let _ = sender.send(CacheMessage::Set(CacheSetMessage {
                    content: Some(Box::new(fetch_response.content)),
                    url: fetch_response.url,
                }));
            }
            else {
                let _ = sender.send(CacheMessage::Set(CacheSetMessage {
                    content: None,
                    url: failure_url,
                }));
            }
        });
        return Some(response_url);
    };
    None
}
