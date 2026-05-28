#[derive(Debug, Clone)]
pub enum RuntimeEvent {
    Token(String),
    Finished,
    Error(String)
}