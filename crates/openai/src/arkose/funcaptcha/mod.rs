mod breaker;
pub mod model;
pub mod solver;

use self::model::{Challenge, ConciseChallenge, FunCaptcha, RequestChallenge};
use super::{crypto, ArkoseSolverContext};
use crate::arkose::error::ArkoseError;
use crate::arkose::funcaptcha::model::SubmitChallenge;
use crate::{now_duration, warn};
use reqwest::header;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

type FunResult<T, E = super::error::ArkoseError> = Result<T, E>;

pub async fn start_challenge(ctx: &ArkoseSolverContext) -> FunResult<Session> {
    let value = ctx.arkose_token.value();
    let fields: Vec<&str> = value.split('|').collect();

    let session_token = fields
        .get(0)
        .ok_or_else(|| ArkoseError::InvalidArkoseToken(value.to_owned()))?
        .to_string();

    let sid = fields
        .get(1)
        .ok_or_else(|| ArkoseError::InvalidArkoseToken(value.to_owned()))?
        .split('=')
        .nth(1)
        .unwrap_or_default()
        .to_owned();

    let referer = format!(
        "{}/fc/assets/ec-game-core/game-core/1.18.0/standard/index.html?session={}",
        "https://client-api.arkoselabs.com",
        value.replace("|", "&")
    );
    let mut headers = header::HeaderMap::new();

    // Try set user agent
    if let Some(ref user_agent) = ctx.user_agent {
        headers.insert(
            header::USER_AGENT,
            header::HeaderValue::from_str(user_agent).map_err(ArkoseError::InvalidHeader)?,
        );
    }

    headers.insert(
        header::REFERER,
        header::HeaderValue::from_bytes(referer.as_bytes()).map_err(ArkoseError::InvalidHeader)?,
    );

    headers.insert(header::DNT, header::HeaderValue::from_static("1"));

    let mut session = Session {
        origin: ctx.typed.origin_url(),
        sid,
        session_token,
        funcaptcha: None,
        challenge: None,
        client: ctx.client.clone(),
        game_type: 0,
        headers,
    };

    // Start funcaptcha challenge
    let concise_challenge = session.request_challenge().await?;

    let images = session
        .download_image_to_base64(&concise_challenge.urls)
        .await?;

    // Warn if images count >= 5
    if concise_challenge.urls.len() >= 5 {
        warn!(
            "Funcaptcha images count >= 5, your features are already in high risk control status"
        );
    }

    let funcaptcha_list = images
        .into_iter()
        .map(|image| FunCaptcha {
            image,
            instructions: concise_challenge.instructions.clone(),
            game_variant: concise_challenge.game_variant.clone(),
        })
        .collect::<Vec<FunCaptcha>>();

    session.funcaptcha = Some(Arc::new(funcaptcha_list));
    Ok(session)
}

#[derive(Debug)]
pub struct Session {
    origin: &'static str,
    client: reqwest::Client,
    sid: String,
    session_token: String,
    headers: header::HeaderMap,
    #[allow(dead_code)]
    challenge: Option<Challenge>,
    funcaptcha: Option<Arc<Vec<FunCaptcha>>>,
    game_type: u32,
}

impl Session {
    async fn global_callback(&self) -> FunResult<()> {
        let _ = self
            .client
            .get(format!(
                "{}/fc/gc/?token={}",
                "https://client-api.arkoselabs.com", self.session_token
            ))
            .send()
            .await?
            .error_for_status()?;
        Ok(())
    }

    async fn callback(
        &self,
        action: &str,
        category: &str,
        game_token: Option<&str>,
        game_type: Option<i32>,
    ) -> FunResult<()> {
        #[derive(Serialize)]
        struct GlobalCallback<'a> {
            sid: &'a str,
            session_token: &'a str,
            analytics_tier: u32,
            #[serde(rename = "disableCookies")]
            disable_cookies: bool,
            game_token: Option<&'a str>,
            game_type: Option<i32>,
            render_type: &'a str,
            category: &'a str,
            action: &'a str,
        }

        let global_callback = GlobalCallback {
            sid: &self.sid,
            session_token: &self.session_token,
            analytics_tier: 40,
            disable_cookies: false,
            game_token,
            game_type,
            render_type: "canvas",
            category,
            action,
        };

        let form = serde_urlencoded::to_string(global_callback)?;

        self.client
            .post(format!("{}/fc/a/", self.origin))
            .header(
                header::CONTENT_TYPE,
                "application/x-www-form-urlencoded; charset=UTF-8",
            )
            .body(form)
            .send()
            .await?;

        Ok(())
    }

    #[inline]
    async fn request_challenge(&mut self) -> FunResult<ConciseChallenge> {
        // Global Callback
        self.global_callback().await?;
        // Init Callback
        self.callback(
            &format!(
                "{}/v2/2.3.4/enforcement.c70df15cb97792b18c2f4978b68954a0.html",
                self.origin
            ),
            "Site URL",
            None,
            None,
        )
        .await?;

        // Init challenge request
        let challenge_request = RequestChallenge {
            sid: &self.sid,
            token: &self.session_token,
            analytics_tier: 40,
            render_type: "canvas",
            lang: "en-us",
            is_audio_game: false,
            api_breaker_version: "green",
        };

        let form = serde_urlencoded::to_string(challenge_request)?;

        let mut headers = self.headers.clone();
        headers.insert("X-NewRelic-Timestamp", get_time_stamp()?.parse()?);
        headers.insert(
            header::CONTENT_TYPE,
            "application/x-www-form-urlencoded; charset=UTF-8".parse()?,
        );

        let challenge = self
            .client
            .post(format!("{}/fc/gfct/", self.origin))
            .body(form)
            .headers(headers)
            .send()
            .await?
            .error_for_status()?
            .json::<Challenge>()
            .await?;

        // Game loaded callback
        self.callback(
            "game loaded",
            "loaded",
            Some(challenge.session_token.as_str()),
            Some(challenge.game_data.game_type),
        )
        .await?;

        // User clicked verify callback
        self.callback(
            "user clicked verify",
            "begin app",
            Some(challenge.session_token.as_str()),
            Some(challenge.game_data.game_type),
        )
        .await?;

        // Set game type
        self.game_type = challenge.game_data.game_type as u32;

        // Build concise challenge
        let (game_type, challenge_urls, key, game_variant) = {
            let game_variant = if challenge.game_data.instruction_string.is_empty() {
                &challenge.game_data.game_variant
            } else {
                &challenge.game_data.instruction_string
            };
            (
                "image",
                &challenge.game_data.custom_gui.challenge_imgs,
                format!(
                    "{}.instructions-{game_variant}",
                    challenge.game_data.game_type
                ),
                game_variant.to_owned(),
            )
        };

        // Remove html tags
        let remove_html_tags = |input: &str| {
            let re = regex::Regex::new(r"<[^>]*>").expect("invalid regex");
            re.replace_all(input, "").to_string()
        };

        // Get html instructions
        let html_instructions = challenge.string_table.get(&key).ok_or_else(|| {
            warn!("unknown challenge type: {challenge:#?}");
            ArkoseError::UnknownChallengeTypeKey(key)
        })?;

        let concise_challenge = ConciseChallenge {
            game_type,
            game_variant,
            urls: challenge_urls.to_vec(),
            instructions: remove_html_tags(html_instructions),
        };
        self.challenge = Some(challenge);

        Ok(concise_challenge)
    }

    pub async fn submit_answer(&self, answers: Vec<i32>) -> FunResult<()> {
        let mut answer_index = Vec::with_capacity(answers.len());
        let c_ui = &self
            .challenge
            .as_ref()
            .ok_or_else(|| ArkoseError::UnknownChallenge)?
            .game_data
            .custom_gui;

        for answer in answers {
            let answer = breaker::hanlde_answer(
                c_ui.api_breaker_v2_enabled != 0,
                self.game_type,
                &c_ui.api_breaker,
                answer,
            )?
            .to_string();
            answer_index.push(answer)
        }

        let answer = answer_index.join(",");
        let submit = SubmitChallenge {
            session_token: &self.session_token,
            sid: &self.sid,
            game_token: &self
                .challenge
                .as_ref()
                .ok_or_else(|| ArkoseError::UnknownChallenge)?
                .challenge_id,
            guess: &crypto::encrypt(&format!("[{answer}]"), &self.session_token)?,
            render_type: "canvas",
            analytics_tier: 40,
            bio: "eyJtYmlvIjoiMTUwLDAsMTE3LDIzOTszMDAsMCwxMjEsMjIxOzMxNywwLDEyNCwyMTY7NTUwLDAsMTI5LDIxMDs1NjcsMCwxMzQsMjA3OzYxNywwLDE0NCwyMDU7NjUwLDAsMTU1LDIwNTs2NjcsMCwxNjUsMjA1OzY4NCwwLDE3MywyMDc7NzAwLDAsMTc4LDIxMjs4MzQsMCwyMjEsMjI4OzI2MDY3LDAsMTkzLDM1MTsyNjEwMSwwLDE4NSwzNTM7MjYxMDEsMCwxODAsMzU3OzI2MTM0LDAsMTcyLDM2MTsyNjE4NCwwLDE2NywzNjM7MjYyMTcsMCwxNjEsMzY1OzI2MzM0LDAsMTU2LDM2NDsyNjM1MSwwLDE1MiwzNTQ7MjYzNjcsMCwxNTIsMzQzOzI2Mzg0LDAsMTUyLDMzMTsyNjQ2NywwLDE1MSwzMjU7MjY0NjcsMCwxNTEsMzE3OzI2NTAxLDAsMTQ5LDMxMTsyNjY4NCwxLDE0NywzMDc7MjY3NTEsMiwxNDcsMzA3OzMwNDUxLDAsMzcsNDM3OzMwNDY4LDAsNTcsNDI0OzMwNDg0LDAsNjYsNDE0OzMwNTAxLDAsODgsMzkwOzMwNTAxLDAsMTA0LDM2OTszMDUxOCwwLDEyMSwzNDk7MzA1MzQsMCwxNDEsMzI0OzMwNTUxLDAsMTQ5LDMxNDszMDU4NCwwLDE1MywzMDQ7MzA2MTgsMCwxNTUsMjk2OzMwNzUxLDAsMTU5LDI4OTszMDc2OCwwLDE2NywyODA7MzA3ODQsMCwxNzcsMjc0OzMwODE4LDAsMTgzLDI3MDszMDg1MSwwLDE5MSwyNzA7MzA4ODQsMCwyMDEsMjY4OzMwOTE4LDAsMjA4LDI2ODszMTIzNCwwLDIwNCwyNjM7MzEyNTEsMCwyMDAsMjU3OzMxMzg0LDAsMTk1LDI1MTszMTQxOCwwLDE4OSwyNDk7MzE1NTEsMSwxODksMjQ5OzMxNjM0LDIsMTg5LDI0OTszMTcxOCwxLDE4OSwyNDk7MzE3ODQsMiwxODksMjQ5OzMxODg0LDEsMTg5LDI0OTszMTk2OCwyLDE4OSwyNDk7MzIyODQsMCwyMDIsMjQ5OzMyMzE4LDAsMjE2LDI0NzszMjMxOCwwLDIzNCwyNDU7MzIzMzQsMCwyNjksMjQ1OzMyMzUxLDAsMzAwLDI0NTszMjM2OCwwLDMzOSwyNDE7MzIzODQsMCwzODgsMjM5OzMyNjE4LDAsMzkwLDI0NzszMjYzNCwwLDM3NCwyNTM7MzI2NTEsMCwzNjUsMjU1OzMyNjY4LDAsMzUzLDI1NzszMjk1MSwxLDM0OCwyNTc7MzMwMDEsMiwzNDgsMjU3OzMzNTY4LDAsMzI4LDI3MjszMzU4NCwwLDMxOSwyNzg7MzM2MDEsMCwzMDcsMjg2OzMzNjUxLDAsMjk1LDI5NjszMzY1MSwwLDI5MSwzMDA7MzM2ODQsMCwyODEsMzA5OzMzNjg0LDAsMjcyLDMxNTszMzcxOCwwLDI2NiwzMTc7MzM3MzQsMCwyNTgsMzIzOzMzNzUxLDAsMjUyLDMyNzszMzc1MSwwLDI0NiwzMzM7MzM3NjgsMCwyNDAsMzM3OzMzNzg0LDAsMjM2LDM0MTszMzgxOCwwLDIyNywzNDc7MzM4MzQsMCwyMjEsMzUzOzM0MDUxLDAsMjE2LDM1NDszNDA2OCwwLDIxMCwzNDg7MzQwODQsMCwyMDQsMzQ0OzM0MTAxLDAsMTk4LDM0MDszNDEzNCwwLDE5NCwzMzY7MzQ1ODQsMSwxOTIsMzM0OzM0NjUxLDIsMTkyLDMzNDsiLCJ0YmlvIjoiIiwia2JpbyI6IiJ9",
        };
        let form = serde_urlencoded::to_string(submit)?;

        let mut headers = self.headers.clone();
        let pwd = format!("REQUESTED{}ID", self.session_token);
        let request_id = crypto::encrypt("{{\"sc\":[147,307]}}", &pwd)?;
        headers.insert(header::DNT, header::HeaderValue::from_static("1"));
        headers.insert("X-Requested-ID", request_id.parse()?);
        headers.insert("X-NewRelic-Timestamp", get_time_stamp()?.parse()?);
        headers.insert(
            header::CONTENT_TYPE,
            "application/x-www-form-urlencoded; charset=UTF-8".parse()?,
        );

        let resp = self
            .client
            .post(format!("{}/fc/ca/", self.origin))
            .headers(headers)
            .body(form)
            .send()
            .await?
            .error_for_status()?;

        #[derive(Deserialize, Default, Debug)]
        #[serde(default)]
        struct Response {
            response: Option<String>,
            solved: bool,
            incorrect_guess: Option<String>,
            score: i32,
            error: Option<String>,
        }

        let resp = resp
            .json::<Response>()
            .await
            .map_err(ArkoseError::DeserializeError)?;

        if let Some(error) = resp.error {
            return Err(ArkoseError::FuncaptchaSubmitError(error));
        }

        if !resp.solved {
            warn!("funcaptcha not solved: {:#?}", self.challenge);
            return Err(ArkoseError::FuncaptchaNotSolvedError(
                resp.incorrect_guess.unwrap_or_default(),
            ));
        }

        Ok(())
    }

    async fn download_image_to_base64(&self, urls: &Vec<String>) -> FunResult<Vec<String>> {
        use base64::{engine::general_purpose, Engine as _};
        let mut b64_imgs = Vec::new();
        for url in urls {
            let bytes = self
                .client
                .get(url)
                .headers(self.headers.clone())
                .send()
                .await?
                .bytes()
                .await?;
            let b64 = general_purpose::STANDARD.encode(bytes);
            b64_imgs.push(b64);
        }

        Ok(b64_imgs)
    }

    pub fn funcaptcha(&self) -> Option<&Arc<Vec<FunCaptcha>>> {
        self.funcaptcha.as_ref()
    }
}

fn get_time_stamp() -> FunResult<String> {
    let since_the_epoch = now_duration()?;
    Ok(since_the_epoch.as_millis().to_string())
}
