# nlsh - Natural Language Shell

Talk to your terminal in plain English.

Requirements: macOS or Linux.

## Install

```bash
curl -fsSL https://raw.githubusercontent.com/talhabw/nlsh/main/install.sh | bash
```

This installs the latest nightly build to `~/.local/bin` and ensures that path is on your shell.

Build from source:

```bash
cargo build --release
cp target/release/nlsh ~/.local/bin/
```

## Uninstall

```bash
rm -f ~/.local/bin/nlsh
```

## Usage

First-time setup:

```bash
nlsh --set-provider gemini
nlsh --set-api-key YOUR_GEMINI_KEY
```

For z.ai:

```bash
nlsh --set-provider zai
nlsh --set-api-key YOUR_ZAI_KEY
```

Examples:

```bash
nlsh list all python files
nlsh git commit with message fixed bug
nlsh create a new directory called test
nlsh show last 5 lines of file.txt
```

Providers:

- gemini/google: https://aistudio.google.com/apikey
- z.ai: https://api.z.ai/api/coding/paas/v4

Config is saved to your shell rc file (`~/.zshrc`, `~/.bashrc`, or `~/.bash_profile`) via `NLSH_PROVIDER`, `GEMINI_API_KEY`, and `ZAI_API_KEY`.
