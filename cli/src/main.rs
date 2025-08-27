mod config;

use anyhow::Result;
// import modules to parse cli arguments and subcommands
use clap::{Parser, Subcommand};
// import necessary modules from the core library
use crate::config::CliConfig;
use kubellm_core::{
    create_database_pool, get_all_prompts, init_database,
    get_models, prompt_model, CoreConfig, Provider
};
use rustyline::error::ReadlineError;
use rustyline::DefaultEditor;
use std::io::{self, Write};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

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
    /// Create a new prompt for a provider
    Prompt {
        /// The prompt content
        #[arg(short, long)]
        prompt: String,
        /// The model to use
        #[arg(short, long)]
        model: Option<String>,
        /// The model provider to use
        #[arg(short = 'r', long)]
        provider: String,
    },
    /// Get a provider's list of models
    GetModels {
        /// The model provider to use
        #[arg(short = 'r', long)]
        provider: String,
    },
    /// Get a list of providers
    GetProviders,
    /// Show database connection status
    Status,
    /// Show usage information
    Usage,
    /// Exit the application
    Exit,
}

// macro to wrap a future and make it interruptible via Ctrl+C
macro_rules! interruptible {
    ($future:expr, $ctrl_c_state:expr) => {{
        let future = $future;
        let state = $ctrl_c_state;
        let mut interval = tokio::time::interval(Duration::from_millis(50));

        tokio::select! {
            result = future => {
                result.map_err(|e| anyhow::anyhow!("{}", e))
            }
            _ = async {
                loop {
                    interval.tick().await;
                    let guard = state.lock().unwrap();
                    if guard.interrupt_command {
                        break;
                    }
                }
            } => {
                Err(anyhow::anyhow!("Command interrupted"))
            }
        }
    }};
}

fn load_history(rl: &mut DefaultEditor) {
    let config = CliConfig::get();
    if let Err(e) = rl.load_history(&config.history_file_path) {
        // Only show error if it's not "file not found"
        if config.history_file_path.exists() {
            eprintln!("Warning: Could not load history: {}", e);
        }
    }
}

fn save_history(rl: &mut DefaultEditor) {
    let config = CliConfig::get();
    if let Err(e) = rl.save_history(&config.history_file_path) {
        eprintln!("Warning: Could not save history: {}", e);
    }
}

#[derive(Debug, Clone)]
struct CtrlCState {
    count: u32,
    last_time: Option<Instant>,
    showing_message: bool,
    force_exit: bool,
    ignore_next_input: bool,
    command_in_progress: bool,
    interrupt_command: bool,
}

impl CtrlCState {
    fn new() -> Self {
        Self {
            count: 0,
            last_time: None,
            showing_message: false,
            force_exit: false,
            ignore_next_input: false,
            command_in_progress: false,
            interrupt_command: false,
        }
    }

    fn reset(&mut self) {
        self.count = 0;
        self.last_time = None;
        self.showing_message = false;
        self.force_exit = false;
        self.ignore_next_input = false;
        self.command_in_progress = false;
        self.interrupt_command = false;
    }
}

fn clear_message_if_showing(state: &Arc<Mutex<CtrlCState>>) {
    let mut state_lock = state.lock().unwrap();
    if state_lock.showing_message {
        // Don't clear if the message was just shown (within last 200ms)
        if let Some(last_time) = state_lock.last_time {
            if Instant::now().duration_since(last_time) < Duration::from_millis(200) {
                return; // Don't clear yet, message is still fresh
            }
        }

        print!("\x1b[s\x1b[1A\x1b[2K\x1b[1B\x1b[2K\x1b[1A\r");
        io::stdout().flush().unwrap();
        state_lock.showing_message = false;
        state_lock.ignore_next_input = true; // Set flag to ignore next input
    }
}

#[tokio::main]
async fn main() {
    println!("Welcome to MyApp Interactive CLI!");
    println!("Type 'help' for available commands or 'exit' to quit.");
    println!("Press Ctrl+C twice quickly to force exit.\n");

    let mut rl = DefaultEditor::new().unwrap();

    load_history(&mut rl);

    let mut use_blank_prompt = false;

    let ctrl_c_state = Arc::new(Mutex::new(CtrlCState::new()));
    let ctrl_c_timeout = Duration::from_secs(2);
    let state_clone = ctrl_c_state.clone();
    std::thread::spawn(move || loop {
        std::thread::sleep(Duration::from_millis(100));
        let mut state = state_clone.lock().unwrap();

        if state.showing_message {
            if let Some(last_time) = state.last_time {
                if Instant::now().duration_since(last_time) >= ctrl_c_timeout {
                    print!("\x1b[1A\x1b[2K\x1b[1B\x1b[2K\x1b[2A\x1b[13G");
                    io::stdout().flush().unwrap();
                    state.reset();
                    state.ignore_next_input = true; // Set flag to ignore next input
                }
            }
        }
    });

    // Main interactive loop
    loop {
        clear_message_if_showing(&ctrl_c_state);

        let readline: Result<String, ReadlineError>;
        if use_blank_prompt {
            readline = rl.readline("");
            use_blank_prompt = false;
        } else {
            readline = rl.readline("prompt-cli> ");
        }

        match readline {
            Ok(line) => {
                // Mark command as starting
                {
                    let mut state = ctrl_c_state.lock().unwrap();
                    state.reset();
                    state.command_in_progress = true;
                }

                rl.add_history_entry(line.as_str()).unwrap();

                if line.trim().is_empty() {
                    continue;
                }

                let args = parse_quoted_args(&line);
                if args.is_empty() {
                    continue;
                }

                // Prepend the program name for clap parsing
                let mut full_args = vec!["prompt-cli"];
                full_args.extend(args.iter().map(|s| s.as_str()));

                match Cli::try_parse_from(full_args) {
                    Ok(cli) => match execute_command(cli.command, &ctrl_c_state).await {
                        Ok(should_continue) => {
                            if !should_continue {
                                break;
                            }
                        }
                        Err(e) => {
                            eprintln!("❌ Error executing command: {}", e);
                        }
                    },
                    Err(e) => {
                        if line == "help" {
                            show_help();
                        } else if line == "exit" || line == "quit" {
                            println!("Goodbye!");
                            break;
                        } else {
                            println!("Error: {}", e);
                            println!("Type 'help' for available commands.");
                        }
                    }
                }
            }
            Err(ReadlineError::Interrupted) => {
                let now = Instant::now();
                let mut state = ctrl_c_state.lock().unwrap();

                /*if state.command_in_progress {
                    println!("Interrupting command...");
                    state.interrupt_command = true;
                    drop(state);
                    continue;
                }*/

                if state.command_in_progress {
                    println!("^C");
                    println!("Interrupting command...");
                    state.interrupt_command = true;
                    drop(state);

                    // Stay in a loop until command finishes or we need to force exit
                    loop {
                        tokio::time::sleep(Duration::from_millis(100)).await;
                        let current_state = ctrl_c_state.lock().unwrap();

                        // If command finished, break out and continue main loop
                        if !current_state.command_in_progress {
                            break;
                        }

                        // If user presses Ctrl+C again while waiting, we'll get another Interrupted
                        // This will be handled in the next iteration
                    }
                    continue;
                }

                // Check if this is within the timeout window of the last Ctrl+C
                let within_timeout = state
                    .last_time
                    .map(|last| now.duration_since(last) < ctrl_c_timeout)
                    .unwrap_or(false);

                if within_timeout {
                    println!("Force exiting...");
                    std::process::exit(0);
                }

                // Show the warning message below the current prompt
                println!("Press Ctrl+C again within 2 seconds to force exit...");
                use_blank_prompt = true;
                state.showing_message = true;
                state.last_time = Some(now);
                drop(state);

                // Continue to next iteration which will call readline normally
                continue;
            }
            Err(ReadlineError::Eof) => {
                println!("Goodbye!");
                break;
            }
            Err(error) => {
                println!("Error: {}", error);
                break;
            }
        }
        // Mark command as finished
        {
            let mut state = ctrl_c_state.lock().unwrap();
            state.command_in_progress = false;
            state.interrupt_command = false;
        }
    }

    save_history(&mut rl);
}

async fn execute_command(
    command: Commands,
    ctrl_c_state: &Mutex<CtrlCState>,
) -> Result<bool, Box<dyn std::error::Error>> {
    let config = CoreConfig::get();
    match command {
        Commands::InitDb => {
            println!("Initializing database...");
            let pool = interruptible!(create_database_pool(&config), ctrl_c_state)?;
            interruptible!(init_database(&pool), ctrl_c_state)?;
            println!("✅ Database initialized successfully");
        }
        Commands::List => {
            let pool = interruptible!(create_database_pool(&config), ctrl_c_state)?;
            let prompts = interruptible!(get_all_prompts(&pool), ctrl_c_state)?;

            if prompts.is_empty() {
                println!("No prompts found");
            } else {
                println!("Found {} prompts:", prompts.len());
                for prompt in prompts {
                    println!(
                        "  [{}] {}: {}",
                        prompt.id,
                        prompt.created_at.format("%Y-%m-%d %H:%M:%S"),
                        prompt.prompt
                    );
                }
            }
        }
        Commands::Prompt { prompt, model, provider } => {
            let pool = interruptible!(create_database_pool(&config), ctrl_c_state)?;
            match interruptible!(prompt_model(&prompt, &provider, model.as_deref(), &pool), ctrl_c_state) {
                Ok(response) => {
                    println!("✅ Response:");
                    if let Some(ref resp) = response.response {
                        println!("{}", resp);
                    } else {
                        println!("No response received");
                    }
                    println!("Prompt ID: {}", response.id);
                }
                Err(e) => {
                    eprintln!("❌ Error calling model: {}", e);
                    return Ok(true);
                }
            }
        }
        Commands::GetModels { provider } => {
            match interruptible!(get_models(&provider), ctrl_c_state) {
                Ok(models) => {
                    if models.is_empty() {
                        println!("No models found for provider '{}'", provider);
                    } else {
                        println!("Available models for provider '{}':", provider);
                        for model in models {
                            println!(" - {}", model);
                        }
                    }
                }
                Err(e) => {
                    eprintln!("❌ Error fetching models: {}", e);
                }
            }
        },
        Commands::GetProviders => {
            let providers = Provider::all();
            println!("Available providers:");
            for provider in providers {
                println!("- {}", provider);
            }
        }
        Commands::Status => {
            println!("Checking database connection...");
            let _pool = interruptible!(create_database_pool(&config), ctrl_c_state)?;
            println!("✅ Database connection successful");
            println!("Database URL: {}", &config.database_url);
        }
        Commands::Usage => {
            show_help();
        }
        Commands::Exit => {
            println!("Goodbye!");
            return Ok(false); // Exit the loop
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
            '"' => {
                // switch to turn on or off quotes mode
                in_quotes = !in_quotes;
            }
            ' ' if !in_quotes => {
                // break into a new arg on space if not in quotes
                if !current_arg.is_empty() {
                    args.push(current_arg.clone());
                    current_arg.clear();
                }
            }
            '\\' if in_quotes => {
                // Handle escaped characters in quotes
                if let Some(next_ch) = chars.next() {
                    match next_ch {
                        'n' => current_arg.push('\n'),
                        't' => current_arg.push('\t'),
                        'r' => current_arg.push('\r'),
                        '\\' => current_arg.push('\\'),
                        '"' => current_arg.push('"'),
                        _ => {
                            // default case, just add the \\
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
    println!("  init-db                                         Initialize the database");
    println!("  list                                            List all prompts");
    println!("  get-providers                                   Get available model providers");
    println!("  get-models -r <provider>                        Get available models for a provider");
    println!("  prompt -p <prompt> -r <provider> [-m <model>]   Create a new prompt");
    println!("  status                                          Show database connection status");
    println!("  help                                            Show this help message");
    println!("  exit                                            Exit the application");
    println!();
    println!("Examples:");
    println!("  prompt -p \"What is 2 + 2?\" -r anthropic");
    println!("  prompt -p \"What is 2 + 2?\" -r anthropic -m claude-sonnet-4-20250514");
    println!("  get-models -r anthropic");
}
