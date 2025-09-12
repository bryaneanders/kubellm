use crate::config::CliConfig;
use crate::PromptFormatter;
use clap::{Parser, Subcommand};
use kubellm_core::{
    create_database_pool, get_all_prompts, get_models, init_database, prompt_model, CoreConfig,
    Provider,
};
use rustyline::error::ReadlineError;
use rustyline::DefaultEditor;
use std::fs::File;
use std::io;
use std::io::Write;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use tokio::task::JoinHandle;

#[derive(Debug)]
pub enum InputEvent {
    Command(String),
    CtrlC,
    Exit,
}

#[derive(Debug, Clone)]
pub struct CtrlCState {
    last_time: Option<Instant>,
    showing_message: bool,
    command_in_progress: bool,
    interrupt_command: bool,
}

impl CtrlCState {
    pub fn new() -> Self {
        Self {
            last_time: None,
            showing_message: false,
            command_in_progress: false,
            interrupt_command: false,
        }
    }
}

impl Default for CtrlCState {
    fn default() -> Self {
        Self::new()
    }
}

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
    /// Exit the application
    Exit,
}

/// macro to wrap a future and make it interruptible via Ctrl+C
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

/// Macro to wrap a future and make it interruptible via Ctrl+C but with an OK/Err wrapper
macro_rules! try_interruptible {
    ($future:expr, $ctrl_c_state:expr, $progress_task:expr, $error_msg:expr) => {
        match interruptible!($future, $ctrl_c_state) {
            Ok(result) => result,
            Err(e) => {
                eprintln!("\r\x1b[2K❌ {}: {}", $error_msg, e);
                reset_prompt($progress_task, $ctrl_c_state).await;
                return Ok(false);
            }
        }
    };
}

/// Loads the history file from disk
fn load_history(rl: &mut DefaultEditor) {
    let config = CliConfig::get();
    match config.history_file_path.exists() {
        true => {}
        false => {
            if let Some(parent) = config.history_file_path.parent() {
                if !parent.exists() {
                    if let Err(e) = std::fs::create_dir_all(parent) {
                        eprintln!(
                            "Warning: Could not create history file directory {:?}: {}",
                            parent, e
                        );
                        return;
                    }
                }
            }

            if let Err(e) = File::create(&config.history_file_path) {
                eprintln!(
                    "Warning: Could not create history file {:?}: {}",
                    config.history_file_path, e
                );
                return;
            }
        }
    }

    if let Err(e) = rl.load_history(&config.history_file_path) {
        // Only show error if it's not "file not found"
        if config.history_file_path.exists() {
            eprintln!("Warning: Could not load history: {}", e);
        }
    }
}

/// Saves the history file to disk
fn save_history(rl: &mut DefaultEditor) {
    let config = CliConfig::get();
    if let Err(e) = rl.save_history(&config.history_file_path) {
        eprintln!("Warning: Could not save history: {}", e);
    }
}

/// The main cli parsing loop
pub async fn main_loop(
    ctrl_c_state: Arc<Mutex<CtrlCState>>,
    input_rx: &mut UnboundedReceiver<InputEvent>,
) {
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
                                print!("\x1b[2K\x1b[1A\x1b[2K\r\x1b[32mprompt-cli>\x1b[97m\x1b[?25h ");
                                io::stdout().flush().unwrap();
                                state.showing_message = false;
                                continue;
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

                                // Spawn command execution in separate task so main loop stays responsive
                                let ctrl_c_state_clone = ctrl_c_state.clone();
                                let mut command_handle = tokio::spawn(async move {
                                    execute_command(cli.command, &ctrl_c_state_clone).await
                                });

                                // Wait for either command completion or keep processing other events
                                let mut command_finished = false;
                                while !command_finished {
                                    tokio::select! {
                                        // Command completed
                                        result = &mut command_handle => {
                                            command_finished = true;

                                            match result {
                                                Ok(Ok(should_continue)) => {
                                                    if !should_continue {
                                                        return; // Exit main loop
                                                    }
                                                }
                                                Ok(Err(e)) => {
                                                    if e.to_string().contains("interrupted") {
                                                        print!("\r\x1b[2K\x1b[1A\x1b[2K");
                                                        io::stdout().flush().unwrap();
                                                        println!("\x1b[1ACommand was interrupted");
                                                        print!("\x1b[32mprompt-cli>\x1b[97m\x1b[?25h ");
                                                        io::stdout().flush().unwrap();
                                                    } else {
                                                        eprintln!("\r\x1b[2K❌ Error executing command: {}", e);
                                                    }
                                                }
                                                Err(e) => {
                                                    eprintln!("\r\x1b[2K❌ Command task failed: {}", e);
                                                }
                                            }
                                        }

                                        // Handle more input while command is running
                                        input_event = input_rx.recv() => {

                                            match input_event {
                                                Some(InputEvent::CtrlC) => {

                                                    let mut state = ctrl_c_state.lock().unwrap();
                                                    if state.command_in_progress {
                                                        state.interrupt_command = true;
                                                        // Continue loop to wait for command to actually stop
                                                    }
                                                }
                                                Some(InputEvent::Command(_line)) => {
                                                    // User tried to run another command while one is running
                                                    print!("\r\x1b[2K\x1b[1A");
                                                    io::stdout().flush().unwrap();
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
                                print!("\r\x1b[2K\x1b[?25l");
                                io::stdout().flush().unwrap();
                                if line == "help" {
                                    show_help();
                                } else if line == "exit" || line == "quit" {
                                    println!("Goodbye!");
                                    break;
                                } else {
                                    println!("Error: {}", e);
                                    println!("Type 'help' for available commands.");
                                }
                                print!("\x1b[32mprompt-cli>\x1b[97m\x1b[?25h ");
                                io::stdout().flush().unwrap();
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

/// Rustyline backround loop that handles Ctrl+C and other input events
pub fn crate_rustyline_background_loop(
    ctrl_c_timeout: Duration,
    input_tx_clone: UnboundedSender<InputEvent>,
    rusty_ctrl_c_state_clone: Arc<Mutex<CtrlCState>>,
) {
    std::thread::spawn(move || {
        let mut rl = DefaultEditor::new().unwrap();
        load_history(&mut rl);

        loop {
            let state = rusty_ctrl_c_state_clone.lock().unwrap();
            // when I ctrl+c it prompts again before the state is set
            let prompt: &str =
                if !state.interrupt_command && !state.command_in_progress && !state.showing_message
                {
                    print!("\x1b[?25h"); // Show cursor
                    io::stdout().flush().unwrap();
                    "\x1b[32mprompt-cli>\x1b[97m " // new promp value
                } else {
                    print!("\x1b[?25l"); // Hide cursor
                    io::stdout().flush().unwrap();
                    ""
                };
            drop(state);

            save_history(&mut rl);
            match rl.readline(prompt) {
                Ok(line) => {
                    let mut state = rusty_ctrl_c_state_clone.lock().unwrap();
                    if state.showing_message {
                        // Clear the message and reset state
                        print!("\x1b[2K\x1b[1A\x1b[2K\r\x1b[32mprompt-cli>\x1b[97m\x1b[?25h ");
                        io::stdout().flush().unwrap();
                        state.showing_message = false;
                        state.last_time = None;
                        continue;
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
                            let within_timeout = state
                                .last_time
                                .map(|last| now.duration_since(last) < ctrl_c_timeout)
                                .unwrap_or(false);

                            if within_timeout {
                                std::process::exit(0);
                            } else {
                                // First Ctrl+C - immediately update state to hide prompt
                                state.last_time = Some(now);
                                state.showing_message = true;

                                // Clear the current line and show message
                                println!("\r\x1b[2K\x1b[1APress Ctrl+C again within 2 seconds to force exit...");
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
    });
}

/// Background loop that handles clearing out
pub fn create_ctrlc_background_loop(
    ctrl_c_timeout: Duration,
    ctrl_c_state_clone: Arc<Mutex<CtrlCState>>,
) {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_millis(100));
        loop {
            interval.tick().await;
            let mut state = ctrl_c_state_clone.lock().unwrap();
            if state.showing_message {
                if let Some(last_time) = state.last_time {
                    if Instant::now().duration_since(last_time) >= ctrl_c_timeout {
                        // Clear the message
                        print!("\x1b[2K\x1b[1A\x1b[2K\r\x1b[32mprompt-cli>\x1b[97m\x1b[?25h "); // Show prompt and cursor
                        io::stdout().flush().unwrap();
                        state.showing_message = false;
                        state.last_time = None;
                    }
                }
            }
        }
    });
}

/// Show a spinner when a command is running
async fn command_in_progress_display(ctrl_c_state: Arc<Mutex<CtrlCState>>, message: &str) {
    let spinner_chars = ['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];
    let mut spinner_index = 0;
    let mut interval = tokio::time::interval(Duration::from_millis(70));

    loop {
        interval.tick().await;

        let state = ctrl_c_state.lock().unwrap();
        if state.interrupt_command || !state.command_in_progress {
            break;
        }

        // don't show spinner for short running commands
        print!(
            "\r\x1b[2K{} {}",
            spinner_chars[spinner_index % spinner_chars.len()],
            message
        );
        io::stdout().flush().unwrap();

        spinner_index += 1;
    }
}

/// Handles running a CLI command
async fn execute_command(
    command: Commands,
    ctrl_c_state: &Arc<Mutex<CtrlCState>>,
) -> anyhow::Result<bool> {
    let config = CoreConfig::get();

    {
        let mut state = ctrl_c_state.lock().unwrap();
        state.command_in_progress = true;
        state.interrupt_command = false;
    }

    print!("\x1b[2K\r\x1b[?25l"); // Clear current line and move up
    io::stdout().flush().unwrap();

    // start the spinner
    let progress_task = tokio::spawn(command_in_progress_display(ctrl_c_state.clone(), ""));

    match command {
        Commands::InitDb => {
            println!("\r\x1b[2KInitializing database...");
            let pool = try_interruptible!(
                create_database_pool(config),
                &ctrl_c_state,
                progress_task,
                "Failed to create database pool"
            );

            match interruptible!(init_database(&pool), &ctrl_c_state) {
                Ok(_) => {
                    println!("\r\x1b[2K✅ Database initialized successfully")
                }
                Err(e) => {
                    eprintln!("\r\x1b[2K❌ Error initializing database: {}", e)
                }
            }
        }
        Commands::List => {
            let pool = try_interruptible!(
                create_database_pool(config),
                &ctrl_c_state,
                progress_task,
                "Failed to create database pool"
            );

            match interruptible!(get_all_prompts(&pool), &ctrl_c_state) {
                Ok(prompts) => {
                    if prompts.is_empty() {
                        println!("\r\x1b[2KNo prompts found");
                    } else {
                        let mut prompt_formatter = PromptFormatter::new();
                        println!("\r\x1b[2KFound {} prompts:", prompts.len());
                        for prompt in prompts {
                            println!("  ╭─ [{}] ──────────────────────────────────────────────────────────────────", prompt.id);
                            println!("  │ Prompt:");
                            prompt_formatter
                                .format_prompt(&prompt.prompt, 80)
                                .iter()
                                .for_each(|line| println!("  │     {}", line));
                            println!("  │ Response: ");
                            prompt_formatter
                                .format_prompt(&prompt.response, 80)
                                .iter()
                                .for_each(|line| println!("  │     {}", line));
                            println!("  │ Model: {}", prompt.model);
                            println!("  │ Provider: {}", prompt.provider);
                            println!("  │ Timestamp: {}", prompt.created_at.timestamp());
                            println!("  ╰──────────────────────────────────────────────────────────────────────────");
                            println!();
                        }
                    }
                }
                Err(e) => {
                    eprintln!("\r\x1b[2K❌ Error fetching prompts: {}", e)
                }
            }
        }
        Commands::Prompt {
            prompt,
            model,
            provider,
        } => {
            let pool = try_interruptible!(
                create_database_pool(config),
                &ctrl_c_state,
                progress_task,
                "Failed to create database pool"
            );

            match interruptible!(
                prompt_model(&prompt, &provider, model.as_deref(), &pool),
                ctrl_c_state
            ) {
                Ok(response) => {
                    let mut prompt_formatter = PromptFormatter::new();
                    println!("\r\x1b[2K✅ Response:");
                    prompt_formatter
                        .format_prompt(response.response.as_str(), 80)
                        .iter()
                        .for_each(|line| println!("  │     {}", line));
                    println!("Prompt ID: {}", response.id);
                }
                Err(e) => {
                    eprintln!("\r\x1b[2K❌ Error calling model: {}", e);
                    reset_prompt(progress_task, ctrl_c_state).await;
                    return Ok(true);
                }
            }
        }
        Commands::GetModels { provider } => {
            match interruptible!(get_models(&provider), ctrl_c_state) {
                Ok(models) => {
                    if models.is_empty() {
                        println!("\r\x1b[2KNo models found for provider '{}'", provider);
                    } else {
                        println!("\r\x1b[2KAvailable models for provider '{}':", provider);
                        models
                            .into_iter()
                            .for_each(|model| println!(" - {}", model));
                    }
                }
                Err(e) => {
                    eprintln!("\r\x1b[2K❌ Error fetching models: {}", e);
                }
            }
        }
        Commands::GetProviders => {
            let providers = Provider::all();
            println!("\r\x1b[2KAvailable providers:");
            for provider in providers {
                println!(" - {}", provider);
            }
        }
        Commands::Status => {
            println!("\r\x1b[2KChecking database connection...");
            let _ = try_interruptible!(
                create_database_pool(config),
                &ctrl_c_state,
                progress_task,
                "Failed to create database pool"
            );
            println!("\r\x1b[2K✅ Database connection successful");
            println!("Database URL: {}", config.database_url);
        }
        Commands::Exit => {
            reset_prompt(progress_task, ctrl_c_state).await;
            println!("\r\x1b[2KGoodbye!");
            return Ok(false); // Signal to exit the loop
        }
    }
    reset_prompt(progress_task, ctrl_c_state).await;

    Ok(true) // Continue the loop
}

/// Resets the prompt back to normal after a command has finished or is interrupted
async fn reset_prompt(progress_task: JoinHandle<()>, ctrl_c_state: &Arc<Mutex<CtrlCState>>) {
    progress_task.abort();
    print!("\r\x1b[32mprompt-cli>\x1b[97m\x1b[?25h "); // Show prompt and cursor≥
    io::stdout().flush().unwrap();

    {
        let mut state = ctrl_c_state.lock().unwrap();
        state.command_in_progress = false;
        state.interrupt_command = false;
    }
}

/// Handles commands like `prompt -p "what is 2+2?"`
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

/// Prints help message
fn show_help() {
    println!("  init-db                                         Initialize the database");
    println!("  list                                            List all prompts");
    println!("  get-providers                                   Get available model providers");
    println!(
        "  get-models -r <provider>                        Get available models for a provider"
    );
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ctrl_c_state_new() {
        let state = CtrlCState::new();
        assert_eq!(state.last_time, None);
        assert!(!state.showing_message);
        assert!(!state.command_in_progress);
        assert!(!state.interrupt_command);
    }

    #[test]
    fn test_ctrl_c_state_default() {
        let state = CtrlCState::default();
        assert_eq!(state.last_time, None);
        assert!(!state.showing_message);
        assert!(!state.command_in_progress);
        assert!(!state.interrupt_command);
    }

    #[test]
    fn test_parse_quoted_args_empty() {
        let result = parse_quoted_args("");
        assert!(result.is_empty());
    }

    #[test]
    fn test_parse_quoted_args_simple() {
        let result = parse_quoted_args("hello world");
        assert_eq!(result, vec!["hello", "world"]);
    }

    #[test]
    fn test_parse_quoted_args_quoted() {
        let result = parse_quoted_args("hello \"world with spaces\"");
        assert_eq!(result, vec!["hello", "world with spaces"]);
    }

    #[test]
    fn test_parse_quoted_args_escaped() {
        let result = parse_quoted_args("\"hello\\nworld\"");
        assert_eq!(result, vec!["hello\nworld"]);
    }

    #[test]
    fn test_parse_quoted_args_complex() {
        let result = parse_quoted_args("prompt -p \"What is 2 + 2?\" -r anthropic");
        assert_eq!(
            result,
            vec!["prompt", "-p", "What is 2 + 2?", "-r", "anthropic"]
        );
    }

    #[test]
    fn test_parse_quoted_args_escaped_quote() {
        let result = parse_quoted_args("\"He said \\\"hello\\\" to me\"");
        assert_eq!(result, vec!["He said \"hello\" to me"]);
    }

    #[test]
    fn test_parse_quoted_args_mixed_quotes() {
        let result = parse_quoted_args("test \"quoted string\" normal");
        assert_eq!(result, vec!["test", "quoted string", "normal"]);
    }

    #[test]
    fn test_input_event_debug() {
        let event = InputEvent::Command("test".to_string());
        assert!(format!("{:?}", event).contains("Command"));

        let event = InputEvent::CtrlC;
        assert!(format!("{:?}", event).contains("CtrlC"));

        let event = InputEvent::Exit;
        assert!(format!("{:?}", event).contains("Exit"));
    }
}
