//! The wire format of Minecraft's protocol: variable-length integers, strings, and the packets they frame. A packet is its
//! length as a VarInt, then its id and its body.

/// The most bytes a VarInt takes. A protocol number, a length or an id never needs more.
const MAX_VARINT_BYTES: usize = 5;

/// Appends `value` as a VarInt: seven bits at a time, least significant first, with the high bit set on every byte but the
/// last. A negative value takes five bytes.
pub fn write_varint(out: &mut Vec<u8>, value: i32) {
    let mut value = value.cast_unsigned();
    loop {
        if value & !0x7F == 0 {
            out.push(value as u8);
            return;
        }
        out.push((value & 0x7F) as u8 | 0x80);
        value >>= 7;
    }
}

/// Reads a VarInt from the start of `bytes`: its value, and how many bytes it took. `None` when the bytes end inside it, or
/// when it runs past its five bytes.
pub fn read_varint(bytes: &[u8]) -> Option<(i32, usize)> {
    let mut value: u32 = 0;
    for (index, byte) in bytes.iter().take(MAX_VARINT_BYTES).enumerate() {
        value |= u32::from(byte & 0x7F) << (7 * index);
        if byte & 0x80 == 0 {
            return Some((value.cast_signed(), index + 1));
        }
    }
    None
}

/// Appends a string as the protocol writes one: its length in bytes as a VarInt, then its UTF-8 bytes.
pub fn write_string(out: &mut Vec<u8>, text: &str) {
    write_varint(out, i32::try_from(text.len()).unwrap_or(i32::MAX));
    out.extend_from_slice(text.as_bytes());
}

/// Frames a packet: its length as a VarInt, then its id and its body.
pub fn frame(id: i32, body: &[u8]) -> Vec<u8> {
    let mut payload = Vec::with_capacity(body.len() + MAX_VARINT_BYTES);
    write_varint(&mut payload, id);
    payload.extend_from_slice(body);
    let mut packet = Vec::with_capacity(payload.len() + MAX_VARINT_BYTES);
    write_varint(&mut packet, i32::try_from(payload.len()).unwrap_or(i32::MAX));
    packet.extend_from_slice(&payload);
    packet
}

#[cfg(test)]
#[path = "../tests/protocol.rs"]
mod tests;
