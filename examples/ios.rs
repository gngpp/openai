use std::collections::HashMap;

use serde::{Deserialize, Serialize};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let email = std::env::var("EMAIL")?;
    let password = std::env::var("PASSWORD")?;
    let store = openai::token::FileStore::default();
    let mut auth = openai::oauth::OpenOAuth0Builder::builder()
        .email(email)
        .password(password)
        .cache(true)
        .cookie_store(true)
        .token_store(store)
        .client_timeout(std::time::Duration::from_secs(20))
        .build();
    let token = auth.do_get_access_token().await?;
    println!("AccessToken: {}", token.access_token());
    println!("RefreshToken: {}", token.refresh_token());
    println!("Profile: {:#?}", token.profile());

    let client = reqwest::Client::new();
    let resp = client.get("https://ios.chat.openai.com/backend-api/conversations?offset=0&limit=20&order=updated&expand=true")
    .header("OAI-Client-Type","ios")
    .header(reqwest::header::AUTHORIZATION,token.get_bearer_access_token())
    .header(reqwest::header::USER_AGENT,"ChatGPT/1.2023.21 (iOS 16.2; iPad11,1; build 623)")
    .header("Cookie", "_devicecheck=user-N9D93ttsgIr3LItSzNNyAfiN:1685522780-9Ud0OEcUPerkXnoQ%2BCB2ACbsb2KUHfmqx52jg9q%2FZjc%3D")
    .header(reqwest::header::CONTENT_TYPE, "application/json")
    .send().await?;

    if resp.status().is_success() {
        let bytes = resp.bytes().await?;
        let data: Conversations = serde_json::from_slice(&bytes)?;
        println!("{}", serde_json::to_string_pretty(&data)?)
    } else {
        let bytes = resp.bytes().await?;
        println!("{}", String::from_utf8(bytes.to_vec())?)
    }
    Ok(())
}

#[derive(Serialize, Deserialize)]
struct Mapping {
    id: String,
    parent: Option<String>,
    message: Option<Message>,
    children: Vec<String>,
}

#[derive(Serialize, Deserialize)]
pub struct Message {
    id: String,
    create_time: f64,
    status: String,
}

#[derive(Serialize, Deserialize)]
struct Content {
    content_type: String,
    parts: Vec<String>,
}

#[derive(Serialize, Deserialize)]
struct Items {
    id: String,
    title: String,
    create_time: String,
    update_time: String,
    current_node: String,
    mapping: HashMap<String, Mapping>,
}

#[derive(Serialize, Deserialize)]
struct Conversations {
    items: Vec<Items>,
    total: i64,
    limit: i64,
    offset: i64,
    has_missing_conversations: bool,
}

// use std::pin::Pin;
// use std::task::{Context, Poll};

// use futures_util::stream::Stream;
// use reqwest::{Client, Response};
// use serde_json::Value;

// use serde::{Deserialize, Serialize};

// #[derive(Serialize, Deserialize, Default)]
// pub struct GptStreamConfig {
//     model: Option<String>,
//     messages: Vec<Message>,
//     temperature: Option<f64>,
//     top_p: Option<f64>,
//     n: Option<usize>,
//     stream: Option<bool>,
//     presence_penalty: Option<f64>,
//     frequency_penalty: Option<f64>,
// }

// #[derive(Serialize, Deserialize)]
// pub struct Message {
//     role: String,
//     content: String,
// }

// pub struct OpenAIStream {
//     api_key: String,
// }

// pub struct GptStream {
//     response: Pin<Box<dyn Stream<Item = Result<bytes::Bytes, reqwest::Error>> + Send>>,
//     buffer: String,
//     first_chunk: bool,
// }

// impl OpenAIStream {
//     pub fn new(api_key: String) -> Self {
//         OpenAIStream { api_key }
//     }

//     pub async fn gpt_stream(&self, input: &str) -> Result<GptStream, String> {
//         let api_url = "https://api.openai.com/v1/chat/completions";

//         let config: GptStreamConfig = match serde_json::from_str(input) {
//             Ok(config) => config,
//             Err(error) => return Err(format!("JSON parsing error: {}", error)),
//         };

//         let payload = serde_json::json!({
//             "model": config.model.unwrap_or("gpt-3.5-turbo".to_string()),
//             "messages": config.messages,
//             "temperature": config.temperature.unwrap_or(1.0),
//             "top_p": config.top_p.unwrap_or(1.0),
//             "n": config.n.unwrap_or(1),
//             "stream": true,
//             "presence_penalty": config.presence_penalty.unwrap_or(0.0),
//             "frequency_penalty": config.frequency_penalty.unwrap_or(0.0)
//         });

//         let client = Client::new();
//         let response: Response = match client
//             .post(api_url)
//             .header("Content-Type", "application/json")
//             .header("Authorization", format!("Bearer {}", self.api_key))
//             .json(&payload)
//             .send()
//             .await
//         {
//             Ok(response) => response,
//             Err(error) => return Err(format!("API request error: {}", error)),
//         };

//         if response.status().is_success() {
//             Ok(GptStream {
//                 response: Box::pin(response.bytes_stream()),
//                 buffer: String::new(),
//                 first_chunk: true,
//             })
//         } else {
//             let error_text = response
//                 .text()
//                 .await
//                 .unwrap_or_else(|_| String::from("Unknown error"));
//             Err(format!("API request error: {}", error_text))
//         }
//     }
// }

// impl Stream for GptStream {
//     type Item = String;

//     fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
//         loop {
//             match self.response.as_mut().poll_next(cx) {
//                 Poll::Ready(Some(Ok(chunk))) => {
//                     let mut utf8_str = String::from_utf8_lossy(&chunk).to_string();

//                     if self.first_chunk {
//                         let lines: Vec<&str> = utf8_str.lines().collect();
//                         utf8_str = if lines.len() >= 2 {
//                             lines[lines.len() - 2].to_string()
//                         } else {
//                             utf8_str.clone()
//                         };
//                         self.first_chunk = false;
//                     }

//                     let trimmed_str = utf8_str.trim_start_matches("data: ");

//                     let json_result: Result<Value, _> = serde_json::from_str(trimmed_str);

//                     match json_result {
//                         Ok(json) => {
//                             if let Some(choices) = json.get("choices") {
//                                 if let Some(choice) = choices.get(0) {
//                                     if let Some(content) =
//                                         choice.get("delta").and_then(|delta| delta.get("content"))
//                                     {
//                                         if let Some(content_str) = content.as_str() {
//                                             self.buffer.push_str(content_str);
//                                             let output = self.buffer.replace("\\n", "\n");
//                                             return Poll::Ready(Some(output));
//                                         }
//                                     }
//                                 }
//                             }
//                         }
//                         Err(_) => {}
//                     }
//                 }
//                 Poll::Ready(Some(Err(error))) => {
//                     eprintln!("Error in stream: {:?}", error);
//                     return Poll::Ready(None);
//                 }
//                 Poll::Ready(None) => {
//                     return Poll::Ready(None);
//                 }
//                 Poll::Pending => {
//                     return Poll::Pending;
//                 }
//             }
//         }
//     }
// }
