use chrono::{DateTime, Utc};
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default)]
pub struct GitlabUser {
    username: String,
    token: String,
}

#[derive(Debug, Deserialize, Default)]
pub struct Commit {
    id: String,
    short_id: String,
    title: String,
    author_name: String,
}

#[derive(Debug, Deserialize, Serialize, Default)]
pub struct  Repository {
    pub id: u32,
    pub name: String,
    pub description: Option<String>,
    pub visibility: String,
}

// get user commit message
impl GitlabUser {

    pub fn new(token: String) -> GitlabUser {
        
        GitlabUser { username: "".to_string(), token: token }
    }

    pub fn set_user(&mut self, username: String) {
        self.username = username
    }

    pub async fn get_commit(&self, repo_id: u32, date: DateTime<Utc>) -> Result<Vec<Commit>, Box<dyn std::error::Error>> {
        let url = format!("https://gitlab.com/api/v4/projects/{}/repository/commits?since={}", repo_id, date);
        let client = reqwest::Client::new();
        let mut headers = HeaderMap::new();
        headers.insert(AUTHORIZATION, HeaderValue::from_str(&format!("Bearer {}", self.token))?);

        let response = client.get(&url).headers(headers).send().await?;
        let commits = response.json::<Vec<Commit>>().await?;

        Ok(commits)
        
    }

    pub async fn get_repositories(&self) -> Result<Vec<Repository>, Box<dyn std::error::Error>> {
        let url = "https://gitlab.com/api/v4/projects";
        let client = reqwest::Client::new();
        let mut headers = HeaderMap::new();
        headers.insert(AUTHORIZATION, HeaderValue::from_str(&format!("Bearer {}", self.token))?);

        let response = client.get(url).headers(headers).send().await?;
        let repositories = response.json::<Vec<Repository>>().await?;

        Ok(repositories)
    }

    pub fn set_token(&mut self, token: String) {
        self.token = token
    }
}
