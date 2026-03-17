//! Simple Goofy binary that works

use anyhow::Result;
use clap::Parser;

#[derive(Parser)]
#[command(name = "goofy", version = "0.1.0", about = "Goofy - AI coding assistant")]
struct Args {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(clap::Subcommand)]
enum Command {
    /// Run a single prompt
    Run {
        /// The prompt to run
        prompt: String,
        
        /// Quiet mode
        #[arg(short, long)]
        quiet: bool,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    
    match args.command {
        Some(Command::Run { prompt, quiet }) => {
            if !quiet {
                println!("Processing: {}", prompt);
            }
            
            // For now, just echo back that we received the prompt
            println!("\n=== Goofy Response ===");
            println!("I received your request to: {}", prompt);
            println!("\nNote: The full AI integration is being fixed.");
            println!("This is a temporary minimal version.");
            Ok(())
        }
        None => {
            println!("Goofy Interactive Mode");
            println!("=====================");
            println!("\nInteractive mode is currently being fixed.");
            println!("Please use: goofy run \"your prompt\"");
            Ok(())
        }
    }
}