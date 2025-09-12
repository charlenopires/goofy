# Goofy 🤪

A comprehensive Rust port of [Charmbracelet's Crush](https://github.com/charmbracelet/crush) - a powerful terminal-based AI coding assistant with advanced TUI and multi-provider support.

## ✨ Features

- **🤖 Multi-LLM Support**: Seamless integration with OpenAI (GPT-4/3.5), Anthropic (Claude), Azure OpenAI, and local Ollama models
- **🎨 Advanced TUI**: Beautiful terminal interface with themes, animations, and rich components
- **💾 Session Management**: Persistent conversation history with SQLite backend
- **🛠️ Comprehensive Tools**: File operations, bash execution, code editing, grep search, and more
- **📁 File System Integration**: Smart workspace navigation with permission management
- **⚡ Streaming Responses**: Real-time AI responses with proper error handling
- **🎯 Smart Completions**: Context-aware autocompletion for commands and text
- **🔐 Security First**: Permission system for file and command execution
- **⚙️ Flexible Configuration**: JSON config files, environment variables, and CLI arguments

## Installation

### Prerequisites

- Rust 1.70+ 
- An API key from one of the supported providers:
  - OpenAI (GPT-4, GPT-3.5, etc.)
  - Anthropic (Claude-3 family)
  - Ollama (local models - no API key required)

### Building

```bash
git clone <this-repository>
cd Goofy
cargo build --release
```

### Installing on macOS

After building the project, you can install the executable globally:

```bash
# Build the release binary
cargo build --release

# Copy to a directory in your PATH (choose one option)
# Option 1: Install to /usr/local/bin (requires sudo)
sudo cp target/release/goofy /usr/local/bin/goofy

# Option 2: Install to ~/.local/bin (user directory)
mkdir -p ~/.local/bin
cp target/release/goofy ~/.local/bin/goofy
echo 'export PATH="$HOME/.local/bin:$PATH"' >> ~/.zshrc
source ~/.zshrc

# Option 3: Install using Homebrew (if you have a tap)
# brew install your-tap/goofy

# Verify installation
which goofy
goofy --help
```

### Installing on Linux

After building the project, you can install the executable globally:

```bash
# Build the release binary
cargo build --release

# Copy to a directory in your PATH (choose one option)
# Option 1: Install to /usr/local/bin (requires sudo)
sudo cp target/release/goofy /usr/local/bin/goofy

# Option 2: Install to ~/.local/bin (user directory)
mkdir -p ~/.local/bin
cp target/release/goofy ~/.local/bin/goofy
echo 'export PATH="$HOME/.local/bin:$PATH"' >> ~/.bashrc
source ~/.bashrc

# Option 3: Install using a package manager (if available)
# For Arch Linux (if you create an AUR package)
# yay -S goofy-git

# For Ubuntu/Debian (if you create a .deb package)
# sudo dpkg -i goofy_*.deb

# Verify installation
which goofy
goofy --help
```

### Installing on Windows

After building the project, you can install the executable:

```powershell
# Build the release binary
cargo build --release

# Option 1: Copy to a directory in your PATH
# Create a directory for the executable (if it doesn't exist)
mkdir C:\Users\%USERNAME%\bin

# Copy the executable
copy target\release\goofy.exe C:\Users\%USERNAME%\bin\goofy.exe

# Add to PATH (run as Administrator or add via System Properties)
setx PATH "%PATH%;C:\Users\%USERNAME%\bin"

# Option 2: Install to a system directory (requires Administrator)
copy target\release\goofy.exe C:\Windows\System32\goofy.exe

# Verify installation (restart PowerShell/CMD after adding to PATH)
where goofy
goofy --help
```

Alternatively, using Command Prompt:

```cmd
REM Build the release binary
cargo build --release

REM Copy to user directory
mkdir "%USERPROFILE%\bin"
copy target\release\goofy.exe "%USERPROFILE%\bin\goofy.exe"

REM Add to PATH
setx PATH "%PATH%;%USERPROFILE%\bin"

REM Verify installation
where goofy
goofy --help
```

Once installed on any platform, you can use `goofy` instead of the full path:

```bash
# Interactive mode
goofy

# Non-interactive mode
goofy run "Explain Rust ownership"

# With environment variables
GOOFY_PROVIDER=ollama GOOFY_MODEL=llama3.2 goofy run "test"
```

## Configuration

### Environment Variables

Copy `.env.example` to `.env` and add your API keys:

```bash
cp .env.example .env
# Edit .env with your API keys
```

For Ollama (local models), no API key is required, but you need to:

1. Install Ollama: https://ollama.ai
2. Pull a model: `ollama pull llama3.2`
3. Start Ollama server: `ollama serve` (usually runs on http://localhost:11434)

### Configuration File

Copy `goofy.example.json` to `goofy.json` and customize:

```bash
cp goofy.example.json goofy.json
# Edit goofy.json with your preferences
```

## Usage

### Interactive Mode (TUI)

Start the interactive terminal interface:

```bash
# Start interactive TUI
goofy

# Or with full path if not installed globally
./target/release/goofy
```

**TUI Features:**
- 📝 **Chat Interface**: Interactive conversation with AI assistant
- 🎨 **Themes**: Dark/Light/High-contrast themes available
- ⌨️ **Keyboard Shortcuts**:
  - `Ctrl+C` or `Ctrl+Q`: Quit the application
  - `Ctrl+G`: Show help
  - `Enter`: Send message
  - `↑/↓`: Scroll through message history
  - `Home/End`: Jump to beginning/end of input

### Non-Interactive Mode

Run single prompts from the command line:

```bash
# Basic usage
goofy run "Explain Rust ownership"

# With specific provider and model
GOOFY_PROVIDER=openai GOOFY_MODEL=gpt-4 goofy run "Write a binary search in Rust"

# Using Anthropic Claude
GOOFY_PROVIDER=anthropic GOOFY_MODEL=claude-3-opus-20240229 goofy run "Explain async/await"

# Using Ollama (local models)
GOOFY_PROVIDER=ollama GOOFY_MODEL=llama3.2 goofy run "What is a closure?"

# From stdin
echo "Generate unit tests for this function" | goofy run

# Quiet mode (no spinner or status messages)
goofy run --quiet "Review this code"

# With custom working directory
goofy run --cwd /path/to/project "Analyze the codebase structure"
```

### Command Options

```bash
goofy [OPTIONS] [COMMAND]

Commands:
  run     Run a single prompt non-interactively
  help    Print help information

Options:
  --cwd <PATH>           Set working directory (default: current directory)
  --debug                Enable debug logging (RUST_LOG=debug)
  --yolo                 Auto-accept all permissions (⚠️ dangerous!)
  --quiet, -q            Suppress status messages (non-interactive mode)
  --profile              Enable performance profiling
  -h, --help             Print help
  -V, --version          Print version

Examples:
  # Interactive mode
  goofy
  
  # Run a prompt
  goofy run "Explain this code"
  
  # With environment configuration
  GOOFY_PROVIDER=ollama GOOFY_MODEL=codellama goofy run "Optimize this function"
  
  # Debug mode
  goofy --debug run "Debug this error"
```

### Environment Variables

Configure Goofy behavior with environment variables:

```bash
# Provider selection
export GOOFY_PROVIDER=openai          # Options: openai, anthropic, ollama, azure
export GOOFY_MODEL=gpt-4              # Model name specific to provider

# API Keys
export OPENAI_API_KEY=sk-...          # For OpenAI
export ANTHROPIC_API_KEY=sk-ant-...   # For Anthropic
export AZURE_API_KEY=...              # For Azure OpenAI

# Ollama configuration (no API key needed)
export OLLAMA_HOST=http://localhost:11434  # Ollama server URL

# Advanced settings
export GOOFY_MAX_TOKENS=2000          # Max response tokens
export GOOFY_TEMPERATURE=0.7          # Model temperature (0.0-2.0)
export GOOFY_TOP_P=0.9                # Top-p sampling
export GOOFY_STREAM=true              # Enable streaming responses

# Logging
export RUST_LOG=debug                 # Enable debug logging
export GOOFY_PROFILE=true             # Enable performance profiling
```

### Configuration File

Create a `goofy.json` or `.goofy.json` file for persistent configuration:

```json
{
  "provider": "openai",
  "model": "gpt-4",
  "max_tokens": 2000,
  "temperature": 0.7,
  "top_p": 0.9,
  "stream": true,
  "yolo_mode": false,
  "read_only": false,
  "working_dir": ".",
  "extra_headers": {},
  "extra_body": {}
}
```

Configuration priority (highest to lowest):
1. Command-line arguments
2. Environment variables
3. Local config file (`./.goofy.json` or `./goofy.json`)
4. Global config file (`~/.config/goofy/goofy.json`)
5. Default values

## Architecture

### Core Components

- **CLI**: Command-line interface using `clap`
- **TUI**: Terminal interface using `ratatui` and `crossterm`
- **LLM**: Provider abstraction for AI services
- **Session**: Conversation and history management
- **Config**: Configuration loading and validation
- **Utils**: File system and text processing utilities

### Dependencies

- **ratatui**: Terminal UI framework
- **clap**: Command-line argument parsing
- **tokio**: Async runtime
- **reqwest**: HTTP client for API calls
- **rusqlite**: SQLite database
- **serde**: JSON serialization
- **tracing**: Structured logging

## Development

### Building from Source

```bash
# Clone the repository
git clone https://github.com/yourusername/goofy.git
cd goofy

# Build in debug mode (faster compilation, slower runtime)
cargo build

# Build in release mode (optimized for production)
cargo build --release

# Run directly with cargo
cargo run -- run "Hello, Goofy!"

# Run tests
cargo test

# Run with debug logging
RUST_LOG=debug cargo run -- run "test prompt"

# Format code
cargo fmt

# Run linter
cargo clippy
```

### Debug Logging

Enable detailed logging for troubleshooting:

```bash
# Set log level
RUST_LOG=debug goofy run "test prompt"

# Different log levels
RUST_LOG=error     # Only errors
RUST_LOG=warn      # Warnings and errors
RUST_LOG=info      # Informational messages (default)
RUST_LOG=debug     # Debug information
RUST_LOG=trace     # Very detailed trace logs

# Module-specific logging
RUST_LOG=goofy::llm=debug,goofy::tui=trace goofy
```

### Performance Profiling

```bash
# Enable profiling
GOOFY_PROFILE=true goofy

# Profile server runs on http://localhost:6060
# Use with tools like pprof or flamegraph
```

### Testing Specific Providers

```bash
# Test OpenAI
GOOFY_PROVIDER=openai GOOFY_MODEL=gpt-4 cargo run -- run "Test OpenAI"

# Test Anthropic
GOOFY_PROVIDER=anthropic GOOFY_MODEL=claude-3-opus-20240229 cargo run -- run "Test Claude"

# Test Ollama (local)
GOOFY_PROVIDER=ollama GOOFY_MODEL=llama3.2 cargo run -- run "Test Ollama"

# Test Azure OpenAI
GOOFY_PROVIDER=azure GOOFY_MODEL=gpt-4 cargo run -- run "Test Azure"
```

## Comparison to Original

This Rust port maintains the same functionality as the original Go version while leveraging Rust's:

- **Memory Safety**: No garbage collection, zero-cost abstractions
- **Performance**: Compiled binary with minimal runtime overhead
- **Concurrency**: Async/await with tokio for efficient I/O
- **Type Safety**: Strong typing prevents many runtime errors

### Go → Rust Mapping

| Go Library | Rust Equivalent | Purpose |
|------------|-----------------|---------|
| `cobra` | `clap` | CLI framework |
| `bubbletea` | `ratatui` | Terminal UI |
| `slog` | `tracing` | Structured logging |
| `godotenv` | `dotenvy` | Environment loading |
| `sqlite3` | `rusqlite` | Database |
| `http` | `reqwest` | HTTP client |

## License

MIT License - see original Charmbracelet Crush repository for details.

## Contributing

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Add tests if applicable
5. Run `cargo test` and `cargo fmt`
6. Submit a pull request

## Troubleshooting

### Common Issues and Solutions

#### API Key Issues

Ensure your API keys are properly set:

```bash
# Check environment variables
echo $OPENAI_API_KEY
echo $ANTHROPIC_API_KEY

# Test with debug logging
RUST_LOG=debug goofy run "test"

# Verify API key format
# OpenAI: Should start with "sk-"
# Anthropic: Should start with "sk-ant-"
```

#### Ollama Issues

If using Ollama locally:

```bash
# 1. Check if Ollama is installed
which ollama

# 2. Start Ollama service (if not running)
ollama serve

# 3. Check if Ollama is running
curl http://localhost:11434/api/tags

# 4. List available models
ollama list

# 5. Pull a model if needed
ollama pull llama3.2
ollama pull codellama
ollama pull mistral

# 6. Test Ollama integration
GOOFY_PROVIDER=ollama GOOFY_MODEL=llama3.2 goofy run "Hello"

# 7. If using custom Ollama host
OLLAMA_HOST=http://192.168.1.100:11434 GOOFY_PROVIDER=ollama goofy run "test"
```

#### Build Issues

```bash
# Clean build
cargo clean
cargo build --release

# Update dependencies
cargo update

# Fix dependency conflicts
rm Cargo.lock
cargo build --release

# Check for missing system dependencies (Linux)
# Install build essentials if needed
sudo apt-get install build-essential pkg-config libssl-dev
```

#### Database Issues

```bash
# Reset session database
rm ~/.goofy/sessions.db
goofy run "test"  # Recreates database automatically

# Check database location
ls -la ~/.goofy/

# Backup sessions before reset
cp ~/.goofy/sessions.db ~/.goofy/sessions.db.backup
```

#### TUI Display Issues

```bash
# Check terminal capabilities
echo $TERM

# Try different terminal emulators
# Recommended: iTerm2 (macOS), Alacritty, WezTerm, Windows Terminal

# Force specific terminal type
TERM=xterm-256color goofy

# Disable mouse if causing issues
# (Edit config or use environment variable when available)
```

#### Permission Issues

```bash
# If getting permission denied errors
# Check file permissions
ls -la ~/.goofy/

# Fix permissions
chmod 755 ~/.goofy
chmod 644 ~/.goofy/sessions.db

# For system-wide installation issues
# Use user directory instead
mkdir -p ~/.local/bin
cp target/release/goofy ~/.local/bin/
export PATH="$HOME/.local/bin:$PATH"
```
