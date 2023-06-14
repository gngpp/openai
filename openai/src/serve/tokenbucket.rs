use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::RwLock;

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
    buckets: Arc<RwLock<HashMap<IpAddr, BucketState>>>,
    _cleanup_task: tokio::task::JoinHandle<()>,
}

impl TokenBucket {
    pub fn new(capacity: u32, fill_rate: u32, expired: u32) -> Self {
        let buckets: Arc<RwLock<HashMap<IpAddr, BucketState>>> =
            Arc::new(RwLock::new(HashMap::new()));
        let b = buckets.clone();
        let task = tokio::task::spawn(async move {
            loop {
                tokio::time::sleep(std::time::Duration::from_secs(expired.into())).await;
                let x = Instant::now();
                let mut b = b.write().await;
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
        let buckets = self.buckets.read().await;
        let bucket = buckets.get(&ip);
        if let Some(bucket) = bucket {
            let b = bucket.tokens > 0;
            if b {
                return b;
            }
        }
        // Release the read lock before acquiring the write lock
        drop(buckets);

        let mut buckets = self.buckets.write().await;
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
