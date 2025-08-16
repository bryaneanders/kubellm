// allow
use anyhow::Result;
// import modules to parse cli arguments and subcommands
use clap::{Parser, Subcommand};
// import necessary modules from the core library
use core::{Config, create_database_pool, init_database, get_all_messages, create_message};

// generates code to parse command line arguments
#[derive(Parser)]
// name of the program
#[command(name = "message-cli")]
// description of the program
#[command(about = "A CLI for managing messages")]
struct Cli {
    // this field will hold the subcommands
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize the database
    InitDb,
    /// List all messages
    List,
    /// Create a new message
    Create {
        /// The message content
        #[arg(short, long)]
        message: String,
    },
    /// Show database connection status
    Status,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let config = Config::from_env()?;

    match cli.command {
        Commands::InitDb => {
            println!("Initializing database...");
            let pool = create_database_pool(&config).await?;
            init_database(&pool).await?;
            println!("✅ Database initialized successfully");
        }
        Commands::List => {
            let pool = create_database_pool(&config).await?;
            let messages = get_all_messages(&pool).await?;
            
            if messages.is_empty() {
                println!("No messages found");
            } else {
                println!("Found {} messages:", messages.len());
                for message in messages {
                    println!("  [{}] {}: {}", 
                        message.id, 
                        message.created_at.format("%Y-%m-%d %H:%M:%S"), 
                        message.message
                    );
                }
            }
        }
        Commands::Create { message } => {
            let pool = create_database_pool(&config).await?;
            let result = create_message(&pool, message).await?;
            println!("✅ Created message with ID: {}", result.id);
        }
        Commands::Status => {
            println!("Checking database connection...");
            let pool = create_database_pool(&config).await?;
            println!("✅ Database connection successful");
            println!("Database URL: {}", config.database_url);
        }
    }

    Ok(())
}