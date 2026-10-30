use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::Result;
use base64::{engine::general_purpose::STANDARD, Engine as _};
use grammers_client::Client;
use grammers_client::tl;
use grammers_mtsender::ConnectionParams;
use hex;
use reqwest::Client as HttpClient;
use serde::{Deserialize, Serialize};
use tokio::time::timeout;
use url::Url;

use crate::config::Config;

/// Parsed MTProto proxy from tg://proxy or https://t.me/proxy URLs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Proxy {
    pub server: String,
    pub port: u16,
    pub secret: String,
    pub source: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latency_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_checked: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub country: Option<String>,
}

impl Proxy {
    pub fn to_socks5_url(&self) -> String {
        format!("socks5://{}:{}", self.server, self.port)
    }

    pub fn connection_params(&self) -> ConnectionParams {
        let mut params = ConnectionParams::default();
        params.proxy_url = Some(self.to_socks5_url());
        params
    }
}

/// Proxy cache stored on disk
#[derive(Debug, Serialize, Deserialize, Default)]
pub struct ProxyCache {
    pub proxies: Vec<Proxy>,
    pub updated_at: u64,
}

impl ProxyCache {
    pub fn load() -> Result<Self> {
        let path = Config::proxy_cache();
        if !path.exists() {
            return Ok(Self::default());
        }
        let raw = std::fs::read_to_string(&path)?;
        let cache: ProxyCache = serde_json::from_str(&raw)?;
        Ok(cache)
    }

    pub fn save(&self) -> Result<()> {
        let path = Config::proxy_cache();
        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(&path, json)?;
        Ok(())
    }

    pub fn upsert(&mut self, proxy: Proxy) {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        if let Some(existing) = self
            .proxies
            .iter_mut()
            .find(|p| p.server == proxy.server && p.port == proxy.port)
        {
            existing.latency_ms = proxy.latency_ms.or(existing.latency_ms);
            existing.last_checked = Some(now);
            existing.secret = proxy.secret;
        } else {
            self.proxies.push(proxy);
        }
        self.updated_at = now;
    }

    pub fn working(&self) -> Vec<Proxy> {
        self.proxies
            .iter()
            .filter(|p| p.latency_ms.is_some())
            .cloned()
            .collect()
    }

    pub fn best(&self) -> Option<Proxy> {
        self.working()
            .into_iter()
            .min_by_key(|p| p.latency_ms.unwrap_or(u64::MAX))
    }
}

/// Well-known proxy sources (GitHub repos, Telegram channels)
const PROXY_SOURCES: &[&str] = &[
    "https://raw.githubusercontent.com/SoliSpirit/mtproto/master/all_proxies.txt",
    "https://raw.githubusercontent.com/MhdiTaheri/ProxyCollector/main/proxy.txt",
    "https://raw.githubusercontent.com/BarakzaiSh/proxy/main/proxy.txt",
    "https://raw.githubusercontent.com/PrxyHunter/MTProto-proxy-list/main/proxies.txt",
    "https://raw.githubusercontent.com/mahdibland/V2Ray-configuration/main/mtproto.txt",
];

/// Fetch proxy lists from all sources
pub async fn fetch_all_sources() -> Result<Vec<Proxy>> {
    let http = HttpClient::builder()
        .timeout(Duration::from_secs(15))
        .build()?;

    let mut all_proxies = Vec::new();

    for source in PROXY_SOURCES {
        match fetch_source(&http, source).await {
            Ok(mut proxies) => {
                for p in &mut proxies {
                    p.source = source.to_string();
                }
                all_proxies.extend(proxies);
            }
            Err(e) => {
                eprintln!("failed to fetch {}: {}", source, e);
            }
        }
    }

    // Deduplicate by server:port
    let mut seen = HashMap::new();
    let mut unique = Vec::new();
    for p in all_proxies {
        let key = (p.server.clone(), p.port);
        if !seen.contains_key(&key) {
            seen.insert(key, true);
            unique.push(p);
        }
    }

    Ok(unique)
}

async fn fetch_source(http: &HttpClient, url: &str) -> Result<Vec<Proxy>> {
    let resp = http.get(url).send().await?;
    let text = resp.text().await?;

    let mut proxies = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some(proxy) = parse_proxy_line(line) {
            proxies.push(proxy);
        }
    }
    Ok(proxies)
}

/// Parse a single proxy line (tg://proxy or https://t.me/proxy format)
fn parse_proxy_line(line: &str) -> Option<Proxy> {
    let url = if line.starts_with("tg://proxy?") {
        Url::parse(&format!("https://t.me/proxy?{}", &line[11..])).ok()?
    } else if line.starts_with("https://t.me/proxy?") {
        Url::parse(line).ok()?
    } else {
        return None;
    };

    let server = url.query_pairs().find(|(k, _)| k == "server")?.1.to_string();
    let port = url
        .query_pairs()
        .find(|(k, _)| k == "port")?
        .1
        .parse::<u16>()
        .ok()?;
    let secret = url.query_pairs().find(|(k, _)| k == "secret")?.1.to_string();

    let secret = normalize_secret(&secret);

    Some(Proxy {
        server,
        port,
        secret,
        source: String::new(),
        latency_ms: None,
        last_checked: None,
        country: None,
    })
}

/// Normalize secret to hex format for SOCKS5 MTProto
fn normalize_secret(secret: &str) -> String {
    // Already hex (32 chars)
    if secret.len() == 32 && secret.chars().all(|c| c.is_ascii_hexdigit()) {
        return secret.to_lowercase();
    }

    // FAST proxy (starts with "ee" or "dd" in hex, 34 chars)
    if secret.len() == 34 && secret.chars().all(|c| c.is_ascii_hexdigit()) {
        return secret.to_lowercase();
    }

    // Base64 (Fake TLS)
    if let Ok(decoded) = STANDARD.decode(secret) {
        if decoded.len() == 16 {
            return hex::encode(decoded);
        }
    }

    // Fallback: return as-is
    secret.to_string()
}

/// Check if a proxy works by connecting through it
pub async fn check_proxy(proxy: &Proxy, timeout_secs: u64) -> Result<Option<u64>> {
    let start = Instant::now();

    let result = timeout(
        Duration::from_secs(timeout_secs),
        test_connection(proxy),
    )
    .await;

    match result {
        Ok(Ok(())) => Ok(Some(start.elapsed().as_millis() as u64)),
        Ok(Err(e)) => {
            eprintln!("proxy {}:{} failed: {}", proxy.server, proxy.port, e);
            Ok(None)
        }
        Err(_) => {
            eprintln!("proxy {}:{} timeout", proxy.server, proxy.port);
            Ok(None)
        }
    }
}

async fn test_connection(proxy: &Proxy) -> Result<()> {
    let params = proxy.connection_params();

    let cfg = Config::load()?;
    let session = Arc::new(grammers_session::storages::MemorySession::default());
    let pool = grammers_mtsender::SenderPool::with_configuration(session, cfg.api_id, params);
    let _updates = pool.updates;
    tokio::spawn(pool.runner.run());
    let handle = pool.handle;
    let raw = Client::new(handle);

    // Try a simple API call to verify the connection works
    raw.invoke(&tl::functions::help::GetConfig {}).await?;
    Ok(())
}

/// Check multiple proxies concurrently and update cache
pub async fn check_proxies(proxies: &[Proxy], concurrency: usize, timeout_secs: u64) -> Result<Vec<Proxy>> {
    use futures::stream::{FuturesUnordered, StreamExt};

    let mut results = Vec::new();
    let mut futures = FuturesUnordered::new();

    for proxy in proxies.iter().cloned() {
        futures.push(async move {
            let latency = check_proxy(&proxy, timeout_secs).await?;
            let mut proxy = proxy;
            proxy.latency_ms = latency;
            proxy.last_checked = Some(
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs(),
            );
            Ok::<Proxy, anyhow::Error>(proxy)
        });

        if futures.len() >= concurrency {
            if let Some(res) = futures.next().await {
                results.push(res?);
            }
        }
    }

    while let Some(res) = futures.next().await {
        results.push(res?);
    }

    Ok(results)
}

/// Discover and verify proxies, return working ones
pub async fn discover(timeout_secs: u64, concurrency: usize) -> Result<Vec<Proxy>> {
    println!("fetching proxy lists from {} sources...", PROXY_SOURCES.len());
    let proxies = fetch_all_sources().await?;
    println!("found {} unique proxies", proxies.len());

    println!("testing proxies (concurrency: {}, timeout: {}s)...", concurrency, timeout_secs);
    let checked = check_proxies(&proxies, concurrency, timeout_secs).await?;

    let mut cache = ProxyCache::load()?;
    for proxy in &checked {
        cache.upsert(proxy.clone());
    }
    cache.save()?;

    let working: Vec<_> = checked.iter().filter(|p| p.latency_ms.is_some()).cloned().collect();
    println!("found {} working proxies", working.len());

    for p in &working {
        println!(
            "  {}:{} - {}ms",
            p.server,
            p.port,
            p.latency_ms.unwrap()
        );
    }

    Ok(working)
}

/// Get the best cached proxy
pub fn get_best_cached() -> Result<Option<Proxy>> {
    let cache = ProxyCache::load()?;
    Ok(cache.best())
}

/// List all cached proxies
pub fn list_cached() -> Result<Vec<Proxy>> {
    let cache = ProxyCache::load()?;
    Ok(cache.proxies)
}

/// Apply proxy to client configuration
pub fn apply_proxy_to_config(config: &mut ConnectionParams, proxy: &crate::config::ProxyConfig) {
    if proxy.enabled {
        config.proxy_url = Some(format!("socks5://{}:{}", proxy.server, proxy.port));
    }
}

/// Re-test all cached proxies
pub async fn retest_cached(timeout_secs: u64, concurrency: usize) -> Result<Vec<Proxy>> {
    let cache = ProxyCache::load()?;
    let proxies: Vec<_> = cache.proxies.into_iter().filter(|p| p.latency_ms.is_none() || p.latency_ms.is_some()).collect();
    if proxies.is_empty() {
        println!("no cached proxies to test");
        return Ok(vec![]);
    }
    println!("re-testing {} cached proxies...", proxies.len());
    let checked = check_proxies(&proxies, concurrency, timeout_secs).await?;

    let mut new_cache = ProxyCache::load()?;
    for proxy in &checked {
        new_cache.upsert(proxy.clone());
    }
    new_cache.save()?;

    let working: Vec<_> = checked.iter().filter(|p| p.latency_ms.is_some()).cloned().collect();
    println!("found {} working proxies", working.len());

    for p in &working {
        println!(
            "  {}:{} - {}ms",
            p.server,
            p.port,
            p.latency_ms.unwrap()
        );
    }

    Ok(working)
}