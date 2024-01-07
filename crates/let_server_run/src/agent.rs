use anyhow::Result;
use serde::Deserialize;
use serde_json::json;

#[derive(Deserialize, Debug)]
pub struct Config {
    pub token: String,
}

pub async fn go(config: Config) {
    let agent = Agent::from(&config);
    loop {
        use tokio::time::{sleep, Duration};
        let job = agent.fetch_job().await;
        log::debug!("fetch job: {:?}", job);
        match job {
            Ok(Some(job)) => {
                tokio::spawn(execute(agent.clone(), job));
                sleep(Duration::from_secs(1)).await;
            }
            Ok(None) => {
                sleep(Duration::from_secs(60)).await;
            }
            Err(err) => {
                log::error!("failed to fetch job: {:?}", err);
                sleep(Duration::from_secs(60)).await;
            }
        }
    }
}

async fn execute(agent: Agent, job: Job) -> Result<(), AgentError> {
    todo!("cannot execute {}", job.message);
}

#[derive(Deserialize, Debug)]
struct AgentErrorHint {
    error: String,
    message: String,
}

#[derive(thiserror::Error, Debug)]
enum AgentError {
    #[error("client is not valid")]
    ClientNotValid,
    #[error("limited exceeded: {0}")]
    LimitExceeded(String),
    #[error("limited exceeded: {0}")]
    NotFound(String),
    #[error("server error: {0}")]
    ServerError(String),
    #[error("invalid query parameters: {0}")]
    InvalidQueryParam(String),
    #[error("unknown error: {0}")]
    UnknownError(String),
}

impl From<&AgentErrorHint> for AgentError {
    fn from(value: &AgentErrorHint) -> AgentError {
        let msg = value.message.clone();
        match &value.error as &str {
            "LimitExceeded" => AgentError::LimitExceeded(msg),
            "NotFound" => AgentError::NotFound(msg),
            "ServerError" => AgentError::ServerError(msg),
            "InvalidQueryParam" => AgentError::InvalidQueryParam(msg),
            _ => AgentError::UnknownError(format!("{}: {}", value.error, value.message)),
        }
    }
}

#[derive(Deserialize, Debug)]
struct Job {
    id: String,
    message: String,
}

#[derive(Clone)]
struct Agent {
    client: Option<reqwest::Client>,
}

impl From<&Config> for Agent {
    fn from(value: &Config) -> Self {
        Agent::new(value)
    }
}

#[allow(dead_code)]
enum JobStatus {
    Succeed,
    Fail,
    Running,
}

impl Agent {
    fn new(config: &Config) -> Self {
        use reqwest::header::HeaderValue;
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            "Authorization",
            HeaderValue::from_str(&format!("Bearer {}", config.token))
                .unwrap_or(HeaderValue::from_static("")),
        );
        let client = match reqwest::Client::builder().default_headers(headers).build() {
            Ok(client) => Some(client),
            Err(_) => {
                log::error!("build client failed");
                None
            }
        };
        Agent { client: client }
    }

    async fn fetch_job(self: &Self) -> Result<Option<Job>, AgentError> {
        const URL: &str = "https://api.letserver.run/agent/job";
        let client = self.client.as_ref().ok_or(AgentError::ClientNotValid)?;
        let response = client
            .get(URL)
            .send()
            .await
            .or_else(|err| Err(AgentError::UnknownError(err.to_string())))?;
        let status = response.status();
        let text = response
            .text()
            .await
            .or_else(|err| Err(AgentError::UnknownError(err.to_string())))?;
        if status == 204 && text.len() == 0 {
            return Ok(None);
        } else if let Ok(job) = serde_json::from_str(&text) {
            log::info!("fetch job: {:?}", job);
            return Ok(Some(job));
        } else if let Ok(error_hint) = serde_json::from_str(&text) {
            return Err(AgentError::from(&error_hint));
        } else {
            return Err(AgentError::UnknownError(text));
        }
    }

    async fn report_job_status(
        self: &Self,
        job: &Job,
        status: &JobStatus,
        result: &str,
    ) -> Result<(), AgentError> {
        let status = match status {
            JobStatus::Succeed => "succeed",
            JobStatus::Fail => "fail",
            JobStatus::Running => "running",
        };
        let url = format!("https://api.letserver.run/agent/jobs/{}/{}", job.id, status);
        let client = self.client.as_ref().ok_or(AgentError::ClientNotValid)?;
        let response = client
            .put(url)
            .json(&json!({"result": result}))
            .send()
            .await
            .or_else(|err| Err(AgentError::UnknownError(err.to_string())))?;
        let status = response.status();
        let text = response
            .text()
            .await
            .or_else(|err| Err(AgentError::UnknownError(err.to_string())))?;
        if status == 204 && text.len() == 0 {
            return Ok(());
        } else if let Ok(error_hint) = serde_json::from_str(&text) {
            return Err(AgentError::from(&error_hint));
        } else {
            return Err(AgentError::UnknownError(text));
        }
    }
}