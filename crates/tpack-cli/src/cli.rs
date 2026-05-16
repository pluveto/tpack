mod parser;

use std::{error::Error, fmt, path::PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum InspectFormat {
    #[default]
    Tree,
    Json,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum InspectSection {
    #[default]
    All,
    Envelope,
    Schema,
    Value,
}

#[derive(Debug, Clone)]
pub enum Command {
    Decode {
        path: PathBuf,
    },
    Inspect {
        path: PathBuf,
        format: InspectFormat,
        section: InspectSection,
    },
    Canonicalize {
        path: PathBuf,
    },
}

#[derive(Debug)]
pub struct CliError(String);

impl CliError {
    fn new(message: impl Into<String>) -> Self {
        Self(message.into())
    }
}

impl fmt::Display for CliError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl Error for CliError {}

pub fn parse() -> Result<Option<Command>, Box<dyn Error>> {
    parser::CommandParser::from_env().parse()
}
