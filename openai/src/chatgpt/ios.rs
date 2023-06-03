use std::{ops::Add, time::Duration};

use anyhow::Context;
use reqwest::{
    header::{self, HeaderMap, HeaderValue},
    Proxy,
};
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

use crate::debug;

use super::{ApiError, OpenAIResult, Api};

const HEADER_OAI_DEVICE_ID: &str = "0E92DAF9-94F0-4F77-BDF4-53A60D19EC65";
const HEADER_OAI_CLIENT: &str = "ios";
const HEADER_UA: &str = "ChatGPT/1.2023.21 (iOS 16.2; iPad11,1; build 623)";
const HEADER_HOST: &str = "ios.chat.openai.com";
const URL_IOS_CHAT_BASE: &str = "https://ios.chat.openai.com";
const DEVICE_TOKEN: &str = "AgAAAHIBP+UwE0Cj9sNzon2ZkMIEUNk0+me89vLfv5ZingpyOOkgXXXyjPzYTzWmWSu+BYqcD47byirLZ++3dJccpF99hWppT7G5xAuU+y56WpSYsAQSC4KVWaOMtq2H7r7uFO4E0+erAglGMMP3i1It+XlxSPMM9CAvEBImQl+Y74bIcPKtA7ycGSzcOPbweEaoVDpDBggAAEmrVYQSAXa62Na0GH3w3pI42Ntf8thXoKwXYWf9RDs0YNVXRvPmpn6SMU7FvdTa85D9C9XZnINiKdCoPubjobCR98zAeVBE8h0h+OQ95XK1Qi+bid0AnK6Kh3DRo+vVFK+dwyFEzHl63lJADkQ9cLwUAgBWsSi0MwUoN/kYklIxhZBLoibUPSXOMNWVLAW2umaUfQeF/Ff5OmrV+FmO/X3PeUFrvBt+T9HUsDIep4eBUN+ikaGnMrjBVM7jWRxXsr9A/NXWAWgxta8bbDLoJZlhtQiCC76eYVSog3sJA0wwMf9RjZSXmHoKYfnZ/XKgtFN8411qXjk4O73qKXnCvoAmfNBwBJ9JC1r/8MrXR+9g26+qK8uOz2xOi71zzgvrSqAnXRbAjJDt1m3RFhvzpzt7n+SwYSeBKZ+bm2If4jBGBauiSH9b3FTCJruvxIJspGV0bThpDjhtrxckSYv6MofE1+TH125ktAbmA4/+hwUHsUKiCRKHZ8ZwSQ7/LSdGfEMo4bngW/kBSWjFMJAXZr7vA2duRcTknxl1/alj5NubSNSmGTG1SQKcNh8D8OjLWbJBBjfAHlC3u90FrvGT4FHNCOhbMVeNbxxIW5tM7kjlsqbmWBYODmwhGTEYOwAIE0y/Ba98IScsqSVpPcwifJWvBf0gp1rAMnWD+FBp1b1Po3Ta74++IExM61mZLywuraNq6Mi7/aJ3bA87tyKAmsRMBiIolEVKjwwopAPbio7bcQlPEN9nUdOF1pEBVqj029eNRUg2yrjf2lfVNVTyGdZr8GeSJv91Eor1LhRfOYC4KJwgvYJnjmY91k+Dee8dsWxAAUk30zh9So4gV6eClTiibbNIfTenpSKOFDs9TC8omw9ychRLGmZw02Huog059v+5MUXTV5iZD58EAv4hXQHm0RbmT1H0rJC4Jdhpbqu4jUrexw7UkHv3pUK8hJMviiUt08ImvLNZ1Road3eALZeVUtjxtgLzzM9J06Po9qI9WhrMXS5RAV/2oKsljJ4P6YceKgd8MJ5dnH542XgUiX9y5ip59E3bgpaK5gtpfo2zPFffagJk3f2eJqkYyrQ++gLofAgDshXGDIBUb0nsXjZKlxeKtUfGwz92Wl4e3oqOtdGeG9xSRBwm8npRrfJu/kOpMgBGIpaV4IqOKL2Z57bFKQOnP2C/u6hVCpMqfuowCzo5XIVJNetaZJez3UTtTfM75vVQB52dLnBtrxLXZsxOdW/YuSA9iRn3K/doSv0N70TLEklFpJxm+nceEzNQn59Q1rWHOB620EsFeq6Lfz4CDQ2AadpkSVDltylCR135rKKfmlKffXL9UG1Ndke/0eal/xWWiR5OefKPN9aAiOrNegr6+FovFCHJZxvL9Ub5GtghgmuhHVVpkhFyfN/DF1yaqEfUb4XnUAxDdQVUSq/cKIPtBFfKdV9qmL5/fGjcAhOK4dttllZcRyZU5HXAEzJyplS2cAGSDidYhli/G9bqtWnc0roM/Rn9JZHj9buQX/hof/iNuJWUqCYoA6b912wmzEu2DSRaj+AIn/at0wLTKxTVCjAxN1zdSsRjb5sDgKt5f3zNRn/vuxqsWOhCSVsxloqyT01+jRIORCds03zNCoB0cuf37u/Nf8tpRgPlrB03+75Wo3MoAbsGHlE7Eq08p98fvP1FFd2S8w2H+luQj+fNLlRYEk/y31PK8kkTrxl+m24VkxqUX0pKX5SMhkyPnhUSDkLi2dlD/C+g+II+7Eh1/yydL02Iy824R6JDEBehd+3PMIpOf00YtXChJZqMwGXHN+vZ6UvEltk1szhtrilJZ1zxAlzCjVzhV/7XVSPJLK0gGpyDZCK4oL/7Wpx9xuqiActZPeNRLp4BfyqeQwbd/9NY6Au0ySxK9nt7ur4aySkkVKa/59rw4J1mlWSIEAm7Ncpc18SGa2dw4Y+9pX/9v+xzUVrr8MqnpmH5/y0ybeg05lTIxDfNrXImdVTNFU5q++o1Hb3MO4dBi+3yMC3XoHBGFShDvLwcBB733Q89TjbUYT9hmdycApU5lgqE82f0dy6K977dmlq4/RrcLM6kTq8T1xMq/UWfMVAnG1JGXNOOw1hDFTNVgGx7tJOXCOCK2Taypy+izDHcuU1Qsc7t5uipoV4dpZFtIBNg+/RWPeJN2N6HD75x/v1XR612JLrCP5jnQxCVONHjjXUX7mZxognjWO2OWfWBH3rIDNUFn+4d04TlI9Gmxe99cHtOSMokwJ7EgNOGWQJx3d3psTqp3UNeDqUonwddXUeEZAScUqDN5ty6zaJ1A3TCsbKRcB54+dVM9qTe/wCIo/ykPc64busrhrRaxKHijU/atF2N6Lps60VCK1jYdaQO4zQvbW5npCBRBzkPvHZHcKFvWjMEvCcRl8kBSS0Ff0gSY7cE47fUByGa/urtVd/GQdJSLbgRHlDiCFA6etHPskQ71dYK1ZvG9A7E/n+k5tRBCsdu/xbeZ2RlTFNrOG11xX9hcQR6jNuKvMFNI5BiAGZkzzvQfrkDU3ONiEw9/oCPQ2wPTaHe72Zc98Lcd46NKx7RsW0atrIIy7zw8Xaqu3fmjTETIBmILRsH6aJtGnOVVTGYcZhZEdDyccwMM9I3WXRUrxAJqwGGP1SwTm1yYEyPZQ1qbXsB8POfe7VKokvJN1IVzpcgfMAbcm9V8xvdcycnDAvAJ0CQcPlR6TRNxCmrqMglyTUc6eRnTISLxcQ2zFZCzxYN";
const BUNDLE_ID: &str = "com.openai.chat";

pub struct IosOpenApi {
    client: reqwest::Client,
    access_token: String,
    expires_time: Mutex<Option<std::time::SystemTime>>,
}

impl IosOpenApi {
    async fn device_check(&mut self) -> OpenAIResult<()> {
        use std::time::{Instant, UNIX_EPOCH};

        let mut last_checked = self.expires_time.lock().await;

        let expired = if let Some(expired_time) = *last_checked {
            let expired_time_timestamp = expired_time
                .duration_since(UNIX_EPOCH)
                .context(ApiError::SystemTimeExceptionError)?
                .as_secs();

            // Confirm half an hour in advance
            let now_timestamp = Instant::now().elapsed().as_secs().add(3000);

            now_timestamp > expired_time_timestamp
        } else {
            false
        };

        if expired {
            let payload = DeviceCheckPayloadBuilder::default()
                .device_token(DEVICE_TOKEN.to_string())
                .bundle_id(BUNDLE_ID.to_string())
                .build()?;

            let resp = self
                .client
                .post(format!(
                    "{}{}",
                    URL_IOS_CHAT_BASE, "/backend-api/devicecheck"
                ))
                .bearer_auth(&self.access_token)
                .json(&payload)
                .send()
                .await?;

            let status = resp.status();
            if status.is_success() {
                if let Some(cookie) = resp.cookies().find(|ele| ele.name().eq("_devicecheck")) {
                    debug!("cookie value: {:?}", cookie.value());
                    debug!("cookie expires: {:?}", cookie.expires());
                    *last_checked = cookie.expires();
                }
            }

            if status.is_client_error() {
                debug!("client error: {:?}", resp.text().await?);
                anyhow::bail!(ApiError::FailedAuthenticationError)
            }
            Ok(())
        } else {
            anyhow::bail!(ApiError::FailedGetCookie)
        }
    }
}

impl Api for IosOpenApi {
    fn get_title(&self, message_id: String) -> OpenAIResult<super::model::TitleData> {
        todo!()
    }

    fn get_models(&self) -> OpenAIResult<super::model::ModelsData> {
        todo!()
    }
}

pub struct IosOpenApiBuilder {
    client_builder: reqwest::ClientBuilder,
    api: IosOpenApi,
}

impl<'a> IosOpenApiBuilder {
    pub fn proxy(mut self, proxy: Option<Proxy>) -> Self {
        if let Some(proxy) = proxy {
            self.client_builder = self.client_builder.proxy(proxy);
        } else {
            self.client_builder = self.client_builder.no_proxy();
        }
        self
    }

    pub fn client_timeout(mut self, timeout: Duration) -> Self {
        self.client_builder = self.client_builder.timeout(timeout);
        self
    }

    pub fn cookie_store(mut self, store: bool) -> Self {
        self.client_builder = self.client_builder.cookie_store(store);
        self
    }

    pub fn access_token(mut self, access_token: String) -> Self {
        self.api.access_token = access_token;
        self
    }

    pub fn build(mut self) -> IosOpenApi {
        self.api.client = self.client_builder.build().expect("ClientBuilder::build()");
        self.api
    }

    pub fn builder() -> IosOpenApiBuilder {
        let mut req_headers = HeaderMap::new();
        req_headers.insert(
            reqwest::header::USER_AGENT,
            HeaderValue::from_static(HEADER_UA),
        );
        req_headers.insert(header::HOST, HeaderValue::from_static(HEADER_HOST));
        req_headers.insert("OAI-Client", HeaderValue::from_static(HEADER_OAI_CLIENT));
        req_headers.insert(
            "OAI-Device-Id",
            HeaderValue::from_static(HEADER_OAI_DEVICE_ID),
        );

        let client = reqwest::ClientBuilder::new()
            .cookie_store(true)
            .default_headers(req_headers);

        IosOpenApiBuilder {
            client_builder: client,
            api: IosOpenApi {
                client: reqwest::Client::new(),
                expires_time: Mutex::new(None),
                access_token: String::new(),
            },
        }
    }
}

#[derive(Serialize, Deserialize, Debug, derive_builder::Builder)]
struct DeviceCheckPayload {
    device_token: String,
    bundle_id: String,
}
