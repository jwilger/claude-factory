//! Forge adapter trait and implementations.
//!
//! The `ForgeAdapter` trait abstracts over code-hosting platforms (Gitea,
//! GitHub). `MemoryForge` is an in-memory substitute used in tests.
//! `GiteaForge` calls the Gitea REST API.

use async_trait::async_trait;
use cfk_core::types::forge::{CiStatus, PrComment, PrPollResult};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex, PoisonError},
};
use thiserror::Error;

/// Error returned when `GiteaForge` cannot be constructed from environment variables.
#[derive(Debug, Error)]
pub enum ForgeConfigError {
    #[error("required environment variable {var} is not set")]
    MissingEnvVar { var: &'static str },
    #[error("failed to build HTTP client: {0}")]
    Client(#[from] reqwest::Error),
}

/// Error type for forge adapter operations.
#[derive(Debug, Error)]
pub enum ForgeError {
    /// An HTTP-level error from the forge API.
    #[error("forge HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    /// The forge response was missing a required field.
    #[error("malformed forge response: missing field `{field}`")]
    MalformedResponse { field: &'static str },

    /// A `PollScript` ran out of scripted results before polling stopped.
    #[error("poll script exhausted after {calls} calls; add more results to the script")]
    PollScriptExhausted { calls: usize },
}

/// Specification for opening a new pull request.
pub struct PrSpec {
    pub title: String,
    pub body: String,
    pub head: String,
    pub base: String,
}

/// The PR number and web URL returned after opening a pull request.
pub struct OpenedPr {
    pub number: u64,
    pub url: String,
}

/// Trait abstracting a code-hosting forge (Gitea, GitHub, …).
#[async_trait]
pub trait ForgeAdapter: Send + Sync {
    async fn open_pr(&self, spec: &PrSpec) -> Result<OpenedPr, ForgeError>;
    async fn poll_pr(&self, number: u64) -> Result<PrPollResult, ForgeError>;
    async fn post_comment(&self, number: u64, body: &str) -> Result<String, ForgeError>;
    async fn merge_pr(&self, number: u64) -> Result<(), ForgeError>;
}

// ── MemoryForge (in-memory test substitute) ─────────────────────────────────

/// Script for what `poll_pr` should return at each call.
#[derive(Clone)]
pub struct PollScript {
    pub results: Vec<PrPollResult>,
    pub index: usize,
}

impl PollScript {
    #[must_use]
    pub fn new(results: Vec<PrPollResult>) -> Self {
        Self { results, index: 0 }
    }

    /// Return the next scripted result, or `Err(PollScriptExhausted)` if the
    /// script has no more entries.
    ///
    /// # Errors
    /// Returns `ForgeError::PollScriptExhausted` when called more times than
    /// there are scripted results.
    pub fn advance(&mut self) -> Result<PrPollResult, ForgeError> {
        if self.index < self.results.len() {
            let r = self.results[self.index].clone();
            self.index += 1;
            Ok(r)
        } else {
            Err(ForgeError::PollScriptExhausted { calls: self.index })
        }
    }
}

struct MemoryForgeInner {
    next_number: u64,
    poll_scripts: HashMap<u64, PollScript>,
    comments: HashMap<u64, Vec<(String, String)>>, // pr_number → [(id, body)]
    merged: Vec<u64>,
}

/// In-memory forge substitute for behavioral tests.
///
/// Constructed via `MemoryForge::new()`; not a mock library — it is a real
/// `ForgeAdapter` implementation that stores state in-process.
pub struct MemoryForge {
    inner: Mutex<MemoryForgeInner>,
}

impl MemoryForge {
    #[must_use]
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            inner: Mutex::new(MemoryForgeInner {
                next_number: 1,
                poll_scripts: HashMap::new(),
                comments: HashMap::new(),
                merged: Vec::new(),
            }),
        })
    }

    /// Pre-load a poll script for a PR number.
    pub fn set_poll_script(&self, pr_number: u64, script: PollScript) {
        self.locked().poll_scripts.insert(pr_number, script);
    }

    #[must_use]
    pub fn is_merged(&self, pr_number: u64) -> bool {
        self.locked().merged.contains(&pr_number)
    }

    #[must_use]
    pub fn comments_on(&self, pr_number: u64) -> Vec<(String, String)> {
        self.locked().comments.get(&pr_number).cloned().unwrap_or_default()
    }

    /// Acquire the inner lock, recovering from mutex poisoning.
    ///
    /// Poison recovery is safe here: `MemoryForgeInner` holds independent
    /// `HashMap`s with no cross-field invariants, so a poisoned lock cannot
    /// leave the data in an inconsistent state that would corrupt subsequent
    /// operations.
    fn locked(&self) -> std::sync::MutexGuard<'_, MemoryForgeInner> {
        self.inner.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

#[async_trait]
impl ForgeAdapter for MemoryForge {
    async fn open_pr(&self, _spec: &PrSpec) -> Result<OpenedPr, ForgeError> {
        let mut inner = self.locked();
        let number = inner.next_number;
        inner.next_number += 1;
        let url = format!("https://forge.example/pr/{number}");
        Ok(OpenedPr { number, url })
    }

    async fn poll_pr(&self, number: u64) -> Result<PrPollResult, ForgeError> {
        let mut inner = self.locked();
        if let Some(script) = inner.poll_scripts.get_mut(&number) {
            return script.advance();
        }
        Ok(PrPollResult {
            ci_status: CiStatus::Passing,
            approved: true,
            comments: Vec::new(),
        })
    }

    async fn post_comment(&self, number: u64, body: &str) -> Result<String, ForgeError> {
        let mut inner = self.locked();
        let comment_id = format!(
            "comment-{}-{}",
            number,
            inner.comments.entry(number).or_default().len() + 1
        );
        inner.comments.entry(number).or_default().push((comment_id.clone(), body.to_string()));
        Ok(comment_id)
    }

    async fn merge_pr(&self, number: u64) -> Result<(), ForgeError> {
        self.locked().merged.push(number);
        Ok(())
    }
}

// ── GiteaForge ───────────────────────────────────────────────────────────────

/// Gitea REST API forge adapter.
///
/// Reads connection details from environment variables:
/// - `GITEA_URL` — base URL (e.g. `https://git.example.com`)
/// - `GITEA_TOKEN` — personal-access or API token
/// - `GITEA_OWNER` — repository owner (user or org name)
/// - `GITEA_REPO` — repository name
pub struct GiteaForge {
    client: reqwest::Client,
    base_url: String,
    token: String,
    owner: String,
    repo: String,
}

impl GiteaForge {
    /// Build from environment variables. Returns an error if any are missing.
    ///
    /// # Errors
    /// Returns an error if any required environment variable is absent.
    pub fn from_env() -> Result<Arc<Self>, ForgeConfigError> {
        let base_url = std::env::var("GITEA_URL")
            .map_err(|_| ForgeConfigError::MissingEnvVar { var: "GITEA_URL" })?;
        let token = std::env::var("GITEA_TOKEN")
            .map_err(|_| ForgeConfigError::MissingEnvVar { var: "GITEA_TOKEN" })?;
        let owner = std::env::var("GITEA_OWNER")
            .map_err(|_| ForgeConfigError::MissingEnvVar { var: "GITEA_OWNER" })?;
        let repo = std::env::var("GITEA_REPO")
            .map_err(|_| ForgeConfigError::MissingEnvVar { var: "GITEA_REPO" })?;

        let client = reqwest::Client::builder()
            .user_agent("cfk/1.0")
            .build()?;

        Ok(Arc::new(Self { client, base_url, token, owner, repo }))
    }

    fn api_url(&self, path: &str) -> String {
        format!("{}/api/v1/repos/{}/{}{}", self.base_url, self.owner, self.repo, path)
    }

    fn auth_header(&self) -> String {
        format!("token {}", self.token)
    }
}

#[async_trait]
impl ForgeAdapter for GiteaForge {
    async fn open_pr(&self, spec: &PrSpec) -> Result<OpenedPr, ForgeError> {
        let body = serde_json::json!({
            "title": spec.title,
            "body": spec.body,
            "head": spec.head,
            "base": spec.base,
        });

        let json: serde_json::Value = self.client
            .post(self.api_url("/pulls"))
            .header("Authorization", self.auth_header())
            .json(&body)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;

        let number = json["number"].as_u64()
            .ok_or(ForgeError::MalformedResponse { field: "number" })?;
        let url = json["html_url"].as_str()
            .ok_or(ForgeError::MalformedResponse { field: "html_url" })?
            .to_string();

        Ok(OpenedPr { number, url })
    }

    async fn poll_pr(&self, number: u64) -> Result<PrPollResult, ForgeError> {
        // Fetch PR state.
        let pr_resp: serde_json::Value = self.client
            .get(self.api_url(&format!("/pulls/{number}")))
            .header("Authorization", self.auth_header())
            .send()
            .await?
            .json()
            .await?;

        // Check review approvals.
        let reviews_resp: serde_json::Value = self.client
            .get(self.api_url(&format!("/pulls/{number}/reviews")))
            .header("Authorization", self.auth_header())
            .send()
            .await?
            .json()
            .await?;

        let approved = reviews_resp.as_array()
            .is_some_and(|arr| arr.iter().any(|r| r["state"].as_str() == Some("APPROVED")));

        // Map Gitea's commit status to `CiStatus` via the statuses endpoint.
        let sha = pr_resp["head"]["sha"].as_str()
            .ok_or(ForgeError::MalformedResponse { field: "head.sha" })?;
        let statuses_resp: serde_json::Value = self.client
            .get(self.api_url(&format!("/commits/{sha}/statuses")))
            .header("Authorization", self.auth_header())
            .send()
            .await?
            .json()
            .await?;

        let ci_status = statuses_resp.as_array()
            .and_then(|arr| arr.first())
            .and_then(|s| s["state"].as_str())
            .map_or(CiStatus::Unknown, |s| match s {
                "success" => CiStatus::Passing,
                "failure" | "error" => CiStatus::Failing,
                "pending" => CiStatus::Pending,
                _ => CiStatus::Unknown,
            });

        // Fetch comments.
        let comments_resp: serde_json::Value = self.client
            .get(self.api_url(&format!("/issues/{number}/comments")))
            .header("Authorization", self.auth_header())
            .send()
            .await?
            .json()
            .await?;

        let comments = comments_resp.as_array().map_or(Vec::new(), |arr| {
            arr.iter().filter_map(|c| {
                Some(PrComment {
                    id: cfk_core::types::forge::CommentId::try_new(c["id"].as_u64()?.to_string()).ok()?,
                    body: cfk_core::types::forge::CommentBody::try_new(c["body"].as_str()?.to_string()).ok()?,
                    author: c["user"]["login"].as_str()?.to_string(),
                })
            }).collect()
        });

        Ok(PrPollResult { ci_status, approved, comments })
    }

    async fn post_comment(&self, number: u64, body: &str) -> Result<String, ForgeError> {
        let json: serde_json::Value = self.client
            .post(self.api_url(&format!("/issues/{number}/comments")))
            .header("Authorization", self.auth_header())
            .json(&serde_json::json!({ "body": body }))
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;

        Ok(json["id"].as_u64()
            .ok_or(ForgeError::MalformedResponse { field: "id" })?
            .to_string())
    }

    async fn merge_pr(&self, number: u64) -> Result<(), ForgeError> {
        self.client
            .post(self.api_url(&format!("/pulls/{number}/merge")))
            .header("Authorization", self.auth_header())
            .json(&serde_json::json!({
                "Do": "merge",
                "merge_message_field": "Merged by Claude-Factory kernel"
            }))
            .send()
            .await?
            .error_for_status()?;
        Ok(())
    }
}
