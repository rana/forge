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
    
    /// List installed tools
    List,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let forge = Forge::new().await?;
    
    match cli.command {
        Commands::Install { tool, installer } => {
            forge.install(&tool, installer.as_deref()).await?;
        }
        Commands::List => {
            forge.list().await?;
        }
    }
    
    Ok(())
}
