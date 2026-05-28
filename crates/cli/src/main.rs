mod app;
mod ui;
mod events;
mod runtime_events;

use std::io;
use std::sync::Arc;
use app::App;
use runtime::Runtime;
use providers::openai::OpenAIProvider;

use crossterm::{
    execute,
    terminal::{
        EnterAlternateScreen,
        LeaveAlternateScreen,
        disable_raw_mode,
        enable_raw_mode
    },
};

use ratatui::{
    backend::CrosstermBackend,
    Terminal,
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    enable_raw_mode()?;

    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    let mut app = App::new();

    let provider = OpenAIProvider::new(
    std::env::var("OPENAI_API_KEY")?
    );

    let runtime = Arc::new(
        Runtime::new(
            Box::new(provider)
        )
    );

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
    
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen
    )?;

    Ok(())
}