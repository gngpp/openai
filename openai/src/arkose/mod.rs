pub mod crypto;
pub mod murmur;

use std::sync::Once;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;

use rand::Rng;
use serde::Deserialize;
use serde::Serialize;
use serde::Serializer;

use crate::arkose::crypto::encrypt;

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
#[derive(Clone, Debug)]
pub struct ArkoseToken(String);

impl ArkoseToken {
    pub async fn new(model: &str) -> anyhow::Result<Self> {
        match GPT4Model::try_from(model) {
            Ok(_) => {
                INIT.call_once(|| {
                    let client = reqwest::Client::builder()
                        .user_agent(crate::HEADER_UA)
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

    pub async fn new_from_endpoint(model: &str) -> anyhow::Result<Self> {
        match GPT4Model::try_from(model) {
            Ok(_) => {
                INIT.call_once(|| {
                    let client = reqwest::Client::builder()
                        .user_agent(crate::HEADER_UA)
                        .chrome_builder(reqwest::browser::ChromeVersion::V108)
                        .build()
                        .unwrap();
                    unsafe { CLIENT = Some(client) };
                });

                Ok(get_endpoint_arkose_token(unsafe { CLIENT.as_ref() }).await?)
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
async fn get_endpoint_arkose_token(
    client: Option<&reqwest::Client>,
) -> anyhow::Result<ArkoseToken> {
    match client {
        Some(client) => {
            let url = "https://ai.fakeopen.com/api/arkose/token";
            let resp = client.get(url).send().await?;
            let arkose = resp.json::<ArkoseResponse>().await?;
            Ok(ArkoseToken(arkose.token))
        }
        None => {
            anyhow::bail!("The requesting client is not initialized")
        }
    }
}

async fn get_arkose_token(client: Option<&reqwest::Client>) -> anyhow::Result<ArkoseToken> {
    match client {
        Some(client) => {
            let timestamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
            let bx = serde_json::json!(

            [
                {
                    "key":"api_type",
                    "value":"js"
                },
                {
                    "key":"p",
                    "value":1
                },
                {
                    "key":"f",
                    "value":"3bafd9d65c4d3ed67bc5fdb2e6f98311"
                },
                {
                    "key":"n",
                    "value": timestamp
                },
                {
                    "key":"wh",
                    "value":"80b13fd48b8da8e4157eeb6f9e9fbedb|5ab5738955e0611421b686bc95655ad0"
                },
                {
                    "key":"enhanced_fp",
                    "value":[
                        {
                            "key":"webgl_extensions",
                            "value":null
                        },
                        {
                            "key":"webgl_extensions_hash",
                            "value":null
                        },
                        {
                            "key":"webgl_renderer",
                            "value":null
                        },
                        {
                            "key":"webgl_vendor",
                            "value":null
                        },
                        {
                            "key":"webgl_version",
                            "value":null
                        },
                        {
                            "key":"webgl_shading_language_version",
                            "value":null
                        },
                        {
                            "key":"webgl_aliased_line_width_range",
                            "value":null
                        },
                        {
                            "key":"webgl_aliased_point_size_range",
                            "value":null
                        },
                        {
                            "key":"webgl_antialiasing",
                            "value":null
                        },
                        {
                            "key":"webgl_bits",
                            "value":null
                        },
                        {
                            "key":"webgl_max_params",
                            "value":null
                        },
                        {
                            "key":"webgl_max_viewport_dims",
                            "value":null
                        },
                        {
                            "key":"webgl_unmasked_vendor",
                            "value":null
                        },
                        {
                            "key":"webgl_unmasked_renderer",
                            "value":null
                        },
                        {
                            "key":"webgl_vsf_params",
                            "value":null
                        },
                        {
                            "key":"webgl_vsi_params",
                            "value":null
                        },
                        {
                            "key":"webgl_fsf_params",
                            "value":null
                        },
                        {
                            "key":"webgl_fsi_params",
                            "value":null
                        },
                        {
                            "key":"webgl_hash_webgl",
                            "value":null
                        },
                        {
                            "key":"user_agent_data_brands",
                            "value":null
                        },
                        {
                            "key":"user_agent_data_mobile",
                            "value":null
                        },
                        {
                            "key":"navigator_connection_downlink",
                            "value":null
                        },
                        {
                            "key":"navigator_connection_downlink_max",
                            "value":null
                        },
                        {
                            "key":"network_info_rtt",
                            "value":null
                        },
                        {
                            "key":"network_info_save_data",
                            "value":null
                        },
                        {
                            "key":"network_info_rtt_type",
                            "value":null
                        },
                        {
                            "key":"screen_pixel_depth",
                            "value":24
                        },
                        {
                            "key":"navigator_device_memory",
                            "value":null
                        },
                        {
                            "key":"navigator_languages",
                            "value":"en-US,en"
                        },
                        {
                            "key":"window_inner_width",
                            "value":0
                        },
                        {
                            "key":"window_inner_height",
                            "value":0
                        },
                        {
                            "key":"window_outer_width",
                            "value":0
                        },
                        {
                            "key":"window_outer_height",
                            "value":0
                        },
                        {
                            "key":"browser_detection_firefox",
                            "value":true
                        },
                        {
                            "key":"browser_detection_brave",
                            "value":false
                        },
                        {
                            "key":"audio_codecs",
                            "value":"{\"ogg\":\"probably\",\"mp3\":\"maybe\",\"wav\":\"probably\",\"m4a\":\"maybe\",\"aac\":\"maybe\"}"
                        },
                        {
                            "key":"video_codecs",
                            "value":"{\"ogg\":\"probably\",\"h264\":\"probably\",\"webm\":\"probably\",\"mpeg4v\":\"\",\"mpeg4a\":\"\",\"theora\":\"\"}"
                        },
                        {
                            "key":"media_query_dark_mode",
                            "value":false
                        },
                        {
                            "key":"headless_browser_phantom",
                            "value":false
                        },
                        {
                            "key":"headless_browser_selenium",
                            "value":false
                        },
                        {
                            "key":"headless_browser_nightmare_js",
                            "value":false
                        },
                        {
                            "key":"document__referrer",
                            "value":""
                        },
                        {
                            "key":"window__ancestor_origins",
                            "value":null
                        },
                        {
                            "key":"window__tree_index",
                            "value":[
                                1
                            ]
                        },
                        {
                            "key":"window__tree_structure",
                            "value":"[[],[]]"
                        },
                        {
                            "key":"window__location_href",
                            "value":"https://tcr9i.chat.openai.com/v2/1.5.2/enforcement.64b3a4e29686f93d52816249ecbf9857.html#35536E1E-65B4-4D96-9D97-6ADB7EFF8147"
                        },
                        {
                            "key":"client_config__sitedata_location_href",
                            "value":"https://chat.openai.com/"
                        },
                        {
                            "key":"client_config__surl",
                            "value":"https://tcr9i.chat.openai.com"
                        },
                        {
                            "key":"mobile_sdk__is_sdk"
                        },
                        {
                            "key":"client_config__language",
                            "value":null
                        },
                        {
                            "key":"audio_fingerprint",
                            "value":"35.73833402246237"
                        }
                    ]
                },
                {
                    "key":"fe",
                    "value":[
                        "DNT:1",
                        "L:en-US",
                        "D:24",
                        "PR:1",
                        "S:0,0",
                        "AS:false",
                        "TO:0",
                        "SS:true",
                        "LS:true",
                        "IDB:true",
                        "B:false",
                        "ODB:false",
                        "CPUC:unknown",
                        "PK:Linux x86_64",
                        "CFP:-1568981108",
                        "FR:false",
                        "FOS:false",
                        "FB:false",
                        "JSF:Arial,Arial Narrow,Bitstream Vera Sans Mono,Bookman Old Style,Century Schoolbook,Courier,Courier New,Helvetica,MS Gothic,MS PGothic,Palatino,Palatino Linotype,Times,Times New Roman",
                        "P:Chrome PDF Viewer,Chromium PDF Viewer,Microsoft Edge PDF Viewer,PDF Viewer,WebKit built-in PDF",
                        "T:0,false,false",
                        "H:2",
                        "SWF:false"
                    ]
                },
                {
                    "key":"ife_hash",
                    "value":"8b524442a7ed10af3f39ec3bca28565c"
                },
                {
                    "key":"cs",
                    "value":1
                },
                {
                    "key":"jsbd",
                    "value":"{\"HL\":2,\"NCE\":true,\"DT\":\"\",\"NWD\":\"false\",\"DOTO\":1,\"DMTO\":1}"
                }
            ]

            );

            let bv = crate::HEADER_UA;

            let bt = SystemTime::now().duration_since(UNIX_EPOCH)?.as_micros() / 1000000;
            let bw = (bt - (bt % 21600)).to_string();

            let bda = encrypt(&bx.to_string(), &(bw + &bv));
            #[allow(deprecated)]
            let bda_encoded = base64::encode(&bda);

            let form: [(&str, &str); 8] = [
                ("bda", &bda_encoded),
                ("public_key", "35536E1E-65B4-4D96-9D97-6ADB7EFF8147"),
                ("site", "https://chat.openai.com"),
                ("userbrowser", bv),
                ("capi_version", "1.5.2"),
                ("capi_mode", "lightbox"),
                ("style_theme", "default"),
                ("rnd", &(&rand::thread_rng().gen::<f64>().to_string())),
            ];

            let resp = client
                    .post("https://tcr9i.chat.openai.com/fc/gt2/public_key/35536E1E-65B4-4D96-9D97-6ADB7EFF8147")
                    .header("Host", "tcr9i.chat.openai.com")
                    .header("User-Agent", bv)
                    .header("Accept", "*/*")
                    .header("Accept-Language", "en-US,en;q=0.5")
                    .header("Accept-Encoding", "gzip, deflate, br")
                    .header("Content-Type", "application/x-www-form-urlencoded; charset=UTF-8")
                    .header("Origin", "https://tcr9i.chat.openai.com")
                    .header("DNT", "1")
                    .header("Connection", "keep-alive")
                    .header("Referer", "https://tcr9i.chat.openai.com/v2/1.5.2/enforcement.64b3a4e29686f93d52816249ecbf9857.html")
                    .header("Sec-Fetch-Dest", "empty")
                    .header("Sec-Fetch-Mode", "cors")
                    .header("Sec-Fetch-Site", "same-origin")
                    .header("TE", "trailers")
                    .form(&form)
                    .send().await?;

            if resp.status() != reqwest::StatusCode::OK {
                anyhow::bail!(format!("get arkose token status code {}", resp.status()));
            }

            let arkose = resp.json::<ArkoseResponse>().await?;
            Ok(ArkoseToken(arkose.token))
        }
        None => {
            anyhow::bail!("The requesting client is not initialized")
        }
    }
}
