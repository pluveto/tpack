use std::{error::Error, fmt, path::PathBuf};

use lexopt::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InspectFormat {
    Tree,
    Json,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InspectSection {
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
    let mut parser = lexopt::Parser::from_env();
    let Some(command) = parser.next()? else {
        usage();
        return Ok(None);
    };
    let command = match command {
        Long("help") | Short('h') => {
            usage();
            return Ok(None);
        }
        Value(command) => command,
        other => return Err(Box::new(other.unexpected())),
    };

    match command.to_string_lossy().as_ref() {
        "decode" => {
            let path = parse_path(&mut parser)?;
            finish(&mut parser)?;
            Ok(Some(Command::Decode { path }))
        }
        "inspect" => parse_inspect(&mut parser),
        "canonicalize" => {
            let path = parse_path(&mut parser)?;
            finish(&mut parser)?;
            Ok(Some(Command::Canonicalize { path }))
        }
        "help" => {
            usage();
            Ok(None)
        }
        other => Err(Box::new(CliError::new(format!("unknown command: {other}")))),
    }
}

fn parse_inspect(parser: &mut lexopt::Parser) -> Result<Option<Command>, Box<dyn Error>> {
    let mut format = InspectFormat::Tree;
    let mut section = InspectSection::All;
    let mut path = None;

    while let Some(arg) = parser.next()? {
        match arg {
            Long("format") => {
                format = match parser.value()?.to_string_lossy().as_ref() {
                    "tree" => InspectFormat::Tree,
                    "json" => InspectFormat::Json,
                    other => {
                        return Err(Box::new(CliError::new(format!(
                            "unknown inspect format: {other}"
                        ))));
                    }
                };
            }
            Long("section") => {
                section = match parser.value()?.to_string_lossy().as_ref() {
                    "all" => InspectSection::All,
                    "envelope" => InspectSection::Envelope,
                    "schema" => InspectSection::Schema,
                    "value" => InspectSection::Value,
                    other => {
                        return Err(Box::new(CliError::new(format!(
                            "unknown inspect section: {other}"
                        ))));
                    }
                };
            }
            Long("help") | Short('h') => {
                inspect_usage();
                return Ok(None);
            }
            Value(value) if path.is_none() => path = Some(value.into()),
            Value(value) => {
                return Err(Box::new(CliError::new(format!(
                    "unexpected argument: {}",
                    value.to_string_lossy()
                ))));
            }
            other => return Err(Box::new(other.unexpected())),
        }
    }

    let path = path.ok_or_else(|| CliError::new("missing input file"))?;
    Ok(Some(Command::Inspect {
        path,
        format,
        section,
    }))
}

fn parse_path(parser: &mut lexopt::Parser) -> Result<PathBuf, Box<dyn Error>> {
    match parser.next()? {
        Some(Value(path)) => Ok(path.into()),
        Some(arg) => Err(Box::new(arg.unexpected())),
        None => Err(Box::new(CliError::new("missing input file"))),
    }
}

fn finish(parser: &mut lexopt::Parser) -> Result<(), Box<dyn Error>> {
    if let Some(arg) = parser.next()? {
        return Err(Box::new(arg.unexpected()));
    }
    Ok(())
}

fn usage() {
    eprintln!("usage: tpack <decode|inspect|canonicalize> [options] <file>");
    eprintln!(
        "       tpack inspect [--format tree|json] [--section all|envelope|schema|value] <file>"
    );
}

fn inspect_usage() {
    eprintln!(
        "usage: tpack inspect [--format tree|json] [--section all|envelope|schema|value] <file>"
    );
}
