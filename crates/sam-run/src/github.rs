use std::fs::File;

use anyhow::{Context, Result};
use reqwest::header::{HeaderMap, HeaderValue, ACCEPT, AUTHORIZATION};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct Release {
    pub tag_name: String,
    pub assets: Vec<Asset>,
}

#[derive(Deserialize)]
pub struct Asset {
    pub id: u64,
    pub name: String,
    pub updated_at: String,
    /// API URL of the asset; downloading it with `Accept: application/octet-stream` works for
    /// both public and token-authorized repositories, unlike `browser_download_url`.
    pub url: String,
}

pub struct Client {
    http: reqwest::blocking::Client,
}

impl Client {
    pub fn new() -> Result<Client> {
        let mut headers = HeaderMap::new();
        headers.insert(
            ACCEPT,
            HeaderValue::from_static("application/vnd.github+json"),
        );
        headers.insert(
            "X-GitHub-Api-Version",
            HeaderValue::from_static("2022-11-28"),
        );
        let token = std::env::var("GITHUB_TOKEN").or_else(|_| std::env::var("GH_TOKEN"));
        if let Ok(token) = token {
            if !token.is_empty() {
                let value = HeaderValue::from_str(&format!("Bearer {token}"))
                    .context("GITHUB_TOKEN contains invalid header characters")?;
                headers.insert(AUTHORIZATION, value);
            }
        }
        let http = reqwest::blocking::Client::builder()
            .user_agent(concat!("sam-run/", env!("CARGO_PKG_VERSION")))
            .default_headers(headers)
            .build()?;
        Ok(Client { http })
    }

    pub fn resolve_release(&self, repo: &str, tag: Option<&str>) -> Result<Release> {
        let url = match tag {
            Some(tag) => format!("https://api.github.com/repos/{repo}/releases/tags/{tag}"),
            None => format!("https://api.github.com/repos/{repo}/releases/latest"),
        };
        let release = self
            .http
            .get(&url)
            .send()
            .and_then(reqwest::blocking::Response::error_for_status)
            .with_context(|| format!("failed to resolve release from {url}"))?
            .json()
            .with_context(|| format!("failed to parse release response from {url}"))?;
        Ok(release)
    }

    pub fn download(&self, asset: &Asset, dest: &mut File) -> Result<()> {
        let mut response = self
            .http
            .get(&asset.url)
            .header(ACCEPT, "application/octet-stream")
            .send()
            .and_then(reqwest::blocking::Response::error_for_status)
            .with_context(|| format!("failed to download asset {}", asset.name))?;
        response
            .copy_to(dest)
            .with_context(|| format!("failed to write asset {}", asset.name))?;
        Ok(())
    }
}
