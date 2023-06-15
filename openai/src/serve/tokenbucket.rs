use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::Mutex;

struct BucketState {
    tokens: u32,
    pub last_time: Instant,
}

pub struct TokenBucket {
    // token bucket capacity `capacity`
    capacity: u32,
    /// token bucket fill rate `fill_rate`
    fill_rate: u32,
    // ip -> token backet
    buckets: Arc<Mutex<HashMap<IpAddr, BucketState>>>,
    _cleanup_task: tokio::task::JoinHandle<()>,
}

impl TokenBucket {
    pub fn new(capacity: u32, fill_rate: u32, expired: u32) -> Self {
        let buckets: Arc<Mutex<HashMap<IpAddr, BucketState>>> =
            Arc::new(Mutex::new(HashMap::new()));
        let b = buckets.clone();
        let task = tokio::task::spawn(async move {
            loop {
                tokio::time::sleep(std::time::Duration::from_secs(expired.into())).await;
                let x = Instant::now();
                let mut b = b.lock().await;
                b.retain(|_, v| v.last_time > x);
                drop(b)
            }
        });
        Self {
            capacity,
            fill_rate,
            buckets,
            _cleanup_task: task,
        }
    }

    pub async fn acquire(&self, ip: IpAddr) -> bool {
        let mut buckets = self.buckets.lock().await;
        let now = Instant::now();

        let bucket = buckets.entry(ip).or_insert(BucketState {
            tokens: self.capacity,
            last_time: now,
        });

        let elapsed = now.duration_since(bucket.last_time);
        let tokens_to_add = (elapsed.as_secs() as u32) * self.fill_rate;
        bucket.tokens = (bucket.tokens + tokens_to_add).min(self.capacity);
        bucket.last_time = now;

        if bucket.tokens > 0 {
            bucket.tokens -= 1;
            drop(buckets);
            true
        } else {
            false
        }
    }
}
