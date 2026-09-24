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
///
/// # Panics
///
/// Panics when FR-143 refuses the records, which a harness defect alone
/// causes: every name is distinct and every field names a declared record.
pub fn text_cluster(records: usize) -> PackageDeclarations {
    let declarations: Vec<_> = (0..records).map(|index| record(index, records)).collect();
    let r0 = || TypeForm::name("R0", SPAN);
    let eq = FunctionDeclaration::new(
        "eq",
        vec![("a".to_owned(), r0()), ("b".to_owned(), r0())],
        TypeForm::builtin(BuiltinType::Boolean, SPAN),
        None,
        Expression::Binary {
            operator: BinaryOperator::Equal,
            left: Box::new(Expression::Name("a".to_owned())),
            right: Box::new(Expression::Name("b".to_owned())),
        },
    );
    PackageDeclarations {
        types: TypeEnvironment::new(declarations, []).expect("FR-143 admits the cluster"),
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
}
