# Goofy

A terminal-based AI coding assistant written in Rust. Complete port of [Charmbracelet's Crush](https://github.com/charmbracelet/crush) with multi-provider LLM support, advanced TUI, and extensible tool integration.

## Features

- **Multi-LLM Support**: OpenAI, Anthropic, Ollama, Azure OpenAI, and Google Gemini
- **Advanced TUI**: Terminal interface with themes, animations, syntax highlighting, and rich components
- **Session Management**: Persistent conversation history with SQLite
- **Tool Framework**: File operations, bash execution, code editing, grep, multi-edit
- **Streaming Responses**: Real-time AI output with proper error handling
- **Smart Completions**: Context-aware autocompletion for commands, files, and code
- **Permission System**: Security controls for file and command execution
- **Flexible Configuration**: JSON config files, environment variables, CLI arguments

## Quick Start

### Prerequisites

- Rust 1.70+
- One of: Ollama (local, no API key), OpenAI API key, Anthropic API key

### Build and Run

```bash
git clone https://github.com/user/goofy.git
cd goofy
cargo build --release

# With Ollama (auto-detected if running)
./target/release/goofy run "Explain Rust ownership"

# With OpenAI
OPENAI_API_KEY=sk-... ./target/release/goofy run "Write a binary search"

# With Anthropic
ANTHROPIC_API_KEY=sk-ant-... ./target/release/goofy run "Explain async/await"
```

### Install

```bash
cargo build --release

# macOS / Linux
sudo cp target/release/goofy /usr/local/bin/goofy
# or
mkdir -p ~/.local/bin && cp target/release/goofy ~/.local/bin/

# Verify
goofy --help
```

## Usage

### Non-Interactive Mode

```bash
# Basic prompt
goofy run "Explain this code"

# Specific provider/model
GOOFY_PROVIDER=ollama GOOFY_MODEL=codellama goofy run "Optimize this function"

# From stdin
echo "Generate unit tests" | goofy run

# Quiet mode (no status messages)
goofy run -q "Review this code"
```

### Interactive Mode (TUI)

```bash
goofy
```

### CLI Options

```
goofy [OPTIONS] [COMMAND]

Commands:
  run     Run a single prompt non-interactively
  help    Print help

Options:
  -c, --cwd <PATH>    Set working directory
  -d, --debug          Enable debug logging
  --yolo               Auto-accept all permissions
  -h, --help           Print help
  -V, --version        Print version
```

## Configuration

### Environment Variables

```bash
# Provider
export GOOFY_PROVIDER=openai          # openai, anthropic, ollama, azure, gemini
export GOOFY_MODEL=gpt-4

# API Keys
export OPENAI_API_KEY=sk-...
export ANTHROPIC_API_KEY=sk-ant-...
export OLLAMA_HOST=http://localhost:11434

# Model Parameters
export GOOFY_MAX_TOKENS=4096
export GOOFY_TEMPERATURE=0.7
export GOOFY_STREAM=true
```

### Config File

Create `.goofy.json` or `goofy.json`:

```json
{
  "provider": "ollama",
  "model": "llama3.2",
  "max_tokens": 4096,
  "temperature": 0.7,
  "stream": true
}
```

Priority order: CLI args > env vars > local config > global config (`~/.config/goofy/goofy.json`) > defaults.

Ollama is auto-detected if running locally.

## Architecture

```
src/
  main.rs              Entry point, panic recovery, logging
  cli/                  CLI parsing (clap)
    root.rs             Root command and global options
    run.rs              Non-interactive run command
  config/               Configuration loading (env + JSON)
  app/                  Application orchestration
    mod.rs              App struct, event loop, non-interactive flow
    agent.rs            LLM agent with tool dispatch
    events.rs           Application event types
  llm/                  LLM provider abstraction
    provider.rs         ProviderFactory, LlmProvider trait
    openai.rs           OpenAI/Azure implementation
    anthropic.rs        Anthropic Claude implementation
    ollama.rs           Ollama local model implementation
    gemini.rs           Google Gemini implementation
    types.rs            ChatRequest, Message, ProviderEvent
    tools/              Tool framework (bash, edit, grep, glob, etc.)
    agent/              Streaming agent with tool loop
  session/              Session management
    service.rs          SessionService with CRUD + pub/sub
    database.rs         Legacy SQLite persistence
    conversation.rs     Conversation state management
    pubsub.rs           Event broker for session events
    db_manager.rs       Database-backed session manager
  db/                   Database layer (canonical)
    connect.rs          Connection + migrations
    models.rs           Session, Message, File models
    queries.rs          Type-safe query builders
    migrations.rs       Schema migrations
  tui/                  Terminal UI (ratatui)
    app.rs              TUI event loop and rendering
    themes/             Theme system with presets
    components/
      chat/             Chat interface (editor, renderer, streaming)
      dialogs/          Modal dialogs (quit, sessions, models, commands)
      completions/      Autocompletion (commands, files, code, history)
      animations/       Animation engine, spinners, transitions
      lists/            Virtual scrolling, filtering, pagination
      files/            File picker, diff viewer, permissions
      highlighting/     Syntax highlighting (syntect)
      markdown/         Markdown renderer
      image/            Terminal image display
  message/              Message management with content parts
  pubsub/               Generic pub/sub event system
  permission/           Permission management
  lsp/                  Language Server Protocol client
  log/                  Structured logging with rotation
  history/              File history tracking
  shell/                Persistent shell execution
```

### LLM Provider Trait

```rust
pub trait LlmProvider: Send + Sync {
    async fn chat_completion(&self, request: ChatRequest) -> LlmResult<ProviderResponse>;
    async fn chat_completion_stream(&self, request: ChatRequest)
        -> LlmResult<Pin<Box<dyn Stream<Item = LlmResult<ProviderEvent>> + Send>>>;
    fn name(&self) -> &str;
    fn model(&self) -> &str;
    fn validate_config(&self) -> LlmResult<()>;
}
```

### Adding a New Provider

1. Create `src/llm/my_provider.rs` implementing `LlmProvider`
2. Add to `ProviderFactory::create_provider()` in `provider.rs`
3. Add to `available_providers()` list

### Technology Stack

| Component | Crate | Purpose |
|-----------|-------|---------|
| CLI | `clap` | Command-line parsing |
| TUI | `ratatui` + `crossterm` | Terminal interface |
| Async | `tokio` | Async runtime |
| HTTP | `reqwest` | API calls |
| Database | `rusqlite` | SQLite persistence |
| Serialization | `serde` + `serde_json` | JSON handling |
| Logging | `tracing` | Structured logging |
| Syntax | `syntect` | Code highlighting |
| Markdown | `pulldown-cmark` | Markdown rendering |
| Images | `image` | Terminal image display |

## Development

```bash
cargo build              # Dev build
cargo build --release    # Release build
cargo test               # Run all tests (447 tests)
cargo check              # Type check without building
cargo fmt                # Format code
cargo clippy             # Lint

# Debug logging
RUST_LOG=debug cargo run -- run "test"
RUST_LOG=goofy::llm=debug cargo run -- run "test"
```

## Project Status

The port from Go (Crush) to Rust (Goofy) is functionally complete:

- **Compilation**: 0 errors (resolved 683)
- **Tests**: 447/447 passing
- **Core Flow**: `goofy run` works end-to-end with all providers
- **Codebase**: ~71K lines of Rust across 187 files

### What Works

- Non-interactive mode with all LLM providers
- Session persistence with SQLite
- Tool framework (bash, file operations, grep, edit)
- Theme system with 6 presets
- Animation engine with 19 easing functions
- Syntax highlighting, markdown rendering
- Permission management

### Remaining Work

- TUI interactive mode (components built, integration pending)
- LSP integration (client implemented, needs wiring)
- MCP support (types defined, client pending)
- Additional providers (Bedrock, Vertex AI)

## Troubleshooting

### Ollama

```bash
# Check if running
curl http://localhost:11434/api/tags

# Pull a model
ollama pull llama3.2

# Test
GOOFY_PROVIDER=ollama GOOFY_MODEL=llama3.2 goofy run "hello"
```

### Database Issues

```bash
# Reset database
rm sessions.db
goofy run "test"  # Recreates automatically
```

### Build Issues

```bash
cargo clean && cargo build --release

# Linux: install system deps
sudo apt-get install build-essential pkg-config libssl-dev
```

## License

MIT License

## Contributing

1. Fork the repository
2. Create a feature branch
3. Run `cargo test` and `cargo fmt`
4. Submit a pull request
