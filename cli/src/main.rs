// allow
use anyhow::Result;
// import modules to parse cli arguments and subcommands
use clap::{Parser, Subcommand};
// import necessary modules from the core library
use core::{Config, create_database_pool, init_database, get_all_prompts, create_prompt_record};
use std::io::{self, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

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
    /// Show usage information
    Usage,
    /// Exit the application
    Exit
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
    /// Usage of Claude commands
    Usage
}

#[tokio::main]
async fn main() {
    println!("Welcome to MyApp Interactive CLI!");
    println!("Type 'help' for available commands or 'exit' to quit.");
    println!("Press Ctrl+C twice quickly to force exit.\n");

    // Set up Ctrl+C handler
    let running = Arc::new(AtomicBool::new(true));
    let ctrl_c_count = Arc::new(AtomicBool::new(false));
    let ctrl_c_ref = ctrl_c_count.clone();

    ctrlc::set_handler(move || {
        if ctrl_c_ref.load(Ordering::SeqCst) {
            println!("\nForce exiting...");
            std::process::exit(0);
        } else {
            println!("\nPress Ctrl+C again within 2 seconds to force exit, or type 'exit' to quit gracefully.");
            ctrl_c_ref.store(true, Ordering::SeqCst);

            // Reset the flag after 2 seconds
            let reset_flag = ctrl_c_ref.clone();
            std::thread::spawn(move || {
                std::thread::sleep(std::time::Duration::from_secs(2));
                reset_flag.store(false, Ordering::SeqCst);
            });
        }
    }).expect("Error setting Ctrl+C handler");

    // Main interactive loop
    while running.load(Ordering::SeqCst) {
        print!("prompt-cli> ");
        io::stdout().flush().unwrap();

        let mut input = String::new();
        match io::stdin().read_line(&mut input) {
            Ok(_) => {
                let input = input.trim();

                if input.is_empty() {
                    continue;
                }

                // Parse the command
                let args = parse_quoted_args(input);
                if args.is_empty() {
                    continue;
                }

                // Prepend the program name for clap parsing
                let mut full_args = vec!["prompt-cli"];
                full_args.extend(args.iter().map(|s| s.as_str()));

                match Cli::try_parse_from(full_args) {
                    Ok(cli) => {
                        match execute_command(cli.command).await {
                            Ok(should_continue) => {
                                if !should_continue {
                                    break;
                                }
                            }
                            Err(e) => {
                                eprintln!("❌ Error executing command: {}", e);
                            }
                        }
                    }
                    Err(e) => {
                        // Handle parsing errors gracefully
                        if input == "help" {
                            show_help();
                        } else if input == "exit" || input == "quit" {
                            println!("Goodbye!");
                            break;
                        } else {
                            println!("Error: {}", e);
                            println!("Type 'help' for available commands.");
                        }
                    }
                }
            }
            Err(error) => {
                println!("Error reading input: {}", error);
                break;
            }
        }
    }
}

async fn execute_command(command: Commands) -> Result<bool, Box<dyn std::error::Error>> {
    let config = Config::get();
    match command {
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
                ClaudeCommands::Usage => {
                    println!("Claude command help:");
                    println!("  prompt -p <prompt> [-m <model>]   Send a prompt to Claude");
                    println!("  get-models                        Get available models");
                    println!("  help                              Show this help message");
                }
            }
        }
        Commands::Status => {
            println!("Checking database connection...");
            let _pool = create_database_pool(&config).await?;
            println!("✅ Database connection successful");
            println!("Database URL: {}", &config.database_url);
        }
        Commands::Usage => {
            show_help();
        }
        Commands::Exit => {
            println!("Goodbye!");
            return Ok(false); // Signal to exit the loop
        }
    }
    Ok(true) // Continue the loop
}

// handle commands like claude prompt -p "what is 2+2?"
fn parse_quoted_args(input: &str) -> Vec<String> {
    let mut args = Vec::new();
    let mut current_arg = String::new();
    let mut in_quotes = false;
    let mut chars = input.chars().peekable();

    while let Some(ch) = chars.next() {
        match ch {
            '"' => { // switch to turn on or off quotes mode
                in_quotes = !in_quotes;
            }
            ' ' if !in_quotes => { // break into a new arg on space if not in quotes
                if !current_arg.is_empty() {
                    args.push(current_arg.clone());
                    current_arg.clear();
                }
            }
            '\\' if in_quotes => { // Handle escaped characters in quotes
                if let Some(next_ch) = chars.next() {
                    match next_ch {
                        'n' => current_arg.push('\n'),
                        't' => current_arg.push('\t'),
                        'r' => current_arg.push('\r'),
                        '\\' => current_arg.push('\\'),
                        '"' => current_arg.push('"'),
                        _ => { // default case, just add the \\
                            current_arg.push('\\');
                            current_arg.push(next_ch);
                        }
                    }
                }
            }
            _ => {
                current_arg.push(ch);
            }
        }
    }

    if !current_arg.is_empty() {
        args.push(current_arg);
    }

    args
}


fn show_help() {
    println!("Available commands:");
    println!("  init-db                          Initialize the database");
    println!("  list                             List all prompts");
    println!("  create -p <prompt>               Create a new prompt");
    println!("  claude prompt -p <prompt>        Send a prompt to Claude");
    println!("  claude prompt -p <prompt> -m <model>  Send a prompt with specific model");
    println!("  claude get-models                Get available models");
    println!("  status                           Show database connection status");
    println!("  help                             Show this help message");
    println!("  exit                             Exit the application");
    println!();
    println!("Examples:");
    println!("  create -p \"My new prompt\"");
    println!("  claude prompt -p \"Hello Claude\"");
    println!("  claude prompt -p \"Hello\" -m \"claude-sonnet-4-20250514\"");
}