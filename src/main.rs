use ra_reconcile::cli;
use std::process;

fn main() {
    match cli::run() {
        Ok(code) => process::exit(code),
        Err(error) => {
            eprintln!("error: {error}\nRun with --help for usage");
            process::exit(2)
        }
    }
}
