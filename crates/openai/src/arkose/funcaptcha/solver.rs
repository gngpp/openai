use std::str::FromStr;

use hyper::header;
use serde::{Deserialize, Serialize};
use typed_builder::TypedBuilder;

use crate::{arkose::error::ArkoseError, with_context};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Solver {
    Yescaptcha,
    Capsolver,
    Fcsrv,
}

impl Default for Solver {
    fn default() -> Self {
        Self::Fcsrv
    }
}

impl FromStr for Solver {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "yescaptcha" => Ok(Self::Yescaptcha),
            "capsolver" => Ok(Self::Capsolver),
            "fcsrv" => Ok(Self::Fcsrv),
            _ => anyhow::bail!("Only support `yescaptcha` / `capsolver` / `fcsrv` solver"),
        }
    }
}

impl ToString for Solver {
    fn to_string(&self) -> String {
        match self {
            Self::Yescaptcha => "yescaptcha".to_string(),
            Self::Capsolver => "capsolver".to_string(),
            Self::Fcsrv => "fcsrv".to_string(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ArkoseSolver {
    pub solver: Solver,
    pub limit: usize,
    client_key: String,
    endpoint: String,
}

impl ArkoseSolver {
    pub fn new(solver: Solver, client_key: String, endpoint: Option<String>, limit: usize) -> Self {
        let endpoint = match solver {
            Solver::Yescaptcha => {
                endpoint.unwrap_or("https://api.yescaptcha.com/createTask".to_string())
            }
            Solver::Capsolver => {
                endpoint.unwrap_or("https://api.capsolver.com/createTask".to_string())
            }
            Solver::Fcsrv => endpoint.unwrap_or("http://127.0.0.1:8000/task".to_string()),
        };
        Self {
            solver,
            client_key,
            endpoint,
            limit,
        }
    }
}

#[derive(Deserialize, Default, Debug)]
#[serde(default)]
struct TaskResp0 {
    #[serde(rename = "errorId")]
    error_id: i32,
    #[serde(rename = "errorCode")]
    error_code: String,
    #[serde(rename = "errorDescription")]
    error_description: Option<String>,
    status: String,
    solution: SolutionResp,
    #[serde(rename = "taskId")]
    task_id: String,
}

#[derive(Deserialize, Default, Debug)]
#[serde(default)]
struct TaskResp1 {
    error: Option<String>,
    solve: bool,
    objects: Vec<i32>,
}

#[derive(Deserialize, Default, Debug)]
#[serde(default)]
struct SolutionResp {
    objects: Vec<i32>,
}

#[derive(Serialize, Debug)]
struct ReqBody0<'a> {
    #[serde(rename = "clientKey")]
    client_key: &'a str,
    task: ReqTask0<'a>,
    #[serde(rename = "softID", skip_serializing_if = "Option::is_none")]
    soft_id: Option<&'static str>,
    #[serde(rename = "appId", skip_serializing_if = "Option::is_none")]
    app_id: Option<&'static str>,
}

#[derive(Serialize, Debug)]
struct ReqBody1<'a> {
    api_key: Option<&'a str>,
    #[serde(rename = "type")]
    typed: &'a str,
    images: Option<Vec<&'a String>>,
}

#[derive(Serialize, Debug)]
struct ReqTask0<'a> {
    #[serde(rename = "type")]
    type_field: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    image: Option<&'a String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    images: Option<Vec<&'a String>>,
    question: &'a str,
}

#[derive(TypedBuilder)]
pub struct SubmitSolver<'a> {
    arkose_solver: &'a ArkoseSolver,
    #[builder(setter(into), default)]
    image: Option<&'a String>,
    #[builder(setter(into), default)]
    images: Option<Vec<&'a String>>,
    question: &'a String,
}

pub async fn submit_task(submit_task: SubmitSolver<'_>) -> anyhow::Result<Vec<i32>> {
    let body = match submit_task.arkose_solver.solver {
        Solver::Yescaptcha => {
            let body = ReqBody0 {
                client_key: &submit_task.arkose_solver.client_key,
                task: ReqTask0 {
                    type_field: "FunCaptchaClassification",
                    image: submit_task.image,
                    images: submit_task.images,
                    question: &submit_task.question,
                },
                soft_id: Some("26299"),
                app_id: None,
            };
            serde_json::to_string(&body)?
        }
        Solver::Capsolver => {
            let body = ReqBody0 {
                client_key: &submit_task.arkose_solver.client_key,
                task: ReqTask0 {
                    type_field: "FunCaptchaClassification",
                    image: submit_task.image,
                    images: submit_task.images,
                    question: &submit_task.question,
                },
                soft_id: None,
                app_id: Some("60632CB0-8BE8-41D3-808F-60CC2442F16E"),
            };
            serde_json::to_string(&body)?
        }
        Solver::Fcsrv => {
            let body = ReqBody1 {
                api_key: Some(&submit_task.arkose_solver.client_key),
                typed: &submit_task.question,
                images: submit_task.images,
            };
            serde_json::to_string(&body)?
        }
    };

    let resp = with_context!(arkose_client)
        .post(&submit_task.arkose_solver.endpoint)
        .header(header::CONTENT_TYPE, mime::APPLICATION_JSON.as_ref())
        .body(body)
        .send()
        .await?;

    match resp.error_for_status_ref() {
        Ok(_) => {
            // Task Respone
            match submit_task.arkose_solver.solver {
                Solver::Yescaptcha | Solver::Capsolver => {
                    let task = resp.json::<TaskResp0>().await?;
                    // If error
                    if let Some(error_description) = task.error_description {
                        anyhow::bail!(ArkoseError::SolverTaskError(error_description))
                    }

                    Ok(task.solution.objects)
                }
                Solver::Fcsrv => {
                    let task = resp.json::<TaskResp1>().await?;
                    // If error
                    if let Some(error) = task.error {
                        anyhow::bail!(ArkoseError::SolverTaskError(error))
                    }

                    Ok(task.objects)
                }
            }
        }
        Err(_) => {
            let body = resp.text().await?;
            anyhow::bail!(ArkoseError::SolverTaskError(body))
        }
    }
}
