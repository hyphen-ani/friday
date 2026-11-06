use anyhow::Result;
use providers::types::ToolDef;
use serde_json::{json, Value};

pub fn definitions() -> Vec<ToolDef> {
    vec![
        ToolDef {
            name: "read_file",
            description: "Read the full contents of a file at the given path.",
            parameters: json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "Relative or absolute path to the file" }
                },
                "required": ["path"]
            }),
        },
        ToolDef {
            name: "list_dir",
            description: "List files and directories inside a directory.",
            parameters: json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "Directory path to list (defaults to current dir if empty)" }
                },
                "required": ["path"]
            }),
        },
        ToolDef {
            name: "run_shell",
            description: "Run a shell command and return its stdout. Use for grep, cargo, git, etc.",
            parameters: json!({
                "type": "object",
                "properties": {
                    "command": { "type": "string", "description": "The shell command to execute" }
                },
                "required": ["command"]
            }),
        },
    ]
}

pub async fn execute(name: &str, args_json: &str) -> Result<String> {
    let args: Value = serde_json::from_str(args_json)
        .unwrap_or(Value::Object(Default::default()));

    match name {
        "read_file" => {
            let path = args["path"].as_str().unwrap_or("");
            read_file(path).await
        }
        "list_dir" => {
            let path = args["path"].as_str().unwrap_or(".");
            list_dir(path).await
        }
        "run_shell" => {
            let cmd = args["command"].as_str().unwrap_or("");
            run_shell(cmd).await
        }
        _ => anyhow::bail!("unknown tool: {}", name),
    }
}

async fn read_file(path: &str) -> Result<String> {
    let content = tokio::fs::read_to_string(path).await
        .map_err(|e| anyhow::anyhow!("cannot read {}: {}", path, e))?;

    // Cap at 8 KB to avoid flooding the context window
    const MAX: usize = 8192;
    if content.len() > MAX {
        Ok(format!("{}\n… (truncated, {} bytes total)", &content[..MAX], content.len()))
    } else {
        Ok(content)
    }
}

async fn list_dir(path: &str) -> Result<String> {
    let path = if path.is_empty() { "." } else { path };
    let mut entries = tokio::fs::read_dir(path).await
        .map_err(|e| anyhow::anyhow!("cannot list {}: {}", path, e))?;

    let mut names: Vec<String> = vec![];
    while let Some(entry) = entries.next_entry().await? {
        let name = entry.file_name().to_string_lossy().into_owned();
        let is_dir = entry.file_type().await.map(|t| t.is_dir()).unwrap_or(false);
        names.push(if is_dir { format!("{}/", name) } else { name });
    }
    names.sort();
    Ok(names.join("\n"))
}

async fn run_shell(command: &str) -> Result<String> {
    let output = tokio::process::Command::new("sh")
        .arg("-c")
        .arg(command)
        .output()
        .await
        .map_err(|e| anyhow::anyhow!("failed to run command: {}", e))?;

    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

    let combined = if stderr.is_empty() {
        stdout
    } else if stdout.is_empty() {
        stderr
    } else {
        format!("{}\nstderr:\n{}", stdout, stderr)
    };

    // Cap at 4 KB
    const MAX: usize = 4096;
    if combined.len() > MAX {
        Ok(format!("{}\n… (truncated)", &combined[..MAX]))
    } else {
        Ok(combined)
    }
}
