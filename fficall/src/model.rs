use derive_builder::Builder;
use serde::{de::DeserializeOwned, Deserialize, Deserializer, Serialize};
use std::{collections::HashMap, ffi::CString};

use crate::{ffi, FiiCallResult};

pub type HeaderMap = HashMap<String, Vec<String>>;
pub type Cookie = HashMap<String, String>;

fn deserialize_null_default<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    T: Default + Deserialize<'de>,
    D: Deserializer<'de>,
{
    let opt = Option::deserialize(deserializer)?;
    Ok(opt.unwrap_or_default())
}

#[derive(Serialize, Deserialize, Debug, Clone, Default, Builder)]
#[serde(rename_all = "camelCase")]
pub struct RequestPayload {
    pub request_url: String,
    pub request_method: RequestMethod,
    #[builder(setter(into, strip_option), default)]
    pub request_body: Option<String>,
    #[builder(setter(into, strip_option), default)]
    pub request_cookies: Option<Vec<HashMap<String, String>>>,
    #[builder(setter(into, strip_option), default)]
    pub tls_client_identifier: Option<Identifier>,
    #[builder(setter(into, strip_option), default)]
    pub follow_redirects: Option<bool>,
    #[builder(setter(into, strip_option), default)]
    pub insecure_skip_verify: Option<bool>,
    #[builder(setter(into, strip_option), default)]
    pub is_byte_response: Option<bool>,
    #[builder(setter(into, strip_option), default)]
    pub without_cookie_jar: Option<bool>,
    #[builder(setter(into, strip_option), default)]
    pub with_random_tls_extension_order: Option<bool>,
    #[builder(setter(into, strip_option), default)]
    pub timeout_seconds: Option<u32>,
    #[builder(setter(into, strip_option), default)]
    pub session_id: Option<String>,
    #[builder(setter(into, strip_option), default)]
    pub proxy_url: Option<String>,
    #[builder(setter(into, strip_option), default)]
    pub headers: Option<HashMap<String, String>>,
    #[builder(setter(into, strip_option), default)]
    pub header_order: Option<Vec<String>>,
    #[builder(setter(into, strip_option), default)]
    pub custom_tls_client: Option<CustomTlsClient>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum RequestMethod {
    GET,
    POST,
    PATCH,
    PUT,
    DELETE,
}

impl ToString for RequestMethod {
    fn to_string(&self) -> String {
        match self {
            RequestMethod::GET => String::from("GET"),
            RequestMethod::POST => String::from("POST"),
            RequestMethod::PATCH => String::from("PATCH"),
            RequestMethod::PUT => String::from("PUT"),
            RequestMethod::DELETE => String::from("DELETE"),
        }
    }
}

impl Default for RequestMethod {
    fn default() -> Self {
        Self::GET
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Identifier {
    Chrome103,
    Chrome104,
    Chrome105,
    Chrome106,
    Chrome107,
    Chrome108,
    Chrome109,
    Chrome110,
    Chrome111,
    Chrome112,
    Safari1561,
    Safari160,
    SafariIpad156,
    SafariIOS155,
    SafariIOS156,
    SafariIOS160,
    Firefox102,
    Firefox104,
    Firefox105,
    Firefox106,
    Firefox108,
    Firefox110,
    Opera89,
    Opera90,
    Opera91,
    ZalandoAndroidMobile,
    ZalandoIOSMobile,
    NikeIOSMobile,
    NikeAndroidMobile,
    Cloudscraper,
    MMSIOS,
    MeshIOS,
    MeshIOS1,
    MeshIOS2,
    MeshAndroid,
    MeshAndroid1,
    MeshAndroid2,
    ConfirmedIOS,
    ConfirmedAndroid,
    Okhttp4Android7,
    Okhttp4Android8,
    Okhttp4Android9,
    Okhttp4Android10,
    Okhttp4Android11,
    Okhttp4Android12,
    Okhttp4Android13,
}

impl ToString for Identifier {
    fn to_string(&self) -> String {
        match self {
            Identifier::Chrome103 => String::from("chrome_103"),
            Identifier::Chrome104 => String::from("chrome_104"),
            Identifier::Chrome105 => String::from("chrome_105"),
            Identifier::Chrome106 => String::from("chrome_106"),
            Identifier::Chrome107 => String::from("chrome_107"),
            Identifier::Chrome108 => String::from("chrome_108"),
            Identifier::Chrome109 => String::from("chrome_109"),
            Identifier::Chrome110 => String::from("chrome_110"),
            Identifier::Chrome111 => String::from("chrome_111"),
            Identifier::Chrome112 => String::from("chrome_112"),
            Identifier::Safari1561 => String::from("safari_15_6_1"),
            Identifier::Safari160 => String::from("safari_16_0"),
            Identifier::SafariIpad156 => String::from("safari_ipad_15_6"),
            Identifier::SafariIOS155 => String::from("safari_ios_15_5"),
            Identifier::SafariIOS156 => String::from("safari_ios_15_6"),
            Identifier::SafariIOS160 => String::from("safari_ios_16_0"),
            Identifier::Firefox102 => String::from("firefox_102"),
            Identifier::Firefox104 => String::from("firefox_104"),
            Identifier::Firefox105 => String::from("firefox_105"),
            Identifier::Firefox106 => String::from("firefox_106"),
            Identifier::Firefox108 => String::from("firefox_108"),
            Identifier::Firefox110 => String::from("firefox_110"),
            Identifier::Opera89 => String::from("opera_89"),
            Identifier::Opera90 => String::from("opera_90"),
            Identifier::Opera91 => String::from("opera_91"),
            Identifier::ZalandoAndroidMobile => String::from("zalando_android_mobile"),
            Identifier::ZalandoIOSMobile => String::from("zalando_ios_mobile"),
            Identifier::NikeIOSMobile => String::from("nike_ios_mobile"),
            Identifier::NikeAndroidMobile => String::from("nike_android_mobile"),
            Identifier::Cloudscraper => String::from("cloudscraper"),
            Identifier::MMSIOS => String::from("mms_ios"),
            Identifier::MeshIOS => String::from("mesh_ios"),
            Identifier::MeshIOS1 => String::from("mesh_ios_1"),
            Identifier::MeshIOS2 => String::from("mesh_ios_2"),
            Identifier::MeshAndroid => String::from("mesh_android"),
            Identifier::MeshAndroid1 => String::from("mesh_android_1"),
            Identifier::MeshAndroid2 => String::from("mesh_android_2"),
            Identifier::ConfirmedIOS => String::from("confirmed_ios"),
            Identifier::ConfirmedAndroid => String::from("confirmed_android"),
            Identifier::Okhttp4Android7 => String::from("okhttp4_android_7"),
            Identifier::Okhttp4Android8 => String::from("okhttp4_android_8"),
            Identifier::Okhttp4Android9 => String::from("okhttp4_android_9"),
            Identifier::Okhttp4Android10 => String::from("okhttp4_android_10"),
            Identifier::Okhttp4Android11 => String::from("okhttp4_android_11"),
            Identifier::Okhttp4Android12 => String::from("okhttp4_android_12"),
            Identifier::Okhttp4Android13 => String::from("okhttp4_android_13"),
        }
    }
}

impl Default for Identifier {
    fn default() -> Self {
        Self::Chrome103
    }
}
#[derive(Serialize, Deserialize, Debug, Clone, Builder)]
#[serde(rename_all = "camelCase")]
pub struct CustomTlsClient {
    pub ja3_string: String,
    pub h2_settings: H2Settings,
    pub h2_settings_order: Vec<String>,
    pub supported_signature_algorithms: Vec<String>,
    pub supported_versions: Vec<String>,
    pub key_share_curves: Vec<String>,
    pub cert_compression_algo: String,
    pub pseudo_header_order: Vec<String>,
    pub connection_flow: u32,
    pub priority_frames: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Builder)]
#[serde(rename_all = "camelCase")]
pub struct H2Settings {
    pub header_table_size: u32,
    pub max_concurrent_streams: u32,
    pub initial_window_size: u32,
    pub max_header_list_size: u32,
}

#[derive(Serialize, Deserialize, Debug, Clone, Builder)]
#[serde(rename_all = "camelCase")]
pub struct ReleaseSessionPayload {
    pub session_id: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, Builder)]
#[serde(rename_all = "camelCase")]
pub struct FetchCookiesForSessionRequestPayload {
    pub session_id: String,
    pub url: String,
}

pub type FetchCookiesForSessionResponse = Vec<Cookie>;

#[derive(Serialize, Deserialize, Debug, Clone, Builder)]
#[serde(rename_all = "camelCase")]
pub struct ReleaseSessionResponse {
    pub success: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub struct StatusCode(u32);

impl StatusCode {
    /// Check if status is within 100-199.
    #[inline]
    pub fn is_informational(&self) -> bool {
        200 > self.0 && self.0 >= 100
    }

    /// Check if status is within 0 (http client request error).
    #[inline]
    pub fn is_error(&self) -> bool {
        self.0 == 0
    }

    /// Check if status is within 200-299.
    #[inline]
    pub fn is_success(&self) -> bool {
        300 > self.0 && self.0 >= 200
    }

    /// Check if status is within 300-399.
    #[inline]
    pub fn is_redirection(&self) -> bool {
        400 > self.0 && self.0 >= 300
    }

    /// Check if status is within 400-499.
    #[inline]
    pub fn is_client_error(&self) -> bool {
        500 > self.0 && self.0 >= 400
    }

    /// Check if status is within 500-599.
    #[inline]
    pub fn is_server_error(&self) -> bool {
        600 > self.0 && self.0 >= 500
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ResponsePayload {
    id: String,
    session_id: Option<String>,
    status: StatusCode,
    body: String,
    #[serde(deserialize_with = "deserialize_null_default")]
    headers: HeaderMap,
    #[serde(deserialize_with = "deserialize_null_default")]
    cookies: Cookie,
    used_protocol: String,
}

#[allow(dead_code)]
impl ResponsePayload {
    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn status(&self) -> StatusCode {
        self.status
    }

    pub fn session_id(&self) -> Option<&str> {
        self.session_id.as_deref()
    }

    pub fn use_protocol(&self) -> &str {
        &self.used_protocol
    }

    /// Get the `Headers` of this `Response`.
    #[inline]
    pub fn headers(&self) -> &HeaderMap {
        &self.headers
    }

    /// Get a mutable reference to the `Headers` of this `Response`.
    #[inline]
    pub fn headers_mut(&mut self) -> &mut HeaderMap {
        &mut self.headers
    }

    /// Get the `Cookies` of this `Response`.
    pub fn cookies(&self) -> &Cookie {
        &self.cookies
    }

    pub fn json<T: DeserializeOwned>(self) -> FiiCallResult<T> {
        let full = self.body.as_bytes();
        Ok(serde_json::from_slice(full)?)
    }

    /// Get text of this 'Response'
    pub fn text(self) -> FiiCallResult<String> {
        Ok(self.body)
    }

    /// Get next line stream body of this 'Response'
    pub fn next(&self) -> FiiCallResult<Option<String>> {
        let body_utf8 = unsafe {
            let raw_id = CString::new(self.id())?.into_raw();
            let body = CString::from_raw(ffi::StreamLine(raw_id));
            // release
            let _ = CString::from_raw(raw_id);
            body.to_bytes().to_vec()
        };
        let body = String::from_utf8(body_utf8)?;
        if body.starts_with("data: [DONE]") {
            return Ok(None);
        }
        Ok(Some(body))
    }

    /// Stop reader stream body of this 'Response'
    pub fn stop(self) -> FiiCallResult<()> {
        unsafe {
            let raw_id = CString::new(self.id)?.into_raw();
            ffi::StopStreamLine(raw_id);
            // release
            let _ = CString::from_raw(raw_id);
        }
        Ok(())
    }
}
