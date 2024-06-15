#![allow(dead_code)]

use std::time::Duration;

pub trait FluentDuration {
    fn milli_seconds(self) -> Duration;
    fn seconds(self) -> Duration;
    fn minutes(self) -> Duration;
    fn hours(self) -> Duration;
    fn days(self) -> Duration;

}

impl FluentDuration for u32 {
    fn milli_seconds(self) -> Duration {
        Duration::from_millis(self as u64)
    }
    fn seconds(self) -> Duration {
        Duration::from_secs(self as u64)
    }

    fn minutes(self) -> Duration {
        Duration::from_secs(self as u64 * 60)
    }

    fn hours(self) -> Duration {
        Duration::from_secs(self as u64 * 3600)
    }

    fn days(self) -> Duration {
        Duration::from_secs(self as u64 * 86400)
    }
}