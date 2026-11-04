//! Just enough base64 for an OSC 52 clipboard write.
//!
//! A terminal takes clipboard text as `ESC ] 52 ; c ; <base64> ST`, so ten lines of
//! encoder is cheaper than a dependency — and the alphabet is fixed by the terminal,
//! not by us.

const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

/// Standard base64, padded, as the escape sequence wants it.
pub fn base64(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let mut buffer = [0_u8; 3];
        buffer[..chunk.len()].copy_from_slice(chunk);
        let value = u32::from(buffer[0]) << 16 | u32::from(buffer[1]) << 8 | u32::from(buffer[2]);
        for index in 0..4 {
            let shifted = (value >> (18 - index * 6)) & 0b11_1111;
            let symbol = u8::try_from(shifted).unwrap_or(0);
            out.push(char::from(
                *ALPHABET.get(usize::from(symbol)).unwrap_or(&b'A'),
            ));
        }
    }
    // a chunk of one byte carries two symbols, a chunk of two carries three
    let padding = (3 - bytes.len() % 3) % 3;
    out.truncate(out.len().saturating_sub(padding));
    out.extend(std::iter::repeat_n('=', padding));
    out
}

#[cfg(test)]
mod tests {
    use super::base64;

    #[test]
    fn encodes_the_cases_the_terminal_will_see() {
        // the RFC 4648 examples, which is what every OSC 52 consumer decodes
        assert_eq!(base64(b""), "");
        assert_eq!(base64(b"f"), "Zg==");
        assert_eq!(base64(b"fo"), "Zm8=");
        assert_eq!(base64(b"foo"), "Zm9v");
        assert_eq!(base64(b"foob"), "Zm9vYg==");
        assert_eq!(base64(b"fooba"), "Zm9vYmE=");
        assert_eq!(base64(b"foobar"), "Zm9vYmFy");
        // and a path, which is the usual payload
        assert_eq!(base64(b"/tmp/a/TODOs.md"), "L3RtcC9hL1RPRE9zLm1k");
    }
}
