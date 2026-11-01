//! GitHub API client: PR status for bookmarks, PR creation/updates, native
//! stacked PRs (`/repos/{o}/{r}/stacks`) and asynchronous merges
//! (`/pulls/{n}/merge-async`, required for stacked PRs).

use std::collections::HashMap;
use std::process::Command;
use std::time::{Duration, Instant};

use anyhow::{anyhow, bail, Context, Result};
use reqwest::blocking::Client;
use reqwest::Method;
use serde::Deserialize;
use serde_json::{json, Value};

const API_VERSION: &str = "2022-11-28";
/// The async merge endpoint is documented under the newer API version.
const MERGE_ASYNC_API_VERSION: &str = "2026-03-10";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RepoRef {
    pub host: String,
    pub owner: String,
    pub name: String,
}

/// Parses `https://github.com/o/r(.git)`, `git@github.com:o/r.git` and
/// `ssh://git@github.com/o/r.git`.
pub fn parse_remote_url(url: &str) -> Option<RepoRef> {
    let url = url.trim();
    let rest = if let Some(r) = url
        .strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"))
    {
        r.to_string()
    } else if let Some(r) = url.strip_prefix("ssh://") {
        r.split_once('@')
            .map(|(_, r)| r.to_string())
            .unwrap_or_else(|| r.to_string())
    } else if let Some((user_host, path)) = url.split_once(':') {
        let host = user_host
            .split_once('@')
            .map(|(_, h)| h)
            .unwrap_or(user_host);
        format!("{host}/{path}")
    } else {
        return None;
    };
    let rest = rest
        .split_once('@')
        .filter(|(a, _)| !a.contains('/'))
        .map(|(_, r)| r.to_string())
        .unwrap_or(rest);
    let mut parts = rest.trim_end_matches('/').splitn(3, '/');
    let host = parts.next()?.split(':').next()?.to_string();
    let owner = parts.next()?.to_string();
    let name = parts.next()?.trim_end_matches(".git").to_string();
    if host.is_empty() || owner.is_empty() || name.is_empty() || name.contains('/') {
        return None;
    }
    Some(RepoRef { host, owner, name })
}

/// `GITHUB_TOKEN` / `GH_TOKEN`, falling back to the GitHub CLI's login.
pub fn resolve_token(host: &str) -> Result<String> {
    for var in ["JJ_HELPER_GITHUB_TOKEN", "GITHUB_TOKEN", "GH_TOKEN"] {
        if let Ok(t) = std::env::var(var) {
            if !t.trim().is_empty() {
                return Ok(t.trim().to_string());
            }
        }
    }
    let out = Command::new("gh")
        .args(["auth", "token", "--hostname", host])
        .output()
        .context("no GITHUB_TOKEN set and the GitHub CLI (gh) is not available")?;
    let token = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if !out.status.success() || token.is_empty() {
        bail!("not logged in to GitHub: set GITHUB_TOKEN or run `gh auth login`");
    }
    Ok(token)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PrState {
    Open,
    Closed,
    Merged,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StackInfo {
    pub number: u64,
    pub size: u64,
    pub position: u64,
}

#[derive(Clone, Debug)]
pub struct Review {
    pub login: String,
    pub oid: String,
    pub state: String,
}

#[derive(Clone, Debug)]
pub struct PrInfo {
    pub number: u64,
    pub title: String,
    pub url: String,
    pub body: String,
    pub state: PrState,
    pub draft: bool,
    pub head_ref: String,
    pub base_ref: String,
    pub head_oid: String,
    pub review_decision: Option<String>,
    pub reviews: Vec<Review>,
    pub checks: Option<String>,
    pub merge_queue: Option<String>,
    pub stack: Option<StackInfo>,
}

impl PrInfo {
    pub fn is_open(&self) -> bool {
        self.state == PrState::Open
    }

    fn from_graphql(v: &Value) -> Option<PrInfo> {
        let s = |k: &str| v.get(k).and_then(Value::as_str).unwrap_or("").to_string();
        let state = match v.get("state")?.as_str()? {
            "OPEN" => PrState::Open,
            "MERGED" => PrState::Merged,
            _ => PrState::Closed,
        };
        let stack = match (
            v.pointer("/stack/number"),
            v.pointer("/stack/size"),
            v.pointer("/stackEntry/position"),
        ) {
            (Some(n), Some(sz), Some(p)) => Some(StackInfo {
                number: n.as_u64()?,
                size: sz.as_u64()?,
                position: p.as_u64()?,
            }),
            _ => None,
        };
        let reviews = v
            .pointer("/latestReviews/nodes")
            .and_then(Value::as_array)
            .map(|a| {
                a.iter()
                    .filter_map(|r| {
                        Some(Review {
                            login: r.pointer("/author/login")?.as_str()?.to_string(),
                            oid: r.pointer("/commit/oid")?.as_str()?.to_string(),
                            state: r.get("state")?.as_str()?.to_string(),
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();
        Some(PrInfo {
            number: v.get("number")?.as_u64()?,
            title: s("title"),
            url: s("url"),
            body: s("body"),
            state,
            draft: v.get("isDraft").and_then(Value::as_bool).unwrap_or(false),
            head_ref: s("headRefName"),
            base_ref: s("baseRefName"),
            head_oid: s("headRefOid"),
            review_decision: v
                .get("reviewDecision")
                .and_then(Value::as_str)
                .map(str::to_string),
            reviews,
            checks: v
                .pointer("/commits/nodes/0/commit/statusCheckRollup/state")
                .and_then(Value::as_str)
                .map(str::to_string),
            merge_queue: v
                .pointer("/mergeQueueEntry/state")
                .and_then(Value::as_str)
                .map(str::to_string),
            stack,
        })
    }
}

fn pr_fragment(with_stack: bool) -> String {
    format!(
        "fragment Pr on PullRequest {{
  number title url body state isDraft headRefName baseRefName headRefOid reviewDecision
  latestReviews(first: 20) {{ nodes {{ state author {{ login }} commit {{ oid }} }} }}
  mergeQueueEntry {{ state }}
  commits(last: 1) {{ nodes {{ commit {{ statusCheckRollup {{ state }} }} }} }}
  {}
}}",
        if with_stack {
            "stack { number size }\n  stackEntry { position }"
        } else {
            ""
        }
    )
}

#[derive(Deserialize, Debug, Clone)]
pub struct StackPr {
    pub number: u64,
    #[serde(default)]
    pub state: String,
    #[serde(default)]
    pub merged_at: Option<String>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Stack {
    pub number: u64,
    #[serde(default)]
    pub pull_requests: Vec<StackPr>,
}

impl Stack {
    /// Open, unmerged PR numbers from bottom to top.
    pub fn open_numbers(&self) -> Vec<u64> {
        self.pull_requests
            .iter()
            .filter(|p| p.merged_at.is_none() && p.state != "closed")
            .map(|p| p.number)
            .collect()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MergeOutcome {
    Merged(Option<String>),
    Enqueued,
}

pub struct GitHub {
    pub repo: RepoRef,
    token: String,
    api: String,
    client: Client,
    /// Whether the GraphQL schema has stacked-PR fields (GHES may not).
    stack_fields: std::sync::atomic::AtomicBool,
}

impl GitHub {
    pub fn new(repo: RepoRef) -> Result<GitHub> {
        let token = resolve_token(&repo.host)?;
        let api = std::env::var("GITHUB_API_URL").unwrap_or_else(|_| {
            if repo.host == "github.com" {
                "https://api.github.com".to_string()
            } else {
                format!("https://{}/api/v3", repo.host)
            }
        });
        let client = Client::builder()
            .user_agent("jj-helper")
            .timeout(Duration::from_secs(60))
            .build()?;
        Ok(GitHub {
            repo,
            token,
            api,
            client,
            stack_fields: true.into(),
        })
    }

    /// Connects to the repo behind the jj remote `remote`.
    pub fn for_remote(jj: &crate::jj::Jj, remote: &str) -> Result<GitHub> {
        // Explicit override for remotes whose URL doesn't name the GitHub
        // repo (url rewrites, mirrors).
        if let Ok(spec) = std::env::var("JJ_HELPER_GITHUB_REPO") {
            let (owner, name) = spec
                .split_once('/')
                .ok_or_else(|| anyhow!("JJ_HELPER_GITHUB_REPO must be owner/name"))?;
            let host =
                std::env::var("JJ_HELPER_GITHUB_HOST").unwrap_or_else(|_| "github.com".into());
            return GitHub::new(RepoRef {
                host,
                owner: owner.into(),
                name: name.into(),
            });
        }
        let remotes = jj.git_remotes()?;
        let url = remotes
            .iter()
            .find(|(n, _)| n == remote)
            .map(|(_, u)| u.clone())
            .ok_or_else(|| anyhow!("no git remote named {remote}"))?;
        let repo = parse_remote_url(&url)
            .ok_or_else(|| anyhow!("remote {remote} ({url}) is not a GitHub URL"))?;
        GitHub::new(repo)
    }

    fn request(
        &self,
        method: Method,
        path: &str,
        body: Option<Value>,
        version: &str,
    ) -> Result<(u16, Value)> {
        let url = format!(
            "{}/repos/{}/{}{path}",
            self.api, self.repo.owner, self.repo.name
        );
        let mut req = self
            .client
            .request(method.clone(), &url)
            .bearer_auth(&self.token)
            .header("Accept", "application/vnd.github+json")
            .header("X-GitHub-Api-Version", version);
        if let Some(b) = body {
            req = req.json(&b);
        }
        let resp = req.send().with_context(|| format!("{method} {path}"))?;
        let status = resp.status().as_u16();
        let text = resp.text().unwrap_or_default();
        let value: Value = if text.trim().is_empty() {
            Value::Null
        } else {
            serde_json::from_str(&text).unwrap_or(Value::String(text))
        };
        if status >= 400 {
            bail!(
                "GitHub {method} {path} → HTTP {status}: {}",
                error_message(&value)
            );
        }
        Ok((status, value))
    }

    fn rest(&self, method: Method, path: &str, body: Option<Value>) -> Result<(u16, Value)> {
        self.request(method, path, body, API_VERSION)
    }

    pub fn graphql(&self, query: &str, variables: Value) -> Result<Value> {
        let url = if self.api.ends_with("/api/v3") {
            self.api.replace("/api/v3", "/api/graphql")
        } else {
            format!("{}/graphql", self.api)
        };
        let resp = self
            .client
            .post(url)
            .bearer_auth(&self.token)
            .json(&json!({ "query": query, "variables": variables }))
            .send()
            .context("GitHub GraphQL request")?;
        let status = resp.status().as_u16();
        let v: Value = resp.json().context("GitHub GraphQL response")?;
        if let Some(errors) = v
            .get("errors")
            .and_then(Value::as_array)
            .filter(|e| !e.is_empty())
        {
            let msgs: Vec<String> = errors
                .iter()
                .filter_map(|e| e.get("message")?.as_str().map(String::from))
                .collect();
            bail!("GitHub GraphQL: {}", msgs.join("; "));
        }
        if status >= 400 {
            bail!("GitHub GraphQL → HTTP {status}: {}", error_message(&v));
        }
        Ok(v)
    }

    /// Latest PR for each head branch (open PRs win over closed/merged ones).
    pub fn prs_for_branches(&self, branches: &[String]) -> Result<HashMap<String, PrInfo>> {
        let mut out = HashMap::new();
        for chunk in branches.chunks(30) {
            let with_stack = self.stack_fields.load(std::sync::atomic::Ordering::Relaxed);
            match self.prs_chunk(chunk, with_stack) {
                Ok(m) => out.extend(m),
                Err(e) if with_stack && e.to_string().contains("stack") => {
                    self.stack_fields
                        .store(false, std::sync::atomic::Ordering::Relaxed);
                    out.extend(self.prs_chunk(chunk, false)?);
                }
                Err(e) => return Err(e),
            }
        }
        Ok(out)
    }

    fn prs_chunk(&self, branches: &[String], with_stack: bool) -> Result<HashMap<String, PrInfo>> {
        let mut vars = json!({ "owner": self.repo.owner, "name": self.repo.name });
        let mut decls = vec!["$owner: String!".to_string(), "$name: String!".to_string()];
        let mut fields = vec![];
        for (i, b) in branches.iter().enumerate() {
            decls.push(format!("$h{i}: String!"));
            vars[format!("h{i}")] = json!(b);
            fields.push(format!(
                "b{i}: pullRequests(headRefName: $h{i}, first: 5, orderBy: {{field: CREATED_AT, direction: DESC}}) {{ nodes {{ ...Pr }} }}"
            ));
        }
        let query = format!(
            "query({}) {{ repository(owner: $owner, name: $name) {{ {} }} }}\n{}",
            decls.join(", "),
            fields.join("\n"),
            pr_fragment(with_stack)
        );
        let v = self.graphql(&query, vars)?;
        let mut out = HashMap::new();
        for (i, b) in branches.iter().enumerate() {
            let nodes = v
                .pointer(&format!("/data/repository/b{i}/nodes"))
                .and_then(Value::as_array);
            let prs: Vec<PrInfo> = nodes
                .map(|n| n.iter().filter_map(PrInfo::from_graphql).collect())
                .unwrap_or_default();
            let best = prs.iter().find(|p| p.is_open()).or_else(|| prs.first());
            if let Some(p) = best {
                out.insert(b.clone(), p.clone());
            }
        }
        Ok(out)
    }

    pub fn create_pr(
        &self,
        head: &str,
        base: &str,
        title: &str,
        body: &str,
        draft: bool,
    ) -> Result<(u64, String)> {
        let (_, v) = self.rest(
            Method::POST,
            "/pulls",
            Some(
                json!({ "head": head, "base": base, "title": title, "body": body, "draft": draft }),
            ),
        )?;
        let number = v
            .get("number")
            .and_then(Value::as_u64)
            .ok_or_else(|| anyhow!("PR response without number"))?;
        let url = v
            .get("html_url")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        Ok((number, url))
    }

    pub fn update_pr(&self, number: u64, fields: Value) -> Result<()> {
        self.rest(Method::PATCH, &format!("/pulls/{number}"), Some(fields))?;
        Ok(())
    }

    pub fn get_stack(&self, number: u64) -> Result<Stack> {
        let (_, v) = self.rest(Method::GET, &format!("/stacks/{number}"), None)?;
        Ok(serde_json::from_value(v)?)
    }

    pub fn create_stack(&self, prs: &[u64]) -> Result<Stack> {
        let (_, v) = self.rest(
            Method::POST,
            "/stacks",
            Some(json!({ "pull_requests": prs })),
        )?;
        Ok(serde_json::from_value(v)?)
    }

    pub fn add_to_stack(&self, stack: u64, prs: &[u64]) -> Result<Stack> {
        let (_, v) = self.rest(
            Method::POST,
            &format!("/stacks/{stack}/add"),
            Some(json!({ "pull_requests": prs })),
        )?;
        Ok(serde_json::from_value(v)?)
    }

    /// Removes unmerged PRs from a stack; dissolves it when none remain.
    pub fn unstack(&self, stack: u64) -> Result<()> {
        self.rest(Method::POST, &format!("/stacks/{stack}/unstack"), None)?;
        Ok(())
    }

    /// Merges a PR (and, for stacked PRs, everything below it) through the
    /// asynchronous merge API, polling until it settles.
    pub fn merge_async(
        &self,
        number: u64,
        method: &str,
        sha: &str,
        log: &mut dyn FnMut(String),
    ) -> Result<MergeOutcome> {
        let (_, v) = self.request(
            Method::PUT,
            &format!("/pulls/{number}/merge-async"),
            Some(json!({ "merge_method": method, "sha": sha })),
            MERGE_ASYNC_API_VERSION,
        )?;
        let mut v = v;
        let start = Instant::now();
        loop {
            let status = v.get("status").and_then(Value::as_str).unwrap_or("pending");
            let message = v
                .pointer("/details/message")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string();
            match status {
                "merged" => {
                    return Ok(MergeOutcome::Merged(
                        v.pointer("/details/sha")
                            .and_then(Value::as_str)
                            .map(str::to_string),
                    ))
                }
                "enqueued" => return Ok(MergeOutcome::Enqueued),
                "failed" => bail!("merge of #{number} failed: {message}"),
                _ => {}
            }
            let uuid = v
                .pointer("/details/uuid")
                .and_then(Value::as_str)
                .ok_or_else(|| anyhow!("merge-async response has no uuid to poll: {v}"))?
                .to_string();
            if start.elapsed() > Duration::from_secs(600) {
                bail!("merge of #{number} still pending after 10 minutes (request {uuid})");
            }
            log(format!("merge pending… {message}"));
            std::thread::sleep(Duration::from_secs(3));
            v = self
                .request(
                    Method::GET,
                    &format!("/pulls/{number}/merge-async/{uuid}"),
                    None,
                    MERGE_ASYNC_API_VERSION,
                )?
                .1;
        }
    }
}

fn error_message(v: &Value) -> String {
    let mut msg = v
        .get("message")
        .and_then(Value::as_str)
        .map(str::to_string)
        .unwrap_or_else(|| v.to_string());
    if let Some(errs) = v.get("errors").and_then(Value::as_array) {
        let details: Vec<String> = errs
            .iter()
            .map(|e| {
                e.get("message")
                    .and_then(Value::as_str)
                    .map(str::to_string)
                    .unwrap_or_else(|| e.to_string())
            })
            .collect();
        if !details.is_empty() {
            msg = format!("{msg} ({})", details.join("; "));
        }
    }
    msg
}

pub fn open_in_browser(url: &str) -> Result<()> {
    let opener = if cfg!(target_os = "macos") {
        "open"
    } else {
        "xdg-open"
    };
    Command::new(opener)
        .arg(url)
        .spawn()
        .with_context(|| format!("failed to run {opener}"))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_remote_urls() {
        let want = RepoRef {
            host: "github.com".into(),
            owner: "SamChou19815".into(),
            name: "SamChou19815".into(),
        };
        assert_eq!(
            parse_remote_url("https://github.com/SamChou19815/SamChou19815"),
            Some(want.clone())
        );
        assert_eq!(
            parse_remote_url("https://github.com/SamChou19815/SamChou19815.git"),
            Some(want.clone())
        );
        assert_eq!(
            parse_remote_url("git@github.com:SamChou19815/SamChou19815.git"),
            Some(want.clone())
        );
        assert_eq!(
            parse_remote_url("ssh://git@github.com/SamChou19815/SamChou19815.git"),
            Some(want.clone())
        );
        assert_eq!(
            parse_remote_url("https://user@github.com/SamChou19815/SamChou19815"),
            Some(want)
        );
        assert_eq!(parse_remote_url("../remote.git"), None);
    }

    #[test]
    fn parses_graphql_pr() {
        let v = json!({
            "number": 12, "title": "t", "url": "u", "body": "b", "state": "OPEN", "isDraft": false,
            "headRefName": "push-abc", "baseRefName": "main", "headRefOid": "deadbeef",
            "reviewDecision": "APPROVED",
            "latestReviews": {"nodes": [{"state": "APPROVED", "author": {"login": "x"}, "commit": {"oid": "c0"}}]},
            "mergeQueueEntry": null,
            "commits": {"nodes": [{"commit": {"statusCheckRollup": {"state": "SUCCESS"}}}]},
            "stack": {"number": 3, "size": 2}, "stackEntry": {"position": 1}
        });
        let pr = PrInfo::from_graphql(&v).unwrap();
        assert_eq!(pr.number, 12);
        assert_eq!(pr.checks.as_deref(), Some("SUCCESS"));
        assert_eq!(
            pr.stack,
            Some(StackInfo {
                number: 3,
                size: 2,
                position: 1
            })
        );
        assert_eq!(pr.reviews[0].oid, "c0");
    }

    #[test]
    fn stack_open_numbers_skip_merged() {
        let s: Stack = serde_json::from_value(json!({
            "number": 1, "open": true,
            "pull_requests": [
                {"number": 10, "state": "closed", "merged_at": "2026-01-01T00:00:00Z"},
                {"number": 11, "state": "open", "merged_at": null},
                {"number": 12, "state": "open"}
            ]
        }))
        .unwrap();
        assert_eq!(s.open_numbers(), vec![11, 12]);
    }
}
