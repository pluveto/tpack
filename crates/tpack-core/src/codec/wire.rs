use alloc::vec::Vec;

use num_bigint::{BigInt, BigUint, Sign};

pub(in crate::codec) fn write_uvarint(out: &mut Vec<u8>, mut value: u64) {
    loop {
        let mut byte = (value & 0x7F) as u8;
        value >>= 7;
        if value != 0 {
            byte |= 0x80;
        }
        out.push(byte);
        if value == 0 {
            break;
        }
    }
}

pub(in crate::codec) fn write_svarint(out: &mut Vec<u8>, value: i64) {
    let raw = ((value as u64) << 1) ^ ((value >> 63) as u64);
    write_uvarint(out, raw);
}

/// Write an unbounded UVarInt (used only for `BigUInt` and bigint zigzag payloads).
pub(in crate::codec) fn write_uvarint_big(out: &mut Vec<u8>, value: &BigUint) {
    if *value == BigUint::ZERO {
        out.push(0);
        return;
    }
    let mut value = value.clone();
    loop {
        let low = value.to_u32_digits().first().copied().unwrap_or(0);
        let mut byte = (low as u8) & 0x7F;
        value >>= 7;
        if value == BigUint::ZERO {
            out.push(byte);
            break;
        }
        byte |= 0x80;
        out.push(byte);
    }
}

/// Zigzag-encode a signed BigInt, matching the i64 SVarInt mapping for in-range values.
pub(in crate::codec) fn zigzag_encode_big(value: &BigInt) -> BigUint {
    match value.sign() {
        Sign::Minus => {
            // raw = ((-n) << 1) - 1
            (value.magnitude() << 1) - 1u8
        }
        Sign::Plus | Sign::NoSign => value.magnitude() << 1,
    }
}

/// Inverse of [`zigzag_encode_big`].
pub(in crate::codec) fn zigzag_decode_big(raw: BigUint) -> BigInt {
    let is_negative = raw.bit(0);
    let half = raw >> 1;
    if is_negative {
        BigInt::from_biguint(Sign::Minus, half + 1u8)
    } else {
        BigInt::from_biguint(Sign::Plus, half)
    }
}

/// Canonical (shortest) byte length of a bigint UVarInt.
pub(in crate::codec) fn uvarint_big_len(value: &BigUint) -> usize {
    if *value == BigUint::ZERO {
        return 1;
    }
    (value.bits() as usize).div_ceil(7)
}

pub(in crate::codec) fn write_text(out: &mut Vec<u8>, value: &str) {
    write_bytes(out, value.as_bytes());
}

pub(in crate::codec) fn write_bytes(out: &mut Vec<u8>, value: &[u8]) {
    write_uvarint(out, value.len() as u64);
    out.extend_from_slice(value);
}

pub(in crate::codec) fn uvarint_len(mut value: u64) -> usize {
    let mut len = 1;
    while value >= 0x80 {
        value >>= 7;
        len += 1;
    }
    len
}

pub(in crate::codec) fn max_count_from_wire(value: u64) -> Option<u64> {
    if value == 0 { None } else { Some(value) }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec::Vec;
    use num_bigint::BigInt;

    #[test]
    fn zigzag_big_matches_i64_for_sample_grid() {
        let samples: [i64; 13] = [
            0,
            1,
            -1,
            2,
            -2,
            63,
            -64,
            127,
            -128,
            i64::MAX,
            i64::MIN,
            1_000_000,
            -1_000_000,
        ];
        for value in samples {
            let mut small = Vec::new();
            write_svarint(&mut small, value);
            let mut big = Vec::new();
            write_uvarint_big(&mut big, &zigzag_encode_big(&BigInt::from(value)));
            assert_eq!(small, big, "zigzag mismatch for {value}");
        }
    }

    #[test]
    fn uvarint_big_matches_u64_for_sample_grid() {
        let samples: [u64; 8] = [0, 1, 127, 128, 255, 16_383, 16_384, u64::MAX];
        for value in samples {
            let mut small = Vec::new();
            write_uvarint(&mut small, value);
            let mut big = Vec::new();
            write_uvarint_big(&mut big, &BigUint::from(value));
            assert_eq!(small, big, "uvarint mismatch for {value}");
            assert_eq!(uvarint_big_len(&BigUint::from(value)), small.len());
        }
    }
}
