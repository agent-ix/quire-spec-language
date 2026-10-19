//! FR-218 "Requirements" (TC-1056, TC-1065) over fixture R of the protocol
//! obligation tests, whose header selects the temporal profile `t` a
//! temporal clause is written `using`.

use ix_trace_rs::trace;
use qsl_forms::build_unit;
use qsl_foundation::SourceIdentity;
use quire_semantic_value::checking::CheckingLimits;

use super::{check, CheckedRequirement, UnitDeclarations};
use crate::check::refusal::ProtocolAnchorCause;
use crate::check::{CheckCause, CheckRefusal, PackageDeclarations};

mod fixture {
    use crate as semantics;
    include!("../../../tests/fixtures/protocol_obligations.rs");
}
use fixture::{obligation_model, HEADER};

/// A temporal clause `name` whose formula holds `condition`.
fn temporal(name: &str, condition: &str) -> String {
    format!(
        "temporal {name} using t over (c: Boolean) clock \"steps\" on origin \
         {{ always holds({condition}) }}\n"
    )
}

/// A protocol `name` with the `members` written before `run`.
fn protocol(name: &str, members: &str) -> String {
    format!(
        "protocol {name} using v over (k: M::Node) on origin {{\n\
         role w on M::Node;\n\
         {members}\n\
         run sequence Main {{\n\
         attempt A by w on M::Node::step contracts [] as (a: M::Flag) {{ true }};\n\
         }}\n\
         finish Done as (d: M::Receipt) {{ true }};\n\
         }}\n"
    )
}

fn assemble(declarations: &str) -> (String, PackageDeclarations) {
    let text = format!("{HEADER}{declarations}");
    let parsed = qsl_cst::parse(
        SourceIdentity::new("a", "u", "git", "1"),
        "unit.native",
        text.as_bytes(),
        qsl_cst::Limits::default(),
    )
    .expect("S1 reads the unit");
    assert!(parsed.is_admissible(), "{text}: {:?}", parsed.diagnostics());
    let unit = build_unit(&parsed).expect("S2 builds the unit");
    let assembled = PackageDeclarations::assemble(
        parsed.source().reference().clone(),
        unit,
        vec![obligation_model()],
        Vec::new(),
    )
    .unwrap_or_else(|refusal| panic!("{text}: the assembler refuses: {refusal:?}"));
    (text, assembled)
}

/// The requirements of the protocol `name` of `declarations`, checked
/// alone.
fn requirements(
    declarations: &str,
    name: &str,
) -> Result<Vec<CheckedRequirement>, Vec<CheckRefusal>> {
    let (_, assembled) = assemble(declarations);
    let index = assembled
        .protocols
        .iter()
        .position(|protocol| protocol.name.name == name)
        .expect("the protocol");
    let unit = UnitDeclarations::new(&assembled.temporal_clauses, &assembled.protocols, []);
    check(
        index,
        &assembled.protocols,
        &assembled.temporal_clauses,
        &unit,
    )
}

/// The protocol refusals the whole check of `declarations` gives, with the
/// unit's text.
fn protocol_refusals(declarations: &str) -> (String, Vec<(ProtocolAnchorCause, CheckRefusal)>) {
    let (text, assembled) = assemble(declarations);
    let refusals = assembled
        .check(CheckingLimits::default())
        .err()
        .unwrap_or_default();
    let found = refusals
        .into_iter()
        .filter_map(|refusal| match &refusal.cause {
            CheckCause::ProtocolAnchor(cause) => Some(((**cause).clone(), refusal.clone())),
            _ => None,
        })
        .collect();
    (text, found)
}

/// The one protocol refusal of `declarations`, its catalog pair and the
/// text at its span.
fn only(
    declarations: &str,
) -> (
    ProtocolAnchorCause,
    (&'static str, Option<&'static str>),
    String,
) {
    let (text, found) = protocol_refusals(declarations);
    let [(cause, refusal)] = found.as_slice() else {
        panic!("{declarations}: one protocol refusal, got {found:?}");
    };
    let span = cause.span();
    (
        cause.clone(),
        (refusal.cause.code().as_str(), refusal.cause.cause()),
        text[span.start..span.end].to_owned(),
    )
}

/// FR-218-AC-16 but its import case: `requires temporal Due;` binds the
/// requirement to `Due`'s position, and `Due` required twice is recorded
/// once; `Nothing` and a name the unit binds only as a model alias refuse
/// `missing_declaration`/`missing-name` at the name, and a predicate refuses
/// `ill_typed`/`operator-ineligible` there, naming the kind.
#[trace("TC-1056")]
#[test]
fn a_requirement_binds_its_temporal_clause_by_position() {
    let unit = format!(
        "{}{}{}",
        temporal("Other", "true"),
        temporal("Due", "true"),
        protocol(
            "Fill",
            "requires temporal Due;\nrequires temporal Other;\nrequires temporal Due;"
        ),
    );
    let checked = requirements(&unit, "Fill").expect("the requirements check");
    let [due, other] = checked.as_slice() else {
        panic!("two requirements, got {checked:?}");
    };
    assert_eq!((due.temporal, other.temporal), (1, 0));
    assert!(due.span.start < other.span.start);
    let (_, found) = protocol_refusals(&unit);
    assert!(found.is_empty(), "{found:?}");

    for name in ["Nothing", "M"] {
        let (cause, pair, at) = only(&format!(
            "{}{}",
            temporal("Due", "true"),
            protocol("Fill", &format!("requires temporal {name};"))
        ));
        assert_eq!(pair, ("missing_declaration", Some("missing-name")));
        assert_eq!(
            cause,
            ProtocolAnchorCause::MissingRequirement {
                name: name.to_owned(),
                span: cause.span(),
            }
        );
        assert_eq!(at, name);
    }

    let (cause, pair, at) = only(&format!(
        "predicate Accept using v(): Boolean {{ true }}\n{}",
        protocol("Fill", "requires temporal Accept;")
    ));
    assert_eq!(pair, ("ill_typed", Some("operator-ineligible")));
    let ProtocolAnchorCause::RequirementKind { name, kind, .. } = &cause else {
        panic!("a requirement kind refusal, got {cause:?}");
    };
    assert_eq!((name.as_str(), *kind), ("Accept", "predicate"));
    assert_eq!(at, "Accept");
}

/// FR-218-AC-25: `requires temporal Due;` where `Due` holds `protocol
/// Fill` and the requiring protocol is `Fill`, and the same through a
/// second clause that a protocol `Fill` names requires, each refuse
/// `invalid_package`/`definition-cycle` at the requirement, naming `Fill`
/// and the clause; the same requirement from a protocol `Drain` that no
/// clause names checks. No refused unit compiles.
#[trace("TC-1065", "FR-218-AC-25")]
#[test]
fn a_requirement_whose_clause_reaches_its_protocol_is_a_cycle() {
    let direct = format!(
        "{}{}",
        temporal("Due", "protocol Fill"),
        protocol("Fill", "requires temporal Due;")
    );
    // `Fill` requires `Due`, which names `Pour`, which requires `Back`, which
    // names `Fill`.
    let through = format!(
        "{}{}{}{}",
        temporal("Due", "protocol Pour"),
        temporal("Back", "protocol Fill"),
        protocol("Fill", "requires temporal Due;"),
        protocol("Pour", "requires temporal Back;"),
    );
    for (unit, temporal) in [(&direct, "Due"), (&through, "Due")] {
        let (_, found) = protocol_refusals(unit);
        let (cause, refusal) = found
            .iter()
            .find(|(cause, _)| {
                matches!(cause, ProtocolAnchorCause::RequirementCycle { protocol, .. }
                    if protocol == "Fill")
            })
            .unwrap_or_else(|| panic!("{unit}: a cycle at Fill, got {found:?}"));
        assert_eq!(
            (refusal.cause.code().as_str(), refusal.cause.cause()),
            ("invalid_package", Some("definition-cycle"))
        );
        let ProtocolAnchorCause::RequirementCycle {
            temporal: found, ..
        } = cause
        else {
            unreachable!("matched above");
        };
        assert_eq!(found, temporal);
    }

    let drain = format!(
        "{}{}{}",
        temporal("Due", "protocol Fill"),
        protocol("Fill", ""),
        protocol("Drain", "requires temporal Due;"),
    );
    assert_eq!(
        requirements(&drain, "Drain").map(|found| found.len()),
        Ok(1)
    );
    let (_, found) = protocol_refusals(&drain);
    assert!(found.is_empty(), "{found:?}");
}
