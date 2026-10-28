// SPDX-License-Identifier: AGPL-3.0-or-later
//! The element type a clock place's slot row is declared by (QSpec FR-322
//! "Timed-model nodes").

use super::super::element_type;

/// SR-1746002 FND-002: a slot row below an `Option<..>` collection, QSL
/// FR-230-AC-19's `o: Option<Sequence<Clock>[0, 2]>`, is declared by the
/// collection's element type, read through the `Option`; a collection and
/// a plain type read as before.
#[test]
fn the_element_type_reads_through_an_option() {
    let clock = "ix://quire/native/Clock";
    assert_eq!(element_type(&format!("Option<Sequence<{clock}>>")), clock);
    assert_eq!(element_type(&format!("Set<{clock}>")), clock);
    assert_eq!(element_type(clock), clock);
}
