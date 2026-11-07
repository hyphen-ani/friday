pub struct ModelPreset {
    pub id:       &'static str,
    pub label:    &'static str,
    pub provider: &'static str,
    pub base_url: &'static str,
    pub env_var:  &'static str,
}

pub const MODELS: &[ModelPreset] = &[
    ModelPreset { id: "gpt-4o",                    label: "GPT-4o",          provider: "OpenAI",    base_url: "https://api.openai.com/v1",        env_var: "OPENAI_API_KEY"    },
    ModelPreset { id: "gpt-4o-mini",               label: "GPT-4o mini",     provider: "OpenAI",    base_url: "https://api.openai.com/v1",        env_var: "OPENAI_API_KEY"    },
    ModelPreset { id: "o1-mini",                   label: "o1 mini",         provider: "OpenAI",    base_url: "https://api.openai.com/v1",        env_var: "OPENAI_API_KEY"    },
    ModelPreset { id: "o3-mini",                   label: "o3 mini",         provider: "OpenAI",    base_url: "https://api.openai.com/v1",        env_var: "OPENAI_API_KEY"    },
    ModelPreset { id: "claude-opus-4-8",           label: "Claude Opus 4",   provider: "Anthropic", base_url: "https://api.anthropic.com/v1",     env_var: "ANTHROPIC_API_KEY" },
    ModelPreset { id: "claude-sonnet-4-6",         label: "Claude Sonnet 4", provider: "Anthropic", base_url: "https://api.anthropic.com/v1",     env_var: "ANTHROPIC_API_KEY" },
    ModelPreset { id: "claude-haiku-4-5-20251001", label: "Claude Haiku 4",  provider: "Anthropic", base_url: "https://api.anthropic.com/v1",     env_var: "ANTHROPIC_API_KEY" },
    ModelPreset { id: "llama-3.1-70b-instruct",    label: "Llama 3.1 70B",   provider: "Meta",      base_url: "https://api.together.xyz/v1",      env_var: "META_API_KEY"      },
    ModelPreset { id: "mixtral-8x7b-instruct",     label: "Mixtral 8x7B",    provider: "Mistral",   base_url: "https://api.mistral.ai/v1",        env_var: "MISTRAL_API_KEY"   },
    ModelPreset { id: "deepseek-r1",               label: "DeepSeek R1",     provider: "DeepSeek",  base_url: "https://api.deepseek.com/v1",      env_var: "DEEPSEEK_API_KEY"  },
];

/// Unique providers in stable order (first appearance in MODELS).
pub fn unique_providers() -> Vec<&'static ModelPreset> {
    let mut seen = std::collections::HashSet::new();
    MODELS.iter().filter(|m| seen.insert(m.provider)).collect()
}
