pub mod har;
pub mod version;

use self::version::ArkoseVersion;
use crate::arkose::Type;
use crate::homedir::home_dir;
use moka::sync::Cache;
use native_db::{Database, DatabaseBuilder};
use std::path::PathBuf;
use std::sync::{Arc, OnceLock};
use std::time::Duration;
use tokio::time::interval;
use tracing::{info, warn};

use super::WORKER_DIR;

const INTERVAL_SECONDS: u16 = 3600;
static DATABASE_BUILDER: OnceLock<DatabaseBuilder> = OnceLock::new();

pub struct ArkoseVersionContext<'a> {
    db: Database<'a>,
    cache: Cache<Type, Arc<ArkoseVersion>>,
}

impl ArkoseVersionContext<'_> {
    /// Create a new ArkoseContext
    pub(crate) fn new() -> Self {
        let builder = DATABASE_BUILDER.get_or_init(|| {
            let mut builder = DatabaseBuilder::new();
            builder
                .define::<ArkoseVersion>()
                .expect("define table failed");
            builder
        });

        let path = home_dir()
            .unwrap_or(PathBuf::new())
            .join(WORKER_DIR)
            .join("arkose.db");

        if let Some(p) = path.parent() {
            // If parent directory does not exist, create it
            if !p.exists() {
                std::fs::create_dir_all(p)
                    .expect(&format!("Failed to create directory: {}", p.display()));
            }
        }

        let db = builder
            .create(path)
            .expect("Failed to create arkose database");

        Self {
            db,
            cache: Cache::builder()
                .time_to_live(Duration::from_secs(INTERVAL_SECONDS.into()))
                .max_capacity(5)
                .build(),
        }
    }

    /// Get the latest version of the given type
    pub fn version(&self, version_type: Type) -> Option<Arc<ArkoseVersion>> {
        // Begin read transaction
        if let Ok(r) = self.db.r_transaction() {
            if let Some(Some(version)) = r.get().primary::<ArkoseVersion>(version_type.pk()).ok() {
                return Some(self.cache.get_with(version_type, || Arc::new(version)));
            }
        }

        None
    }

    /// Run a periodic task to upgrade the arkose version
    pub async fn periodic_upgrade(&self) {
        info!("Arkose Periodic task is running");
        let mut interval = interval(Duration::from_secs(INTERVAL_SECONDS.into()));
        loop {
            interval.tick().await;
            self.upgrade().await;
        }
    }

    /// Upgrade the arkose version
    async fn upgrade(&self) {
        // Auth
        self.insert_version(Type::Auth).await;
        // GPT-4
        self.insert_version(Type::GPT4).await;
        // GPT-3.5
        self.insert_version(Type::GPT3).await;
        // Platform
        self.insert_version(Type::Platform).await;
        // SignUp
        self.insert_version(Type::SignUp).await;

        if let Some(v) = self.version(Type::Auth) {
            info!("Arkose version: {}", v.version());
        }
    }

    async fn insert_version(&self, version_type: Type) {
        match version::latest_arkose_version(version_type).await {
            Ok(version) => {
                if let Ok(rw) = self.db.rw_transaction() {
                    if let Some(err) = rw.insert(version).err() {
                        warn!("Failed to insert arkose version: {}", err)
                    }
                    if let Some(err) = rw.commit().err() {
                        warn!("Failed to commit transaction: {}", err)
                    }
                }
            }
            Err(err) => {
                warn!("Failed to get latest arkose version: {}", err)
            }
        }
    }
}
