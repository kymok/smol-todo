use pond_core::TaskStore;
use std::io::{self, Write};
use taskpond_cli::{run, CliError};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let store = TaskStore::open_default();
    let mut stdout = io::stdout();
    match run(&args, &store, &mut stdout) {
        Ok(()) => {
            let _ = stdout.flush();
        }
        Err(CliError::Parse(e)) => {
            // clap prints help/usage itself and chooses the exit code (0 for --help, 2 for misuse).
            e.exit();
        }
        Err(CliError::Store(e)) => {
            eprintln!("{e}");
            std::process::exit(1);
        }
        Err(CliError::Usage(message)) => {
            eprintln!("{message}");
            std::process::exit(1);
        }
    }
}
