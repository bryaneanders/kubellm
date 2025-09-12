use prompts_cli::{
    crate_rustyline_background_loop, create_ctrlc_background_loop, main_loop, CtrlCState,
    InputEvent,
};
use std::sync::{Arc, Mutex};
use std::time::Duration;

#[tokio::main]
async fn main() {
    println!("Welcome to MyApp Interactive CLI!");
    println!("Type 'help' for available commands or 'exit' to quit.");
    println!("Press Ctrl+C twice quickly to force exit.\n");

    let ctrl_c_state = Arc::new(Mutex::new(CtrlCState::default()));
    let ctrl_c_timeout = Duration::from_secs(2);

    // Channel for communication between rustyline and main async task
    let (input_tx, mut input_rx) = tokio::sync::mpsc::unbounded_channel::<InputEvent>();

    // Spawn rustyline in a blocking thread (always listening)
    let input_tx_clone = input_tx.clone();
    let rusty_ctrl_c_state_clone = ctrl_c_state.clone();
    crate_rustyline_background_loop(ctrl_c_timeout, input_tx_clone, rusty_ctrl_c_state_clone);

    // Background task to clear Ctrl+C timeout messages
    let ctrl_c_state_clone = ctrl_c_state.clone();
    create_ctrlc_background_loop(ctrl_c_timeout, ctrl_c_state_clone);

    // Main async loop - handles both commands and input
    main_loop(ctrl_c_state, &mut input_rx).await;
}
