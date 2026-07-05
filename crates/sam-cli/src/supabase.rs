//! Thin Supabase client: GoTrue auth + PostgREST data access.
//!
//! Auth mirrors the web app's email/password flow. The resulting session is
//! cached so subsequent invocations are non-interactive; an expired access
//! token is silently refreshed via its refresh token.

use anyhow::{anyhow, bail, Context, Result};
use reqwest::blocking::{Client, RequestBuilder, Response};
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE};
use reqwest::Method;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::config::{self, Config, Session};

/// An authenticated Supabase client bound to a single user session.
pub struct Supabase {
    http: Client,
    config: Config,
    session: Session,
}

#[derive(Deserialize)]
struct TokenResponse {
    access_token: String,
    refresh_token: String,
    /// Seconds until expiry, relative to now.
    expires_in: i64,
    user: TokenUser,
}

#[derive(Deserialize)]
struct TokenUser {
    id: String,
    email: Option<String>,
}

impl Supabase {
    /// Build a client, reusing a cached session when possible and otherwise
    /// authenticating with email/password (from env vars or interactive prompt).
    pub fn connect() -> Result<Self> {
        let config = Config::load();
        let http = Client::builder()
            .user_agent(concat!("sam-cli/", env!("CARGO_PKG_VERSION")))
            .build()
            .context("building HTTP client")?;

        let session = match config::load_session()? {
            Some(session) if !session.is_expired() => session,
            Some(session) => refresh(&http, &config, &session).or_else(|_| {
                // Refresh token rejected (e.g. revoked) — fall back to a fresh login.
                password_login(&http, &config)
            })?,
            None => password_login(&http, &config)?,
        };
        config::save_session(&session)?;

        Ok(Self {
            http,
            config,
            session,
        })
    }

    /// The signed-in user's id, used to scope PostgREST queries.
    pub fn user_id(&self) -> &str {
        &self.session.user_id
    }

    /// GET `/rest/v1/{table}` with the given query string, deserialized into `T`.
    pub fn select<T: DeserializeOwned>(&self, table: &str, query: &str) -> Result<T> {
        let url = format!("{}/rest/v1/{table}?{query}", self.config.url);
        let resp = self.request(Method::GET, &url).send()?;
        let resp = check(resp)?;
        resp.json::<T>().context("decoding PostgREST response")
    }

    /// GET every row of `/rest/v1/{table}` matching `query`, paginating past
    /// PostgREST's server-side row cap (1,000 by default on Supabase), which
    /// silently truncates plain unbounded selects. `query` must not contain
    /// `order`/`limit`/`offset`; pages are fetched in `id` order so offsets
    /// are stable — callers that need a display order should sort in memory.
    pub fn select_all<T: DeserializeOwned>(&self, table: &str, query: &str) -> Result<Vec<T>> {
        const PAGE: usize = 1000;
        let mut rows: Vec<T> = Vec::new();
        loop {
            let page_query = format!("{query}&order=id&limit={PAGE}&offset={}", rows.len());
            let batch: Vec<T> = self.select(table, &page_query)?;
            let last_page = batch.len() < PAGE;
            rows.extend(batch);
            if last_page {
                return Ok(rows);
            }
        }
    }

    /// POST a row (or rows) to `/rest/v1/{table}`. `on_conflict` enables upsert.
    pub fn insert<B: Serialize + ?Sized>(
        &self,
        table: &str,
        body: &B,
        on_conflict: Option<&str>,
    ) -> Result<()> {
        let mut url = format!("{}/rest/v1/{table}", self.config.url);
        let mut prefer = "return=minimal".to_string();
        if let Some(cols) = on_conflict {
            url.push_str(&format!("?on_conflict={cols}"));
            prefer.push_str(",resolution=merge-duplicates");
        }
        let resp = self
            .request(Method::POST, &url)
            .header("Prefer", prefer)
            .json(body)
            .send()?;
        check(resp)?;
        Ok(())
    }

    /// POST a row to `/rest/v1/{table}` and return the inserted rows
    /// (PostgREST's `return=representation`), e.g. to learn generated ids.
    pub fn insert_returning<B: Serialize + ?Sized, T: DeserializeOwned>(
        &self,
        table: &str,
        body: &B,
    ) -> Result<T> {
        let url = format!("{}/rest/v1/{table}", self.config.url);
        let resp = self
            .request(Method::POST, &url)
            .header("Prefer", "return=representation")
            .json(body)
            .send()?;
        let resp = check(resp)?;
        resp.json::<T>().context("decoding PostgREST response")
    }

    /// PATCH rows in `/rest/v1/{table}` matching the query string filters.
    pub fn update<B: Serialize + ?Sized>(&self, table: &str, query: &str, body: &B) -> Result<()> {
        let url = format!("{}/rest/v1/{table}?{query}", self.config.url);
        let resp = self
            .request(Method::PATCH, &url)
            .header("Prefer", "return=minimal")
            .json(body)
            .send()?;
        check(resp)?;
        Ok(())
    }

    /// DELETE rows in `/rest/v1/{table}` matching the query string filters.
    pub fn delete(&self, table: &str, query: &str) -> Result<()> {
        let url = format!("{}/rest/v1/{table}?{query}", self.config.url);
        let resp = self.request(Method::DELETE, &url).send()?;
        check(resp)?;
        Ok(())
    }

    fn request(&self, method: Method, url: &str) -> RequestBuilder {
        self.http
            .request(method, url)
            .header("apikey", &self.config.anon_key)
            .header(
                AUTHORIZATION,
                format!("Bearer {}", self.session.access_token),
            )
    }
}

/// Authenticate with email/password, prompting interactively when needed.
fn password_login(http: &Client, config: &Config) -> Result<Session> {
    let email = match std::env::var("SAM_CLI_EMAIL") {
        Ok(v) if !v.trim().is_empty() => v.trim().to_string(),
        _ => prompt("Email: ")?,
    };
    let password = match std::env::var("SAM_CLI_PASSWORD") {
        Ok(v) if !v.is_empty() => v,
        _ => rpassword::prompt_password("Password: ").context("reading password")?,
    };
    if email.is_empty() || password.is_empty() {
        bail!("email and password are required to sign in");
    }

    let body = serde_json::json!({ "email": email, "password": password });
    let resp = auth_request(http, config, "password", &body)?;
    token_to_session(resp).map_err(|e| {
        // GoTrue returns 400 with a generic message for bad credentials.
        anyhow!("sign-in failed: {e}")
    })
}

/// Exchange a refresh token for a fresh session.
fn refresh(http: &Client, config: &Config, session: &Session) -> Result<Session> {
    let body = serde_json::json!({ "refresh_token": session.refresh_token });
    let resp = auth_request(http, config, "refresh_token", &body)?;
    token_to_session(resp)
}

fn auth_request(http: &Client, config: &Config, grant: &str, body: &Value) -> Result<Response> {
    let mut headers = HeaderMap::new();
    headers.insert(
        "apikey",
        HeaderValue::from_str(&config.anon_key).context("invalid anon key")?,
    );
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
    let url = format!("{}/auth/v1/token?grant_type={grant}", config.url);
    http.post(&url)
        .headers(headers)
        .json(body)
        .send()
        .context("contacting Supabase auth")
}

fn token_to_session(resp: Response) -> Result<Session> {
    let resp = check(resp)?;
    let token: TokenResponse = resp.json().context("decoding auth response")?;
    Ok(Session {
        access_token: token.access_token,
        refresh_token: token.refresh_token,
        expires_at: chrono::Utc::now().timestamp() + token.expires_in,
        user_id: token.user.id,
        email: token.user.email,
    })
}

/// Turn a non-2xx response into a useful error, surfacing the server message.
fn check(resp: Response) -> Result<Response> {
    let status = resp.status();
    if status.is_success() {
        return Ok(resp);
    }
    let body = resp.text().unwrap_or_default();
    let message = serde_json::from_str::<Value>(&body)
        .ok()
        .and_then(|v| {
            v.get("msg")
                .or_else(|| v.get("message"))
                .or_else(|| v.get("error_description"))
                .or_else(|| v.get("error"))
                .and_then(Value::as_str)
                .map(str::to_string)
        })
        .unwrap_or(body);
    bail!("request failed ({status}): {message}");
}

fn prompt(label: &str) -> Result<String> {
    use std::io::Write;
    print!("{label}");
    std::io::stdout().flush()?;
    let mut line = String::new();
    std::io::stdin()
        .read_line(&mut line)
        .context("reading input")?;
    Ok(line.trim().to_string())
}
