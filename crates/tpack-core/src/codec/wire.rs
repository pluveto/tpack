use alloc::vec::Vec;

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
