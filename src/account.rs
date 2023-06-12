use std::{collections::HashMap, ops::Not, path::PathBuf};

use anyhow::Context;
use async_trait::async_trait;

pub type AccountResult<T, E = anyhow::Error> = anyhow::Result<T, E>;

#[async_trait]
pub(crate) trait AccountStore: Send + Sync {
    // Add store account return an old account
    async fn add_account(&self, token: Account) -> AccountResult<Option<Account>>;

    // Read account return a copy of the account
    async fn get_account<'a>(&self, email: &'a str) -> AccountResult<Option<Account>>;

    // Delete account return an current Account
    async fn delete_account<'a>(&self, email: &'a str) -> AccountResult<Option<Account>>;

    /// List account return an current account list
    async fn list_account(&self) -> AccountResult<Option<Vec<Account>>>;
}

static mut FILE_STORAGE: std::mem::MaybeUninit<AccountFileStore> = std::mem::MaybeUninit::uninit();
pub struct AccountFileStore(PathBuf);

impl AccountFileStore {
    pub async fn new(path: Option<PathBuf>) -> AccountResult<Self> {
        let path = path.unwrap_or(PathBuf::from(".opengpt-accounts"));
        if let Some(parent) = path.parent() {
            if path.exists().not() {
                tokio::fs::create_dir_all(parent)
                    .await
                    .context("Unable to create default file Account storage file")?
            }
        }
        if path.exists().not() {
            tokio::fs::File::create(&path).await?;
        }
        crate::ONCE_INIT
            .call_once(|| unsafe { FILE_STORAGE.as_mut_ptr().write(AccountFileStore(path)) });
        Ok(unsafe { FILE_STORAGE.as_mut_ptr().read() })
    }
}

#[async_trait]
impl AccountStore for AccountFileStore {
    async fn add_account(&self, account: Account) -> AccountResult<Option<Account>> {
        let bytes = tokio::fs::read(&self.0).await?;
        let mut data: HashMap<String, Account> = if bytes.len() == 0 {
            HashMap::new()
        } else {
            serde_json::from_slice(&bytes).map_err(|e| anyhow::anyhow!(e))?
        };
        let v = data.insert(account.email.to_string(), account);
        let json = serde_json::to_string_pretty(&data)?;
        tokio::fs::write(&self.0, json.as_bytes()).await?;
        Ok(v)
    }

    async fn get_account<'a>(&self, email: &'a str) -> AccountResult<Option<Account>> {
        let bytes = tokio::fs::read(&self.0).await?;
        if bytes.len() == 0 {
            return Ok(None);
        }
        let data: HashMap<String, Account> =
            serde_json::from_slice(&bytes).map_err(|e| anyhow::anyhow!(e))?;
        Ok(data.get(email).cloned())
    }

    async fn delete_account<'a>(&self, email: &'a str) -> AccountResult<Option<Account>> {
        let bytes = tokio::fs::read(&self.0).await?;
        if bytes.len() == 0 {
            return Ok(None);
        }
        let mut data: HashMap<String, Account> =
            serde_json::from_slice(&bytes).map_err(|e| anyhow::anyhow!(e))?;
        let v = data.remove(email);
        let json = serde_json::to_string_pretty(&data)?;
        tokio::fs::write(&self.0, json).await?;
        Ok(v)
    }

    async fn list_account(&self) -> AccountResult<Option<Vec<Account>>> {
        let bytes = tokio::fs::read(&self.0).await?;
        if bytes.len() == 0 {
            return Ok(None);
        }
        let data: HashMap<String, Account> =
            serde_json::from_slice(&bytes).map_err(|e| anyhow::anyhow!(e))?;
        let v = data.into_values().collect::<Vec<Account>>();
        Ok(Some(v))
    }
}

#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub(crate) struct Account {
    email: String,
    password: String,
}

impl Account {
    pub(crate) fn new(email: String, password: String) -> Self {
        Self { email, password }
    }
}
