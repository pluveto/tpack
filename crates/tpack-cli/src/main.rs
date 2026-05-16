use std::{env, fs, process};

use tpack::{CanonicalMode, DecodeOptions, Decoder, EncodeOptions, Encoder, EnvelopeMode};

fn main() {
    if let Err(err) = run() {
        eprintln!("tpack-cli: {err}");
        process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = env::args().skip(1);
    let Some(command) = args.next() else {
        usage();
        return Ok(());
    };
    let Some(path) = args.next() else {
        usage();
        return Ok(());
    };
    let bytes = fs::read(&path)?;
    match command.as_str() {
        "decode" | "inspect" => {
            let mut decoder = Decoder::new(&bytes);
            let message = decoder.decode_message()?;
            println!("{message:#?}");
        }
        "canonicalize" => {
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
        _ => usage(),
    }
    Ok(())
}

fn usage() {
    eprintln!("usage: tpack-cli <decode|inspect|canonicalize> <file>");
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
