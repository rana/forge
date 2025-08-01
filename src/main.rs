use anyhow::Result;
use clap::{Parser, Subcommand};
use forge::forge::Forge;

#[derive(Parser)]
#[command(name = "forge")]
#[command(about = "A knowledge system for developer tools")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Install a tool
    Install {
        /// Name of the tool
        tool: String,

        /// Specific installer to use
        #[arg(long, short = 'i')]
        installer: Option<String>,
    },

    /// Update installed tools
    Update {
        /// Name of specific tool to update (updates all if not specified)
        tool: Option<String>,

        /// Skip updating package managers/installers
        #[arg(long)]
        tools_only: bool,
    },

    /// Uninstall a tool
    Uninstall {
        /// Name of the tool
        tool: String,
    },

    /// Explain why a tool exists
    Why {
        /// Name of the tool
        tool: String,
    },

    /// List installed tools
    List,

    /// Format TOML files
    Fmt {
        /// Path to TOML file (searches for forge.toml if not specified)
        file: Option<String>,

        /// Check if formatting is needed without modifying
        #[arg(long)]
        check: bool,
    },

    /// Share your local knowledge via GitHub Gist
    Share {
        /// Create private gist
        #[arg(long)]
        private: bool,
    },

    /// Load knowledge from a GitHub Gist URL
    Load {
        /// GitHub Gist URL
        url: String,

        /// Replace all local knowledge instead of merging
        #[arg(long)]
        replace: bool,
    },

    /// Sync with your GitHub Gist
    Sync {
        /// Disable sync
        #[arg(long)]
        disable: bool,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let forge = Forge::new().await?;

    match cli.command {
        Commands::Install { tool, installer } => {
            forge.install(&tool, installer.as_deref()).await?;
        }
        Commands::Update { tool, tools_only } => {
            forge.update(tool.as_deref(), tools_only).await?;
        }
        Commands::Uninstall { tool } => {
            forge.uninstall(&tool).await?;
        }
        Commands::Why { tool } => {
            forge.why(&tool)?;
        }
        Commands::List => {
            forge.list().await?;
        }
        Commands::Fmt { file, check } => {
            forge.fmt(file.as_deref(), check).await?;
        }
        Commands::Share { private } => {
            forge.share(private).await?;
        }
        Commands::Load { url, replace } => {
            forge.load(&url, replace).await?;
        }
        Commands::Sync { disable } => {
            forge.sync(disable).await?;
        }
    }

    Ok(())
}
