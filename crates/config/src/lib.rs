use std::collections::HashMap;
use std::path::PathBuf;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub api_key: Option<String>,
    pub model: String,
    pub base_url: Option<String>,
    #[serde(default)]
    pub api_keys: HashMap<String, String>,
    pub theme: Option<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            api_key: None,
            model: "gpt-4o-mini".to_string(),
            base_url: None,
            api_keys: HashMap::new(),
            theme: None,
        }
    }
}

impl Config {
    pub fn path() -> Result<PathBuf> {
        Ok(std::env::current_dir()?.join("config.toml"))
    }

    pub fn load() -> Result<Self> {
        let path = Self::path()?;
        if !path.exists() {
            return Ok(Self::default());
        }
        let raw = std::fs::read_to_string(&path)
            .with_context(|| format!("reading {}", path.display()))?;
        toml::from_str(&raw)
            .with_context(|| format!("parsing {}", path.display()))
    }

    pub fn save(&self) -> Result<()> {
        let path = Self::path()?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let raw = toml::to_string_pretty(self)?;
        std::fs::write(&path, raw)?;
        Ok(())
    }

    /// Look up the API key for a provider.
    /// Checks (in order): api_keys map → legacy api_key (OpenAI only) → env var.
    pub fn key_for_provider(&self, provider: &str) -> Option<String> {
        if let Some(k) = self.api_keys.get(provider) {
            if !k.is_empty() {
                return Some(k.clone());
            }
        }
        // Legacy single-key fallback for OpenAI
        if provider == "OpenAI" {
            if let Some(k) = &self.api_key {
                if !k.is_empty() {
                    return Some(k.clone());
                }
            }
            if let Ok(k) = std::env::var("OPENAI_API_KEY") {
                return Some(k);
            }
        }
        // Env var by convention: "OPENAI_API_KEY", "ANTHROPIC_API_KEY", etc.
        let env_var = format!(
            "{}_API_KEY",
            provider.to_uppercase().replace([' ', '-'], "_")
        );
        std::env::var(&env_var).ok().filter(|k| !k.is_empty())
    }

    pub fn set_key_for_provider(&mut self, provider: impl Into<String>, key: impl Into<String>) {
        self.api_keys.insert(provider.into(), key.into());
    }

    /// Legacy resolver kept for compatibility with existing callers.
    pub fn resolve_api_key(&self) -> Result<String> {
        self.key_for_provider("OpenAI").ok_or_else(|| {
            anyhow::anyhow!(
                "no API key found — set OPENAI_API_KEY or add `api_key = \"sk-...\"` to {}",
                Self::path()
                    .map(|p| p.display().to_string())
                    .unwrap_or_else(|_| "config.toml".to_string())
            )
        })
    }
}
