use std::io::{Read, Write};
use std::sync::Arc;
use std::time::SystemTime;
use flate2::Compression;
use flate2::read::ZlibDecoder;
use flate2::write::ZlibEncoder;
use serde::__private::from_utf8_lossy;

pub struct CacheData {
    content: Box<[u8]>,
    compressed: bool,
    pub creation: SystemTime,
    pub last_access: SystemTime,
    pub last_update: SystemTime,

}

impl CacheData {
    pub fn new(content: Arc<Box<String>>) -> CacheData {
        let now = SystemTime::now();
        // let mut compressor = ZlibEncoder::new(Vec::new(), Compression::default());
        // let _ = compressor.write((**content).as_bytes());
        // let compression_res = compressor.finish();

        let mut data = CacheData {
            content: Box::new([]),
            compressed: false,
            creation: now,
            last_access: now,
            last_update: now
        };
        data.set_content(content);
        return data;
    }
    pub fn get_content(&self) -> Box<String> {
        if !self.compressed{
            return Box::new(from_utf8_lossy(&self.content).to_string())
        }

        let mut decoder = ZlibDecoder::new(self.content.as_ref());
        let mut target: String = String::new();
        let _ = decoder.read_to_string(&mut target);
        return Box::new(target);
    }

    pub fn set_content(&mut self, content: Arc<Box<String>>) {
        let now = SystemTime::now();
        let mut compressor = ZlibEncoder::new(Vec::new(), Compression::default());
        let _ = compressor.write((**content).as_bytes());
        let compression_res = compressor.finish();
        if let Ok(data) = compression_res {
            self.content = data.into_boxed_slice();
            self.compressed = true;
        }
        else {
            self.content = (**content).clone().into_bytes().into_boxed_slice();
            self.compressed = false;
        }
        self.last_update = now;
    }
}
