// SPDX-License-Identifier: AGPL-3.0-or-later
//! Feature-invariant recognition of JSON number tokens.

/// Visits every syntactically valid JSON number outside strings.
///
/// Full JSON grammar remains the owning decoder's responsibility. Malformed
/// number-like tokens are deliberately left for that decoder so this scan does
/// not reclassify syntax failures.
pub fn all(bytes: &[u8], mut accepts: impl FnMut(&str) -> bool) -> bool {
    let mut position = 0;
    let mut quoted = false;
    while position < bytes.len() {
        match bytes[position] {
            b'"' => {
                quoted = !quoted;
                position += 1;
            }
            b'\\' if quoted => {
                // JSON escapes consume the following byte. A missing or
                // malformed escape remains the owning decoder's syntax error.
                position = position.saturating_add(2);
            }
            b'-' | b'0'..=b'9' if !quoted && value_can_start(bytes, position) => {
                let start = position;
                let Some(end) = number_end(bytes, start) else {
                    position = number_like_end(bytes, start);
                    continue;
                };
                if end < bytes.len() && is_number_byte(bytes[end]) {
                    position = number_like_end(bytes, end);
                    continue;
                }
                let Ok(number) = std::str::from_utf8(&bytes[start..end]) else {
                    return false;
                };
                if !accepts(number) {
                    return false;
                }
                position = end;
            }
            _ => position += 1,
        }
    }
    true
}

fn number_end(bytes: &[u8], start: usize) -> Option<usize> {
    let mut position = start;
    if bytes.get(position) == Some(&b'-') {
        position += 1;
    }
    match bytes.get(position) {
        Some(b'0') => position += 1,
        Some(b'1'..=b'9') => {
            position += 1;
            while matches!(bytes.get(position), Some(b'0'..=b'9')) {
                position += 1;
            }
        }
        _ => return None,
    }
    if bytes.get(position) == Some(&b'.') {
        position += 1;
        let first = position;
        while matches!(bytes.get(position), Some(b'0'..=b'9')) {
            position += 1;
        }
        if position == first {
            return None;
        }
    }
    if matches!(bytes.get(position), Some(b'e' | b'E')) {
        position += 1;
        if matches!(bytes.get(position), Some(b'+' | b'-')) {
            position += 1;
        }
        let first = position;
        while matches!(bytes.get(position), Some(b'0'..=b'9')) {
            position += 1;
        }
        if position == first {
            return None;
        }
    }
    Some(position)
}

fn number_like_end(bytes: &[u8], mut position: usize) -> usize {
    while matches!(bytes.get(position), Some(byte) if is_number_byte(*byte)) {
        position += 1;
    }
    position
}

const fn value_can_start(bytes: &[u8], position: usize) -> bool {
    position == 0
        || matches!(
            bytes[position - 1],
            b' ' | b'\t' | b'\r' | b'\n' | b'[' | b',' | b':'
        )
}

const fn is_number_byte(byte: u8) -> bool {
    matches!(byte, b'0'..=b'9' | b'.' | b'e' | b'E' | b'+' | b'-')
}
