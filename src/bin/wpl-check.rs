#[path = "wpl-check/app.rs"]
mod app;
#[path = "wpl-check/cli.rs"]
mod cli;
#[path = "wpl-check/model.rs"]
mod model;

use std::process::ExitCode;

fn main() -> ExitCode {
    match app::run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("{message}");
            ExitCode::FAILURE
        }
    }
}
