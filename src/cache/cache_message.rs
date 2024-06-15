use std::sync::Arc;
use tokio::sync::oneshot;

pub enum CacheMessage {
    Get(CacheGetMessage),
    Set(CacheSetMessage)
}

pub struct CacheGetMessage{
    pub url: String,
    pub response_channel: oneshot::Sender<Option<Arc<Box<String>>>>,
}

pub struct CacheSetMessage{
    pub url: String,
    pub content: Option<Box<String>>,
}
