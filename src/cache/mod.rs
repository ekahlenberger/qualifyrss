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

    loop {
        match receiver.try_recv() {
            Ok(message) => match message {
                CacheMessage::Get(get_msg) => {
                    let _ = get_msg
                        .response_channel
                        .send(match cache.get(&get_msg.url) {
                            None => None,
                            Some(value) => Some(value.content.clone()),
                        });
                }
                CacheMessage::Set(set_msg) => {
                    let content = Arc::new(set_msg.content);
                    let data = cache
                        .entry(set_msg.url)
                        .or_insert(CacheData::new(content.clone()));
                    data.last_update = SystemTime::now();
                    data.content = content;
                }
            },
            Err(e) => match e {
                TryRecvError::Empty => {
                    let update_limit = SystemTime::now().sub(2.hours());
                    let updateable = cache.iter().find_map(|(key, value)| {
                        if value.last_update < update_limit {
                            Some(key.to_owned())
                        } else {
                            None
                        }
                    });
                    if let Some(url) = updateable {
                        let sender = sender.clone();
                        tokio::spawn(async move {
                            if let Ok(fetch_response) = fetch_html(url).await {
                                let _ = sender.send(CacheMessage::Set(CacheSetMessage {
                                    content: Box::new(fetch_response.content),
                                    url: fetch_response.url,
                                }));
                            };
                        });
                    };
                    sleep(100.milli_seconds()).await
                }
                TryRecvError::Disconnected => break,
            },
        }
    }
}
