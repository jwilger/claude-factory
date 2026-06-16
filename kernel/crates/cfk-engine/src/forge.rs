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
    #[error("could not derive forge config from git remote: {reason}")]
    GitRemote { reason: String },
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
        if let Some(r) = self.results.get(self.index).cloned() {
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
    /// Build from environment variables, falling back to the git remote for
    /// non-secret values (`GITEA_URL`, `GITEA_OWNER`, `GITEA_REPO`).
    ///
    /// Only `GITEA_TOKEN` is always required from the environment — the host,
    /// owner, and repo can be inferred from `git remote get-url origin`.
    ///
    /// # Errors
    /// Returns an error if the token is absent or the remote cannot be parsed.
    pub fn from_env() -> Result<Arc<Self>, ForgeConfigError> {
        let token = std::env::var("GITEA_TOKEN")
            .map_err(|_| ForgeConfigError::MissingEnvVar { var: "GITEA_TOKEN" })?;

        let (base_url, owner, repo) = match (
            std::env::var("GITEA_URL").ok(),
            std::env::var("GITEA_OWNER").ok(),
            std::env::var("GITEA_REPO").ok(),
        ) {
            (Some(u), Some(o), Some(r)) => (u, o, r),
            (url_opt, owner_opt, repo_opt) => {
                let (git_url, git_owner, git_repo) = parse_git_remote()?;
                (
                    url_opt.unwrap_or(git_url),
                    owner_opt.unwrap_or(git_owner),
                    repo_opt.unwrap_or(git_repo),
                )
            }
        };

        let client = reqwest::Client::builder()
            .user_agent("cfk/1.0")
            .build()?;

        Ok(Arc::new(Self { client, base_url, token, owner, repo }))
    }
}

/// Parse `git remote get-url origin` into `(base_url, owner, repo)`.
///
/// Handles three URL forms:
/// - `ssh://[user@]host[:port]/owner/repo[.git]`
/// - `git@host:owner/repo[.git]` (SCP-style)
/// - `https://host/owner/repo[.git]`
fn parse_git_remote() -> Result<(String, String, String), ForgeConfigError> {
    let output = std::process::Command::new("git")
        .args(["remote", "get-url", "origin"])
        .output()
        .map_err(|e| ForgeConfigError::GitRemote { reason: format!("git not found: {e}") })?;

    if !output.status.success() {
        return Err(ForgeConfigError::GitRemote {
            reason: "git remote get-url origin failed — no 'origin' remote configured".to_string(),
        });
    }

    let raw = String::from_utf8_lossy(&output.stdout).trim().to_string();
    split_remote_url(&raw).ok_or_else(|| ForgeConfigError::GitRemote {
        reason: format!("cannot parse remote URL {raw:?} into host/owner/repo"),
    })
}

fn split_remote_url(raw: &str) -> Option<(String, String, String)> {
    let raw = raw.trim();

    // SCP-style: [user@]host:owner/repo[.git]  (no "://" present)
    if !raw.contains("://") {
        let rest = raw.find('@').map_or(raw, |at| &raw[at + 1..]);
        let colon = rest.find(':')?;
        let host = &rest[..colon];
        let path = rest[colon + 1..].trim_end_matches(".git");
        let slash = path.find('/')?;
        return Some((
            format!("https://{host}"),
            path[..slash].to_string(),
            path[slash + 1..].to_string(),
        ));
    }

    // Scheme-based: ssh:// or https://
    let after_scheme = raw.find("://")?;
    let authority_and_path = &raw[after_scheme + 3..];
    // Strip optional user@
    let authority_and_path =
        authority_and_path.find('@').map_or(authority_and_path, |at| &authority_and_path[at + 1..]);
    // Split host[:port] from /owner/repo path on the first '/'
    let slash = authority_and_path.find('/')?;
    let host_part = &authority_and_path[..slash]; // may be "host:port"
    let path = authority_and_path[slash + 1..].trim_end_matches(".git");
    // Drop optional port from host
    let host = host_part.find(':').map_or(host_part, |colon| &host_part[..colon]);
    let owner_slash = path.find('/')?;
    Some((
        format!("https://{host}"),
        path[..owner_slash].to_string(),
        path[owner_slash + 1..].to_string(),
    ))
}

impl GiteaForge {

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

        let number = json.get("number").and_then(serde_json::Value::as_u64)
            .ok_or(ForgeError::MalformedResponse { field: "number" })?;
        let url = json.get("html_url").and_then(serde_json::Value::as_str)
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
            .is_some_and(|arr| arr.iter().any(|r| r.get("state").and_then(serde_json::Value::as_str) == Some("APPROVED")));

        // Map Gitea's commit status to `CiStatus` via the statuses endpoint.
        let sha = pr_resp.get("head").and_then(|h| h.get("sha")).and_then(serde_json::Value::as_str)
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
            .and_then(|s| s.get("state").and_then(serde_json::Value::as_str))
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
                    id: cfk_core::types::forge::CommentId::try_new(c.get("id")?.as_u64()?.to_string()).ok()?,
                    body: cfk_core::types::forge::CommentBody::try_new(c.get("body")?.as_str()?.to_string()).ok()?,
                    author: c.get("user")?.get("login")?.as_str()?.to_string(),
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

        Ok(json.get("id").and_then(serde_json::Value::as_u64)
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

#[cfg(test)]
mod tests {
    use super::split_remote_url;

    #[test]
    fn parses_ssh_with_port() {
        let (url, owner, repo) = split_remote_url(
            "ssh://forgejo@git.example.com:2222/Acme/my-repo.git"
        ).unwrap();
        assert_eq!(url, "https://git.example.com");
        assert_eq!(owner, "Acme");
        assert_eq!(repo, "my-repo");
    }

    #[test]
    fn parses_scp_style() {
        let (url, owner, repo) = split_remote_url("git@github.com:user/repo.git").unwrap();
        assert_eq!(url, "https://github.com");
        assert_eq!(owner, "user");
        assert_eq!(repo, "repo");
    }

    #[test]
    fn parses_https() {
        let (url, owner, repo) = split_remote_url("https://github.com/user/repo.git").unwrap();
        assert_eq!(url, "https://github.com");
        assert_eq!(owner, "user");
        assert_eq!(repo, "repo");
    }

    #[test]
    fn returns_none_for_garbage() {
        assert!(split_remote_url("not-a-url").is_none());
    }
}
