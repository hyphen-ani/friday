pub struct ModelPreset {
    pub id: &'static str,
    pub label: &'static str,
    pub provider: &'static str,
}

pub const MODELS: &[ModelPreset] = &[
    ModelPreset { id: "gpt-4o",                     label: "GPT-4o",          provider: "OpenAI"    },
    ModelPreset { id: "gpt-4o-mini",                label: "GPT-4o mini",     provider: "OpenAI"    },
    ModelPreset { id: "o1-mini",                    label: "o1 mini",         provider: "OpenAI"    },
    ModelPreset { id: "o3-mini",                    label: "o3 mini",         provider: "OpenAI"    },
    ModelPreset { id: "claude-opus-4-8",            label: "Claude Opus 4",   provider: "Anthropic" },
    ModelPreset { id: "claude-sonnet-4-6",          label: "Claude Sonnet 4", provider: "Anthropic" },
    ModelPreset { id: "claude-haiku-4-5-20251001",  label: "Claude Haiku 4",  provider: "Anthropic" },
    ModelPreset { id: "llama-3.1-70b-instruct",     label: "Llama 3.1 70B",   provider: "Meta"      },
    ModelPreset { id: "mixtral-8x7b-instruct",      label: "Mixtral 8x7B",    provider: "Mistral"   },
    ModelPreset { id: "deepseek-r1",                label: "DeepSeek R1",     provider: "DeepSeek"  },
];
