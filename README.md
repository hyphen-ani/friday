# friday

```
 ✦ Friday  v0.1.0
```

A fast, themeable AI coding assistant that lives entirely in your terminal. No Electron. No browser. No cloud dashboard. Just you, your models, and a TUI built in Rust.

---

## Why Friday Exists

Most AI coding tools are built for the browser first and bolted onto the terminal as an afterthought. They are heavy, opinionated about which model you use, and ultimately owned by someone else's roadmap.

Friday started from a simple frustration: wanting an assistant that felt like a native terminal citizen — something that starts instantly, renders markdown properly, talks to any model you already have access to, and gets out of the way. Something you could actually understand, modify, and extend yourself because you read the source, not the docs.

The name is intentional. A friday is a right hand — present when you need it, invisible when you don't. Not an agent that takes over, not a subscription service, not a productivity suite. A sharp tool that amplifies what you're already doing.

**The philosophy in three sentences:**

> Your terminal is your home. Your models should be your choice. Your tooling should be yours to own.

Friday is what you get when you apply those constraints seriously.

---

## What It Is Right Now

Friday is a TUI chat interface that streams responses from any OpenAI-compatible API — which covers OpenAI, Anthropic (via their compatibility endpoint), DeepSeek, Mistral, local Ollama models, and any other API that speaks the same protocol.

It is early. It works. It is being built in public, in the open, one honest feature at a time.

### What works today

- **Streaming responses** — tokens appear as they arrive, with a live cursor
- **Markdown rendering** — headers, bold/italic, code blocks with syntax highlighting, tables, inline LaTeX
- **Tool use** — the model can read files, list directories, and run shell commands as part of a response
- **Model switching** — switch between cloud models and local Ollama models without restarting; Friday discovers your Ollama models automatically
- **API key management** — store keys per provider inside Friday; prompted inline when you switch to a provider you haven't configured yet
- **8 themes** — live-preview mode: arrow through themes and the entire UI updates in real time before you commit
- **Command palette** — `/` to browse commands with fuzzy autocomplete
- **Persistent config** — model choice, API keys, and theme all saved to `config.toml`

### Commands

| Command | Description |
|---|---|
| `/model` | Switch model — cloud or local Ollama |
| `/keys` | Manage API keys per provider |
| `/theme` | Live-preview and switch themes |
| `/tools` | List tools available to the model |
| `/clear` | Clear conversation history |
| `/help` | Show commands and shortcuts |
| `/exit` | Quit |

### Themes

Each theme name is shown in its own accent color in the picker. Navigating previews the full UI before you confirm.

| Name | Vibe |
|---|---|
| **Ember** | warm amber on deep black *(default)* |
| **Aurora** | glacial teal over obsidian |
| **Crimson** | smoldering red on carbon black |
| **Sakura** | cherry blossom pink on dark plum |
| **Neptune** | deep sea blue on midnight black |
| **Dusk** | twilight violet on deep purple-black |
| **Venom** | neon green on absolute black |
| **Obsidian** | minimal silver on obsidian |

### Supported providers

Friday uses the OpenAI-compatible API format. Set your key with `/keys` or via environment variable.

| Provider | Env var | Base URL |
|---|---|---|
| OpenAI | `OPENAI_API_KEY` | `api.openai.com` |
| Anthropic | `ANTHROPIC_API_KEY` | `api.anthropic.com` |
| DeepSeek | `DEEPSEEK_API_KEY` | `api.deepseek.com` |
| Mistral | `MISTRAL_API_KEY` | `api.mistral.ai` |
| Meta (via Together) | `META_API_KEY` | `api.together.xyz` |
| Ollama (local) | — | `localhost:11434` |

---

## Getting Started

**Prerequisites:** Rust stable (1.75+), `cargo`

```bash
git clone https://github.com/yourname/friday
cd friday
cargo build --release
./target/release/friday
```

On first launch with no config, Friday will start and show an inline prompt to add your API key. Type `/keys` to manage keys for each provider, or set an environment variable before launching:

```bash
OPENAI_API_KEY=sk-... friday
```

**Config file** (`config.toml` in the working directory):

```toml
model = "gpt-4o-mini"
theme = "Ember"

[api_keys]
OpenAI    = "sk-..."
Anthropic = "sk-ant-..."
DeepSeek  = "..."
```

**Local Ollama models** are discovered automatically at startup. Run `ollama serve` and they appear in `/model` under "Local (Ollama)" without any extra configuration.

---

## Architecture

Friday is a Cargo workspace. Each crate has a single responsibility.

```
friday/
├── crates/
│   ├── cli/          # TUI: ratatui + crossterm, all rendering and input
│   │   ├── app.rs    # App state, event processing
│   │   ├── events.rs # Keyboard event handler
│   │   ├── ui.rs     # All render functions
│   │   ├── models.rs # Model presets (id, label, provider, base_url)
│   │   ├── themes.rs # Theme definitions and current() resolver
│   │   ├── markdown.rs # Markdown + LaTeX → ratatui Lines
│   │   └── ollama.rs # Local model discovery
│   │
│   ├── runtime/      # Agentic loop: stream → tool calls → continue
│   ├── providers/    # ModelProvider trait + OpenAIProvider (SSE streaming)
│   ├── tools/        # Tool definitions and dispatch (read_file, list_dir, run_shell)
│   ├── config/       # Config struct, TOML load/save, per-provider key resolution
│   ├── memory/       # Stub — session memory (planned)
│   └── shared/       # Shared types
│
└── web/              # Landing page (static HTML)
```

**Key design decisions:**

- **`ModelProvider` trait** — the runtime never touches HTTP. Swapping providers means implementing one trait.
- **`Arc<RwLock<String>>` for live state** — model, API key, and base URL are shared Arcs between the app and the provider, so switching a model mid-session updates the next API call with no restart.
- **Pre-split rows for scroll** — markdown is split to exact terminal-column-width rows before rendering, giving precise scroll counts without depending on ratatui's word-wrapper estimates.
- **Event channel** — the runtime runs in a tokio task and sends `RuntimeEvent` tokens back through an mpsc channel; the UI drains it on every frame tick.

---

## Where This Is Going

Friday is being built incrementally. The honest roadmap, in rough priority order:

### Near term

- **Session persistence** — save and reload conversation history between launches
- **Write file tool** — the model can propose and write file changes, with a diff preview before applying
- **Web fetch tool** — let the model pull documentation or reference pages during a response
- **Better agent loop visibility** — show tool calls and results more clearly in the conversation view

### Medium term

- **Memory** — persistent notes the model can read and write across sessions; Friday should remember your project, your preferences, what you were working on
- **Multiple conversations** — tab between parallel sessions with different models or contexts
- **Proper Anthropic provider** — native message format with extended thinking support, not just the compat shim
- **Plugin tools** — define custom tools in `config.toml` that map to shell commands, so Friday can run your project's specific scripts

### Longer term

- **Inline diff editing** — propose code changes directly in a file viewer, accept or reject hunks
- **Project context** — automatically include relevant files in context based on what you're asking about
- **MCP support** — connect to Model Context Protocol servers for richer tool ecosystems
- **Collaboration** — share a Friday session with another person, working in the same context

### What Friday will not become

A web app. A cloud service. A product that requires an account. A wrapper around another company's IDE extension.

The terminal is the right level of abstraction for this kind of tool. It composes with everything you already use — git, vim, tmux, your shell. Friday should stay there.

---

## Contributing

Friday is early and rough in places. The codebase is small enough to read in an afternoon. If you find something that annoys you, the fix is probably twenty lines.

The only rule: keep the crates single-responsibility. If you're adding something that spans the whole stack, start with the trait boundary in `providers/` or `tools/` and work outward.

---

## License

MIT
