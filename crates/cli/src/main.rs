mod app;
mod ui;
mod events;
mod runtime_events;
mod markdown;
mod models;
mod shortcuts;
mod ollama;
mod themes;

use std::io;
use std::sync::{Arc, RwLock};
use app::App;
use runtime::Runtime;
use providers::openai::OpenAIProvider;
use config::Config;
use models::MODELS;

use crossterm::{
    execute,
    terminal::{
        EnterAlternateScreen,
        LeaveAlternateScreen,
        disable_raw_mode,
        enable_raw_mode,
    },
};

use ratatui::{
    backend::CrosstermBackend,
    Terminal,
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cfg = Config::load().unwrap_or_default();

    // Determine the starting model and its provider info
    let start_model = cfg.model.clone();
    let preset = MODELS.iter().find(|m| m.id == start_model);
    let start_provider = preset.map(|p| p.provider).unwrap_or("OpenAI");
    let start_base_url = preset
        .map(|p| p.base_url)
        .unwrap_or("https://api.openai.com/v1")
        .to_string();

    let start_api_key = cfg.key_for_provider(start_provider).unwrap_or_default();
    let missing_key   = start_api_key.is_empty();

    // Shared live state — provider and app both hold clones of these Arcs
    let current_model:    Arc<RwLock<String>> = Arc::new(RwLock::new(start_model));
    let current_api_key:  Arc<RwLock<String>> = Arc::new(RwLock::new(start_api_key));
    let current_base_url: Arc<RwLock<String>> = Arc::new(RwLock::new(start_base_url));

    let provider = OpenAIProvider::new(
        Arc::clone(&current_api_key),
        Arc::clone(&current_model),
        Arc::clone(&current_base_url),
    );
    let runtime = Arc::new(Runtime::new(Box::new(provider)));

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    let mut app = App::new(
        Arc::clone(&current_model),
        Arc::clone(&current_api_key),
        Arc::clone(&current_base_url),
    );

    if missing_key {
        let env_hint = preset.map(|p| p.env_var).unwrap_or("OPENAI_API_KEY");
        app.messages.push(app::ChatMessage {
            role: app::MessageRole::System,
            content: format!(
                "No API key for {} — type /keys to add one, or set the {} env var",
                start_provider, env_hint
            ),
        });
    }

    // Pre-populate local models before the first draw (Ollama is local — instant or <50ms)
    app.local_models = ollama::discover().await;

    // Restore saved theme
    app.theme_idx = cfg.theme.as_deref().map(themes::by_name).unwrap_or(0);

    loop {
        app.process_runtime_events();
        terminal.draw(|f| {
            ui::render(f, &app);
        })?;

        events::handle_events(&mut app, runtime.clone()).await?;

        if app.should_quit {
            break;
        }
    }

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;

    Ok(())
}
