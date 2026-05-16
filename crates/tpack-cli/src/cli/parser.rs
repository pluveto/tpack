use std::{error::Error, ffi::OsString, path::PathBuf};

use lexopt::prelude::*;

use super::{CliError, Command, InspectFormat, InspectSection};

pub(super) struct CommandParser {
    parser: lexopt::Parser,
}

impl CommandParser {
    pub(super) fn from_env() -> Self {
        Self {
            parser: lexopt::Parser::from_env(),
        }
    }

    pub(super) fn parse(mut self) -> Result<Option<Command>, Box<dyn Error>> {
        let Some(command) = self.parse_command_name()? else {
            Self::usage();
            return Ok(None);
        };

        self.parse_named_command(command)
    }

    fn parse_command_name(&mut self) -> Result<Option<OsString>, Box<dyn Error>> {
        match self.parser.next()? {
            Some(Long("help") | Short('h')) => Ok(None),
            Some(Value(command)) => Ok(Some(command)),
            Some(other) => Err(Box::new(other.unexpected())),
            None => Ok(None),
        }
    }

    fn parse_named_command(
        &mut self,
        command: OsString,
    ) -> Result<Option<Command>, Box<dyn Error>> {
        match command.to_string_lossy().as_ref() {
            "decode" => self.parse_single_path_command(|path| Command::Decode { path }),
            "inspect" => self.parse_inspect(),
            "canonicalize" => self.parse_single_path_command(|path| Command::Canonicalize { path }),
            "help" => {
                Self::usage();
                Ok(None)
            }
            other => Err(Box::new(CliError::new(format!("unknown command: {other}")))),
        }
    }

    fn parse_single_path_command<F>(&mut self, build: F) -> Result<Option<Command>, Box<dyn Error>>
    where
        F: FnOnce(PathBuf) -> Command,
    {
        let path = self.parse_path()?;
        self.finish()?;
        Ok(Some(build(path)))
    }

    fn parse_inspect(&mut self) -> Result<Option<Command>, Box<dyn Error>> {
        let mut args = InspectArgs::default();

        while let Some(arg) = self.parser.next()? {
            match arg {
                Long("format") => args.format = InspectArgs::parse_format(self.parser.value()?)?,
                Long("section") => args.section = InspectArgs::parse_section(self.parser.value()?)?,
                Long("help") | Short('h') => {
                    Self::inspect_usage();
                    return Ok(None);
                }
                Value(value) if args.path.is_none() => args.path = Some(value.into()),
                Value(value) => {
                    return Err(Box::new(CliError::new(format!(
                        "unexpected argument: {}",
                        value.to_string_lossy()
                    ))));
                }
                other => return Err(Box::new(other.unexpected())),
            }
        }

        Ok(Some(args.into_command()?))
    }

    fn parse_path(&mut self) -> Result<PathBuf, Box<dyn Error>> {
        match self.parser.next()? {
            Some(Value(path)) => Ok(path.into()),
            Some(arg) => Err(Box::new(arg.unexpected())),
            None => Err(Box::new(CliError::new("missing input file"))),
        }
    }

    fn finish(&mut self) -> Result<(), Box<dyn Error>> {
        if let Some(arg) = self.parser.next()? {
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
}

#[derive(Default)]
struct InspectArgs {
    format: InspectFormat,
    section: InspectSection,
    path: Option<PathBuf>,
}

impl InspectArgs {
    fn into_command(self) -> Result<Command, CliError> {
        let path = self
            .path
            .ok_or_else(|| CliError::new("missing input file"))?;
        Ok(Command::Inspect {
            path,
            format: self.format,
            section: self.section,
        })
    }

    fn parse_format(value: OsString) -> Result<InspectFormat, Box<dyn Error>> {
        match value.to_string_lossy().as_ref() {
            "tree" => Ok(InspectFormat::Tree),
            "json" => Ok(InspectFormat::Json),
            other => Err(Box::new(CliError::new(format!(
                "unknown inspect format: {other}"
            )))),
        }
    }

    fn parse_section(value: OsString) -> Result<InspectSection, Box<dyn Error>> {
        match value.to_string_lossy().as_ref() {
            "all" => Ok(InspectSection::All),
            "envelope" => Ok(InspectSection::Envelope),
            "schema" => Ok(InspectSection::Schema),
            "value" => Ok(InspectSection::Value),
            other => Err(Box::new(CliError::new(format!(
                "unknown inspect section: {other}"
            )))),
        }
    }
}
