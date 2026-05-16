mod app;
mod cli;
mod inspect;

use std::process;

fn main() {
    if let Err(err) = app::run() {
        eprintln!("tpack: {err}");
        process::exit(1);
    }
}
