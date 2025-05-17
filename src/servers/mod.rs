use once_cell::sync::Lazy;
use reqwest::{self, header};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use anyhow::Result;
use futures_util::StreamExt;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Provider {
    pub name: String,
    pub description: String,
    pub urls: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UrlTestResult {
    pub speed: f64,
    pub status: u16,
}

static SPEED_TEST_SERVERS: Lazy<HashMap<String, Provider>> = Lazy::new(|| {
    let mut servers = HashMap::new();
    
    servers.insert(
        "selectel".to_string(),
        Provider {
            name: "Selectel".to_string(),
            description: "Стабильные сервера с высокой скоростью".to_string(),
            urls: vec![
                "https://speedtest.selectel.ru/1GB".to_string(),
                "https://speedtest.selectel.ru/10GB".to_string(),
                "http://speedtest.selectel.ru/1GB".to_string(),
                "http://speedtest.selectel.ru/10GB".to_string(),
            ],
        },
    );
    
    servers.insert(
        "rastrnet".to_string(),
        Provider {
            name: "Rastrnet".to_string(),
            description: "Надежные сервера с хорошей скоростью".to_string(),
            urls: vec![
                "http://speedtest.rastrnet.ru/1GB.zip".to_string(),
                "http://speedtest.rastrnet.ru/500MB.zip".to_string(),
                "https://speedtest.rastrnet.ru/500MB.zip".to_string(),
                "https://speedtest.rastrnet.ru/1GB.zip".to_string(),
            ],
        },
    );
    
    servers.insert(
        "iwakurahome".to_string(),
        Provider {
            name: "Lain Looking Glass".to_string(),
            description: "Тестовые файлы от меня".to_string(),
            urls: vec![
                "http://lg.iwakurahome.ru/files/file_1GB.bin".to_string(),
                "http://lg.iwakurahome.ru/files/file_100MB.bin".to_string(),
                "http://lg.iwakurahome.ru/files/file_10GB.bin".to_string(),
            ],
        },
    );
    
    servers
});

pub struct SpeedTester {
    tested_urls: Arc<RwLock<HashMap<String, UrlTestResult>>>,
}

impl SpeedTester {
    pub fn new() -> Self {
        Self {
            tested_urls: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn test_url(&self, url: String) -> Result<()> {
        let client = reqwest::Client::builder()
            .danger_accept_invalid_certs(true)
            .build()?;

        let mut headers = header::HeaderMap::new();
        headers.insert(
            header::USER_AGENT,
            header::HeaderValue::from_static(
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36",
            ),
        );
        headers.insert(
            header::ACCEPT,
            header::HeaderValue::from_static("*/*"),
        );
        headers.insert(
            header::ACCEPT_ENCODING,
            header::HeaderValue::from_static("gzip, deflate"),
        );
        headers.insert(
            header::CONNECTION,
            header::HeaderValue::from_static("keep-alive"),
        );
        headers.insert(
            header::RANGE,
            header::HeaderValue::from_static("bytes=0-1048576"), // Test first megabyte
        );

        let start_time = Instant::now();
        
        match client.get(&url).headers(headers).timeout(Duration::from_secs(5)).send().await {
            Ok(response) => {
                let status = response.status().as_u16();
                let mut downloaded = 0;
                let mut stream = response.bytes_stream();
                
                while let Some(chunk_result) = stream.next().await {
                    if let Ok(chunk) = chunk_result {
                        downloaded += chunk.len();
                        if downloaded >= 1048576 {
                            break;
                        }
                    } else {
                        break;
                    }
                }
                
                let duration = start_time.elapsed();
                let speed = (downloaded as f64 / duration.as_secs_f64()) / (1024.0 * 1024.0); // MB/s
                
                let mut tested_urls = self.tested_urls.write().await;
                tested_urls.insert(url, UrlTestResult { speed, status });
            }
            Err(_) => {
                let mut tested_urls = self.tested_urls.write().await;
                tested_urls.insert(url, UrlTestResult { speed: 0.0, status: 0 });
            }
        }
        
        Ok(())
    }

    pub async fn test_all_urls(&self) -> Result<Vec<String>> {
        let urls = get_all_urls();
        let mut tasks = Vec::new();
        
        for url in urls {
            let url_clone = url.clone();
            let tester = self.clone();
            let task = tokio::spawn(async move {
                let _ = tester.test_url(url_clone).await;
            });
            tasks.push(task);
        }
        
        for task in tasks {
            let _ = task.await;
        }
        
        let tested_urls = self.tested_urls.read().await;
        
        let mut url_results: Vec<(String, &UrlTestResult)> = tested_urls
            .iter()
            .map(|(url, result)| (url.clone(), result))
            .collect();
        
        url_results.sort_by(|a, b| b.1.speed.partial_cmp(&a.1.speed).unwrap_or(std::cmp::Ordering::Equal));
        
        let working_urls: Vec<String> = url_results
            .into_iter()
            .filter(|(_, result)| (result.status == 200 || result.status == 206) && result.speed > 0.0)
            .map(|(url, _)| url)
            .collect();
        
        Ok(working_urls)
    }
}

impl Clone for SpeedTester {
    fn clone(&self) -> Self {
        Self {
            tested_urls: Arc::clone(&self.tested_urls),
        }
    }
}

pub fn get_all_urls() -> Vec<String> {
    let mut urls = Vec::new();
    for provider in SPEED_TEST_SERVERS.values() {
        urls.extend(provider.urls.clone());
    }
    urls
}

#[allow(dead_code)]
pub fn get_provider_urls(provider_name: &str) -> Vec<String> {
    match SPEED_TEST_SERVERS.get(&provider_name.to_lowercase()) {
        Some(provider) => provider.urls.clone(),
        None => Vec::new(),
    }
}

#[allow(dead_code)]
pub fn get_providers_list() -> Vec<(String, String, String)> {
    SPEED_TEST_SERVERS
        .iter()
        .map(|(k, v)| (k.clone(), v.name.clone(), v.description.clone()))
        .collect()
}

pub async fn get_sorted_urls() -> Result<Vec<String>> {
    let tester = SpeedTester::new();
    tester.test_all_urls().await
} 