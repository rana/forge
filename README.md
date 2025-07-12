# Forge: Development Environment Orchestrator

> Keep your tools sharp, and ship what matters

Forge orchestrates your development environment, working with native package managers to install and manage tools intelligently.

## Installation

### Install From GitHub Releases

#### macOS Install 🚀
```zsh
# Install binary
mkdir -p ~/.local/bin
curl -L https://github.com/rana/forge/releases/latest/download/forge-aarch64-apple-darwin.tar.xz | tar xJ
mv forge ~/.local/bin/

# Ensure PATH includes ~/.local/bin
grep -q 'export PATH="$HOME/.local/bin:$PATH"' ~/.zshrc || echo 'export PATH="$HOME/.local/bin:$PATH"' >> ~/.zshrc

# Reload shell configuration
source ~/.zshrc
```

#### Linux Install 🚀
```bash
# Install binary
mkdir -p ~/.local/bin
curl -L https://github.com/rana/forge/releases/latest/download/forge-x86_64-unknown-linux-gnu.tar.xz | tar xJ
mv forge ~/.local/bin/

# Ensure PATH includes ~/.local/bin
grep -q 'export PATH="$HOME/.local/bin:$PATH"' ~/.bashrc || echo 'export PATH="$HOME/.local/bin:$PATH"' >> ~/.bashrc

# Reload shell configuration
source ~/.bashrc
```


### Keep Forge Updated

```bash
forge install forge
```

## What is Forge?

Forge knows how to install your tools—and more importantly, *why* they exist. It works with your existing package managers, not against them.

```bash
# Install a tool using the best available method
forge install ripgrep

# Install from any GitHub repo with releases
forge install uv

# Understand why it exists
forge why ripgrep

# Keep everything current
forge update

# See what you have
forge list
```

## Commands

- `forge install <tool>` - Install a tool using the best available method
- `forge uninstall <tool>` - Remove an installed tool
- `forge update [tool]` - Update installed tools (all or specific)
- `forge list` - Show installed tools
- `forge why <tool>` - Explain why a tool exists
- `forge fmt [file]` - Format TOML files

## Philosophy

**Orchestrate, don't replace.** Forge uses the right tool for the job—cargo for Rust, brew for macOS, apt for Linux, direct downloads from GitHub.

**Knowledge drives everything.** Every tool has a purpose. Forge maintains a knowledge base of what tools do and why they matter.

**GitHub as a universal installer.** Any tool with GitHub releases can be installed by Forge—no custom installer needed.

## How It Works

Forge maintains just two files:
- `forge.toml` - The knowledge base of tools and how to install them
- `~/.forge/facts.toml` - What you've actually installed

No complex state. No version locks. No environments. Just tools and knowledge.

## Unique Features

**Smart GitHub Installer**: Forge can install from any GitHub repository with releases. It automatically:
- Discovers the right asset for your platform
- Extracts archives (tar.gz, tar.xz, zip)
- Handles raw binaries
- Places executables in ~/.local/bin

```toml
# Just specify the tool and repo
[tools.uv]
description = "An extremely fast Python package installer and resolver"
[tools.uv.installers.github]
repo = "astral-sh/uv"
```

## Example

```bash
$ forge install bat
ℹ️ Installing bat...
⚡ Using brew installer
✅ bat v0.24.0 installed successfully!

$ forge install uv
ℹ️ Installing uv...
⚡ Using github installer
🔍 Discovering assets for astral-sh/uv (macos-aarch64)
  Found: uv-aarch64-apple-darwin.tar.gz
✅ uv v0.1.24 installed successfully!

$ forge why bat
bat
A cat clone with syntax highlighting

$ forge list
Installed tools:
  • bat 0.24.0 - A cat clone with syntax highlighting
  • forge 0.1.1 - Development environment orchestrator
  • uv 0.1.24 - An extremely fast Python package installer and resolver
  • ripgrep 14.0.3 - Blazing fast search tool
```

## Status

Forge is young but capable. It orchestrates installations via:
- ✅ GitHub releases (any repo!)
- ✅ Cargo (Rust ecosystem)
- ✅ Homebrew (macOS)
- ✅ APT (Linux)
- ✅ Custom scripts

## Contributing

Found a tool that should be in Forge? Add it to `forge.toml` and submit a PR. Just need the tool name and description—Forge figures out the rest.

## License

MIT
