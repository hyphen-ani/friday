use serde::Deserialize;

pub const OLLAMA_BASE_URL: &str = "http://localhost:11434/v1";
pub const OLLAMA_TAGS_URL: &str = "http://localhost:11434/api/tags";

#[derive(Deserialize)]
struct TagsResponse {
    models: Vec<OllamaModel>,
}

#[derive(Deserialize)]
struct OllamaModel {
    name: String,
}

/// Query the local Ollama daemon for available models.
/// Returns an empty vec (never errors) if Ollama is not running.
pub async fn discover() -> Vec<String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(1))
        .build()
        .unwrap_or_default();

    let resp = match client.get(OLLAMA_TAGS_URL).send().await {
        Ok(r) => r,
        Err(_) => return vec![],
    };

    match resp.json::<TagsResponse>().await {
        Ok(tags) => tags.models.into_iter().map(|m| m.name).collect(),
        Err(_)   => vec![],
    }
}
