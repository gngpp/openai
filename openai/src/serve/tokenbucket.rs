use redis::RedisResult;
use redis_macros::{FromRedisValue, ToRedisArgs};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::Mutex;

fn now_timestamp() -> u64 {
    let now_duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards");
    now_duration.as_secs()
}

#[async_trait::async_trait]
pub trait TokenBucket: Send + Sync {
    async fn acquire(&self, ip: IpAddr) -> anyhow::Result<bool>;
}

#[derive(Clone)]
pub enum Strategy {
    Mem,
    Redis,
}

impl std::str::FromStr for Strategy {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "mem" => Ok(Strategy::Mem),
            "redis" => Ok(Strategy::Redis),
            _ => anyhow::bail!("storage policy: {} is not supported", s),
        }
    }
}

#[derive(Serialize, Deserialize, FromRedisValue, ToRedisArgs)]
struct BucketState {
    tokens: u32,
    last_time: u64,
}

pub struct MemTokenBucket {
    enable: bool,
    /// token bucket capacity `capacity`
    capacity: u32,
    /// token bucket fill rate `fill_rate`
    fill_rate: u32,
    /// ip -> token backet
    buckets: Arc<Mutex<HashMap<IpAddr, BucketState>>>,
    _cleanup_task: Option<tokio::task::JoinHandle<()>>,
}

impl MemTokenBucket {
    pub fn new(enable: bool, capacity: u32, fill_rate: u32, expired: u32) -> Self {
        let buckets: Arc<Mutex<HashMap<IpAddr, BucketState>>> =
            Arc::new(Mutex::new(HashMap::new()));
        let b = buckets.clone();
        let task = if enable {
            let task = tokio::task::spawn(async move {
                loop {
                    tokio::time::sleep(std::time::Duration::from_secs(expired.into())).await;
                    let mut b = b.lock().await;
                    b.retain(|_, v| v.last_time > now_timestamp());
                    drop(b)
                }
            });
            Some(task)
        } else {
            None
        };
        Self {
            enable,
            capacity,
            fill_rate,
            buckets,
            _cleanup_task: task,
        }
    }
}

#[async_trait::async_trait]
impl TokenBucket for MemTokenBucket {
    async fn acquire(&self, ip: IpAddr) -> anyhow::Result<bool> {
        if !self.enable {
            return Ok(true);
        }
        let mut buckets = self.buckets.lock().await;

        let now_timestamp = now_timestamp();

        let bucket = buckets.entry(ip).or_insert(BucketState {
            tokens: self.capacity,
            last_time: now_timestamp,
        });

        let elapsed = now_timestamp - bucket.last_time;
        let tokens_to_add = (elapsed as u32) * self.fill_rate;
        bucket.tokens = (bucket.tokens + tokens_to_add).min(self.capacity);
        bucket.last_time = now_timestamp;

        if bucket.tokens > 0 {
            bucket.tokens -= 1;
            drop(buckets);
            Ok(true)
        } else {
            Ok(false)
        }
    }
}

pub struct RedisTokenBucket {
    enable: bool,
    /// token bucket capacity `capacity`
    capacity: u32,
    /// token bucket fill rate `fill_rate`
    fill_rate: u32,
    /// token bucket expired
    expired: u32,
    /// redis client
    client: redis::Client,
}

impl RedisTokenBucket {
    pub fn new(enable: bool, capacity: u32, fill_rate: u32, expired: u32) -> RedisResult<Self> {
        // connect to redis
        let client = redis::Client::open("redis://127.0.0.1/")?;
        Ok(Self {
            enable,
            capacity,
            fill_rate,
            client,
            expired,
        })
    }
}

#[async_trait::async_trait]
impl TokenBucket for RedisTokenBucket {
    async fn acquire(&self, ip: IpAddr) -> anyhow::Result<bool> {
        use redis::AsyncCommands;
        if !self.enable {
            return Ok(true);
        }
        let mut con = self.client.get_tokio_connection().await?;
        let now_timestamp = now_timestamp();
        let mut bucket: BucketState = con
            .get_ex(ip.to_string(), redis::Expiry::EX(self.expired as usize))
            .await
            .unwrap_or(BucketState {
                tokens: self.capacity,
                last_time: now_timestamp,
            });

        let elapsed = now_timestamp - bucket.last_time;
        let tokens_to_add = (elapsed as u32) * self.fill_rate;
        bucket.tokens = (bucket.tokens + tokens_to_add).min(self.capacity);
        bucket.last_time = now_timestamp;

        if bucket.tokens > 0 {
            bucket.tokens -= 1;
            con.set_ex(ip.to_string(), bucket, self.expired as usize)
                .await?;
            Ok(true)
        } else {
            Ok(false)
        }
    }
}

pub struct TokenBucketContext(Box<dyn TokenBucket>);

impl From<(Strategy, bool, u32, u32, u32)> for TokenBucketContext {
    fn from(value: (Strategy, bool, u32, u32, u32)) -> Self {
        let strategy = match value.0 {
            Strategy::Mem => Self(Box::new(MemTokenBucket::new(
                value.1, value.2, value.3, value.4,
            ))),
            Strategy::Redis => Self(Box::new(
                RedisTokenBucket::new(value.1, value.2, value.3, value.4).unwrap(),
            )),
        };
        strategy
    }
}

#[async_trait::async_trait]
impl TokenBucket for TokenBucketContext {
    async fn acquire(&self, ip: IpAddr) -> anyhow::Result<bool> {
        Ok(self.0.acquire(ip).await?)
    }
}
