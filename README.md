# erpnext-cli

A CLI tool for interacting with [ERPNext](https://erpnext.com) via its REST API —
designed and optimized for **agentic automation** (AI agents, CI/CD pipelines, shell scripts).
Every operation is non-interactive, fully configurable via environment variables and flags,
with machine-parseable JSON output when stdout is not a TTY.

## Features

- **Authentication** — login with API key/secret or username/password, manage sessions
- **Doctype CRUD** — list, get, create, update, and delete documents
- **Method calls** — invoke custom server-side methods
- **File upload** — upload files to ERPNext via multipart form
- **Shell completions** — generate completions for bash, zsh, fish
- **Agentic-ready** — machine-parseable output, env-var config, zero interactive prompts, deterministic exit codes

## Agentic Design

| Principle | Implementation |
|-----------|---------------|
| **Zero interactivity** | No prompts — auth via env vars or CLI flags only. `erpnext config set-*` for setup. |
| **Env-var config** | `ERPNEXT_URL`, `ERPNEXT_API_KEY`, `ERPNEXT_API_SECRET` — no config file needed at runtime. |
| **Machine output** | JSON by default when stdout is not a TTY. Use `--output json` to force. |
| **Deterministic exit codes** | Nonzero on error, zero on success — script/compose friendly. |
| **Single static binary** | No runtime deps. Cross-compile for Linux with `make build-linux`. |

## Installation

### From source

```bash
git clone <repo-url>
cd erpnext-cli
cargo build --release
```

### Linux static binary

```bash
make build-linux
# Binary at: target/x86_64-unknown-linux-musl/release/erpnext
```

## Usage

```bash
# Authenticate
erpnext auth login --url https://your-instance.erpnext.com --api-key <key> --api-secret <secret>

# Check auth status
erpnext auth status

# List documents
erpnext doctype list "Sales Order" --filters '[["status","=","Draft"]]'

# Get a document
erpnext doctype get "Sales Order" SAL-ORD-2024-00001

# Call a method
erpnext method call erpnext.hooks.run_custom_method --params '{"key":"value"}'

# Upload a file
erpnext upload --file report.pdf --doctype "File" --field file_url
```

## Configuration

Settings are stored in the platform-specific config directory:
- **Linux/macOS**: `~/.config/erpnext/config.toml`
- **Windows**: `%APPDATA%\erpnext\config.toml`

Environment variables override config file values:
- `ERPNEXT_URL` — instance URL
- `ERPNEXT_API_KEY` — API key
- `ERPNEXT_API_SECRET` — API secret

## Development

```bash
# Check
make check

# Lint
make clippy

# Format
make fmt

# Test
make test

# Cross-compile for Linux
make build-linux
```

## License

MIT
