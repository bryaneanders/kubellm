// allow
use anyhow::Result;
// import modules to parse cli arguments and subcommands
use clap::{Parser, Subcommand};
// import necessary modules from the core library
use core::{Config, create_database_pool, init_database, get_all_prompts, create_prompt_record};

// generates code to parse command line arguments
#[derive(Parser)]
// name of the program
#[command(name = "prompt-cli")]
// description of the program
#[command(about = "A CLI for managing prompts")]
struct Cli {
    // this field will hold the subcommands
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize the database
    InitDb,
    /// List all prompts
    List,
    /// Create a new prompt
    Create {
        /// The prompt content
        #[arg(short, long)]
        prompt: String,
    },

    /// Interact with Anthropic's Claude models
    Claude {
        #[command(subcommand)]
        action: ClaudeCommands,
    },
    /// Show database connection status
    Status,
}

#[derive(Subcommand)]
enum ClaudeCommands {
    /// Send a prompt to Claude
    Prompt {
        /// The prompt content
        #[arg(short, long)]
        prompt: String,
        /// Model to use
        #[arg(short, long)]
        model: Option<String>,
    },
    /// Get available models
    GetModels,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let config = Config::get();

    match cli.command {
        Commands::InitDb => {
            println!("Initializing database...");
            let pool = create_database_pool(&config).await?;
            init_database(&pool).await?;
            println!("✅ Database initialized successfully");
        }
        Commands::List => {
            let pool = create_database_pool(&config).await?;
            let prompts = get_all_prompts(&pool).await?;
            
            if prompts.is_empty() {
                println!("No prompts found");
            } else {
                println!("Found {} prompts:", prompts.len());
                for prompt in prompts {
                    println!("  [{}] {}: {}", 
                        prompt.id, 
                        prompt.created_at.format("%Y-%m-%d %H:%M:%S"), 
                        prompt.prompt
                    );
                }
            }
        }
        Commands::Create { prompt } => {
            let pool = create_database_pool(&config).await?;
            let result = create_prompt_record(&pool, prompt, None, None).await?;
            println!("✅ Created prompt with ID: {}", result.id);
        }
        Commands::Claude { action } => {
            match action {
                ClaudeCommands::Prompt { prompt, model } => {
                    let pool = create_database_pool(&config).await?;
                    match core::call_claude(&prompt, model.as_deref(), &pool).await {
                        Ok(response) => {
                            println!("✅ Claude response:");
                            if let Some(ref resp) = response.response {
                                println!("{}", resp);
                            } else {
                                println!("(No response received)");
                            }
                            println!("Prompt ID: {}", response.id);
                        }
                        Err(e) => {
                            eprintln!("❌ Error calling Claude: {}", e);
                        }
                    }
                }
                ClaudeCommands::GetModels => {
                    match core::get_claude_models().await {
                        Ok(models) => {
                            println!("Available Claude models:");
                            for model in models {
                                println!(" - {} ({})", model.display_name, model.id);
                            }
                        }
                        Err(e) => {
                            eprintln!("❌ Error fetching models: {}", e);
                        }
                    }
                }
            }
        }
        Commands::Status => {
            println!("Checking database connection...");
            let _pool = create_database_pool(&config).await?;
            println!("✅ Database connection successful");
            println!("Database URL: {}", &config.database_url);
        }
    }

    Ok(())
}