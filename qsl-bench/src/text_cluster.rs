// SPDX-License-Identifier: AGPL-3.0-or-later
//! QSL-215: a Text-reachable recursive cluster, for the cost of FR-093's
//! text-leaf walk. Kept apart from [`crate::check`], whose generators are
//! MP-002's protected apparatus.
//!
//! `n` records `R0` .. `R{n-1}`, each holding a required
//! `label: Text[0, 8; nfc]` and an optional field of every other record,
//! and one function `eq(a: R0, b: R0): Boolean { a = b }`. The structural
//! equality's leaf list (FR-093 "Text leaves") holds one leaf per simple
//! path through the cluster, on the order of `(n - 1)!` leaves.
//!
//! [`deep_wide`] is the QSL-214 review's long-path shape: few enough leaves
//! for the node ceiling, but each under a long path of long field names.

use qsl_forms::{BinaryOperator, BuiltinType, Expression, FunctionDeclaration, TypeForm};
use qsl_semantics::check::{LockEvidence, PackageDeclarations};
use qsl_semantics::value::declaration::{
    CompositeDeclaration, CompositeShape, FieldDeclaration, TypeEnvironment,
};
use qsl_semantics::value::{DefinitionReference, DefinitionRevision};
use quire_exact::{NodeKey, Presence, TextProfile, TextType, ValueType};

use crate::check::owner;

const SPAN: qsl_foundation::Span = qsl_foundation::Span { start: 0, end: 0 };

/// The caller's handle for record `R{index}`: its index in the first eight
/// bytes of the digest. `check` re-keys every declared record (FR-092).
fn handle(index: usize) -> NodeKey {
    let mut digest = [0_u8; 32];
    digest[..8].copy_from_slice(&crate::widen(index).to_be_bytes());
    NodeKey::from_digest(digest)
}

fn record(index: usize, records: usize) -> CompositeDeclaration {
    let label = TextType::new(0, 8, TextProfile::Nfc).expect("a valid text type");
    let mut fields = vec![FieldDeclaration::new(
        "label",
        ValueType::Text(label),
        Presence::Required,
    )];
    fields.extend((0..records).filter(|other| *other != index).map(|other| {
        FieldDeclaration::new(
            format!("r{other}"),
            ValueType::Composite(handle(other)),
            Presence::Optional,
        )
    }));
    CompositeDeclaration::new(
        handle(index),
        format!("R{index}"),
        CompositeShape::Record(fields),
    )
}

/// QSpec's text-profile definition, the law each text leaf names.
fn text_profile() -> DefinitionReference {
    DefinitionReference {
        authority: "agent-ix".to_owned(),
        identity: "quire.value.text.unicode-17.0.0/v1".to_owned(),
        revision: DefinitionRevision {
            namespace: "quire-draft".to_owned(),
            value: "1-draft.1".to_owned(),
        },
        digest_domain: "quire.definition.bytes/v1".to_owned(),
        digest: "cd4a985a0d7d2f2b3d3625caee3787832c00c5244e805fb49e1c2c7075b9de5e".to_owned(),
    }
}

/// `records` mutually referencing, Text-reachable records and one
/// structural equality over `R0`.
pub fn text_cluster(records: usize) -> PackageDeclarations {
    let declarations: Vec<_> = (0..records).map(|index| record(index, records)).collect();
    equality_package(declarations, "R0")
}

/// The review's deep-and-wide chain length.
pub const DEEP_WIDE_CHAIN: usize = 46;

/// The review's deep-and-wide tree depth: `2^16` = 65,536 text leaves.
pub const DEEP_WIDE_LEVELS: usize = 17;

/// A `chain`-record chain `C0 .. C{chain-1}` of optional fields into a
/// binary tree `T0 .. T{levels-1}` of optional fields, whose last level
/// holds one text field, every field name `name_bytes` long, and one
/// structural equality over `C0`: `2^(levels - 1)` text leaves, each under
/// a path of `2 * (chain + levels - 1) + 1` segments.
///
/// # Panics
///
/// Panics when `name_bytes` is zero, or when FR-143 refuses the records,
/// which a harness defect alone causes.
pub fn deep_wide(chain: usize, levels: usize, name_bytes: usize) -> PackageDeclarations {
    assert!(name_bytes > 0, "a field name is at least one byte");
    let name = |tag: char| format!("{tag}{}", "x".repeat(name_bytes - 1));
    let optional = |field: String, to: usize| {
        FieldDeclaration::new(field, ValueType::Composite(handle(to)), Presence::Optional)
    };
    let mut declarations: Vec<CompositeDeclaration> = (0..chain)
        .map(|at| {
            CompositeDeclaration::new(
                handle(at),
                format!("C{at}"),
                CompositeShape::Record(vec![optional(name('c'), at + 1)]),
            )
        })
        .collect();
    declarations.extend((0..levels).map(|level| {
        let at = chain + level;
        let fields = if level + 1 < levels {
            vec![optional(name('l'), at + 1), optional(name('r'), at + 1)]
        } else {
            let label = TextType::new(0, 8, TextProfile::Nfc).expect("a valid text type");
            vec![FieldDeclaration::new(
                name('t'),
                ValueType::Text(label),
                Presence::Required,
            )]
        };
        CompositeDeclaration::new(
            handle(at),
            format!("T{level}"),
            CompositeShape::Record(fields),
        )
    }));
    equality_package(declarations, "C0")
}

/// `declarations` and one structural equality over `compared`.
fn equality_package(
    declarations: Vec<CompositeDeclaration>,
    compared: &str,
) -> PackageDeclarations {
    let operand = || TypeForm::name(compared, SPAN);
    let eq = FunctionDeclaration::new(
        "eq",
        vec![("a".to_owned(), operand()), ("b".to_owned(), operand())],
        TypeForm::builtin(BuiltinType::Boolean, SPAN),
        None,
        Expression::Binary {
            operator: BinaryOperator::Equal,
            left: Box::new(Expression::Name("a".to_owned())),
            right: Box::new(Expression::Name("b".to_owned())),
        },
    );
    PackageDeclarations {
        types: TypeEnvironment::new(declarations, []).expect("FR-143 admits the records"),
        functions: vec![eq],
        lock_evidence: LockEvidence::default().with_text_profile(text_profile()),
        ..PackageDeclarations::new(owner())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::check::check;

    #[test]
    fn a_small_cluster_checks_at_default_limits() {
        assert!(check(text_cluster(3)).is_ok());
    }

    #[test]
    fn a_small_deep_wide_package_checks_at_default_limits() {
        assert!(check(deep_wide(2, 3, 4)).is_ok());
    }
}
