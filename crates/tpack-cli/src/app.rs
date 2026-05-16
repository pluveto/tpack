use std::{error::Error, fs};

use tpack::{CanonicalMode, DecodeOptions, Decoder, EncodeOptions, Encoder, EnvelopeMode};

use crate::{
    cli::{self, Command, InspectFormat},
    inspect,
};

pub fn run() -> Result<(), Box<dyn Error>> {
    let Some(command) = cli::parse()? else {
        return Ok(());
    };

    match command {
        Command::Decode { path } => {
            let bytes = fs::read(path)?;
            let mut decoder = Decoder::new(&bytes);
            let message = decoder.decode_message()?;
            println!("{message:#?}");
        }
        Command::Inspect {
            path,
            format,
            section,
        } => {
            let bytes = fs::read(path)?;
            let mut decoder = Decoder::new(&bytes);
            let message = decoder.decode_message()?;
            match format {
                InspectFormat::Tree => inspect::print_tree(&message, section),
                InspectFormat::Json => inspect::print_json(&message, section),
            }
        }
        Command::Canonicalize { path } => {
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
        }
    }

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
