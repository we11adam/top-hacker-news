use serde::Deserialize;
use worker::{Error, Fetch, Result, Url, console_error};

#[derive(Debug, Deserialize)]
pub struct Story {
    pub id: u64,
    #[serde(default)]
    pub title: String,
    pub url: Option<String>,
    #[serde(default)]
    pub score: u64,
    #[serde(default)]
    pub descendants: u64, // comment count
}

impl Story {
    pub fn link_url(&self) -> String {
        self.url.clone().unwrap_or_else(|| self.comments_url())
    }

    pub fn comments_url(&self) -> String {
        format!("https://news.ycombinator.com/item?id={}", self.id)
    }
}

pub struct HackerNewsClient;

impl HackerNewsClient {
    const API_BASE: &'static str = "https://hacker-news.firebaseio.com/v0";

    pub async fn get_top_stories(&self, limit: usize) -> Result<Vec<u64>> {
        let url = Url::parse(&format!("{}/topstories.json", Self::API_BASE))?;
        let mut resp = Fetch::Url(url).send().await?;

        let status = resp.status_code();
        if status != 200 {
            return Err(Error::RustError(format!(
                "Failed to fetch top stories: status {}",
                status
            )));
        }

        let text = resp.text().await?;
        let ids: Vec<u64> = serde_json::from_str(&text)
            .map_err(|e| Error::RustError(format!("Failed to parse story IDs: {}", e)))?;

        Ok(ids.into_iter().take(limit).collect())
    }

    pub async fn get_story(&self, id: u64) -> Result<Story> {
        let url = Url::parse(&format!("{}/item/{}.json", Self::API_BASE, id))?;
        let mut resp = Fetch::Url(url).send().await?;

        let status = resp.status_code();
        if status != 200 {
            console_error!("Failed to fetch story {}: status {}", id, status);
            return Err(Error::RustError(format!(
                "Failed to fetch story {}: status {}",
                id, status
            )));
        }

        let text = resp.text().await?;
        let story: Story = serde_json::from_str(&text)
            .map_err(|e| Error::RustError(format!("Failed to parse story {}: {}", id, e)))?;

        Ok(story)
    }
}
