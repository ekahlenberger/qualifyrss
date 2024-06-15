use std::sync::Arc;
use std::time::SystemTime;

pub struct CacheData {
    pub content: Arc<Box<String>>,
    pub creation: SystemTime,
    pub last_access: SystemTime,
    pub last_update: SystemTime,
}

impl CacheData {
    pub fn new(content: Arc<Box<String>>) -> CacheData {
        let now = SystemTime::now();
        CacheData{
            content: content,
            creation: now,
            last_access: now,
            last_update: now,
        }

    }
}
