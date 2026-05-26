use crate::deserializer::{ErrorResponse, SearchResponse};
use futures::future::join_all;
use reqwest::StatusCode;
use std::time::Duration;
use tokio::time::sleep;

const FOFA_BASE_URL: &str = "https://en.fofa.info/api/v1/search/all";
const RESULTS_PER_PAGE: u32 = 100;
const MAX_RETRIES: u32 = 3;
const RATE_LIMIT_SLEEP_SECS: u64 = 5;
const RETRY_BACKOFF_SECS: u64 = 2;

#[derive(Debug)]
pub enum SearchError {
    Request(reqwest::Error),
    Json(serde_json::Error),
    LimitExceeded(String),
    InvalidQuery,
    Semaphore,
}

impl From<reqwest::Error> for SearchError {
    fn from(err: reqwest::Error) -> Self {
        SearchError::Request(err)
    }
}

impl From<serde_json::Error> for SearchError {
    fn from(err: serde_json::Error) -> Self {
        SearchError::Json(err)
    }
}

impl std::fmt::Display for SearchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SearchError::Request(e) => write!(f, "request failed: {e}"),
            SearchError::Json(e) => write!(f, "json parse failed: {e}"),
            SearchError::LimitExceeded(msg) => write!(f, "rate limit: {msg}"),
            SearchError::InvalidQuery => write!(f, "invalid query format"),
            SearchError::Semaphore => write!(f, "internal concurrency error"),
        }
    }
}

impl std::error::Error for SearchError {}

pub struct FofaSearcher {
    client: reqwest::Client,
    apikey: String,
    sem: tokio::sync::Semaphore,
}

impl FofaSearcher {
    pub fn new(apikey: &str, threads: u8) -> Self {
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            reqwest::header::ACCEPT,
            reqwest::header::HeaderValue::from_static("application/json"),
        );

        let client = reqwest::ClientBuilder::new()
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
            .default_headers(headers)
            .build()
            .expect("build http client");

        Self {
            client,
            apikey: apikey.to_string(),
            sem: tokio::sync::Semaphore::new(threads as usize),
        }
    }

    pub async fn search(&self, query: &str) -> Result<Vec<SearchResponse>, SearchError> {
        println!("🔍 Starting search…");

        let initial = self
            .rate_limited_get(self.build_uri(query, 1), Some(1))
            .await?;

        if initial.error {
            return Err(SearchError::InvalidQuery);
        }

        let total_size = initial.size;
        let total_pages = (total_size + RESULTS_PER_PAGE - 1) / RESULTS_PER_PAGE;

        println!("\n📊 Statistics: {} results, {} pages", total_size, total_pages);

        let mut results = Vec::with_capacity(total_pages as usize);
        results.push(initial);

        if total_pages > 1 {
            println!("📚 Fetching remaining {} pages…\n", total_pages - 1);

            let futures: Vec<_> = (2..=total_pages)
                .map(|page| {
                    let uri = self.build_uri(query, page);
                    self.rate_limited_get(uri, Some(page))
                })
                .collect();

            for response in join_all(futures).await {
                match response {
                    Ok(r) => results.push(r),
                    Err(e) => eprintln!("❌ Error processing response: {e}"),
                }
            }
        }

        println!(
            "\n✨ Completed: fetched {}/{} pages",
            results.len(),
            total_pages
        );
        Ok(results)
    }

    async fn rate_limited_get(
        &self,
        uri: String,
        page: Option<u32>,
    ) -> Result<SearchResponse, SearchError> {
        let _permit = self
            .sem
            .acquire()
            .await
            .map_err(|_| SearchError::Semaphore)?;

        if let Some(n) = page {
            println!("📄 Fetching page {n}…");
        }

        self.get_with_retry(uri).await
    }

    async fn get_with_retry(&self, uri: String) -> Result<SearchResponse, SearchError> {
        for attempt in 1..=MAX_RETRIES {
            match self.get(&uri).await {
                Ok(resp) => return Ok(resp),
                Err(SearchError::LimitExceeded(msg)) => {
                    eprintln!("⏳ Rate limited ({attempt}/{MAX_RETRIES}): {msg}");
                    if attempt < MAX_RETRIES {
                        sleep(Duration::from_secs(
                            RATE_LIMIT_SLEEP_SECS + (attempt as u64 * RETRY_BACKOFF_SECS),
                        ))
                        .await;
                    }
                }
                Err(e) => return Err(e),
            }
        }
        Err(SearchError::LimitExceeded(
            "max retries exceeded".to_string(),
        ))
    }

    async fn get(&self, uri: &str) -> Result<SearchResponse, SearchError> {
        let response = self.client.get(uri).send().await?;

        match response.status() {
            StatusCode::TOO_MANY_REQUESTS => Err(SearchError::LimitExceeded(
                "429 Too Many Requests".to_string(),
            )),
            StatusCode::BAD_REQUEST | StatusCode::NOT_FOUND => Err(SearchError::InvalidQuery),
            _ => {
                let text = response.text().await?;
                if text.contains("errmsg") {
                    let err: ErrorResponse = serde_json::from_str(&text)?;
                    return Err(SearchError::LimitExceeded(err.errmsg));
                }
                Ok(serde_json::from_str(&text)?)
            }
        }
    }

    fn build_uri(&self, query: &str, page: u32) -> String {
        format!(
            "{FOFA_BASE_URL}?key={key}&qbase64={query}&page={page}",
            key = self.apikey
        )
    }
}
