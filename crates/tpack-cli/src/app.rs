mod command;

use std::error::Error;

use crate::cli;

pub fn run() -> Result<(), Box<dyn Error>> {
    let Some(command) = cli::parse()? else {
        return Ok(());
    };

    command::run(command)
}
