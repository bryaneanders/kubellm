mod config;

// import modules to parse cli arguments and subcommands
use clap::{Parser, Subcommand};
// import necessary modules from the core library
use crate::config::CliConfig;
use core::{
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
    TestInterrupt,
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
    last_time: Option<Instant>,
    showing_message: bool,
    command_in_progress: bool,
    interrupt_command: bool,
}

impl CtrlCState {
    fn new() -> Self {
        Self {
            last_time: None,
            showing_message: false,
            command_in_progress: false,
            interrupt_command: false,
        }
    }
}

#[derive(Debug)]
enum InputEvent {
    Command(String),
    CtrlC,
    Exit,
}

#[tokio::main]
async fn main() {
    println!("Welcome to MyApp Interactive CLI!");
    println!("Type 'help' for available commands or 'exit' to quit.");
    println!("Press Ctrl+C twice quickly to force exit.\n");

    let ctrl_c_state = Arc::new(Mutex::new(CtrlCState::new()));
    let ctrl_c_timeout = Duration::from_secs(2);

    // Channel for communication between rustyline and main async task
    let (input_tx, mut input_rx) = tokio::sync::mpsc::unbounded_channel::<InputEvent>();

    // Spawn rustyline in a blocking thread (always listening)
    let input_tx_clone = input_tx.clone();
    let rusty_ctrl_c_state_clone = ctrl_c_state.clone();

    std::thread::spawn(move || {
        let mut rl = DefaultEditor::new().unwrap();
        load_history(&mut rl);

        loop {
            let prompt : &str;
            let state = rusty_ctrl_c_state_clone.lock().unwrap();

            //println!("Debug: CtrlCState: {:?}", *state);
            // when I ctrl+c it prompts again before the state is set
            if !state.interrupt_command && !state.showing_message {
                print!("\x1b[?25h"); // Show cursor
                io::stdout().flush().unwrap();
                prompt = "\x1b[32mprompt-cli>\x1b[97m ";
            } else {
                print!("\x1b[?25l"); // Hide cursor
                io::stdout().flush().unwrap();
                prompt = "";
            }
            drop(state);

            match rl.readline(prompt) {
                Ok(line) => {
                    let mut state = rusty_ctrl_c_state_clone.lock().unwrap();
                    if state.showing_message {
                        // Clear the message and reset state
                        print!("\r\x1b[2K\x1b[1A\x1b[2K"); // Clear message line and move up
                        io::stdout().flush().unwrap();
                        state.showing_message = false;
                        state.last_time = None;
                    }

                    rl.add_history_entry(line.as_str()).unwrap();
                    if input_tx_clone.send(InputEvent::Command(line)).is_err() {
                        break; // Main task has stopped
                    }
                }
                Err(ReadlineError::Interrupted) => {
                    // Update state immediately in the rustyline thread
                    {
                        let mut state = rusty_ctrl_c_state_clone.lock().unwrap();
                        let now = Instant::now();

                        if !state.command_in_progress {
                            // Handle double Ctrl+C for exit
                            let within_timeout = state.last_time
                                .map(|last| now.duration_since(last) < ctrl_c_timeout)
                                .unwrap_or(false);

                            if within_timeout {
                                std::process::exit(0);
                            } else {
                                // First Ctrl+C - immediately update state to hide prompt
                                state.last_time = Some(now);
                                state.showing_message = true;

                                // Clear the current line and show message
                                print!("\r\x1b[2K\x1b[1A");
                                io::stdout().flush().unwrap();
                                println!("Press Ctrl+C again within 2 seconds to force exit...");
                            }
                        }
                    }

                    if input_tx_clone.send(InputEvent::CtrlC).is_err() {
                        break;
                    }
                }
                Err(ReadlineError::Eof) => {
                    let _ = input_tx_clone.send(InputEvent::Exit);
                    break;
                }
                Err(error) => {
                    eprintln!("Readline error: {}", error);
                    break;
                }
            }
        }

        save_history(&mut rl);
    });

    // Background task to clear Ctrl+C timeout messages
    let ctrl_c_state_clone = ctrl_c_state.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_millis(100));
        loop {
            interval.tick().await;
            let mut state = ctrl_c_state_clone.lock().unwrap();

            if state.showing_message {
                if let Some(last_time) = state.last_time {
                    if Instant::now().duration_since(last_time) >= ctrl_c_timeout {
                        // Clear the message
                        print!("\x1b[1A\x1b[2K\x1b[1B\x1b[2K\x1b[1A\r\x1b[32mprompt-cli>\x1b[97m ");
                        io::stdout().flush().unwrap();
                        state.showing_message = false;
                        state.last_time = None;
                    }
                }
            }
        }
    });

    // Main async loop - handles both commands and input
    loop {
        tokio::select! {
            // Handle input from rustyline
            input_event = input_rx.recv() => {
                match input_event {
                    Some(InputEvent::Command(line)) => {
                        if line.trim().is_empty() {
                            continue;
                        }

                        // Reset Ctrl+C state on new command
                        {
                            let mut state = ctrl_c_state.lock().unwrap();
                            state.last_time = None;
                            state.interrupt_command = false;
                            if state.showing_message {
                                // Clear any existing message
                                print!("\x1b[1A\x1b[2K\x1b[1B\x1b[2K\x1b[2A");
                                io::stdout().flush().unwrap();
                                state.showing_message = false;
                            }
                        }

                        // Parse and execute command
                        let args = parse_quoted_args(&line);
                        if args.is_empty() {
                            continue;
                        }

                        let mut full_args = vec!["prompt-cli"];
                        full_args.extend(args.iter().map(|s| s.as_str()));

                        match Cli::try_parse_from(full_args) {
                            Ok(cli) => {
                                // Mark command as starting
                                {
                                    let mut state = ctrl_c_state.lock().unwrap();
                                    state.command_in_progress = true;
                                    state.interrupt_command = false;
                                }

                                // Spawn command execution in separate task so main loop stays responsive
                                let ctrl_c_state_clone = ctrl_c_state.clone();
                                let mut command_handle = tokio::spawn(async move {
                                    execute_command(cli.command, &ctrl_c_state_clone).await
                                });

                                // Wait for either command completion or keep processing other events
                                let mut command_finished = false;
                                while !command_finished { // todo the problem is probably in here
                                    tokio::select! {
                                        // Command completed
                                        result = &mut command_handle => {
                                            command_finished = true;

                                            // Mark command as finished
                                            {
                                                let mut state = ctrl_c_state.lock().unwrap();
                                                state.command_in_progress = false;
                                                state.interrupt_command = false;
                                                drop(state);
                                            }

                                            match result {
                                                Ok(Ok(should_continue)) => {
                                                    if !should_continue {
                                                        return; // Exit main loop
                                                    }
                                                }
                                                Ok(Err(e)) => {
                                                    if e.to_string().contains("interrupted") {
                                                        print!("\r\x1b[2K\x1b[1A\x1b[2K"); // Move to start and clear line
                                                        io::stdout().flush().unwrap();
                                                        //println!("^C");
                                                        println!("Command was interrupted");
                                                        print!("\r\x1b[1A\x1b[25C");
                                                        io::stdout().flush().unwrap();
                                                    } else {
                                                        eprintln!("❌ Error executing command: {}", e);
                                                    }
                                                }
                                                Err(e) => {
                                                    eprintln!("❌ Command task failed: {}", e);
                                                }
                                            }
                                        }

                                        // Handle more input while command is running
                                        input_event = input_rx.recv() => {
                                            match input_event {
                                                Some(InputEvent::CtrlC) => {
                                                    let now = Instant::now();
                                                    let mut state = ctrl_c_state.lock().unwrap();

                                                    if state.command_in_progress {
                                                        print!("Interrupting command...");
                                                        state.interrupt_command = true;
                                                        // Continue loop to wait for command to actually stop
                                                    } else {
                                                        // Handle double Ctrl+C logic
                                                        let within_timeout = state.last_time
                                                            .map(|last| now.duration_since(last) < ctrl_c_timeout)
                                                            .unwrap_or(false);

                                                        if within_timeout {
                                                            println!("Force exiting...");
                                                            return; // Exit main loop
                                                        }

                                                        println!("Press Ctrl+C again within 2 seconds to force exit...");
                                                        state.last_time = Some(now);
                                                        state.showing_message = true;
                                                        println!("Debug: CtrlCState after Ctrl+C: {:?}", *state);
                                                    }
                                                    drop(state);
                                                }
                                                Some(InputEvent::Command(line)) => {
                                                    // User tried to run another command while one is running
                                                    //println!("⚠️ Command '{}' ignored - another command is still running. Press Ctrl+C to interrupt it.", line.trim());
                                                    continue;
                                                }
                                                Some(InputEvent::Exit) => {
                                                    println!("Goodbye!");
                                                    return; // Exit main loop
                                                }
                                                None => {
                                                    println!("Input channel closed, exiting...");
                                                    return; // Exit main loop
                                                }
                                            }
                                        }
                                    }
                                }
                            }
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

                    Some(InputEvent::CtrlC) => {
                        // this is handled in the readline loop
                        continue;
                    }
                    Some(InputEvent::Exit) => {
                        println!("Goodbye!");
                        break;
                    }

                    None => {
                        println!("Input channel closed, exiting...");
                        break; // Channel closed
                    }
                }
            }
        }
    }
}

async fn execute_command(
    command: Commands,
    ctrl_c_state: &Arc<Mutex<CtrlCState>>
) -> anyhow::Result<bool> {  // Changed from Result<bool> to anyhow::Result<bool>
    let config = CoreConfig::get();

    match command {
        Commands::InitDb => {
            println!("Initializing database...");

            let pool = interruptible!(
                create_database_pool(&config),
                &ctrl_c_state
            )?;

            interruptible!(
                init_database(&pool),
                &ctrl_c_state
            )?;

            println!("✅ Database initialized successfully");
        }
        Commands::List => {
            let pool = interruptible!(
                create_database_pool(&config),
                &ctrl_c_state
            )?;

            let prompts = interruptible!(
                get_all_prompts(&pool),
                &ctrl_c_state
            )?;

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
                    // Quick check for interruption during output
                    {
                        let state = ctrl_c_state.lock().unwrap();
                        if state.interrupt_command {
                            return Err(anyhow::anyhow!("Command interrupted"));
                        }
                        drop(state);
                    }
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
            let _pool = interruptible!(
                create_database_pool(&config),
                &ctrl_c_state
            )?;
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
        Commands::TestInterrupt => {
            println!("Starting 10-second test... (press Ctrl+C to interrupt)");

            let test_result = interruptible!(
                async {
                    for i in 0..10 {
                        println!("Test step {}...", i + 1);
                        tokio::time::sleep(Duration::from_secs(1)).await;
                    }
                    Ok::<(), anyhow::Error>(())
                },
                &ctrl_c_state
            );

            match test_result {
                Ok(_) => println!("✅ Test completed successfully"),
                Err(e) => println!("❌ Test interrupted: {}", e),
            }
        }
    }
    print!("\x1b[32mprompt-cli>\x1b[97m ");
    io::stdout().flush().unwrap();
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
