// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-070: the canonical name of a structural node in a witness transcript.

use qsl_foundation::node_wire::node_key_to_text;
use quire_exact::NodeKey;

use crate::NodeWireError;

const HEX: &[u8; 16] = b"0123456789ABCDEF";

fn transcript_separator(byte: u8) -> bool {
    matches!(byte, b'%' | b'=' | b';' | b'|' | b'<' | b'>') || byte < 0x20 || byte == 0x7f
}

/// The canonical transcript entry name for `node`.
///
/// The name is the compact structural node JSON spelling with `%`, `=`, `;`,
/// `|`, `<`, `>` and control bytes percent-escaped using uppercase
/// hexadecimal. It identifies a transcript entry only; it grants no replay,
/// admission or provenance authority.
///
/// # Errors
///
/// Returns [`NodeWireError::Allocation`] when storage cannot be reserved or
/// [`NodeWireError::Shape`] when the shared node-wire codec cannot write the
/// structural identity.
pub fn witness_entry_name(node: &NodeKey) -> Result<String, NodeWireError> {
    let compact = node_key_to_text(node)?;
    let capacity = compact
        .bytes()
        .try_fold(0usize, |capacity, byte| {
            capacity.checked_add(if transcript_separator(byte) { 3 } else { 1 })
        })
        .ok_or(NodeWireError::Allocation)?;

    let mut name = String::new();
    name.try_reserve_exact(capacity)
        .map_err(|_| NodeWireError::Allocation)?;
    for character in compact.chars() {
        if character.is_ascii() && transcript_separator(character as u8) {
            let byte = character as u8;
            name.push('%');
            name.push(char::from(HEX[(byte >> 4) as usize]));
            name.push(char::from(HEX[(byte & 0x0f) as usize]));
        } else {
            name.push(character);
        }
    }
    Ok(name)
}
