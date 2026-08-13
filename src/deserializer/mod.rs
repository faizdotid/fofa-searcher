use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct SearchResponse {
    pub error: bool,
    #[serde(default)]
    pub errmsg: String,
    #[serde(default)]
    pub size: u32,
    #[serde(default)]
    pub results: Vec<Vec<String>>,
}
