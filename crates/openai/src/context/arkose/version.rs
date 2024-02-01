use std::sync::OnceLock;

use anyhow::Context;
use anyhow::Result;
use native_db::*;
use native_model::{native_model, Model};
use serde::{Deserialize, Serialize};

use crate::{arkose::Type, with_context};

static RE: OnceLock<regex::Regex> = OnceLock::new();
static RE_VERSION: OnceLock<regex::Regex> = OnceLock::new();

#[native_db]
#[native_model(id = 1, version = 1)]
#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub struct ArkoseVersion {
    #[primary_key]
    pk: String,
    version: String,
    ref_enforcement_js: String,
    ref_enforcement_html: String,
}

impl ArkoseVersion {
    pub fn version(&self) -> &str {
        &self.version
    }

    pub fn ref_enforcement_js(&self) -> &str {
        &self.ref_enforcement_js
    }

    pub fn ref_enforcement_html(&self) -> &str {
        &self.ref_enforcement_html
    }

    pub fn pk(&self) -> &str {
        &self.pk
    }
}

pub(super) async fn latest_arkose_version(typed: Type) -> Result<ArkoseVersion> {
    let client = with_context!(api_client);
    // Response content
    let content = client
        .get(format!("{}/v2/{}/api.js", typed.origin_url(), typed.pk()))
        .send()
        .await?
        .text()
        .await?;

    // Regex to find the enforcement.html file
    let re = RE.get_or_init(|| {
        regex::Regex::new(r#"file:"([^"]*/enforcement\.[^"]*\.html)""#).expect("Invalid regex")
    });

    let ref_cap = re.captures(&content).context("No match found")?;
    let ref_html = ref_cap.get(1).context("No match found")?;
    let ref_js = ref_html.as_str().replace(".html", ".js");

    // Regex to find the version
    let re_version = RE_VERSION.get_or_init(|| {
        regex::Regex::new(r#"([^/]*)/enforcement\.[^"]*\.html"#).expect("Invalid regex")
    });

    let version_cap = re_version
        .captures(ref_html.as_str())
        .context("No match found")?;

    let ref_enforcement_js = format!("/v2/{}", ref_js);
    let ref_enforcement_html = format!("/v2/{}", ref_html.as_str());

    Ok(ArkoseVersion {
        pk: typed.pk().to_owned(),
        version: version_cap
            .get(1)
            .context("No match found")?
            .as_str()
            .to_string(),
        ref_enforcement_js,
        ref_enforcement_html,
    })
}
