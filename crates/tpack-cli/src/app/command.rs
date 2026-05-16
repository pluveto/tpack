use std::{error::Error, fs, path::PathBuf};

use tpack::{CanonicalMode, DecodeOptions, Decoder, EncodeOptions, Encoder, EnvelopeMode};

use crate::{
    cli::{Command, InspectFormat, InspectSection},
    inspect,
};

pub(super) fn run(command: Command) -> Result<(), Box<dyn Error>> {
    match command {
        Command::Decode { path } => decode(path),
        Command::Inspect {
            path,
            format,
            section,
        } => inspect_path(path, format, section),
        Command::Canonicalize { path } => canonicalize(path),
    }
}

fn decode(path: PathBuf) -> Result<(), Box<dyn Error>> {
    let bytes = fs::read(path)?;
    let mut decoder = Decoder::new(&bytes);
    let message = decoder.decode_message()?;
    println!("{message:#?}");
    Ok(())
}

fn inspect_path(
    path: PathBuf,
    format: InspectFormat,
    section: InspectSection,
) -> Result<(), Box<dyn Error>> {
    let bytes = fs::read(path)?;
    let mut decoder = Decoder::new(&bytes);
    let message = decoder.decode_message()?;

    match format {
        InspectFormat::Tree => inspect::print_tree(&message, section),
        InspectFormat::Json => inspect::print_json(&message, section),
    }

    Ok(())
}

fn canonicalize(path: PathBuf) -> Result<(), Box<dyn Error>> {
    let bytes = fs::read(path)?;
    let mut decoder = Decoder::with_options(
        &bytes,
        DecodeOptions {
            canonical: CanonicalMode::Strict,
            ..DecodeOptions::default()
        },
    );
    let message = decoder.decode_message()?;
    let mut encoder = Encoder::with_options(EncodeOptions {
        canonical: CanonicalMode::Strict,
        ..EncodeOptions::default()
    });
    let schema_id = message
        .envelope
        .schema_id
        .as_ref()
        .map(|schema_id| schema_id.as_bytes());
    let mode = if message.envelope.mode == EnvelopeMode::SchemaRef {
        EnvelopeMode::FullSchemaWithId
    } else {
        message.envelope.mode
    };

    encoder.encode_message(&message.schema, &message.value, mode, schema_id)?;
    print_hex(&encoder.into_vec());
    Ok(())
}

fn print_hex(bytes: &[u8]) {
    for (index, byte) in bytes.iter().enumerate() {
        if index > 0 {
            print!(" ");
        }
        print!("{byte:02X}");
    }
    println!();
}
