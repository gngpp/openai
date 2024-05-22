use super::{Store, StoreId, StoreResult};
use openai::{arkose::funcaptcha::Solver, homedir::home_dir};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, ops::Not, path::PathBuf};

const DEFAULT_ID: &str = "ninja-default-config";

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct Conf {
    id: String,
    pub using_user: Option<String>,
    /// Oofficial API prefix. Format: https://example.com
    pub official_api: Option<String>,
    /// Unofficial API prefix. Format: https://example.com
    pub unofficial_api: Option<String>,
    /// Client proxy. Format: protocol://user:pass@ip:port
    pub proxy: Option<String>,
    /// Config wss endpoint. Format: wss://example.com
    pub websocket_endpoint: Option<String>,
    /// About the solver client by ArkoseLabs
    pub arkose_solver: Solver,
    /// About the solver client key by ArkoseLabs
    pub arkose_solver_key: Option<String>,
    /// About the browser HAR file path requested by ChatGPT ArkoseLabs
    pub arkose_chat_har_path: Option<String>,
    /// About the browser HAR file path requested by Auth0 ArkoseLabs
    pub arkose_auth_har_path: Option<String>,
    /// About the browser HAR directory path requested by Platform ArkoseLabs
    pub arkose_platform_har_path: Option<String>,
    /// Client timeout (seconds)
    pub timeout: usize,
    /// Client connect timeout (seconds)
    pub connect_timeout: usize,
    /// TCP keepalive (seconds)
    pub tcp_keepalive: usize,
}

impl StoreId for Conf {
    fn id(&self) -> String {
        self.id.to_owned()
    }
}

impl Conf {
    pub fn new() -> Self {
        Self {
            timeout: 60,
            connect_timeout: 600,
            tcp_keepalive: 75,
            id: DEFAULT_ID.to_owned(),
            ..Default::default()
        }
    }
}

pub struct ConfFileStore(PathBuf);

impl ConfFileStore {
    pub fn new() -> Self {
        let path = match home_dir() {
            Some(home_dir) => home_dir.join(".ninja_config"),
            None => PathBuf::from(".ninja_config"),
        };
        if let Some(parent) = path.parent() {
            if path.exists().not() {
                std::fs::create_dir_all(parent)
                    .expect("Unable to create default file Account storage directory")
            }
        }
        if path.exists().not() {
            std::fs::File::create(&path)
                .unwrap_or_else(|_| panic!("Unable to create file: {}", path.display()));
        }
        ConfFileStore(path)
    }
}

impl Default for ConfFileStore {
    fn default() -> Self {
        Self::new()
    }
}

impl Store<Conf> for ConfFileStore {
    fn store(&self, target: Conf) -> StoreResult<Option<Self::Obj>> {
        let bytes = std::fs::read(&self.0)?;
        let mut data: HashMap<String, Conf> = if bytes.is_empty() {
            HashMap::new()
        } else {
            serde_json::from_slice(&bytes).map_err(|e| anyhow::anyhow!(e))?
        };
        let v = data.insert(target.id(), target);
        let json = serde_json::to_string_pretty(&data)?;
        std::fs::write(&self.0, json.as_bytes())?;
        Ok(v)
    }

    fn read(&self, target: Conf) -> StoreResult<Option<Self::Obj>> {
        let bytes = std::fs::read(&self.0)?;
        if bytes.is_empty() {
            return Ok(None);
        }
        let data: HashMap<String, Conf> =
            serde_json::from_slice(&bytes).map_err(|e| anyhow::anyhow!(e))?;
        Ok(data.get(&target.id()).cloned())
    }

    fn remove(&self, target: Conf) -> StoreResult<Option<Self::Obj>> {
        let bytes = std::fs::read(&self.0)?;
        if bytes.is_empty() {
            return Ok(None);
        }
        let mut data: HashMap<String, Conf> =
            serde_json::from_slice(&bytes).map_err(|e| anyhow::anyhow!(e))?;
        let v = data.remove(&target.id());
        let json = serde_json::to_string_pretty(&data)?;
        std::fs::write(&self.0, json)?;
        Ok(v)
    }

    fn list(&self) -> StoreResult<Vec<Self::Obj>> {
        let bytes = std::fs::read(&self.0)?;
        if bytes.is_empty() {
            return Ok(vec![]);
        }
        let data: HashMap<String, Conf> =
            serde_json::from_slice(&bytes).map_err(|e| anyhow::anyhow!(e))?;
        Ok(data.into_values().collect::<Vec<Conf>>())
    }

    type Obj = Conf;
}
