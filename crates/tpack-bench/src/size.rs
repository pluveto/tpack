//! Deterministic size metrics (safe to check in without a host label).

use tpack::EnvelopeMode;

use crate::codec_ops::{EncodeError, encode, encode_blob, encode_list, encode_wide};
use crate::format::Format;
use crate::payload::{AMORTIZED_NS, BLOB_SIZES, FIELD_COUNTS, LIST_LENGTHS, Scenario};

/// One measured encode size for a fixed scenario.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SizeSample {
    pub scenario: Scenario,
    pub format: Format,
    pub bytes: usize,
}

/// TPACK message component sizes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TpackBreakdown {
    pub scenario: Scenario,
    pub format: Format,
    pub total: usize,
    pub header_and_mode: usize,
    /// SchemaId payload bytes (not including its length prefix).
    pub schema_id: usize,
    /// Schema descriptor payload bytes (not including SchemaLen prefix).
    pub schema: usize,
    pub data: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AmortizedSample {
    pub scenario: Scenario,
    pub n: usize,
    pub label: &'static str,
    pub total_bytes: usize,
    pub per_message: f64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScaleSizeSample {
    pub axis: &'static str,
    pub param: usize,
    pub format: Format,
    pub bytes: usize,
}

pub fn measure_all_sizes() -> Result<Vec<SizeSample>, EncodeError> {
    let mut out = Vec::with_capacity(Scenario::ALL.len() * Format::ALL.len());
    for scenario in Scenario::ALL {
        for format in Format::ALL {
            let bytes = encode(scenario, format)?;
            out.push(SizeSample {
                scenario,
                format,
                bytes: bytes.len(),
            });
        }
    }
    Ok(out)
}

pub fn breakdown_tpack(scenario: Scenario, format: Format) -> Result<TpackBreakdown, EncodeError> {
    let bytes = encode(scenario, format)?;
    let mode = match format {
        Format::TpackFullSchema => EnvelopeMode::FullSchema,
        Format::TpackFullSchemaWithId => EnvelopeMode::FullSchemaWithId,
        Format::TpackSchemaRef => EnvelopeMode::SchemaRef,
        _ => {
            return Err(EncodeError::Other(
                "breakdown_tpack only applies to TPACK formats".into(),
            ));
        }
    };
    parse_tpack_breakdown(scenario, format, mode, &bytes)
}

fn parse_tpack_breakdown(
    scenario: Scenario,
    format: Format,
    mode: EnvelopeMode,
    bytes: &[u8],
) -> Result<TpackBreakdown, EncodeError> {
    if bytes.len() < 6 || &bytes[0..4] != b"TPAK" {
        return Err(EncodeError::Other("not a TPACK message".into()));
    }
    let mut i = 5;
    let mode_byte = bytes[i];
    i += 1;
    if mode_byte != mode.tag() {
        return Err(EncodeError::Other("envelope mode mismatch".into()));
    }

    let header_and_mode = 6;
    let mut schema_id_len = 0usize;
    let mut schema_len = 0usize;

    match mode {
        EnvelopeMode::FullSchema => {
            let (slen, ni) = read_uvarint(bytes, i)?;
            schema_len = slen as usize;
            i = ni + schema_len;
        }
        EnvelopeMode::FullSchemaWithId => {
            let (id_len, ni) = read_uvarint(bytes, i)?;
            schema_id_len = id_len as usize;
            i = ni + schema_id_len;
            let (slen, ni) = read_uvarint(bytes, i)?;
            schema_len = slen as usize;
            i = ni + schema_len;
        }
        EnvelopeMode::SchemaRef => {
            let (id_len, ni) = read_uvarint(bytes, i)?;
            schema_id_len = id_len as usize;
            i = ni + schema_id_len;
        }
    }
    let data = bytes.len().saturating_sub(i);
    Ok(TpackBreakdown {
        scenario,
        format,
        total: bytes.len(),
        header_and_mode,
        schema_id: schema_id_len,
        schema: schema_len,
        data,
    })
}

fn read_uvarint(bytes: &[u8], mut i: usize) -> Result<(u64, usize), EncodeError> {
    let mut value = 0u64;
    let mut shift = 0u32;
    loop {
        let byte = *bytes
            .get(i)
            .ok_or_else(|| EncodeError::Other("truncated uvarint".into()))?;
        i += 1;
        value |= u64::from(byte & 0x7f) << shift;
        if byte & 0x80 == 0 {
            return Ok((value, i));
        }
        shift += 7;
        if shift > 63 {
            return Err(EncodeError::Other("uvarint too long".into()));
        }
    }
}

/// 1× FullSchemaWithId + (n-1)× SchemaRef.
pub fn measure_amortized(scenario: Scenario, n: usize) -> Result<AmortizedSample, EncodeError> {
    assert!(n >= 1);
    let cold = encode(scenario, Format::TpackFullSchemaWithId)?;
    let hot = encode(scenario, Format::TpackSchemaRef)?;
    let total = cold.len() + hot.len() * n.saturating_sub(1);
    Ok(AmortizedSample {
        scenario,
        n,
        label: "tpack-1×with-id+(n-1)×schemaref",
        total_bytes: total,
        per_message: total as f64 / n as f64,
    })
}

pub fn measure_amortized_competitor(
    scenario: Scenario,
    format: Format,
    n: usize,
) -> Result<AmortizedSample, EncodeError> {
    let one = encode(scenario, format)?;
    let total = one.len() * n;
    Ok(AmortizedSample {
        scenario,
        n,
        label: format.label(),
        total_bytes: total,
        per_message: one.len() as f64,
    })
}

pub fn measure_amortized_scan(
    scenario: Scenario,
) -> Result<Vec<(AmortizedSample, AmortizedSample, AmortizedSample)>, EncodeError> {
    let mut out = Vec::with_capacity(AMORTIZED_NS.len());
    for &n in &AMORTIZED_NS {
        let tpack = measure_amortized(scenario, n)?;
        let json = measure_amortized_competitor(scenario, Format::Json, n)?;
        let cbor = measure_amortized_competitor(scenario, Format::Cbor, n)?;
        out.push((tpack, json, cbor));
    }
    Ok(out)
}

pub fn measure_blob_scale() -> Result<Vec<ScaleSizeSample>, EncodeError> {
    let mut out = Vec::new();
    for &len in &BLOB_SIZES {
        // Skip 1MiB for FullSchema competitors? No - size is cheap.
        for format in Format::STEADY {
            let bytes = encode_blob(format, len)?;
            out.push(ScaleSizeSample {
                axis: "blob_bytes",
                param: len,
                format,
                bytes: bytes.len(),
            });
        }
        // Also cold FullSchema for tax visibility on small blobs only.
        if len <= 1024 {
            for format in Format::COLD_TPACK {
                let bytes = encode_blob(format, len)?;
                out.push(ScaleSizeSample {
                    axis: "blob_bytes",
                    param: len,
                    format,
                    bytes: bytes.len(),
                });
            }
        }
    }
    Ok(out)
}

pub fn measure_list_scale() -> Result<Vec<ScaleSizeSample>, EncodeError> {
    let mut out = Vec::new();
    for &len in &LIST_LENGTHS {
        // Cap expensive FullSchema+large list sizes for cold modes: still measure SchemaRef/JSON/CBOR/MsgPack.
        for format in Format::STEADY {
            let bytes = encode_list(format, len)?;
            out.push(ScaleSizeSample {
                axis: "list_len",
                param: len,
                format,
                bytes: bytes.len(),
            });
        }
        if len <= 100 {
            for format in Format::COLD_TPACK {
                let bytes = encode_list(format, len)?;
                out.push(ScaleSizeSample {
                    axis: "list_len",
                    param: len,
                    format,
                    bytes: bytes.len(),
                });
            }
        }
    }
    Ok(out)
}

pub fn measure_field_scale() -> Result<Vec<ScaleSizeSample>, EncodeError> {
    let mut out = Vec::new();
    for &n in &FIELD_COUNTS {
        for format in Format::ALL {
            let bytes = encode_wide(format, n)?;
            out.push(ScaleSizeSample {
                axis: "field_count",
                param: n,
                format,
                bytes: bytes.len(),
            });
        }
    }
    Ok(out)
}
