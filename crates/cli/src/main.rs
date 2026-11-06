mod app;
mod ui;
mod events;
mod runtime_events;
mod markdown;
mod models;

use std::io;
use std::sync::{Arc, RwLock};
use app::App;
use runtime::Runtime;
use providers::openai::OpenAIProvider;
use config::Config;

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

    let api_key = cfg.resolve_api_key().unwrap_or_else(|e| {
        eprintln!("friday: {}", e);
        std::process::exit(1);
    });

    // Shared model string — app and provider both hold a clone of this Arc
    let current_model: Arc<RwLock<String>> = Arc::new(RwLock::new(cfg.model.clone()));

    let provider = OpenAIProvider::new(api_key, Arc::clone(&current_model));
    let runtime = Arc::new(Runtime::new(Box::new(provider)));

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    let mut app = App::new(Arc::clone(&current_model));

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
