pub mod fofa_searcher {
    use crate::deserializer::fofa_response;
    use futures::future::join_all;
    use reqwest;
    use serde_json;
    use std::result::Result;

    #[allow(dead_code)]
    #[derive(Debug)]
    pub enum SearchError {
        RequestError(reqwest::Error),
        JsonError(serde_json::Error),
        LimitExceeded(String),
        InvalidQuery,
        SemaphoreError,
    }

    impl From<reqwest::Error> for SearchError {
        fn from(err: reqwest::Error) -> Self {
            SearchError::RequestError(err)
        }
    }

    impl From<serde_json::Error> for SearchError {
        fn from(err: serde_json::Error) -> Self {
            SearchError::JsonError(err)
        }
    }

    pub struct FofaSearcher<'a> {
        client: reqwest::Client,
        apikey: &'a str,
        sem: tokio::sync::Semaphore,
    }

    pub trait Searcher {
        async fn search(
            &self,
            query: &str,
        ) -> Result<Vec<fofa_response::SearchResponse>, SearchError>;
        async fn get(&self, uri: String) -> Result<fofa_response::SearchResponse, SearchError>;
    }

    impl<'a> FofaSearcher<'a> {
        pub fn new(apikey: &'a str, threads: u8) -> FofaSearcher {
            FofaSearcher {
                client: reqwest::ClientBuilder::new()
                    .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/58.0.3029.110 Safari/537.36")
                    .default_headers({
                        let mut headers = reqwest::header::HeaderMap::new();
                        headers.insert(
                            reqwest::header::ACCEPT,
                            reqwest::header::HeaderValue::from_static("application/json"),
                        );
                        headers.insert(
                            reqwest::header::CONTENT_TYPE,
                            reqwest::header::HeaderValue::from_static("application/json"),
                        );
                        headers
                    })
                    .build()
                    .unwrap(),
                apikey,
                sem: tokio::sync::Semaphore::new(threads as usize),
            }
        }

        async fn rate_limited_get(
            &self,
            uri: String,
            page: Option<u32>,
        ) -> Result<fofa_response::SearchResponse, SearchError> {
            let permit = self
                .sem
                .acquire()
                .await
                .map_err(|_| SearchError::SemaphoreError)?;
            drop(permit);
            if let Some(page_num) = page {
                println!("📄 Fetching page {} ...", page_num);
            }

            let result = self.get(uri).await?;
            // Print status based on result
            Ok(result)
        }
    }

    impl<'a> Searcher for FofaSearcher<'a> {
        async fn get(&self, uri: String) -> Result<fofa_response::SearchResponse, SearchError> {
            let response = self.client.get(&uri).send().await?;

            match response.status() {
                reqwest::StatusCode::TOO_MANY_REQUESTS => {
                    println!("🚫 Rate limit exceeded, sleeping for 5 seconds...");
                    tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
                    Err(SearchError::LimitExceeded(
                        "Rate limit exceeded".to_string(),
                    ))
                }
                reqwest::StatusCode::BAD_REQUEST => Err(SearchError::InvalidQuery),
                reqwest::StatusCode::NOT_FOUND => Err(SearchError::InvalidQuery),
                _ => {
                    let text = response.text().await?;
                    if text.contains("errmsg") {
                        let error_response: fofa_response::ErrorResponse =
                            serde_json::from_str(&text)?;
                        return Err(SearchError::LimitExceeded(error_response.errmsg));
                    }
                    let result = serde_json::from_str(&text)?;
                    Ok(result)
                }
            }
        }

        async fn search(
            &self,
            query: &str,
        ) -> Result<Vec<fofa_response::SearchResponse>, SearchError> {
            println!("🔍 Starting search with query: {}", query);

            let initial_uri = format!(
                "https://en.fofa.info/api/v1/search/all?&key={key}&qbase64={query}",
                key = self.apikey,
                query = query
            );

            let initial_response = self.rate_limited_get(initial_uri, Some(1)).await?;
            if initial_response.error {
                return Err(SearchError::InvalidQuery);
            }
            let total_size = initial_response.size;
            let total_pages = (total_size + 99) / 100; // Round up division

            println!("\n📊 Search Statistics:");
            println!("   Total Results: {}", total_size);
            println!("   Total Pages: {}", total_pages);
            println!("   Results per Page: 100\n");

            let mut results = Vec::with_capacity(total_pages as usize);
            results.push(initial_response);

            if total_pages > 1 {
                println!("📚 Fetching remaining {} pages...\n", total_pages - 1);

                let futures: Vec<_> = (2..=total_pages)
                    .map(|page| {
                        let uri = format!(
                            "https://en.fofa.info/api/v1/search/all?&key={key}&qbase64={query}&page={page}",
                            key = self.apikey,
                            query = query,
                            page = page
                        );
                        self.rate_limited_get(uri, Some(page))
                    })
                    .collect();

                let responses = join_all(futures).await;

                for response in responses {
                    match response {
                        Ok(response) => results.push(response),
                        Err(e) => eprintln!("❌ Error processing response: {:?}", e),
                    }
                }
            }

            println!("\n✨ Search completed!");
            println!(
                "   Successfully fetched: {}/{} pages",
                results.len(),
                total_pages
            );

            Ok(results)
        }
    }
}
