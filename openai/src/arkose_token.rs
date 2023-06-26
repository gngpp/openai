use std::sync::Once;

use serde::Deserialize;
use serde::Serialize;
use serde::Serializer;

static INIT: Once = Once::new();
static mut CLIENT: Option<reqwest::Client> = None;

#[derive(PartialEq, Eq)]
enum GPT4Model {
    Gpt4model,
    Gpt4browsingModel,
    Gpt4pluginsModel,
    Gpt4Other,
}

impl TryFrom<&str> for GPT4Model {
    type Error = ();

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "gpt-4" => Ok(GPT4Model::Gpt4model),
            "gpt-4-browsing" => Ok(GPT4Model::Gpt4browsingModel),
            "gpt-4-plugins" => Ok(GPT4Model::Gpt4pluginsModel),
            _ => {
                if value.starts_with("gpt-4") || value.starts_with("gpt4") {
                    return Ok(GPT4Model::Gpt4Other);
                }
                Err(())
            }
        }
    }
}

/// curl 'https://tcr9i.chat.openai.com/fc/gt2/public_key/35536E1E-65B4-4D96-9D97-6ADB7EFF8147' --data-raw 'public_key=35536E1E-65B4-4D96-9D97-6ADB7EFF8147'
#[derive(Clone)]
pub struct ArkoseToken(String);

impl ArkoseToken {
    pub async fn new(model: &str) -> anyhow::Result<Self> {
        match GPT4Model::try_from(model) {
            Ok(_) => {
                INIT.call_once(|| {
                    let client = reqwest::Client::builder()
                        .chrome_builder(reqwest::browser::ChromeVersion::V108)
                        .build()
                        .unwrap();
                    unsafe { CLIENT = Some(client) };
                });

                Ok(get_arkose_token(unsafe { CLIENT.as_ref() }).await?)
            }
            Err(_) => anyhow::bail!("Models are not supported: {}", model),
        }
    }
}

impl Serialize for ArkoseToken {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.0)
    }
}

#[derive(Deserialize)]
struct ArkoseResponse {
    token: String,
}

/// Build it yourself: https://github.com/gngpp/arkose-generator
async fn get_arkose_token(client: Option<&reqwest::Client>) -> anyhow::Result<ArkoseToken> {
    match client {
        Some(client) => {
            let url = "http://bypass.churchless.tech/api/arkose";
            let resp = client.get(url).send().await?;
            let payload = resp.text().await?;
            let url =
                "https://tcr9i.chat.openai.com/fc/gt2/public_key/35536E1E-65B4-4D96-9D97-6ADB7EFF8147";
            let mut req = client.post(url).body(payload);
            req = req.header("Host", "tcr9i.chat.openai.com")
                .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; rv:114.0) Gecko/20100101 Firefox/114.0")
                .header("Accept", "*/*")
                .header("Accept-Language", "en-US,en;q=0.5")
                .header("Accept-Encoding", "gzip, deflate, br")
                .header("Content-Type", "application/x-www-form-urlencoded")
                .header("Origin", "https://tcr9i.chat.openai.com")
                .header("DNT", "1")
                .header("Connection", "keep-alive")
                .header("Referer", "https://tcr9i.chat.openai.com/v2/1.5.2/enforcement.64b3a4e29686f93d52816249ecbf9857.html")
                .header("Sec-Fetch-Dest", "empty")
                .header("Sec-Fetch-Mode", "cors")
                .header("Sec-Fetch-Site", "same-origin")
                .header("TE", "trailers");
            let resp = req.send().await?;
            let arkose = resp.json::<ArkoseResponse>().await?;
            Ok(ArkoseToken(arkose.token))
        }
        None => {
            anyhow::bail!("The requesting client is not initialized")
        }
    }
}
