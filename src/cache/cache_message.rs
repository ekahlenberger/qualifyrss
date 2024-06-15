use tokio::sync::oneshot;

pub enum CacheMessage {
    Get(CacheGetMessage),
    Set(CacheSetMessage)
}

pub struct CacheGetMessage{
    pub url: String,
    pub response_channel: oneshot::Sender<Option<Box<String>>>,
}

pub struct CacheSetMessage{
    pub url: String,
    pub content: Option<Box<String>>,
}
