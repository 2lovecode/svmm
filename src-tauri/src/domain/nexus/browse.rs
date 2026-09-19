use std::time::Duration;

use serde::Serialize;
use serde_json::{json, Value};

use crate::domain::nexus::client::{APPLICATION_NAME, APPLICATION_VERSION};
use crate::error::{AppError, AppResult};
use crate::storage::secure_key;

const GRAPHQL_URL: &str = "https://api.nexusmods.com/v2/graphql";
pub const PAGE_SIZE: u32 = 20;
/// Nexus game id for Stardew Valley. Required when a query filters by `modId`.
const STARDEW_GAME_ID: &str = "1303";

const BROWSE_QUERY: &str = r#"
query Browse($filter: ModsFilter, $sort: [ModsSort!], $offset: Int, $count: Int) {
  mods(
    filter: $filter
    sort: $sort
    offset: $offset
    count: $count
    viewUploaderHidden: false
  ) {
    totalCount
    nodes {
      modId
      name
      author
      summary
      version
      category
      downloads
      endorsements
      thumbnailUrl
    }
  }
}
"#;

const CATEGORIES_QUERY: &str = r#"
query Categories($filter: ModsFilter, $facets: ModsFacet) {
  mods(
    filter: $filter
    facets: $facets
    count: 0
    offset: 0
    viewUploaderHidden: false
  ) {
    facetsData
  }
}
"#;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct NexusCategory {
    pub name: String,
    pub count: u32,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct NexusCatalogMod {
    pub mod_id: u32,
    pub name: String,
    pub author: String,
    pub summary: String,
    pub version: String,
    pub category: String,
    pub downloads: u64,
    pub endorsements: u64,
    pub thumbnail_url: Option<String>,
    /// `missing` | `owned` | `update`, filled by the command layer.
    pub library_status: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct NexusBrowsePage {
    pub page: u32,
    pub page_size: u32,
    pub total: u32,
    pub mods: Vec<NexusCatalogMod>,
}

pub fn list_categories() -> AppResult<Vec<NexusCategory>> {
    let body = json!({
        "query": CATEGORIES_QUERY,
        "variables": {
            "filter": base_filter(None, None, None),
            "facets": { "categoryName": [] }
        }
    });
    let value = post_graphql(&body)?;
    parse_categories(&value)
}

pub fn browse_query(
    page: u32,
    category: Option<&str>,
    keyword: Option<&str>,
    mod_id: Option<u32>,
) -> AppResult<NexusBrowsePage> {
    let page = page.max(1);
    let body = browse_request_body_query(page, category, keyword, mod_id);
    let value = post_graphql(&body)?;
    let mut result = parse_browse_page(&value)?;
    result.page = page;
    Ok(result)
}

#[cfg(test)]
pub fn browse_request_body(page: u32, category: Option<&str>) -> Value {
    browse_request_body_query(page, category, None, None)
}

pub fn browse_request_body_query(
    page: u32,
    category: Option<&str>,
    keyword: Option<&str>,
    mod_id: Option<u32>,
) -> Value {
    let page = page.max(1);
    let offset = (page - 1) * PAGE_SIZE;
    json!({
        "query": BROWSE_QUERY,
        "variables": {
            "count": PAGE_SIZE,
            "offset": offset,
            "filter": base_filter(category, keyword, mod_id),
            "sort": [{ "downloads": { "direction": "DESC" } }]
        }
    })
}

fn base_filter(category: Option<&str>, keyword: Option<&str>, mod_id: Option<u32>) -> Value {
    let mut filter = json!({
        "gameDomainName": [{ "value": "stardewvalley", "op": "EQUALS" }],
        "adultContent": [{ "value": false, "op": "EQUALS" }]
    });
    if let Some(name) = category.map(str::trim).filter(|name| !name.is_empty()) {
        filter["categoryName"] = json!([{ "value": name, "op": "EQUALS" }]);
    }
    if let Some(word) = keyword.map(str::trim).filter(|word| !word.is_empty()) {
        // Nexus applies its own leading and trailing wildcards for this operator.
        // Wrapping the term in `*` makes the query match nothing.
        filter["name"] = json!([{ "value": word, "op": "WILDCARD" }]);
    }
    if let Some(id) = mod_id {
        filter["modId"] = json!([{ "value": id.to_string(), "op": "EQUALS" }]);
        filter["gameId"] = json!([{ "value": STARDEW_GAME_ID, "op": "EQUALS" }]);
    }
    filter
}

pub fn parse_categories(value: &Value) -> AppResult<Vec<NexusCategory>> {
    ensure_no_graphql_errors(value)?;
    let map = value
        .pointer("/data/mods/facetsData/categoryName")
        .and_then(Value::as_object)
        .ok_or_else(|| AppError::new("nexus_browse_failed", "无法解析 Nexus 分类"))?;
    let mut categories: Vec<NexusCategory> = map
        .iter()
        .filter_map(|(name, count)| {
            let count = count.as_u64()? as u32;
            if name.trim().is_empty() {
                None
            } else {
                Some(NexusCategory {
                    name: name.clone(),
                    count,
                })
            }
        })
        .collect();
    categories.sort_by(|a, b| b.count.cmp(&a.count).then_with(|| a.name.cmp(&b.name)));
    Ok(categories)
}

pub fn parse_browse_page(value: &Value) -> AppResult<NexusBrowsePage> {
    ensure_no_graphql_errors(value)?;
    let mods_value = value
        .pointer("/data/mods")
        .ok_or_else(|| AppError::new("nexus_browse_failed", "无法解析 Nexus 模组列表"))?;
    let total = mods_value
        .get("totalCount")
        .and_then(Value::as_u64)
        .unwrap_or(0) as u32;
    let nodes = mods_value
        .get("nodes")
        .and_then(Value::as_array)
        .ok_or_else(|| AppError::new("nexus_browse_failed", "无法解析 Nexus 模组列表"))?;
    let mods = nodes.iter().filter_map(parse_catalog_mod).collect();
    Ok(NexusBrowsePage {
        page: 1,
        page_size: PAGE_SIZE,
        total,
        mods,
    })
}

fn parse_catalog_mod(value: &Value) -> Option<NexusCatalogMod> {
    let mod_id = value.get("modId")?.as_u64()? as u32;
    Some(NexusCatalogMod {
        mod_id,
        name: value
            .get("name")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string(),
        author: value
            .get("author")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string(),
        summary: value
            .get("summary")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string(),
        version: value
            .get("version")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string(),
        category: value
            .get("category")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string(),
        downloads: value.get("downloads").and_then(Value::as_u64).unwrap_or(0),
        endorsements: value
            .get("endorsements")
            .and_then(Value::as_u64)
            .unwrap_or(0),
        thumbnail_url: value
            .get("thumbnailUrl")
            .and_then(Value::as_str)
            .filter(|url| url.starts_with("https://"))
            .map(str::to_string),
        library_status: "missing".into(),
    })
}

fn ensure_no_graphql_errors(value: &Value) -> AppResult<()> {
    let Some(errors) = value.get("errors").and_then(Value::as_array) else {
        return Ok(());
    };
    if errors.is_empty() {
        return Ok(());
    }
    let message = errors
        .iter()
        .filter_map(|err| err.get("message").and_then(Value::as_str))
        .collect::<Vec<_>>()
        .join("; ");
    Err(
        AppError::new("nexus_browse_failed", "Nexus 模组列表查询失败").with_detail(
            if message.is_empty() {
                "未知错误".to_string()
            } else {
                message
            },
        ),
    )
}

fn post_graphql(body: &Value) -> AppResult<Value> {
    let client = crate::domain::http::blocking_client(Duration::from_secs(30))?;
    let mut request = client
        .post(GRAPHQL_URL)
        .header("Application-Name", APPLICATION_NAME)
        .header("Application-Version", APPLICATION_VERSION)
        .header("Accept", "application/json");
    if let Ok(key) = secure_key::get_nexus_api_key() {
        request = request.header("apikey", key);
    }
    let response = request.json(body).send().map_err(|e| {
        AppError::new("nexus_browse_failed", "无法连接 Nexus").with_detail(e.to_string())
    })?;
    let status = response.status();
    if status.as_u16() == 401 || status.as_u16() == 403 {
        return Err(AppError::new(
            "nexus_unauthorized",
            "Nexus API 密钥无效或已过期",
        ));
    }
    if status.as_u16() == 429 {
        return Err(AppError::new(
            "nexus_rate_limited",
            "Nexus 请求过于频繁，请稍后再试",
        ));
    }
    if !status.is_success() {
        return Err(
            AppError::new("nexus_browse_failed", "无法获取 Nexus 模组列表")
                .with_detail(status.to_string()),
        );
    }
    response.json().map_err(|e| {
        AppError::new("nexus_browse_failed", "无法解析 Nexus 响应").with_detail(e.to_string())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn browse_body_pages_and_filters_category() {
        let body = browse_request_body(3, Some(" Maps "));
        let vars = &body["variables"];
        assert_eq!(vars["count"], 20);
        assert_eq!(vars["offset"], 40);
        assert_eq!(vars["filter"]["categoryName"][0]["value"], "Maps");
        assert_eq!(
            vars["filter"]["gameDomainName"][0]["value"],
            "stardewvalley"
        );
        assert_eq!(vars["filter"]["adultContent"][0]["value"], false);
        assert_eq!(vars["sort"][0]["downloads"]["direction"], "DESC");
    }

    #[test]
    fn keyword_and_mod_id_filters() {
        let body = browse_request_body_query(1, None, Some("tractor"), Some(2400));
        let filter = &body["variables"]["filter"];
        assert_eq!(filter["name"][0]["value"], "tractor");
        assert_eq!(filter["name"][0]["op"], "WILDCARD");
        assert_eq!(filter["modId"][0]["value"], "2400");
        assert_eq!(filter["gameId"][0]["value"], "1303");
        assert_eq!(filter["gameId"][0]["op"], "EQUALS");
    }

    #[test]
    fn empty_category_is_omitted() {
        let body = browse_request_body(0, Some("  "));
        assert!(body["variables"]["filter"].get("categoryName").is_none());
        assert_eq!(body["variables"]["offset"], 0);
    }

    #[test]
    fn parses_categories_by_count() {
        let value = json!({
            "data": {
                "mods": {
                    "facetsData": {
                        "categoryName": { "Maps": 3, "Items": 9 }
                    }
                }
            }
        });
        let categories = parse_categories(&value).unwrap();
        assert_eq!(categories[0].name, "Items");
        assert_eq!(categories[1].name, "Maps");
    }

    #[test]
    fn parses_browse_page_and_rejects_unsafe_thumbnail() {
        let value = json!({
            "data": {
                "mods": {
                    "totalCount": 2,
                    "nodes": [
                        {
                            "modId": 2400,
                            "name": "SMAPI",
                            "author": "Pathoschild",
                            "summary": "API",
                            "version": "4.5.2",
                            "category": "Modding Tools",
                            "downloads": 10,
                            "endorsements": 2,
                            "thumbnailUrl": "https://example.com/a.jpg"
                        },
                        {
                            "modId": 1,
                            "name": "Bad",
                            "thumbnailUrl": "http://example.com/a.jpg"
                        }
                    ]
                }
            }
        });
        let page = parse_browse_page(&value).unwrap();
        assert_eq!(page.total, 2);
        assert_eq!(
            page.mods[0].thumbnail_url.as_deref(),
            Some("https://example.com/a.jpg")
        );
        assert_eq!(page.mods[1].thumbnail_url, None);
    }
}
