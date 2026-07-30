//! Encode/decode helpers (cold one-shot and steady-state prepared paths).

use std::sync::Arc;

use tpack::{
    Decoder, Encoder, EnvelopeMode, PreparedSchema, Schema, StdSchemaRegistry, TpackValue,
    encode_message,
};

use crate::format::Format;
use crate::payload::{
    Scenario, SerdePayload, blob_schema, blob_serde, blob_value, list_schema, list_serde,
    list_value, schema_id_bytes, schema_id_for, serde_payload, tpack_schema, tpack_value,
    wide_schema, wide_serde, wide_value,
};

/// Errors from harness encoders (not part of the public TPACK API).
#[derive(Debug)]
pub enum EncodeError {
    Tpack(tpack::Error),
    Json(serde_json::Error),
    Cbor(String),
    MsgPack(rmp_serde::encode::Error),
    MsgPackDecode(rmp_serde::decode::Error),
    Other(String),
}

impl std::fmt::Display for EncodeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EncodeError::Tpack(e) => write!(f, "tpack: {e}"),
            EncodeError::Json(e) => write!(f, "json: {e}"),
            EncodeError::Cbor(e) => write!(f, "cbor: {e}"),
            EncodeError::MsgPack(e) => write!(f, "msgpack: {e}"),
            EncodeError::MsgPackDecode(e) => write!(f, "msgpack decode: {e}"),
            EncodeError::Other(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for EncodeError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            EncodeError::Tpack(e) => Some(e),
            EncodeError::Json(e) => Some(e),
            EncodeError::Cbor(_) | EncodeError::Other(_) => None,
            EncodeError::MsgPack(e) => Some(e),
            EncodeError::MsgPackDecode(e) => Some(e),
        }
    }
}

fn mode_for(format: Format) -> Result<(EnvelopeMode, bool), EncodeError> {
    match format {
        Format::TpackFullSchema => Ok((EnvelopeMode::FullSchema, false)),
        Format::TpackFullSchemaWithId => Ok((EnvelopeMode::FullSchemaWithId, true)),
        Format::TpackSchemaRef => Ok((EnvelopeMode::SchemaRef, true)),
        _ => Err(EncodeError::Other(
            "TPACK mode helper only applies to TPACK formats".into(),
        )),
    }
}

/// Encode a fixed scenario (one-shot). Prefer [`SteadyEncoder`] in hot loops.
pub fn encode(scenario: Scenario, format: Format) -> Result<Vec<u8>, EncodeError> {
    match format {
        Format::TpackFullSchema | Format::TpackFullSchemaWithId | Format::TpackSchemaRef => {
            let mut steady = SteadyEncoder::for_scenario(scenario, format)?;
            steady.encode_once()
        }
        Format::Json => serde_json::to_vec(&serde_payload(scenario)).map_err(EncodeError::Json),
        Format::Cbor => encode_cbor(&serde_payload(scenario)),
        Format::MsgPack => {
            rmp_serde::to_vec_named(&serde_payload(scenario)).map_err(EncodeError::MsgPack)
        }
    }
}

/// Cold FullSchema path that re-encodes the schema every call (baseline footgun).
pub fn encode_tpack_naive(
    schema: &Schema,
    value: &TpackValue<'_>,
    mode: EnvelopeMode,
    schema_id: Option<&[u8]>,
) -> Result<Vec<u8>, EncodeError> {
    encode_message(schema, value, mode, schema_id).map_err(EncodeError::Tpack)
}

fn encode_cbor<T: serde::Serialize>(value: &T) -> Result<Vec<u8>, EncodeError> {
    let mut buf = Vec::new();
    ciborium::into_writer(value, &mut buf).map_err(|e| EncodeError::Cbor(e.to_string()))?;
    Ok(buf)
}

/// Reusable TPACK encoder: prepared schema + cleared output buffer.
pub struct SteadyEncoder {
    prepared: PreparedSchema,
    value: TpackValue<'static>,
    schema_id: [u8; 8],
    encoder: Encoder,
    mode: EnvelopeMode,
    with_id: bool,
}

impl SteadyEncoder {
    pub fn for_scenario(scenario: Scenario, format: Format) -> Result<Self, EncodeError> {
        let (mode, with_id) = mode_for(format)?;
        Self::from_parts(
            tpack_schema(scenario),
            tpack_value(scenario),
            schema_id_bytes(scenario),
            mode,
            with_id,
        )
    }

    pub fn for_blob(blob_len: usize, format: Format) -> Result<Self, EncodeError> {
        let (mode, with_id) = mode_for(format)?;
        let schema = blob_schema();
        let id = schema_id_for(&schema);
        Self::from_parts(schema, blob_value(blob_len), id, mode, with_id)
    }

    pub fn for_wide(field_count: usize, format: Format) -> Result<Self, EncodeError> {
        let (mode, with_id) = mode_for(format)?;
        let schema = wide_schema(field_count);
        let id = schema_id_for(&schema);
        Self::from_parts(schema, wide_value(field_count), id, mode, with_id)
    }

    pub fn for_list(len: usize, format: Format) -> Result<Self, EncodeError> {
        let (mode, with_id) = mode_for(format)?;
        let schema = list_schema(len);
        let id = schema_id_for(&schema);
        Self::from_parts(schema, list_value(len), id, mode, with_id)
    }

    fn from_parts(
        schema: Schema,
        value: TpackValue<'static>,
        schema_id: [u8; 8],
        mode: EnvelopeMode,
        with_id: bool,
    ) -> Result<Self, EncodeError> {
        let prepared = PreparedSchema::prepare_default(schema).map_err(EncodeError::Tpack)?;
        Ok(Self {
            prepared,
            value,
            schema_id,
            encoder: Encoder::new(),
            mode,
            with_id,
        })
    }

    pub fn schema(&self) -> &Schema {
        self.prepared.schema()
    }

    pub fn schema_id(&self) -> &[u8; 8] {
        &self.schema_id
    }

    pub fn prepared(&self) -> &PreparedSchema {
        &self.prepared
    }

    pub fn encode_once(&mut self) -> Result<Vec<u8>, EncodeError> {
        self.encode_in_place()?;
        Ok(self.encoder.take_vec())
    }

    pub fn encode_in_place(&mut self) -> Result<&[u8], EncodeError> {
        self.encoder.clear();
        let id = if self.with_id {
            Some(self.schema_id.as_slice())
        } else {
            None
        };
        self.encoder
            .encode_prepared_message(&self.prepared, &self.value, self.mode, id)
            .map_err(EncodeError::Tpack)?;
        Ok(self.encoder.as_slice())
    }
}

/// Encode scaling payloads for non-TPACK formats.
pub fn encode_blob(format: Format, blob_len: usize) -> Result<Vec<u8>, EncodeError> {
    match format {
        Format::TpackFullSchema | Format::TpackFullSchemaWithId | Format::TpackSchemaRef => {
            SteadyEncoder::for_blob(blob_len, format)?.encode_once()
        }
        Format::Json => serde_json::to_vec(&blob_serde(blob_len)).map_err(EncodeError::Json),
        Format::Cbor => encode_cbor(&blob_serde(blob_len)),
        Format::MsgPack => {
            rmp_serde::to_vec_named(&blob_serde(blob_len)).map_err(EncodeError::MsgPack)
        }
    }
}

pub fn encode_wide(format: Format, field_count: usize) -> Result<Vec<u8>, EncodeError> {
    match format {
        Format::TpackFullSchema | Format::TpackFullSchemaWithId | Format::TpackSchemaRef => {
            SteadyEncoder::for_wide(field_count, format)?.encode_once()
        }
        Format::Json => serde_json::to_vec(&wide_serde(field_count)).map_err(EncodeError::Json),
        Format::Cbor => encode_cbor(&wide_serde(field_count)),
        Format::MsgPack => {
            rmp_serde::to_vec_named(&wide_serde(field_count)).map_err(EncodeError::MsgPack)
        }
    }
}

pub fn encode_list(format: Format, len: usize) -> Result<Vec<u8>, EncodeError> {
    match format {
        Format::TpackFullSchema | Format::TpackFullSchemaWithId | Format::TpackSchemaRef => {
            SteadyEncoder::for_list(len, format)?.encode_once()
        }
        Format::Json => serde_json::to_vec(&list_serde(len)).map_err(EncodeError::Json),
        Format::Cbor => encode_cbor(&list_serde(len)),
        Format::MsgPack => rmp_serde::to_vec_named(&list_serde(len)).map_err(EncodeError::MsgPack),
    }
}

/// Decode previously encoded bytes.
pub fn decode(scenario: Scenario, format: Format, bytes: &[u8]) -> Result<(), EncodeError> {
    match format {
        Format::TpackFullSchema | Format::TpackFullSchemaWithId => {
            let mut decoder = Decoder::new(bytes);
            decoder.decode_message().map_err(EncodeError::Tpack)?;
            Ok(())
        }
        Format::TpackSchemaRef => {
            let (registry, _, _) = preload_registry(scenario);
            let mut decoder = Decoder::new(bytes);
            decoder
                .decode_message_with_registry(&registry)
                .map_err(EncodeError::Tpack)?;
            Ok(())
        }
        Format::Json => {
            let _: SerdePayload = serde_json::from_slice(bytes).map_err(EncodeError::Json)?;
            Ok(())
        }
        Format::Cbor => {
            let _: SerdePayload =
                ciborium::from_reader(bytes).map_err(|e| EncodeError::Cbor(e.to_string()))?;
            Ok(())
        }
        Format::MsgPack => {
            let _: SerdePayload =
                rmp_serde::from_slice(bytes).map_err(EncodeError::MsgPackDecode)?;
            Ok(())
        }
    }
}

pub fn decode_tpack_with_registry(
    bytes: &[u8],
    registry: &StdSchemaRegistry,
    format: Format,
) -> Result<(), EncodeError> {
    let mut decoder = Decoder::new(bytes);
    match format {
        Format::TpackSchemaRef => {
            decoder
                .decode_message_with_registry(registry)
                .map_err(EncodeError::Tpack)?;
        }
        Format::TpackFullSchema | Format::TpackFullSchemaWithId => {
            decoder.decode_message().map_err(EncodeError::Tpack)?;
        }
        _ => {
            return Err(EncodeError::Other(
                "decode_tpack_with_registry only for TPACK formats".into(),
            ));
        }
    }
    Ok(())
}

/// Preload a registry for SchemaRef decode (shared setup cost).
pub fn preload_registry(scenario: Scenario) -> (StdSchemaRegistry, Arc<Schema>, [u8; 8]) {
    let schema = Arc::new(tpack_schema(scenario));
    let id = schema_id_bytes(scenario);
    let registry = StdSchemaRegistry::new();
    registry
        .insert_shared(id, Arc::clone(&schema))
        .expect("fresh registry has no conflicts");
    (registry, schema, id)
}

pub fn preload_registry_from_schema(schema: Schema) -> (StdSchemaRegistry, Arc<Schema>, [u8; 8]) {
    let id = schema_id_for(&schema);
    let schema = Arc::new(schema);
    let registry = StdSchemaRegistry::new();
    registry
        .insert_shared(id, Arc::clone(&schema))
        .expect("fresh registry has no conflicts");
    (registry, schema, id)
}
