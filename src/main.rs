use clap::{ArgAction, Parser};
use crossterm::event::{self, Event, KeyCode};
use crossterm::terminal;
use dirs::home_dir;
use reqwest::blocking::Client;
use serde::Serialize;
use std::env;
use std::fs;
use std::io::{self, Write};
use std::process::{Command, Stdio};

const GEMINI_API_URL: &str =
    "https://generativelanguage.googleapis.com/v1beta/models/gemini-2.5-flash:generateContent";
const ZAI_API_URL: &str = "https://api.z.ai/api/coding/paas/v4/chat/completions";

#[derive(Parser, Debug)]
#[command(name = "nlsh", about = "Natural language shell", version)]
struct Args {
    #[arg(
        short = 'P',
        long = "set-provider",
        value_parser = ["gemini", "zai"],
        help = "Set default provider (gemini or zai)"
    )]
    set_provider: Option<String>,

    #[arg(short = 'A', long = "set-api-key", help = "Set API key for provider")]
    set_api_key: Option<String>,

    #[arg(action = ArgAction::Append, trailing_var_arg = true)]
    prompt: Vec<String>,
}

#[derive(Clone, Copy, Debug)]
enum Provider {
    Gemini,
    Zai,
}

impl Provider {
    fn from_str(value: &str) -> Option<Self> {
        match value.to_lowercase().as_str() {
            "gemini" | "google" => Some(Self::Gemini),
            "zai" | "z.ai" | "z-ai" => Some(Self::Zai),
            _ => None,
        }
    }

    fn env_key(self) -> &'static str {
        match self {
            Self::Gemini => "GEMINI_API_KEY",
            Self::Zai => "ZAI_API_KEY",
        }
    }

    fn name(self) -> &'static str {
        match self {
            Self::Gemini => "gemini",
            Self::Zai => "zai",
        }
    }
}

#[derive(Serialize)]
struct GeminiRequest {
    contents: Vec<GeminiContent>,
}

#[derive(Serialize)]
struct GeminiContent {
    parts: Vec<GeminiPart>,
}

#[derive(Serialize)]
struct GeminiPart {
    text: String,
}

#[derive(Serialize)]
struct ZaiRequest {
    model: String,
    messages: Vec<ZaiMessage>,
}

#[derive(Serialize)]
struct ZaiMessage {
    role: String,
    content: String,
}

fn env_file_path() -> Option<std::path::PathBuf> {
    let home = home_dir()?;
    Some(home.join(".nlsh").join(".env"))
}

fn load_env_file() -> io::Result<()> {
    let Some(path) = env_file_path() else {
        return Ok(());
    };

    if !path.exists() {
        return Ok(());
    }

    let content = fs::read_to_string(path)?;
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        if let Some((key, value)) = trimmed.split_once('=') {
            env::set_var(key.trim(), value.trim());
        }
    }

    Ok(())
}

fn ensure_env_dir() -> io::Result<()> {
    let Some(path) = env_file_path() else {
        return Ok(());
    };
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    Ok(())
}

fn write_env_var(key: &str, value: &str) -> io::Result<()> {
    ensure_env_dir()?;
    let Some(path) = env_file_path() else {
        return Ok(());
    };

    let mut vars = std::collections::BTreeMap::new();
    if path.exists() {
        let content = fs::read_to_string(&path)?;
        for line in content.lines() {
            if let Some((k, v)) = line.split_once('=') {
                vars.insert(k.trim().to_string(), v.trim().to_string());
            }
        }
    }
    vars.insert(key.to_string(), value.to_string());

    let mut rendered = String::new();
    for (k, v) in vars {
        rendered.push_str(&format!("{}={}\n", k, v));
    }
    fs::write(path, rendered)?;
    Ok(())
}

fn set_shell_env(key: &str, value: &str) -> io::Result<()> {
    let rc_files = [".zshrc", ".zprofile", ".bashrc", ".bash_profile"];
    let Some(home) = home_dir() else {
        return Ok(());
    };

    let export_line = format!("export {}=\"{}\"", key, value);
    for rc in rc_files.iter() {
        let path = home.join(rc);
        let mut content = String::new();
        if path.exists() {
            content = fs::read_to_string(&path)?;
            content = content
                .lines()
                .filter(|line| !line.trim_start().starts_with(&format!("export {}=", key)))
                .map(|line| format!("{}\n", line))
                .collect();
        }
        content.push_str(&format!("{}\n", export_line));
        fs::write(path, content)?;
    }

    Ok(())
}

fn current_provider() -> Provider {
    if let Ok(value) = env::var("NLSH_PROVIDER") {
        if let Some(provider) = Provider::from_str(&value) {
            return provider;
        }
    }
    Provider::Gemini
}

fn ensure_api_key(provider: Provider) -> Result<String, String> {
    let key = provider.env_key();
    match env::var(key) {
        Ok(value) if !value.trim().is_empty() => Ok(value),
        _ => Err(format!(
            "Missing {}. Set one via `nlsh --set-api-key`.",
            key
        )),
    }
}

fn gemini_request(prompt: &str, api_key: &str) -> Result<String, String> {
    let client = Client::new();
    let request = GeminiRequest {
        contents: vec![GeminiContent {
            parts: vec![GeminiPart {
                text: prompt.to_string(),
            }],
        }],
    };

    let response = client
        .post(format!("{}?key={}", GEMINI_API_URL, api_key))
        .json(&request)
        .send()
        .map_err(|err| err.to_string())?;

    let value: serde_json::Value = response.json().map_err(|err| err.to_string())?;
    let text = value
        .get("candidates")
        .and_then(|c| c.get(0))
        .and_then(|c| c.get("content"))
        .and_then(|c| c.get("parts"))
        .and_then(|p| p.get(0))
        .and_then(|p| p.get("text"))
        .and_then(|t| t.as_str())
        .ok_or_else(|| "Gemini response missing content".to_string())?;

    Ok(text.trim().to_string())
}

fn zai_request(prompt: &str, api_key: &str) -> Result<String, String> {
    let client = Client::new();
    let request = ZaiRequest {
        model: "glm-4.5".to_string(),
        messages: vec![ZaiMessage {
            role: "user".to_string(),
            content: prompt.to_string(),
        }],
    };

    let response = client
        .post(ZAI_API_URL)
        .bearer_auth(api_key)
        .json(&request)
        .send()
        .map_err(|err| err.to_string())?;
    let status = response.status();
    let body = response.text().map_err(|err| err.to_string())?;
    let value: serde_json::Value =
        serde_json::from_str(&body).map_err(|err| format!("{}: {}", err, body))?;

    let text = value
        .get("choices")
        .and_then(|c| c.get(0))
        .and_then(|choice| {
            choice
                .get("message")
                .and_then(|m| m.get("content"))
                .and_then(|t| t.as_str())
                .or_else(|| choice.get("text").and_then(|t| t.as_str()))
                .or_else(|| choice.get("content").and_then(|t| t.as_str()))
        })
        .ok_or_else(|| format!("z.ai response missing content (status: {})", status))?;

    Ok(text.trim().to_string())
}

fn build_prompt(user_input: &str, cwd: &str) -> String {
    format!(
        "You are a shell command translator. Convert the user's request into a shell command for Linux/zsh.\n\
Current directory: {cwd}\n\n\
Rules:\n\
- Output ONLY the command, nothing else\n\
- No explanations, no markdown, no backticks\n\
- If unclear, make a reasonable assumption\n\
- Prefer simple, common commands\n\n\
User request: {user_input}",
        cwd = cwd,
        user_input = user_input
    )
}

fn run_command(command: &str) -> io::Result<i32> {
    let mut child = Command::new("sh")
        .arg("-c")
        .arg(command)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()?;
    let status = child.wait()?;
    Ok(status.code().unwrap_or(1))
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    load_env_file().ok();
    let args = Args::parse();

    if let Some(provider) = args.set_provider {
        let provider = Provider::from_str(&provider)
            .ok_or_else(|| "Provider must be gemini or zai".to_string())?;
        write_env_var("NLSH_PROVIDER", provider.name())?;
        set_shell_env("NLSH_PROVIDER", provider.name())?;
        println!("Default provider set to {}", provider.name());
        return Ok(());
    }

    if let Some(api_key) = args.set_api_key {
        let provider = current_provider();
        write_env_var(provider.env_key(), &api_key)?;
        set_shell_env(provider.env_key(), &api_key)?;
        println!("API key saved for {}", provider.name());
        return Ok(());
    }

    if args.prompt.is_empty() {
        eprintln!("Usage: nlsh <prompt>");
        return Ok(());
    }

    let prompt_input = args.prompt.join(" ");
    let cwd = env::current_dir()?.display().to_string();
    let prompt = build_prompt(&prompt_input, &cwd);

    let provider = current_provider();
    let api_key = ensure_api_key(provider).map_err(|err| {
        println!("{}", err);
        err
    })?;

    let command = match provider {
        Provider::Gemini => gemini_request(&prompt, &api_key),
        Provider::Zai => zai_request(&prompt, &api_key),
    }
    .map_err(|err| {
        println!("error: {}", err);
        err
    })?;

    println!("→ {}", command);
    print!("[Enter] to run, [Esc] to cancel: ");
    io::stdout().flush()?;

    terminal::enable_raw_mode()?;
    let decision = loop {
        if let Event::Key(key_event) = event::read()? {
            match key_event.code {
                KeyCode::Enter => break Some(()),
                KeyCode::Esc => break None,
                _ => {}
            }
        }
    };
    terminal::disable_raw_mode()?;
    println!();

    if decision.is_some() {
        let code = run_command(&command)?;
        if code != 0 {
            std::process::exit(code);
        }
    }

    Ok(())
}
