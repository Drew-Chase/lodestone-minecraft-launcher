use super::*;

fn encoded(value: i32) -> Vec<u8> {
    let mut out = Vec::new();
    write_varint(&mut out, value);
    out
}

#[test]
fn small_values_take_one_byte() {
    assert_eq!(encoded(0), [0x00]);
    assert_eq!(encoded(1), [0x01]);
    assert_eq!(encoded(127), [0x7F]);
}

#[test]
fn larger_values_take_more_bytes_low_seven_bits_first() {
    assert_eq!(encoded(128), [0x80, 0x01]);
    assert_eq!(encoded(255), [0xFF, 0x01]);
    assert_eq!(encoded(25565), [0xDD, 0xC7, 0x01]);
}

#[test]
fn a_negative_value_takes_five_bytes() {
    assert_eq!(encoded(-1), [0xFF, 0xFF, 0xFF, 0xFF, 0x0F]);
}

#[test]
fn every_value_reads_back_with_the_length_it_took() {
    for value in [0, 1, 127, 128, 25565, i32::MAX, -1, i32::MIN] {
        let bytes = encoded(value);
        assert_eq!(read_varint(&bytes), Some((value, bytes.len())), "{value}");
    }
}

#[test]
fn a_varint_cut_short_or_running_past_five_bytes_does_not_read() {
    assert_eq!(read_varint(&[0x80]), None);
    assert_eq!(read_varint(&[0x80, 0x80, 0x80, 0x80, 0x80, 0x01]), None);
}

#[test]
fn a_string_is_its_byte_length_then_its_bytes() {
    let mut out = Vec::new();
    write_string(&mut out, "mc.hypixel.net");
    assert_eq!(out[0], 14);
    assert_eq!(&out[1..], b"mc.hypixel.net");
}

#[test]
fn a_frame_starts_with_the_length_of_its_id_and_body() {
    assert_eq!(frame(0x00, &[]), [0x01, 0x00]);
    assert_eq!(frame(0x01, &[1, 2, 3]), [0x04, 0x01, 1, 2, 3]);
}
