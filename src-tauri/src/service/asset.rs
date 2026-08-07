use crate::api::endpoint;
use crate::api::request::{ApiRequestClient, ApiResponse};
use log::info;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::cmp::Ordering;
use std::collections::HashSet;
use url::Url;

#[derive(Debug, Serialize, Deserialize, Clone, Copy, Default)]
#[serde(rename_all = "lowercase")]
pub enum Category {
    #[default]
    Linux,
    Windows,
    #[serde(rename = "windows_ad")]
    WindowsAd,
    Unix,
    Other,
    Database,
    Device,
    Web,
}

#[derive(Debug, Deserialize)]
struct AssetListResponse {
    count: usize,
    next: Option<String>,
    previous: Option<String>,
    results: Vec<Value>,
}

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
pub struct AssetQuery {
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub r#type: Option<Category>,

    #[serde(rename = "category", skip_serializing_if = "Option::is_none")]
    pub category: Option<Category>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub offset: Option<u32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub search: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub order: Option<String>,

    #[serde(default)]
    pub oid: String,
}

impl AssetQuery {
    // Initialize asset query parameters from the asset type and organization
    #[allow(dead_code)]
    pub fn new(asset_type: Category, org: String) -> Self {
        // The r#type form is used because `type` is a Rust keyword, so it must be written as r#type
        let (r#type, category) = match asset_type {
            Category::Database | Category::Device => (None, Some(asset_type)),
            Category::Web => (None, Some(asset_type)),
            Category::Linux
            | Category::Windows
            | Category::WindowsAd
            | Category::Unix
            | Category::Other => (Some(asset_type), None),
        };

        Self {
            r#type,
            category,
            offset: None,
            limit: None,
            search: None,
            order: None,
            oid: org,
        }
    }

    /// Get the asset category used by the current query
    pub fn get_category(&self) -> Category {
        // Prefer category; fall back to type if it's not set
        self.category.or(self.r#type).unwrap_or_default()
    }
}

#[derive(Serialize)]
struct RenameBody {
    asset: String,
    name: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    oid: String,
}

#[derive(Serialize)]
struct FavoriteAssetBody {
    asset: String,
}

pub struct AssetService {
    api: ApiRequestClient,
}

impl AssetService {
    pub fn new(api: ApiRequestClient) -> Self {
        Self { api }
    }

    /// Get the asset list for a given category, supporting both regular assets and favorite-node assets
    pub async fn get_category_assets(&self, query: &AssetQuery, favorite: bool) -> ApiResponse {
        let path = if favorite {
            endpoint::assets::FAVORITE_NODE_ASSETS
        } else {
            endpoint::assets::USER_ASSETS
        };

        let url = self.api.endpoint(path);

        info!(
            "Fetching asset info of type: {:?}, request url: {}, oid: {}",
            query.get_category(),
            url,
            query.oid
        );
        info!("query: {:?}", query);

        if favorite {
            let favorite_query = AssetQuery {
                r#type: None,
                category: None,
                offset: Some(query.offset.unwrap_or(0)),
                limit: Some(query.limit.unwrap_or(20)),
                search: Some(query.search.clone().unwrap_or_default()),
                order: Some(query.order.clone().unwrap_or_default()),
                oid: query.oid.clone(),
            };

            return self
                .api
                .get_with_query_response(&url, &favorite_query)
                .await;
        }

        let queries = Self::expand_queries(query);

        if queries.len() == 1 {
            return self.api.get_with_query_response(&url, &queries[0]).await;
        }

        self.get_combined_category_assets(&url, query, &queries)
            .await
    }

    /// Get the current user's favorite asset list
    pub async fn get_favorite_assets(&self) -> ApiResponse {
        let url = self.api.endpoint(endpoint::assets::FAVORITE_ASSETS);
        self.api.get_with_response(&url).await
    }

    /// Get the detail info for a given asset
    pub async fn get_asset_detail(&self, asset_id: &str) -> ApiResponse {
        let path = endpoint::assets::detail(asset_id);
        let url = self.api.endpoint(&path);

        self.api.get_with_response(&url).await
    }

    /// Add an asset to favorites
    pub async fn favorite(&self, asset_id: &str) -> ApiResponse {
        let url = self.api.endpoint(endpoint::assets::FAVORITE_ASSETS);
        let body = FavoriteAssetBody {
            asset: asset_id.to_string(),
        };

        self.api.post_json_with_response(&url, &body).await
    }

    /// Remove an asset from the favorites list
    pub async fn unfavorite(&self, asset_id: &str) -> ApiResponse {
        let mut url = self.api.endpoint(endpoint::assets::FAVORITE_ASSETS);

        if let Ok(mut parsed) = Url::parse(&url) {
            parsed.query_pairs_mut().append_pair("asset", asset_id);
            url = parsed.to_string();
        };

        self.api.delete_with_response(&url).await
    }

    /// Submit an asset rename request
    pub async fn rename(&self, asset_id: &str, name: &str, oid: &str) -> ApiResponse {
        let url = self.api.endpoint(endpoint::assets::MY_ASSET);
        let body = RenameBody {
            asset: asset_id.to_string(),
            name: name.to_string(),
            oid: oid.to_string(),
        };

        self.api.post_json_with_response(&url, &body).await
    }

    fn expand_queries(query: &AssetQuery) -> Vec<AssetQuery> {
        let offset = Some(query.offset.unwrap_or(0));
        let limit = Some(query.limit.unwrap_or(20));
        let search = Some(query.search.clone().unwrap_or_default());
        let order = Some(query.order.clone().unwrap_or_default());
        let oid = query.oid.clone();

        let exact = |r#type: Option<Category>, category: Option<Category>| AssetQuery {
            r#type,
            category,
            offset,
            limit,
            search: search.clone(),
            order: order.clone(),
            oid: oid.clone(),
        };

        // The menu doesn't map 1:1 to JumpServer's raw type/category:
        // Windows needs to merge windows + windows_ad, Other needs to merge unix + other.
        match query.get_category() {
            Category::Linux => vec![exact(Some(Category::Linux), None)],
            Category::Windows => vec![
                exact(Some(Category::Windows), None),
                exact(Some(Category::WindowsAd), None),
            ],
            Category::WindowsAd => vec![exact(Some(Category::WindowsAd), None)],
            Category::Unix => vec![exact(Some(Category::Unix), None)],
            Category::Other => vec![
                exact(Some(Category::Unix), None),
                exact(Some(Category::Other), None),
            ],
            Category::Database => vec![exact(None, Some(Category::Database))],
            Category::Device => vec![exact(None, Some(Category::Device))],
            Category::Web => vec![exact(None, Some(Category::Web))],
        }
    }

    async fn get_combined_category_assets(
        &self,
        url: &str,
        query: &AssetQuery,
        queries: &[AssetQuery],
    ) -> ApiResponse {
        let mut combined: Vec<Value> = Vec::new();
        let mut seen_ids = HashSet::new();

        for sub_query in queries {
            let response = self.fetch_all_assets(url, sub_query).await;
            let list = match response {
                Ok(list) => list,
                Err(err) => return err,
            };

            for item in list {
                let asset_id = item
                    .get("id")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string();

                // Dedupe by asset ID when merging multiple endpoint results, to avoid duplicate display if the backend types converge in the future.
                if asset_id.is_empty() || seen_ids.insert(asset_id) {
                    combined.push(item);
                }
            }
        }

        // After concatenating multiple independent queries, a full re-sort is needed, otherwise the list stays segmented by sub-query.
        Self::sort_assets(&mut combined, query.order.as_deref());

        let total = combined.len();
        let offset = query.offset.unwrap_or(0) as usize;
        let limit = query.limit.unwrap_or(20) as usize;
        let results = combined
            .into_iter()
            .skip(offset)
            .take(limit)
            .collect::<Vec<_>>();

        ApiResponse::ok(
            200,
            json!({
                "count": total,
                "next": Value::Null,
                "previous": Value::Null,
                "results": results,
            })
            .to_string(),
        )
    }

    async fn fetch_all_assets(
        &self,
        url: &str,
        query: &AssetQuery,
    ) -> Result<Vec<Value>, ApiResponse> {
        const BATCH_LIMIT: u32 = 200;

        let mut offset = 0;
        let mut all_results = Vec::new();

        loop {
            let page_query = AssetQuery {
                r#type: query.r#type,
                category: query.category,
                offset: Some(offset),
                limit: Some(BATCH_LIMIT),
                search: Some(query.search.clone().unwrap_or_default()),
                order: Some(query.order.clone().unwrap_or_default()),
                oid: query.oid.clone(),
            };

            let response = self.api.get_with_query_response(url, &page_query).await;
            if !response.success {
                return Err(response);
            }

            let payload: AssetListResponse = match serde_json::from_str(&response.data) {
                Ok(payload) => payload,
                Err(error) => {
                    return Err(ApiResponse::failed(format!(
                        "parse asset list response failed: {}",
                        error
                    )))
                }
            };

            let AssetListResponse {
                count,
                next: _next,
                previous: _previous,
                results,
            } = payload;

            let page_size = results.len();
            all_results.extend(results);

            // Deliberately page through all backend results here, so the frontend menu gets an accurate total and slice in the "combined type" case.
            if page_size == 0 || all_results.len() >= count {
                break;
            }

            offset += page_size as u32;
        }

        Ok(all_results)
    }

    fn sort_assets(results: &mut [Value], order: Option<&str>) {
        let normalized = order
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .unwrap_or("name");

        // This client only supports sorting by name / date_updated; fall back to name when empty to keep combined results stable.
        let descending = normalized.starts_with('-');
        let field = normalized.trim_start_matches('-');

        results.sort_by(|left, right| {
            let ordering = Self::compare_asset_field(left, right, field);
            if descending {
                ordering.reverse()
            } else {
                ordering
            }
        });
    }

    fn compare_asset_field(left: &Value, right: &Value, field: &str) -> Ordering {
        let left_value = Self::extract_asset_field(left, field);
        let right_value = Self::extract_asset_field(right, field);

        left_value.cmp(&right_value).then_with(|| {
            Self::extract_asset_field(left, "id").cmp(&Self::extract_asset_field(right, "id"))
        })
    }

    fn extract_asset_field(item: &Value, field: &str) -> String {
        item.get(field)
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_lowercase()
    }
}
