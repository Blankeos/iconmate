use reqwest::{Client, StatusCode, Url};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use std::collections::HashMap;

const DEFAULT_ICONIFY_BASE_URL: &str = "https://api.iconify.design";
pub const ICONIFY_BASE_URL_ENV: &str = "ICONMATE_ICONIFY_BASE_URL";

#[derive(Debug, Clone)]
pub struct IconifyClient {
    client: Client,
    base_url: Url,
}

impl IconifyClient {
    pub fn new() -> Result<Self, IconifyError> {
        Self::from_base_url(DEFAULT_ICONIFY_BASE_URL)
    }

    pub fn from_env() -> Result<Self, IconifyError> {
        let base = std::env::var(ICONIFY_BASE_URL_ENV)
            .unwrap_or_else(|_| DEFAULT_ICONIFY_BASE_URL.to_string());
        Self::from_base_url(&base)
    }

    pub fn from_base_url(base_url: &str) -> Result<Self, IconifyError> {
        let normalized = if base_url.ends_with('/') {
            base_url.to_string()
        } else {
            format!("{base_url}/")
        };

        let base_url = Url::parse(&normalized).map_err(|source| IconifyError::InvalidBaseUrl {
            base_url: base_url.to_string(),
            source: source.to_string(),
        })?;

        Ok(Self {
            client: Client::new(),
            base_url,
        })
    }

    pub async fn collections(&self) -> Result<IconifyCollectionsResponse, IconifyError> {
        let collections: HashMap<String, IconifyCollectionMeta> =
            self.get_json("collections", &[]).await?;
        Ok(IconifyCollectionsResponse { collections })
    }

    pub async fn collection(
        &self,
        prefix: &str,
    ) -> Result<IconifyCollectionResponse, IconifyError> {
        let response: IconifyCollectionApiResponse = self
            .get_json("collection", &[("prefix".to_string(), prefix.to_string())])
            .await?;

        let icons = merge_collection_icons(
            response.icons,
            response.uncategorized.as_ref(),
            response.categories.as_ref(),
        );

        Ok(IconifyCollectionResponse {
            prefix: response.prefix,
            icons,
            uncategorized: response.uncategorized,
        })
    }

    pub async fn search(
        &self,
        query: &str,
        limit: Option<u32>,
        start: Option<u32>,
        include_collections: bool,
    ) -> Result<IconifySearchResponse, IconifyError> {
        let mut params = vec![("query".to_string(), query.to_string())];

        if let Some(limit) = limit {
            params.push(("limit".to_string(), limit.to_string()));
        }

        if let Some(start) = start {
            params.push(("start".to_string(), start.to_string()));
        }

        let mut response: IconifySearchResponse = self.get_json("search", &params).await?;

        if !include_collections {
            response.collections = None;
        }

        Ok(response)
    }

    pub async fn svg(&self, prefix_icon: &str) -> Result<String, IconifyError> {
        let path = format!("{prefix_icon}.svg");
        self.get_text(&path, &[]).await
    }

    pub async fn icon_json(
        &self,
        prefix: &str,
        icon: &str,
    ) -> Result<serde_json::Value, IconifyError> {
        let path = format!("{prefix}.json");
        self.get_json(&path, &[("icons".to_string(), icon.to_string())])
            .await
    }

    pub async fn icon_json_by_name(
        &self,
        prefix_icon: &str,
    ) -> Result<serde_json::Value, IconifyError> {
        let (prefix, icon) = prefix_icon
            .split_once(':')
            .ok_or_else(|| IconifyError::InvalidIconName(prefix_icon.to_string()))?;
        self.icon_json(prefix, icon).await
    }

    async fn get_json<T: DeserializeOwned>(
        &self,
        path: &str,
        query: &[(String, String)],
    ) -> Result<T, IconifyError> {
        let url = self.build_url(path, query)?;
        let endpoint = url.to_string();

        let response = self
            .client
            .get(url)
            .send()
            .await
            .map_err(IconifyError::Network)?;
        let status = response.status();
        let body = response.text().await.map_err(IconifyError::Network)?;

        if !status.is_success() {
            return Err(IconifyError::HttpStatus {
                status,
                endpoint,
                body,
            });
        }

        serde_json::from_str(&body).map_err(|source| IconifyError::JsonDecode { endpoint, source })
    }

    async fn get_text(
        &self,
        path: &str,
        query: &[(String, String)],
    ) -> Result<String, IconifyError> {
        let url = self.build_url(path, query)?;
        let endpoint = url.to_string();

        let response = self
            .client
            .get(url)
            .send()
            .await
            .map_err(IconifyError::Network)?;
        let status = response.status();
        let body = response.text().await.map_err(IconifyError::Network)?;

        if !status.is_success() {
            return Err(IconifyError::HttpStatus {
                status,
                endpoint,
                body,
            });
        }

        Ok(body)
    }

    fn build_url(&self, path: &str, query: &[(String, String)]) -> Result<Url, IconifyError> {
        let relative_path = if path.starts_with('/') {
            format!("./{}", path.trim_start_matches('/'))
        } else {
            format!("./{path}")
        };

        let mut url =
            self.base_url
                .join(&relative_path)
                .map_err(|source| IconifyError::InvalidEndpoint {
                    path: path.to_string(),
                    source: source.to_string(),
                })?;

        if !query.is_empty() {
            let mut pairs = url.query_pairs_mut();
            for (key, value) in query {
                pairs.append_pair(key, value);
            }
        }

        Ok(url)
    }
}

fn merge_collection_icons(
    icons: Vec<String>,
    uncategorized: Option<&Vec<String>>,
    categories: Option<&HashMap<String, Vec<String>>>,
) -> Vec<String> {
    let mut merged = icons;

    if let Some(uncategorized) = uncategorized {
        merged.extend(uncategorized.iter().cloned());
    }

    if let Some(categories) = categories {
        for category_icons in categories.values() {
            merged.extend(category_icons.iter().cloned());
        }
    }

    let mut deduped = Vec::new();
    let mut seen = std::collections::HashSet::new();

    for icon in merged {
        if seen.insert(icon.clone()) {
            deduped.push(icon);
        }
    }

    deduped
}

#[derive(Debug)]
pub enum IconifyError {
    InvalidBaseUrl {
        base_url: String,
        source: String,
    },
    InvalidEndpoint {
        path: String,
        source: String,
    },
    InvalidIconName(String),
    Network(reqwest::Error),
    HttpStatus {
        status: StatusCode,
        endpoint: String,
        body: String,
    },
    JsonDecode {
        endpoint: String,
        source: serde_json::Error,
    },
}

impl std::fmt::Display for IconifyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IconifyError::InvalidBaseUrl { base_url, .. } => {
                write!(f, "invalid Iconify base URL: {base_url}")
            }
            IconifyError::InvalidEndpoint { path, .. } => {
                write!(f, "invalid Iconify endpoint path: {path}")
            }
            IconifyError::InvalidIconName(name) => {
                write!(
                    f,
                    "invalid Iconify icon name (expected <prefix:icon>): {name}"
                )
            }
            IconifyError::Network(source) => write!(f, "Iconify network error: {source}"),
            IconifyError::HttpStatus {
                status, endpoint, ..
            } => {
                write!(f, "Iconify request failed ({status}) for {endpoint}")
            }
            IconifyError::JsonDecode { endpoint, source } => {
                write!(
                    f,
                    "failed to parse Iconify response from {endpoint}: {source}"
                )
            }
        }
    }
}

impl std::error::Error for IconifyError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            IconifyError::InvalidBaseUrl { .. } => None,
            IconifyError::InvalidEndpoint { .. } => None,
            IconifyError::InvalidIconName(_) => None,
            IconifyError::Network(source) => Some(source),
            IconifyError::JsonDecode { source, .. } => Some(source),
            IconifyError::HttpStatus { .. } => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IconifySearchResponse {
    #[serde(default)]
    pub icons: Vec<String>,
    #[serde(default)]
    pub total: u32,
    #[serde(default)]
    pub limit: u32,
    #[serde(default)]
    pub start: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub collections: Option<HashMap<String, serde_json::Value>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IconifyCollectionsResponse {
    pub collections: HashMap<String, IconifyCollectionMeta>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct IconifyCollectionMeta {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub total: Option<u32>,
    #[serde(flatten, default)]
    pub extra: HashMap<String, serde_json::Value>,
}

impl IconifyCollectionMeta {
    pub fn display_name(&self, fallback: &str) -> String {
        if let Some(name) = &self.name {
            return name.clone();
        }

        if let Some(title) = &self.title {
            return title.clone();
        }

        fallback.to_string()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IconifyCollectionResponse {
    pub prefix: String,
    #[serde(default)]
    pub icons: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub uncategorized: Option<Vec<String>>,
}

#[derive(Debug, Clone, Deserialize)]
struct IconifyCollectionApiResponse {
    pub prefix: String,
    #[serde(default)]
    pub icons: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub uncategorized: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub categories: Option<HashMap<String, Vec<String>>>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_search_response_without_collections() {
        let fixture = r#"
        {
            "icons": ["mdi:home", "mdi:heart"],
            "total": 2,
            "limit": 20,
            "start": 0
        }
        "#;

        let response: IconifySearchResponse =
            serde_json::from_str(fixture).expect("fixture should deserialize");

        assert_eq!(response.icons.len(), 2);
        assert_eq!(response.total, 2);
        assert!(response.collections.is_none());
    }

    #[test]
    fn parse_search_response_with_collections() {
        let fixture = r#"
        {
            "icons": ["mdi:home"],
            "total": 1,
            "limit": 1,
            "start": 0,
            "collections": {
                "mdi": {"name": "Material Design Icons", "total": 7000}
            }
        }
        "#;

        let response: IconifySearchResponse =
            serde_json::from_str(fixture).expect("fixture should deserialize");

        assert_eq!(response.icons, vec!["mdi:home"]);
        assert!(response.collections.is_some());
        assert!(
            response
                .collections
                .as_ref()
                .expect("collections should exist")
                .contains_key("mdi")
        );
    }

    #[test]
    fn parse_collections_response() {
        let fixture = r#"
        {
            "mdi": {
                "name": "Material Design Icons",
                "total": 7447
            },
            "heroicons": {
                "title": "Heroicons",
                "total": 292
            }
        }
        "#;

        let response: HashMap<String, IconifyCollectionMeta> =
            serde_json::from_str(fixture).expect("fixture should deserialize");

        let mdi = response.get("mdi").expect("mdi should exist");
        assert_eq!(mdi.display_name("mdi"), "Material Design Icons");
        assert_eq!(mdi.total, Some(7447));

        let heroicons = response.get("heroicons").expect("heroicons should exist");
        assert_eq!(heroicons.display_name("heroicons"), "Heroicons");
    }

    #[test]
    fn parse_collection_response_with_optional_uncategorized() {
        let fixture = r#"
        {
            "prefix": "mdi",
            "icons": ["home", "heart"],
            "uncategorized": ["orphan"]
        }
        "#;

        let response: IconifyCollectionResponse =
            serde_json::from_str(fixture).expect("fixture should deserialize");

        assert_eq!(response.prefix, "mdi");
        assert_eq!(response.icons, vec!["home", "heart"]);
        assert_eq!(response.uncategorized, Some(vec!["orphan".to_string()]));
    }

    #[test]
    fn merge_icons_from_uncategorized_and_categories() {
        let categories = HashMap::from([
            (
                "Actions".to_string(),
                vec!["home".to_string(), "heart".to_string()],
            ),
            (
                "Shapes".to_string(),
                vec!["star".to_string(), "home".to_string()],
            ),
        ]);

        let merged = merge_collection_icons(
            Vec::new(),
            Some(&vec!["orphan".to_string(), "home".to_string()]),
            Some(&categories),
        );

        assert!(merged.contains(&"orphan".to_string()));
        assert!(merged.contains(&"home".to_string()));
        assert!(merged.contains(&"heart".to_string()));
        assert!(merged.contains(&"star".to_string()));
        assert_eq!(
            merged.iter().filter(|icon| icon.as_str() == "home").count(),
            1
        );
    }
}
