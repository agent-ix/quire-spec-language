// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-755 to TC-757 and TC-759: the front-end operations are typed library
//! operations over a caller-owned handle, and compose to a checked,
//! emitted package.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, OnceLock};

use crate::spine::spine_model;
use ix_trace_rs::trace;
use qsl_cst::Limits as SourceLimits;
use qsl_eval::value::QualifiedName;
use qsl_foundation::diagnostic::{
    LimitExceeded, LimitKind as FoundationKind, StageFailure, Staged,
};
use qsl_foundation::{Code, Setting, SourceIdentity};
use qsl_package::{emit_checked, read_import_view, AdmittedPackages};
use qsl_semantics::library::LibraryName;
use qsl_semantics::model::accounting::ModelNormalizationLimits;
use qsl_semantics::model::object_environment::ObjectEnvironment;
use quire_eval::CallFailure;
use quire_eval::FamilyOutcome;
use quire_exact::{Cancel, CancelCause, Integer, Outcome, Value};
use quire_semantic_value::catalog::Category;
use quire_semantic_value::checking::CheckingLimits;
use quire_semantic_value::supplied_stop::SuppliedLimits;

use super::{
    admit_parsed_unit_list, check, check_units, check_with_refinement_internals, execute,
    failure_of, package, package_with_temporal_selections, parse, preflight_unit_list,
    refusal_or_fault, select, select_units, temporal_selections_for_source, AdmittedModels,
    CheckedUnit, EmittedUnit, ExecuteRequest, FrontEndFailure, PackageLimits, ParseRequest,
    ParsedUnitSource,
};
use crate::bounds::ReplayLimits;
use crate::limits::CallerLimits;
use crate::outcome::OutcomeDocument;
use crate::request::StageLimits;
use crate::spine::RunRefusal;
use crate::spine::{
    default_accounting, CompileRefusal, DependencyInput, SpineLimits, SpineStage, SuppliedLibrary,
    DEFAULT_WORK_UNITS,
};

const FIXTURE: &str = include_str!("../../../../tests/fixtures/spine-compile.native");
const MODEL_DOCUMENT: &[u8] = spine_model::DOCUMENT;

const HEADER: &str = "language \"ix:native\" edition \"1-draft\";\n\
    profile v = \"quire.value.complete/v1\";\n\
    profile Complete = \"quire.protocol.finite-global/v1\";\n";

fn identity() -> SourceIdentity {
    SourceIdentity::new("agent-ix", "test:lifecycle", "fixture", "fixture:1")
}

#[trace("FR-050-AC-6")]
#[test]
fn temporal_selection_partition_uses_the_exact_source_reference() {
    use qsl_foundation::source::provenance::RawSourceRef;
    use qsl_foundation::Span;
    use qsl_package::{ClockConfiguration, TemporalClockSelection};

    let root = RawSourceRef::new("test-authority", "root.native").expect("valid source");
    let library = RawSourceRef::new("test-authority", "library.native").expect("valid source");
    let selections = [
        TemporalClockSelection {
            source: root.clone(),
            declaration_span: Span { start: 10, end: 20 },
            clock: ClockConfiguration::EventPosition {
                sequence_authority: "root-events".into(),
            },
        },
        TemporalClockSelection {
            source: library.clone(),
            declaration_span: Span { start: 30, end: 40 },
            clock: ClockConfiguration::EventPosition {
                sequence_authority: "library-events".into(),
            },
        },
    ];

    assert_eq!(
        temporal_selections_for_source(&selections, &root),
        [selections[0].clone()]
    );
    assert_eq!(
        temporal_selections_for_source(&selections, &library),
        [selections[1].clone()]
    );
}

fn request(bytes: &[u8]) -> ParseRequest<'_> {
    // The identity lives for the whole process, so a request can borrow it.
    static IDENTITY: OnceLock<SourceIdentity> = OnceLock::new();
    ParseRequest {
        source: IDENTITY.get_or_init(identity),
        path: "unit.native",
        bytes,
    }
}

fn parse_named(identity: &SourceIdentity, bytes: &[u8]) -> ParsedUnitSource {
    parse(
        &ParseRequest {
            source: identity,
            path: "unit.native",
            bytes,
        },
        SourceLimits::default(),
        &Cancel::new(),
    )
    .expect("source unit parses")
    .into_value()
}

#[trace("FR-480-AC-4")]
#[test]
fn unit_list_preflight_refuses_empty_duplicate_and_library_owner_before_parsing() {
    let dependencies = DependencyInput::default();
    assert!(matches!(
        preflight_unit_list(&[], &dependencies).map_err(|refusal| *refusal),
        Err(CompileRefusal::UnitList(
            crate::spine::UnitListRefusal::Empty
        ))
    ));

    let owner = SourceIdentity::new("test", "unit", "git", "1");
    let duplicate_requests = [
        ParseRequest {
            source: &owner,
            path: "bad-a",
            bytes: b"not valid",
        },
        ParseRequest {
            source: &owner,
            path: "bad-b",
            bytes: b"also not valid",
        },
    ];
    assert!(matches!(
        preflight_unit_list(&duplicate_requests, &dependencies).map_err(|refusal| *refusal),
        Err(CompileRefusal::UnitList(
            crate::spine::UnitListRefusal::DuplicateOwner {
                first: 0,
                second: 1,
                ..
            }
        ))
    ));

    let library = SuppliedLibrary {
        identity: "library".into(),
        source: owner.clone(),
        path: "library.native".into(),
        bytes: Vec::new(),
    };
    let dependencies = DependencyInput::new([library]).expect("one library is valid");
    let requests = [ParseRequest {
        source: &owner,
        path: "bad",
        bytes: b"not valid",
    }];
    assert!(matches!(
        preflight_unit_list(&requests, &dependencies).map_err(|refusal| *refusal),
        Err(CompileRefusal::UnitList(
            crate::spine::UnitListRefusal::LibraryOwner { unit: 0, .. }
        ))
    ));
}

#[trace("FR-480-AC-2", "FR-480-AC-4")]
#[test]
fn parsed_unit_list_rejects_cross_unit_names_and_orders_distinct_units_canonically() {
    let first = SourceIdentity::new("test", "z-unit", "git", "1");
    let second = SourceIdentity::new("test", "a-unit", "git", "1");
    let declaration = format!("{HEADER}function shared(x: Int[0, 9]): Boolean pure {{ x < 5 }}\n");
    let duplicate = admit_parsed_unit_list(
        vec![
            parse_named(&first, declaration.as_bytes()),
            parse_named(&second, declaration.as_bytes()),
        ],
        &DependencyInput::default(),
    );
    assert!(matches!(
        duplicate.map_err(|refusal| *refusal),
        Err(CompileRefusal::UnitList(crate::spine::UnitListRefusal::DuplicateName { name, first_unit, second_unit, .. })) if name == "shared" && first_unit.identity == "z-unit" && second_unit.identity == "a-unit"
    ));

    let unit_a = format!("{HEADER}function left(x: Int[0, 9]): Boolean pure {{ x < 5 }}\n");
    let unit_z = format!("{HEADER}function right(x: Int[0, 9]): Boolean pure {{ x < 5 }}\n");
    let admitted = admit_parsed_unit_list(
        vec![
            parse_named(&first, unit_z.as_bytes()),
            parse_named(&second, unit_a.as_bytes()),
        ],
        &DependencyInput::default(),
    )
    .expect("distinct empty units share compatible headers");
    assert_eq!(admitted.units()[0].source().identity().identity, "a-unit");
    assert_eq!(admitted.units()[1].source().identity().identity, "z-unit");

    let sibling_import = format!(
        "{HEADER}import \"a-unit\";\nfunction local(x: Int[0, 9]): Boolean pure {{ x < 5 }}\n"
    );
    let refused = admit_parsed_unit_list(
        vec![
            parse_named(&first, sibling_import.as_bytes()),
            parse_named(&second, unit_a.as_bytes()),
        ],
        &DependencyInput::default(),
    );
    assert!(matches!(
        refused.map_err(|refusal| *refusal),
        Err(CompileRefusal::UnitList(crate::spine::UnitListRefusal::SiblingImport { identity, unit, region: Some(region) })) if identity == "a-unit" && unit.identity == "a-unit" && region.source().identity() == "z-unit" && region.start() < region.end()
    ));

    let different_profiles =
        "language \"ix:native\" edition \"1-draft\";\nprofile v = \"other.profile/v1\";\nfunction local(x: Int[0, 9]): Boolean pure { x < 5 }\n"
            .to_owned();
    let refused = admit_parsed_unit_list(
        vec![
            parse_named(&first, unit_z.as_bytes()),
            parse_named(&second, different_profiles.as_bytes()),
        ],
        &DependencyInput::default(),
    );
    assert!(matches!(
        refused.map_err(|refusal| *refusal),
        Err(CompileRefusal::UnitList(crate::spine::UnitListRefusal::HeaderConflict {
            first_unit,
            second_unit,
            first_header,
            second_header,
            region: Some(region),
        })) if first_unit.identity == "z-unit"
            && second_unit.identity == "a-unit"
            && first_header.start == 0
            && second_header.start == 0
            && region.source().identity() == "a-unit"
            && usize::try_from(region.end()).unwrap() == second_header.end
    ));

    let profile_order_first =
        "language \"ix:native\" edition \"1-draft\";\nprofile x = \"p1\";\nprofile y = \"p2\";\nfunction ordered_a(x: Int[0, 9]): Boolean pure { x < 5 }\n"
            .to_owned();
    let profile_order_second =
        "language \"ix:native\" edition \"1-draft\";\nprofile y = \"p2\";\nprofile x = \"p1\";\nfunction ordered_b(x: Int[0, 9]): Boolean pure { x < 5 }\n"
            .to_owned();
    assert!(matches!(
        admit_parsed_unit_list(
            vec![
                parse_named(&first, profile_order_first.as_bytes()),
                parse_named(&second, profile_order_second.as_bytes()),
            ],
            &DependencyInput::default(),
        )
        .map_err(|refusal| *refusal),
        Err(CompileRefusal::UnitList(
            crate::spine::UnitListRefusal::HeaderConflict { .. }
        ))
    ));

    assert!(!super::headers_match(
        Some("1-draft"),
        &["p1"],
        Some("0-draft"),
        &["p1"],
    ));

    let alias_first =
        "language \"ix:native\" edition \"1-draft\";\nprofile v = \"quire.value.complete/v1\";\nfunction from_v using v(x: Int[0, 9]): Boolean pure { x < 5 }\n"
            .to_owned();
    let alias_second =
        "language \"ix:native\" edition \"1-draft\";\nprofile w = \"quire.value.complete/v1\";\nfunction from_w using w(x: Int[0, 9]): Boolean pure { x < 5 }\n"
            .to_owned();
    assert!(admit_parsed_unit_list(
        vec![
            parse_named(&first, alias_first.as_bytes()),
            parse_named(&second, alias_second.as_bytes()),
        ],
        &DependencyInput::default(),
    )
    .is_ok());
}

fn protocol_source(name: &str, close: bool) -> String {
    format!(
        "protocol {name} using Complete over (input: M::Widget) on origin {{\n\
         role R on M::Widget;\n\
         run sequence Main {{ }}\n\
         finish End as (outcome: Boolean) {{ true }};\n\
         {}",
        if close { "}" } else { "" }
    )
}

/// FR-503-AC-2 (TC-1298 rows 1-2): S1's attributable refusal retains every
/// delimited protocol result in declaration order.
#[trace("TC-1298", "FR-503-AC-2")]
#[test]
fn parse_carries_protocol_results_for_a_confined_syntax_refusal() {
    let w = protocol_source("W", true);
    let x = protocol_source("X", false);
    let z = protocol_source("Z", true);
    let unit = format!("{HEADER}{w}\n{x}\n{z}");
    let failure = parse(
        &request(unit.as_bytes()),
        SourceLimits::default(),
        &Cancel::new(),
    )
    .expect_err("X has an attributable syntax refusal");
    let StageFailure::RefusedWithPayload(refusal, results) = failure else {
        panic!("expected a refusal with protocol results, got {failure:?}");
    };
    assert_eq!(refusal.code(), Code::InvalidSyntax);
    assert!(matches!(
        results.as_slice(),
        [
            crate::spine::ProtocolResult::Cleared { declaration: w, stage: SpineStage::Forms },
            crate::spine::ProtocolResult::Refused { declaration: x, source_index: 1, .. },
            crate::spine::ProtocolResult::Cleared { declaration: z, stage: SpineStage::Forms },
        ] if w == "W" && x == "X" && z == "Z"
    ));
}

/// FR-503-AC-2 (TC-1298 row 2): header syntax errors are unattributable and
/// carry no protocol result payload.
#[trace("TC-1298", "FR-503-AC-2")]
#[test]
fn header_syntax_error_does_not_attach_protocol_results() {
    let complete = protocol_source("W", true);
    let unit = format!("language broken;\n{HEADER}{complete}");
    let failure = parse(
        &request(unit.as_bytes()),
        SourceLimits::default(),
        &Cancel::new(),
    )
    .expect_err("the header syntax is invalid");
    assert!(failure.payload().is_none(), "{failure:?}");
}

/// FR-503-AC-2 (TC-1298 row 2): an unbounded protocol EOF remains
/// unattributable and carries no protocol result payload.
#[trace("TC-1298", "FR-503-AC-2")]
#[test]
fn unbounded_protocol_eof_does_not_attach_protocol_results() {
    let unit = format!("{HEADER}{}", protocol_source("X", false));
    let failure = parse(
        &request(unit.as_bytes()),
        SourceLimits::default(),
        &Cancel::new(),
    )
    .expect_err("the protocol reaches EOF without its closing delimiter");
    assert!(failure.payload().is_none(), "{failure:?}");
}

/// FR-503-AC-2 (TC-1298 row 2): a bounded non-protocol refusal after the
/// only protocol still reports that protocol as cleared.
#[trace("TC-1298", "FR-503-AC-2")]
#[test]
fn bounded_nonprotocol_error_after_single_protocol_keeps_it_cleared() {
    let w = protocol_source("W", true);
    let broken_clause = "invariant Bad using Complete on M::Widget at current { true; }";
    let unit = format!("{HEADER}\n{w}\n{broken_clause}");
    let failure = parse(
        &request(unit.as_bytes()),
        SourceLimits::default(),
        &Cancel::new(),
    )
    .expect_err("the bounded state-clause syntax error refuses the unit");
    let results = failure
        .payload()
        .expect("the bounded protocol retains its reached-stage result");
    assert!(matches!(
        results.as_slice(),
        [crate::spine::ProtocolResult::Cleared {
            declaration,
            stage: SpineStage::Forms
        }] if declaration == "W"
    ));
}

/// FR-503-AC-2 (TC-1298 row 2): `protocol QualifiedName` in a function body
/// is an expression, not another top-level protocol result.
#[trace("TC-1298", "FR-503-AC-2")]
#[test]
fn protocol_expression_is_not_counted_as_a_declaration() {
    let w = protocol_source("W", true);
    let expression =
        "function references using Complete (): Boolean pure { protocol QualifiedName }";
    let broken_clause = "invariant Bad using Complete on M::Widget at current { true; }";
    let unit = format!("{HEADER}\n{w}\n{expression}\n{broken_clause}");
    let failure = parse(
        &request(unit.as_bytes()),
        SourceLimits::default(),
        &Cancel::new(),
    )
    .expect_err("the bounded state-clause syntax error refuses the unit");
    let results = failure
        .payload()
        .expect("W remains the only actual protocol declaration");
    assert!(matches!(
        results.as_slice(),
        [crate::spine::ProtocolResult::Cleared {
            declaration,
            stage: SpineStage::Forms
        }] if declaration == "W"
    ));
}

/// FR-503-AC-2 (TC-1298 row 2): a non-protocol declaration that runs to EOF
/// is unbounded and carries no protocol result payload.
#[trace("TC-1298", "FR-503-AC-2")]
#[test]
fn unbounded_nonprotocol_eof_does_not_attach_protocol_results() {
    let w = protocol_source("W", true);
    let broken_clause = "invariant Bad using Complete on M::Widget at current { true";
    let unit = format!("{HEADER}\n{w}\n{broken_clause}");
    let failure = parse(
        &request(unit.as_bytes()),
        SourceLimits::default(),
        &Cancel::new(),
    )
    .expect_err("the state clause reaches EOF without a closing delimiter");
    assert!(failure.payload().is_none(), "{failure:?}");
}

/// FR-503-AC-2 (TC-1298 row 2): a non-protocol brace error between two
/// protocols is unit-level; both bounded protocols remain Cleared.
#[trace("TC-1298", "FR-503-AC-2")]
#[test]
fn non_protocol_unterminated_state_clause_has_no_protocol_results() {
    let w = protocol_source("W", true);
    let z = protocol_source("Z", true);
    let broken_clause = "invariant Bad using Complete on M::Widget at current { true";
    let unit = format!("{HEADER}\n{w}\n{broken_clause}\n{z}");
    let failure = parse(
        &request(unit.as_bytes()),
        SourceLimits::default(),
        &Cancel::new(),
    )
    .expect_err("a state-clause brace error cannot be attributed to protocol W");

    let results = failure
        .payload()
        .expect("bounded protocols retain their reached-stage results");
    assert!(matches!(
        results.as_slice(),
        [
            crate::spine::ProtocolResult::Cleared { declaration: w, stage: SpineStage::Forms },
            crate::spine::ProtocolResult::Cleared { declaration: z, stage: SpineStage::Forms },
        ] if w == "W" && z == "Z"
    ));
}

/// FR-503-AC-1 (TC-1298 row 3): S2's recovery-aware build retains clean
/// siblings and attributes X to its own declaration in the library source.
#[trace("TC-1298", "FR-503-AC-1")]
#[test]
fn forms_refusal_is_attributed_to_its_declaration_and_source() {
    use crate::spine::ProtocolResult;
    use qsl_cst::Production;
    use qsl_forms::FormsCause;

    let root_bytes = format!(
        "{HEADER}\n{}\n{}\n{}",
        protocol_source("W", true),
        protocol_source("X", true),
        protocol_source("Z", true)
    );
    let library_bytes = format!(
        "{HEADER}\n{}\n{}\n{}",
        protocol_source("W", true),
        protocol_source("X", false),
        protocol_source("Z", true)
    );
    let cancel = Cancel::new();
    let parse_source = |location: &str, bytes: &str| {
        let source = SourceIdentity::new("agent-ix", location, "fixture", location);
        super::parse_unit(
            source,
            "unit.native",
            bytes.as_bytes(),
            SourceLimits::default(),
            &cancel,
        )
        .unwrap_or_else(|_| panic!("the source reaches the protocol-aware forms builder"))
    };
    let root = parse_source("test:lifecycle-root", &root_bytes);
    let library = parse_source("test:lifecycle-library", &library_bytes);
    assert!(root.protocol_forms_refusal.is_none());
    assert_eq!(root.unit().forms().len(), 3);
    assert_eq!(library.unit().forms().len(), 2, "W and Z forms survive");
    let forms_refusal = library
        .protocol_forms_refusal
        .as_ref()
        .expect("the recovering CST produces an S2 refusal");
    assert!(matches!(
        &forms_refusal.cause,
        FormsCause::DiagnosedSource(Code::InvalidSyntax)
    ));
    assert_eq!(forms_refusal.recoveries.len(), 1);

    let root_results = root.protocol_results();
    assert!(root_results.iter().all(|result| matches!(
        result,
        ProtocolResult::Cleared {
            stage: SpineStage::Forms,
            ..
        }
    )));

    let (_, forms_refusals) = qsl_forms::build_unit_with_protocol_results(library.syntax())
        .expect("the recovery is scoped to protocol X");
    assert_eq!(forms_refusals.len(), 1);
    assert_eq!(forms_refusals[0].declaration, "X");
    assert!(matches!(
        &forms_refusals[0].refusal.cause,
        FormsCause::DiagnosedSource(Code::InvalidSyntax)
    ));
    let declaration_span = library
        .syntax()
        .cst()
        .nodes()
        .iter()
        .filter(|node| node.production() == Production::Declaration)
        .find(|node| {
            library
                .syntax()
                .source()
                .text()
                .get(node.span().start..node.span().end)
                .is_some_and(|text| text.contains("protocol X"))
        })
        .expect("X's declaration node")
        .span();
    assert_eq!(forms_refusals[0].declaration_span, declaration_span);
    let library_results = super::forms_protocol_results(library.syntax(), &forms_refusals)
        .expect("library protocol results");
    assert!(matches!(
        library_results.as_slice(),
        [
            ProtocolResult::Cleared { declaration: w, stage: SpineStage::Forms },
            ProtocolResult::Refused {
                declaration: x,
                source_index: 1,
                locus: Some(qsl_foundation::diagnostic::Locus::Region(locus)),
                ..
            },
            ProtocolResult::Cleared { declaration: z, stage: SpineStage::Forms },
        ] if w == "W"
            && x == "X"
            && z == "Z"
            && locus.source() == library.source().reference()
            && locus.source() != root.source().reference()
            && locus.start() == declaration_span.start as u64
            && locus.end() == declaration_span.end as u64
    ));
}

/// FR-503-AC-3..6 (TC-1298 rows 3-8): S4 replaces cleared declarations with
/// emission results in place and preserves earlier scoped refusals.
#[trace("TC-1298", "FR-503-AC-3", "FR-503-AC-4", "FR-503-AC-5", "FR-503-AC-6")]
#[test]
fn emission_results_preserve_order_and_prior_refusals() {
    use crate::spine::ProtocolResult;
    use qsl_foundation::{Code, Span};

    let checked = vec![
        ProtocolResult::Cleared {
            declaration: "W".to_owned(),
            stage: SpineStage::Check,
        },
        ProtocolResult::Refused {
            declaration: "X".to_owned(),
            source_index: 0,
            declaration_span: Span { start: 10, end: 30 },
            code: Code::InvalidSyntax,
            cause: "invalid-syntax".to_owned(),
            locus: None,
        },
        ProtocolResult::Cleared {
            declaration: "Z".to_owned(),
            stage: SpineStage::Check,
        },
    ];
    let emission = vec![
        ProtocolResult::Emitted {
            declaration: "W".to_owned(),
            clause_key: "W::complete".to_owned(),
            nodes: vec![serde_json::json!({"kind": "protocol"})],
        },
        ProtocolResult::Emitted {
            declaration: "Z".to_owned(),
            clause_key: "Z::complete".to_owned(),
            nodes: vec![serde_json::json!({"kind": "protocol"})],
        },
    ];
    let merged = super::merge_emission_results(&checked, emission);
    assert!(matches!(
        merged.as_slice(),
        [
            ProtocolResult::Emitted { declaration: w, .. },
            ProtocolResult::Refused { declaration: x, .. },
            ProtocolResult::Emitted { declaration: z, .. },
        ] if w == "W" && x == "X" && z == "Z"
    ));
}

fn protocol_check_stage(
    bytes: &[u8],
    checking: CheckingLimits,
) -> Result<Staged<CheckedUnit>, FrontEndFailure> {
    let cancel = Cancel::new();
    let parsed = super::parse_for_continuation(&request(bytes), SourceLimits::default(), &cancel)
        .unwrap_or_else(|failure| panic!("protocol source parses: {failure:?}"))
        .into_value();
    let packages = qsl_semantics::model::intake::package_input([MODEL_DOCUMENT])
        .expect("fixture model package input");
    let models = select(
        &parsed,
        &packages,
        ModelNormalizationLimits::default(),
        &cancel,
    )
    .unwrap_or_else(|failure| panic!("fixture model admits: {failure:?}"))
    .into_value();
    let limits = SpineLimits {
        checking,
        ..SpineLimits::default()
    };
    super::check(
        &parsed,
        "fixture".to_owned(),
        &models,
        &DependencyInput::default(),
        limits,
        &cancel,
    )
}

fn standalone_protocol_check_code(bytes: &[u8]) -> Code {
    let cancel = Cancel::new();
    let parsed = parse(&request(bytes), SourceLimits::default(), &cancel)
        .expect("standalone protocol source parses")
        .into_value();
    let packages = qsl_semantics::model::intake::package_input([MODEL_DOCUMENT])
        .expect("fixture model package input");
    let models = select(
        &parsed,
        &packages,
        ModelNormalizationLimits::default(),
        &cancel,
    )
    .expect("fixture model admits")
    .into_value();
    let failure = check(
        &parsed,
        "fixture".to_owned(),
        &models,
        &DependencyInput::default(),
        SpineLimits::default(),
        &cancel,
    )
    .expect_err("the standalone protocol refuses");
    refusal_or_fault(failure)
        .expect("the standalone check returns a refusal")
        .code()
}

fn checked_protocol_source(bytes: &[u8]) -> CheckedUnit {
    protocol_check_stage(bytes, CheckingLimits::default())
        .unwrap_or_else(|failure| panic!("protocol source checks: {failure:?}"))
        .into_value()
}

/// FR-514-AC-1: the checked unit retains source-owned declaration type facts,
/// with no source identity copied into the report.
#[trace("FR-514-AC-1")]
#[test]
fn checked_unit_type_report_lists_its_checked_declarations() {
    use qsl_semantics::check::NodeTag;

    static SOURCE: OnceLock<SourceIdentity> = OnceLock::new();
    let source = SOURCE.get_or_init(|| {
        SourceIdentity::new("agent-ix", "test:lifecycle", "type-report", "type-report:1")
    });
    let text = format!(
        "{HEADER}profile inf = \"quire.temporal.infinite-trace/v1\";\n\
         model M = \"acme/orders\";\n\
         function g using v(x: Int[0, 9]): Int[0, 10] pure {{ x + 1 }}\n\
         function f using v(x: Int[0, 9]): Int[0, 10] pure {{ g(x) }}\n\
         post Keep using v on M::Widget::touch {{ true }}\n\
         temporal Due using inf over (input: M::Widget) clock \"model-steps\" on origin {{ always holds(true) }}\n\
         protocol Fill using Complete over (input: M::Widget) on origin {{\n\
           role R on M::Widget;\n\
           requires temporal Due;\n\
           run sequence Main {{ check After using v {{ true }}; }}\n\
           finish End as (outcome: Boolean) {{ true }};\n\
         }}"
    );
    let parsed = parse_named(source, text.as_bytes());
    let (_, packages) = widget_with_touch(&[]);
    let models = select(
        &parsed,
        &packages,
        ModelNormalizationLimits::default(),
        &Cancel::new(),
    )
    .expect("model and profile selections admit")
    .into_value();
    let checked = check(
        &parsed,
        "type-report".to_owned(),
        &models,
        &DependencyInput::default(),
        SpineLimits::default(),
        &Cancel::new(),
    )
    .expect("function, post-claim, and protocol checks")
    .into_value();
    let declarations = checked.type_report().declarations();
    assert!(declarations
        .windows(2)
        .all(|pair| pair[0].node() < pair[1].node()));
    let functions: Vec<_> = declarations
        .iter()
        .filter(|declaration| declaration.kind() == NodeTag::Function)
        .collect();
    assert_eq!(
        functions.len(),
        2,
        "both function declarations are retained"
    );
    assert!(functions
        .iter()
        .all(|declaration| declaration.value_type().is_some()));
    assert!(!checked.type_report().requirements().is_empty());
    assert!(checked
        .type_report()
        .requirements()
        .iter()
        .all(|requirement| requirement.span().start < requirement.span().end));
    assert_eq!(
        checked.type_report().requirements().len(),
        checked
            .type_report()
            .requirements()
            .iter()
            .map(|requirement| requirement.occurrence())
            .collect::<std::collections::BTreeSet<_>>()
            .len(),
        "each checked requirement occurrence has one report row"
    );
    assert!(checked
        .type_report()
        .requirements()
        .iter()
        .any(|requirement| {
            requirement.record().requirements().kind().to_wire() == "value-validity"
        }));
    let source_text = checked.source().text();
    let report_slice = |span: qsl_foundation::Span| {
        source_text
            .get(span.start..span.end)
            .expect("report spans index the source unit")
    };
    let post_start = source_text
        .find("post Keep")
        .expect("the post claim is present");
    let post_end = source_text
        .find("temporal Due")
        .expect("the temporal claim follows the post claim");
    let check_start = source_text
        .find("check After")
        .expect("the protocol check claim is present");
    let check_end = source_text
        .find("finish End")
        .expect("the protocol finish follows the check claim");
    let exact_body_span = |range: std::ops::Range<usize>| {
        let relative = source_text[range.clone()]
            .find("true")
            .expect("the claim body contains true");
        qsl_foundation::Span {
            start: range.start + relative,
            end: range.start + relative + "true".len(),
        }
    };
    let post_claim_span = exact_body_span(post_start..post_end);
    let protocol_check_span = exact_body_span(check_start..check_end);
    assert!(checked
        .type_report()
        .requirements()
        .iter()
        .filter(|requirement| {
            requirement.record().requirements().kind().to_wire() == "value-validity"
        })
        .any(|requirement| report_slice(requirement.span()) == "x + 1"));
    assert!(checked
        .type_report()
        .obligations()
        .iter()
        .any(|obligation| obligation.capability_kind() == "value-validity"));
    assert!(
        checked
            .type_report()
            .obligations()
            .iter()
            .any(|obligation| {
                obligation.capability_kind() == "operation-contract"
                    && obligation.span() == post_claim_span
            }),
        "the post claim has one operation-contract obligation at its exact span"
    );
    assert!(checked
        .type_report()
        .obligations()
        .iter()
        .all(|obligation| obligation.state() == super::ObligationState::Pending));
    let temporal_requirements: Vec<_> = checked
        .type_report()
        .obligations()
        .iter()
        .filter(|obligation| obligation.capability_kind() == "temporal-satisfaction")
        .collect();
    assert_eq!(temporal_requirements.len(), 2);
    assert!(temporal_requirements
        .iter()
        .all(|obligation| { obligation.span().start < obligation.span().end }));
    assert!(
        temporal_requirements
            .iter()
            .any(|obligation| report_slice(obligation.span()) == "Due"),
        "temporal obligation spans: {:?}",
        temporal_requirements
            .iter()
            .map(|obligation| report_slice(obligation.span()))
            .collect::<Vec<_>>()
    );
    let true_spans: Vec<_> = checked
        .type_report()
        .obligations()
        .iter()
        .filter(|obligation| report_slice(obligation.span()) == "true")
        .map(|obligation| obligation.span())
        .collect();
    assert!(
        true_spans.iter().any(|span| {
            (post_start..post_end).contains(&span.start)
                && (post_start..post_end).contains(&span.end)
        }),
        "the post claim's pending obligation is located in its own body"
    );
    assert!(
        true_spans.iter().any(|span| {
            (check_start..check_end).contains(&span.start)
                && (check_start..check_end).contains(&span.end)
        }),
        "the protocol check's pending obligation is located in its own body"
    );
    let global_conformance: Vec<_> = checked
        .type_report()
        .obligations()
        .iter()
        .filter(|obligation| obligation.capability_kind() == "global-conformance")
        .collect();
    assert_eq!(global_conformance.len(), 1);
    assert_eq!(global_conformance[0].span(), protocol_check_span);
}

/// FR-480-AC-2/AC-8 and FR-514-AC-2: aggregate type resolution returns two
/// checked units sharing the package graph, each with only its own report.
#[trace("FR-480-AC-2", "FR-480-AC-8", "FR-514-AC-2")]
#[test]
fn aggregate_check_returns_source_bound_units_and_reports() {
    use qsl_semantics::check::NodeTag;

    static SOURCE_A: OnceLock<SourceIdentity> = OnceLock::new();
    static SOURCE_B: OnceLock<SourceIdentity> = OnceLock::new();
    let source_a = SOURCE_A.get_or_init(|| {
        SourceIdentity::new("agent-ix", "test:lifecycle:unit-a", "unit-a", "unit-a:1")
    });
    let source_b = SOURCE_B.get_or_init(|| {
        SourceIdentity::new("agent-ix", "test:lifecycle:unit-b", "unit-b", "unit-b:1")
    });
    let a = parse_named(
        source_a,
        format!("{HEADER}record Point {{ x: Integer; }}").as_bytes(),
    );
    let b = parse_named(
        source_b,
        format!("{HEADER}function px using v(p: Point): Integer pure {{ p.x + 1 }}").as_bytes(),
    );
    let dependencies = DependencyInput::default();
    let admitted = admit_parsed_unit_list(vec![b, a], &dependencies)
        .expect("the distinct source units are admitted");
    let checked = check_units(
        &admitted,
        "aggregate-package".to_owned(),
        &AdmittedModels::default(),
        &dependencies,
        SpineLimits::default(),
        &Cancel::new(),
    )
    .expect("cross-unit type references check")
    .into_value();
    let [checked_a, checked_b] = checked.as_slice() else {
        panic!("one checked unit per canonical source is returned");
    };
    assert_eq!(checked_a.source().identity(), source_a);
    assert_eq!(checked_b.source().identity(), source_b);
    assert!(Arc::ptr_eq(
        checked_a.shared_package(),
        checked_b.shared_package()
    ));
    assert_eq!(checked_a.type_report().declarations().len(), 1);
    assert_eq!(checked_b.type_report().declarations().len(), 1);
    assert!(checked_a.type_report().requirements().is_empty());
    assert!(!checked_b.type_report().requirements().is_empty());
    assert!(checked_b
        .type_report()
        .requirements()
        .iter()
        .all(|requirement| requirement.span().start < requirement.span().end));
    assert_eq!(
        checked_b.type_report().declarations()[0].kind(),
        NodeTag::Function
    );
}

/// FR-480-AC-4: each source unit's admitted profile alias remains available
/// while the aggregate namespace checks declarations from later units.
#[trace("FR-480-AC-4")]
#[test]
fn aggregate_check_resolves_profiles_selected_under_later_unit_aliases() {
    static SOURCE_A: OnceLock<SourceIdentity> = OnceLock::new();
    static SOURCE_B: OnceLock<SourceIdentity> = OnceLock::new();
    let source_a = SOURCE_A.get_or_init(|| {
        SourceIdentity::new(
            "agent-ix",
            "test:lifecycle:profile-a",
            "profile-a",
            "profile-a:1",
        )
    });
    let source_b = SOURCE_B.get_or_init(|| {
        SourceIdentity::new(
            "agent-ix",
            "test:lifecycle:profile-b",
            "profile-b",
            "profile-b:1",
        )
    });
    let a = parse_named(
        source_a,
        b"language \"ix:native\" edition \"1-draft\";\nprofile v = \"quire.value.complete/v1\";\nfunction from_v using v(x: Int[0, 9]): Boolean pure { x < 5 }",
    );
    let b = parse_named(
        source_b,
        b"language \"ix:native\" edition \"1-draft\";\nprofile w = \"quire.value.complete/v1\";\nfunction from_w using w(x: Int[0, 9]): Boolean pure { x < 5 }",
    );
    let dependencies = DependencyInput::default();
    let admitted = admit_parsed_unit_list(vec![b, a], &dependencies)
        .expect("distinct source units are admitted");
    let checked = check_units(
        &admitted,
        "aggregate-profile-package".to_owned(),
        &AdmittedModels::default(),
        &dependencies,
        SpineLimits::default(),
        &Cancel::new(),
    )
    .expect("both source-local profile aliases resolve")
    .into_value();
    assert_eq!(checked.len(), 2);
    assert!(checked
        .iter()
        .all(|unit| unit.type_report().declarations().len() == 1));
}

/// FR-480-AC-1: I1 has a public composition path for all model selections in
/// an admitted list, preserving every unit's alias and admitting the shared
/// identity once through the aggregate meter.
#[trace("FR-480-AC-1")]
#[test]
fn aggregate_model_selection_admits_selections_from_every_unit() {
    static SOURCE_A: OnceLock<SourceIdentity> = OnceLock::new();
    static SOURCE_B: OnceLock<SourceIdentity> = OnceLock::new();
    let source_a = SOURCE_A.get_or_init(|| {
        SourceIdentity::new("agent-ix", "test:lifecycle:model-a", "model-a", "model-a:1")
    });
    let source_b = SOURCE_B.get_or_init(|| {
        SourceIdentity::new("agent-ix", "test:lifecycle:model-b", "model-b", "model-b:1")
    });
    let a = parse_named(
        source_a,
        format!(
            "{HEADER}model A = \"acme/orders\";\n\
             function fa using v(x: Int[0, 9]): Boolean pure {{ x > 0 }}"
        )
        .as_bytes(),
    );
    let b = parse_named(
        source_b,
        format!(
            "{HEADER}model B = \"acme/orders\";\n\
             function fb using v(x: Int[0, 9]): Boolean pure {{ x > 0 }}"
        )
        .as_bytes(),
    );
    let dependencies = DependencyInput::default();
    let admitted = admit_parsed_unit_list(vec![b, a], &dependencies)
        .expect("distinct source units are admitted");
    let packages = qsl_semantics::model::intake::package_input([MODEL_DOCUMENT])
        .expect("fixture model package input");
    let selected = select_units(
        &admitted,
        &packages,
        ModelNormalizationLimits::default(),
        SourceLimits::default(),
        &Cancel::new(),
    )
    .expect("the aggregate model selections are admitted")
    .into_value();
    assert_eq!(selected.models().len(), 2);
    assert_eq!(
        selected
            .models()
            .iter()
            .map(|model| model.alias.as_str())
            .collect::<Vec<_>>(),
        ["A", "B"]
    );
}

/// FR-480-AC-8: E4 accepts a clock selection whose source is a non-first
/// root unit in the checked package and routes it to emission.
#[trace("FR-480-AC-8")]
#[test]
fn aggregate_package_accepts_temporal_selection_from_later_source() {
    use qsl_forms::DeclarationForm;
    use qsl_package::{ClockConfiguration, TemporalClockSelection};

    static SOURCE_A: OnceLock<SourceIdentity> = OnceLock::new();
    static SOURCE_B: OnceLock<SourceIdentity> = OnceLock::new();
    let source_a = SOURCE_A.get_or_init(|| {
        SourceIdentity::new(
            "agent-ix",
            "test:lifecycle:temporal-a",
            "temporal-a",
            "temporal-a:1",
        )
    });
    let source_b = SOURCE_B.get_or_init(|| {
        SourceIdentity::new(
            "agent-ix",
            "test:lifecycle:temporal-b",
            "temporal-b",
            "temporal-b:1",
        )
    });
    let header =
        format!("{HEADER}profile ev = \"quire.temporal.event-position.false-extension/v1\";\n");
    let a_text = format!(
        "{header}model M = \"acme/orders\";\n\
         record Point {{ x: Int[0, 9]; }}\n\
         temporal Root using ev over (input: M::Widget) clock \"events\" on origin {{ eventually[0,1] holds(true) }}"
    );
    let a = parse_named(source_a, a_text.as_bytes());
    let b_text = format!(
        "{header}temporal Due using ev over (input: M::Widget) clock \"events\" on origin {{ eventually[0,1] holds(true) }}"
    );
    let b = parse_named(source_b, b_text.as_bytes());
    let temporal_span = b
        .unit()
        .forms()
        .iter()
        .find(|form| matches!(form.form(), DeclarationForm::Temporal(_)))
        .expect("the later source contains Due")
        .span();
    let first_temporal_span = a
        .unit()
        .forms()
        .iter()
        .find(|form| matches!(form.form(), DeclarationForm::Temporal(_)))
        .expect("the first source contains Root")
        .span();
    let dependencies = DependencyInput::default();
    let admitted = admit_parsed_unit_list(vec![b, a], &dependencies)
        .expect("the distinct source units are admitted");
    let packages = qsl_semantics::model::intake::package_input([MODEL_DOCUMENT])
        .expect("fixture model package input");
    let models = select_units(
        &admitted,
        &packages,
        ModelNormalizationLimits::default(),
        SourceLimits::default(),
        &Cancel::new(),
    )
    .expect("the later source's model is admitted")
    .into_value();
    let checked = check_units(
        &admitted,
        "aggregate-temporal-package".to_owned(),
        &models,
        &dependencies,
        SpineLimits::default(),
        &Cancel::new(),
    )
    .expect("the aggregate temporal clause checks")
    .into_value();
    assert_eq!(checked[0].source().identity(), source_a);
    let first_selection = TemporalClockSelection {
        source: checked[0].source().reference().clone(),
        declaration_span: first_temporal_span,
        clock: ClockConfiguration::EventPosition {
            sequence_authority: "aggregate-events".into(),
        },
    };
    let selection = TemporalClockSelection {
        source: checked[1].source().reference().clone(),
        declaration_span: temporal_span,
        clock: ClockConfiguration::EventPosition {
            sequence_authority: "aggregate-events".into(),
        },
    };
    let emitted = package_with_temporal_selections(
        &checked[0],
        &[first_selection, selection],
        PackageLimits::default(),
        &Cancel::new(),
    )
    .expect("a selection for a later root source is not surplus")
    .into_value();
    assert!(!emitted.package().bytes().is_empty());

    let uncompiled = TemporalClockSelection {
        source: qsl_foundation::source::provenance::RawSourceRef::new(
            "test:lifecycle:uncompiled",
            "uncompiled:1",
        )
        .expect("the uncompiled source reference is valid"),
        declaration_span: temporal_span,
        clock: ClockConfiguration::EventPosition {
            sequence_authority: "aggregate-events".into(),
        },
    };
    let refusal = package_with_temporal_selections(
        &checked[0],
        &[uncompiled],
        PackageLimits::default(),
        &Cancel::new(),
    )
    .expect_err("a selection for a source outside the checked package is refused");
    let refusal = refusal_or_fault(refusal).expect("the surplus selection is a refusal");
    assert!(matches!(
        &*refusal,
        CompileRefusal::Emit(qsl_package::EmitRefusal::SurplusTemporalClockSelection { .. })
    ));
}

/// FR-503-AC-3 (TC-1298 row 4): the compile continuation emits clean
/// siblings byte-for-byte and returns the ordered refusal payload without a
/// package.
#[trace("TC-1298", "FR-503-AC-3")]
#[test]
fn syntax_refusal_does_not_suppress_clean_protocol_emission() {
    use crate::spine::ProtocolResult;
    use qsl_package::{emit_checked_with_protocol_results, ProtocolEmissionResult};

    let source = |protocols: &str| format!("{HEADER}model M = \"acme/orders\";\n{protocols}");
    let standalone_nodes = |name: &str| {
        let checked = checked_protocol_source(source(&protocol_source(name, true)).as_bytes());
        let emission = emit_checked_with_protocol_results(&checked.package)
            .expect("a standalone protocol emits");
        let [ProtocolEmissionResult::Emitted { nodes, .. }] = emission.results() else {
            panic!("one standalone protocol emits: {:?}", emission.results());
        };
        nodes.clone()
    };
    let w_nodes = standalone_nodes("W");
    let z_nodes = standalone_nodes("Z");

    let unit = source(&format!(
        "{}\n{}\n{}",
        protocol_source("W", true),
        protocol_source("X", false),
        protocol_source("Z", true)
    ));
    let failure = crate::compile::compile_package(
        crate::compile::CompileUnit {
            source: identity(),
            package_name: "fixture".to_owned(),
            path: "unit.native",
            bytes: unit.as_bytes(),
        },
        [MODEL_DOCUMENT],
        &DependencyInput::default(),
        StageLimits::default(),
        ReplayLimits::default(),
    )
    .expect_err("X's syntax refusal prevents a unit package");
    assert_eq!(failure.code(), Code::InvalidSyntax);
    let results = failure
        .protocol_results()
        .expect("compile carries per-protocol results");
    let [ProtocolResult::Emitted {
        declaration: w,
        nodes: emitted_w,
        ..
    }, ProtocolResult::Refused { declaration: x, .. }, ProtocolResult::Emitted {
        declaration: z,
        nodes: emitted_z,
        ..
    }] = results
    else {
        panic!("ordered W/X/Z results: {results:?}");
    };
    assert_eq!(w, "W");
    assert_eq!(x, "X");
    assert_eq!(z, "Z");
    assert_eq!(emitted_w, &w_nodes);
    assert_eq!(emitted_z, &z_nodes);
}

/// FR-510-AC-5: continuation retains a graph for the clean sibling when one
/// protocol refuses, but exposes neither a subject nor partial references.
#[trace("FR-510-AC-5", "TC-1305")]
#[test]
fn a_continued_protocol_refusal_has_no_subject_or_partial_reference_rows() {
    let unit = format!(
        "{HEADER}model M = \"acme/orders\";\n\
         function base using v(x: Int[0, 9]): Int[0, 10] pure {{ x + 1 }}\n\
         function answer using v(x: Int[0, 9]): Int[0, 10] pure {{ base(x) }}\n\
         {}\n\
         {}",
        protocol_source("W", true),
        protocol_source("X", true).replace("M::Widget", "M::Absent")
    );
    let cancel = Cancel::new();
    let parsed =
        super::parse_for_continuation(&request(unit.as_bytes()), SourceLimits::default(), &cancel)
            .expect("the attributed protocol refusal is continuable")
            .into_value();
    let packages = qsl_semantics::model::intake::package_input([MODEL_DOCUMENT])
        .expect("the fixture model package input");
    let models = select(
        &parsed,
        &packages,
        ModelNormalizationLimits::default(),
        &cancel,
    )
    .expect("the fixture model admits")
    .into_value();
    let checked = super::check_for_continuation(
        &parsed,
        super::ContinuationInputs {
            package_name: "fixture".to_owned(),
            execution_points: &BTreeMap::new(),
            temporal_selections: &[],
        },
        &models,
        &DependencyInput::default(),
        SpineLimits::default(),
        &cancel,
    )
    .expect("the compiler retains the clean sibling graph")
    .into_value();
    assert!(checked
        .protocol_results()
        .iter()
        .any(|result| matches!(result, super::super::ProtocolResult::Refused { declaration, .. } if declaration == "X")));
    assert!(!checked.package().graph().protocol_refusals().is_empty());
    assert!(checked.static_subject().is_err());
    assert!(checked.package().graph().dependency_references().is_empty());
}

/// FR-503-AC-4..5 (TC-1298 rows 5-6): an S3 refusal is attached to the
/// declaration and source index the checker owns, while clean siblings pass.
#[trace("TC-1298", "FR-503-AC-4", "FR-503-AC-5")]
#[test]
fn check_retains_the_owning_protocol_locus() {
    use crate::spine::ProtocolResult;

    let broken = protocol_source("X", true).replace("M::Widget", "M::Absent");
    let standalone_unit = format!("{HEADER}model M = \"acme/orders\";\n{broken}");
    let standalone_code = standalone_protocol_check_code(standalone_unit.as_bytes());
    let unit = format!(
        "{HEADER}model M = \"acme/orders\";\n{}\n{}",
        protocol_source("W", true),
        broken
    );
    let cancel = Cancel::new();
    let parsed = parse(&request(unit.as_bytes()), SourceLimits::default(), &cancel)
        .expect("the unit syntax is valid")
        .into_value();
    let packages = qsl_semantics::model::intake::package_input([MODEL_DOCUMENT])
        .expect("fixture model package input");
    let models = select(
        &parsed,
        &packages,
        ModelNormalizationLimits::default(),
        &cancel,
    )
    .expect("fixture model admits")
    .into_value();
    let failure = check(
        &parsed,
        "fixture".to_owned(),
        &models,
        &DependencyInput::default(),
        SpineLimits::default(),
        &cancel,
    )
    .expect_err("X's missing type refuses check");
    let x_start = unit.find("protocol X").expect("X's declaration") + "protocol ".len();
    let StageFailure::RefusedWithPayload(refusal, results) = failure else {
        panic!("check retains protocol results: {failure:?}");
    };
    assert_eq!(refusal.code(), standalone_code);
    let with_payload_code = refusal.code();
    let with_payload_category = with_payload_code.category();
    let with_payload_exit = with_payload_category.exit_code();
    assert!(matches!(
        results.as_slice(),
        [
            ProtocolResult::Cleared {
                declaration: w,
                stage: SpineStage::Check
            },
            ProtocolResult::Refused {
                declaration: x,
                source_index: 1,
                declaration_span,
                locus: Some(qsl_foundation::diagnostic::Locus::Region(locus)),
                ..
            }
        ] if w == "W"
            && x == "X"
            && declaration_span.start == x_start
            && declaration_span.end == x_start + 1
            && locus.source() == parsed.source().reference()
            && locus.start() == x_start as u64
            && locus.end() == (x_start + 1) as u64
    ));
    let without_payload = crate::ReplayRefusal::Recompile(*refusal);
    assert_eq!(with_payload_category, without_payload.code().category());
    assert_eq!(
        with_payload_exit,
        without_payload.code().category().exit_code()
    );
}

/// FR-503-AC-5 (TC-1298 row 7): the shared S3 budget leaves the completed
/// first protocol cleared and the declarations it cannot reach incomplete.
#[trace("TC-1298", "FR-503-AC-5")]
#[test]
fn check_limit_retains_completed_and_incomplete_protocol_results() {
    use crate::spine::ProtocolResult;

    let unit = |protocols: &str| {
        format!(
            "{HEADER}model M = \"acme/orders\";\n\
             function baseline using v(): Boolean pure {{ true }}\n{protocols}"
        )
    };
    let no_protocols = protocol_check_stage(unit("").as_bytes(), CheckingLimits::default())
        .expect("the no-protocol unit checks");
    let base = no_protocols.work().s3;
    let one = protocol_check_stage(
        unit(&protocol_source("W", true)).as_bytes(),
        CheckingLimits::default(),
    )
    .expect("one protocol checks");
    let per_protocol = one.work().s3 - base;
    assert!(per_protocol > 0);

    let three = format!(
        "{}\n{}\n{}",
        protocol_source("W", true),
        protocol_source("Z", true),
        protocol_source("V", true)
    );
    let budget = base + per_protocol;
    let failure = protocol_check_stage(
        unit(&three).as_bytes(),
        CheckingLimits::default().with_work_budget(budget),
    )
    .expect_err("the second and third protocols exhaust the shared S3 budget");
    let StageFailure::LimitWithPayload(limit, results) = failure else {
        panic!("expected S3 limit with protocol results: {failure:?}");
    };
    assert_eq!(limit.setting(), Setting::S3WorkUnits);
    assert!(matches!(
        results.as_slice(),
        [
            ProtocolResult::Cleared { declaration: first, stage: SpineStage::Check },
            ProtocolResult::Incomplete { declaration: second, limit: second_limit },
            ProtocolResult::Incomplete { declaration: third, limit: third_limit },
        ] if first == "W"
            && second == "Z"
            && third == "V"
            && second_limit.setting() == Setting::S3WorkUnits
            && third_limit.setting() == Setting::S3WorkUnits
    ));
    assert_eq!(Code::StageLimitExceeded.category(), Category::Incomplete);
}

/// FR-503-AC-6 (TC-1298 row 8): a separate state-clause refusal keeps W
/// cleared through check and its payload does not change category or exit.
#[trace("TC-1298", "FR-503-AC-6")]
#[test]
fn state_clause_refusal_keeps_protocol_result_cleared() {
    use crate::spine::ProtocolResult;

    let unit = format!(
        "{HEADER}model M = \"acme/orders\";\n{}\n\
         invariant Bad using v on M::Widget at current {{ self.absent }}\n",
        protocol_source("W", true)
    );
    let cancel = Cancel::new();
    let parsed = parse(&request(unit.as_bytes()), SourceLimits::default(), &cancel)
        .expect("the unit syntax is valid")
        .into_value();
    let packages = qsl_semantics::model::intake::package_input([MODEL_DOCUMENT])
        .expect("fixture model package input");
    let models = select(
        &parsed,
        &packages,
        ModelNormalizationLimits::default(),
        &cancel,
    )
    .expect("fixture model admits")
    .into_value();
    let failure = check(
        &parsed,
        "fixture".to_owned(),
        &models,
        &DependencyInput::default(),
        SpineLimits::default(),
        &cancel,
    )
    .expect_err("the state clause's missing member refuses check");
    let StageFailure::RefusedWithPayload(refusal, results) = failure else {
        panic!("state-clause refusal keeps protocol results: {failure:?}");
    };
    let expected_code = refusal.code();
    let with_payload_category = expected_code.category();
    let with_payload_exit = with_payload_category.exit_code();
    assert_eq!(with_payload_category, Category::Refusal);
    assert_eq!(with_payload_exit, 20);
    assert!(matches!(
        results.as_slice(),
        [ProtocolResult::Cleared { declaration, stage: SpineStage::Check }]
            if declaration == "W"
    ));
    let without_payload = crate::ReplayRefusal::Recompile(*refusal);
    assert_eq!(with_payload_category, without_payload.code().category());
    assert_eq!(
        with_payload_exit,
        without_payload.code().category().exit_code()
    );

    let compile_failure = crate::compile::compile_package(
        crate::compile::CompileUnit {
            source: identity(),
            package_name: "fixture".to_owned(),
            path: "unit.native",
            bytes: unit.as_bytes(),
        },
        [MODEL_DOCUMENT],
        &DependencyInput::default(),
        StageLimits::default(),
        ReplayLimits::default(),
    )
    .expect_err("the state clause prevents a unit package");
    assert_eq!(compile_failure.code(), expected_code);
    assert!(matches!(
        compile_failure.protocol_results(),
        Some([ProtocolResult::Emitted { declaration, .. }]) if declaration == "W"
    ));
}

/// The one library the import tests supply: `x < 5` over a digit.
fn geometry() -> SuppliedLibrary {
    SuppliedLibrary {
        identity: "test/geometry".to_owned(),
        source: SourceIdentity::new("a", "geometry", "git", "1"),
        path: "geometry.native".to_owned(),
        bytes: format!("{HEADER}function f using v(x: Int[0, 9]): Boolean pure {{ x < 5 }}\n")
            .into_bytes(),
    }
}

/// Every operation's output over one source, from a live handle.
struct Chain {
    parsed: ParsedUnitSource,
    models: AdmittedModels,
    checked: CheckedUnit,
    emitted: EmittedUnit,
}

fn chain_of(
    bytes: &[u8],
    packages: &qsl_semantics::model::intake::PackageInput,
    dependencies: &DependencyInput,
    limits: SpineLimits,
) -> Result<Chain, FrontEndFailure> {
    let cancel = Cancel::new();
    let parsed = parse(&request(bytes), limits.source, &cancel)?.into_value();
    let models = select(&parsed, packages, limits.model, &cancel)?.into_value();
    let checked = check(
        &parsed,
        "fixture".to_owned(),
        &models,
        dependencies,
        limits,
        &cancel,
    )?
    .into_value();
    let emitted = package(&checked, PackageLimits::default(), &cancel)?.into_value();
    Ok(Chain {
        parsed,
        models,
        checked,
        emitted,
    })
}

fn fixture_chain() -> Chain {
    chain_of(
        FIXTURE.as_bytes(),
        &BTreeMap::new(),
        &DependencyInput::default(),
        SpineLimits::default(),
    )
    .unwrap_or_else(|failure| panic!("the fixture compiles: {failure:?}"))
}

fn refusal(failure: FrontEndFailure) -> Box<CompileRefusal> {
    match failure {
        StageFailure::Refused(refusal) => refusal,
        other => panic!("expected a refusal, got {other:?}"),
    }
}

fn seven() -> QualifiedName {
    QualifiedName::unqualified("seven").expect("an identifier")
}

fn call_seven(
    checked: &CheckedUnit,
    accounting: quire_exact::ScalarLimits,
    cancel: &Cancel,
) -> Result<quire_eval::Evaluation, CallFailure> {
    let function = seven();
    execute(
        checked,
        &ExecuteRequest {
            function: &function,
            arguments: &[],
            objects: &ObjectEnvironment::default(),
            supplied: SuppliedLimits::default(),
        },
        accounting,
        cancel,
    )
}

/// FR-275-AC-1 (TC-755 steps 1 to 3): `parse`, `select`, `check` and
/// `package` over the compile fixture each return a `Staged` value, and
/// `execute` of `seven` over the checked package completes with the integer
/// 7.
#[trace("TC-755", "FR-275-AC-1")]
#[test]
fn the_chain_over_the_compile_fixture_returns_a_package_that_executes() {
    let chain = fixture_chain();
    let wire: serde_json::Value =
        serde_json::from_slice(chain.emitted.package().bytes()).expect("the wire is JSON");
    assert_eq!(wire["contract_version"], "quire.checked-package/v2");
    assert_eq!(chain.models.models().len(), 0);

    let evaluation = call_seven(
        &chain.checked,
        default_accounting(DEFAULT_WORK_UNITS),
        &Cancel::new(),
    )
    .expect("seven admits no argument and takes no object");
    match evaluation.outcome {
        FamilyOutcome::Evaluated(Outcome::Completed(Value::Integer(value))) => {
            assert_eq!(value, Integer::from(7_i64));
        }
        other => panic!("expected the integer 7, got {other:?}"),
    }
}

/// FR-275-AC-2 (TC-755 step 5): the operations take their predecessor's own
/// type. Each `compile_fail` doctest of this module's header passes a value
/// of another stage and each has a compiling control; this test is the
/// control for the runtime half, that the correct types compose.
#[trace("TC-755", "FR-275-AC-2")]
#[test]
fn each_operation_takes_exactly_its_predecessors_output() {
    let chain = fixture_chain();
    let cancel = Cancel::new();
    // `check` takes the `ParsedUnitSource` and `AdmittedModels` of one source,
    // `package` the `CheckedUnit` of that check, and `execute` the same.
    let rechecked = check(
        &chain.parsed,
        "fixture".to_owned(),
        &chain.models,
        &DependencyInput::default(),
        SpineLimits::default(),
        &cancel,
    )
    .expect("the parsed source checks again")
    .into_value();
    let reemitted = package(&rechecked, PackageLimits::default(), &cancel)
        .expect("the checked package emits again")
        .into_value();
    assert_eq!(reemitted.package(), chain.emitted.package());
}

/// FR-275-AC-3 (TC-755 step 6): each operation, called twice with equal
/// requests and limits, returns equal outcomes.
#[trace("TC-755", "FR-275-AC-3")]
#[test]
fn equal_requests_return_equal_outcomes() {
    let first = fixture_chain();
    let second = fixture_chain();
    assert_eq!(
        first.parsed.source().reference(),
        second.parsed.source().reference()
    );
    assert_eq!(
        first.parsed.syntax().diagnostics(),
        second.parsed.syntax().diagnostics()
    );
    assert_eq!(first.models.models().len(), second.models.models().len());
    assert_eq!(
        first.checked.source().reference(),
        second.checked.source().reference()
    );
    assert_eq!(first.emitted.package(), second.emitted.package());
    assert_eq!(references(&first.emitted), references(&second.emitted));

    let outcome = |chain: &Chain| {
        call_seven(
            &chain.checked,
            default_accounting(DEFAULT_WORK_UNITS),
            &Cancel::new(),
        )
        .map(|evaluation| format!("{:?}", evaluation.outcome))
    };
    assert_eq!(outcome(&first), outcome(&second));

    let broken = b"function";
    let refusals = |bytes: &[u8]| {
        let failure = parse(
            &request(bytes),
            SpineLimits::default().source,
            &Cancel::new(),
        )
        .expect_err("a truncated unit does not parse");
        let refusal = refusal(failure);
        (refusal.code(), refusal.stage(), refusal.region().cloned())
    };
    assert_eq!(refusals(broken), refusals(broken));
}

/// A deterministic byte generator: the same seed gives the same bytes, so a
/// failing input reproduces.
struct Bytes(u64);

impl Bytes {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }

    fn below(&mut self, bound: usize) -> usize {
        usize::try_from(self.next() % u64::try_from(bound).expect("a small bound"))
            .expect("below the bound")
    }
}

/// FR-275-AC-4 (TC-756): arbitrary source bytes, and arbitrary edits of a
/// valid unit, make `parse`, `select`, `check` and `package` each return a
/// typed outcome and never panic.
#[trace("TC-756", "FR-275-AC-4")]
#[test]
fn arbitrary_source_bytes_never_panic_an_operation() {
    let mut generator = Bytes(0x9e37_79b9_7f4a_7c15);
    let valid = FIXTURE.as_bytes();
    let mut checked_units = 0_u32;
    for round in 0..600 {
        let mut bytes = valid.to_vec();
        match round % 3 {
            // Random bytes.
            0 => {
                bytes = (0..generator.below(200))
                    .map(|_| u8::try_from(generator.below(256)).expect("a byte"))
                    .collect();
            }
            // A few overwritten bytes of the valid unit.
            1 => {
                for _ in 0..=generator.below(4) {
                    let at = generator.below(bytes.len());
                    bytes[at] = u8::try_from(generator.below(256)).expect("a byte");
                }
            }
            // The valid unit cut short.
            _ => bytes.truncate(generator.below(bytes.len())),
        }
        if let Ok(chain) = chain_of(
            &bytes,
            &BTreeMap::new(),
            &DependencyInput::default(),
            SpineLimits::default(),
        ) {
            checked_units += 1;
            assert!(!chain.emitted.package().bytes().is_empty());
        }
    }
    // The valid unit's own edit-free rounds are among those that check.
    assert!(checked_units > 0, "some generated unit must compile");
}

/// A handle whose observer counts every charge made under it and, at charge
/// `at`, holds that charge until another thread has cancelled the handle. The
/// cancellation therefore lands at exactly one charge, whatever the
/// scheduler does.
struct Gate {
    cancel: Cancel,
    charges: Arc<AtomicU64>,
    after_cancel: Arc<AtomicU64>,
    reached: Arc<AtomicBool>,
    cancelled: Arc<AtomicBool>,
    finished: Arc<AtomicBool>,
}

/// A handle that holds charge `at`, or only counts when `at` is `None`.
fn gated(at: Option<u64>) -> Gate {
    let charges = Arc::new(AtomicU64::new(0));
    let after_cancel = Arc::new(AtomicU64::new(0));
    let reached = Arc::new(AtomicBool::new(false));
    let cancelled = Arc::new(AtomicBool::new(false));
    let finished = Arc::new(AtomicBool::new(false));
    let cancel = {
        let (charges, after_cancel, reached, cancelled) = (
            Arc::clone(&charges),
            Arc::clone(&after_cancel),
            Arc::clone(&reached),
            Arc::clone(&cancelled),
        );
        Cancel::observing(move || {
            let charge = charges.fetch_add(1, Ordering::SeqCst) + 1;
            if Some(charge) == at {
                reached.store(true, Ordering::SeqCst);
                while !cancelled.load(Ordering::SeqCst) {
                    std::thread::yield_now();
                }
            }
            if cancelled.load(Ordering::SeqCst) {
                after_cancel.fetch_add(1, Ordering::SeqCst);
            }
        })
    };
    Gate {
        cancel,
        charges,
        after_cancel,
        reached,
        cancelled,
        finished,
    }
}

impl Gate {
    /// A second thread that cancels the handle with `cause` once a charge
    /// holds at the gate. It returns whether it did: a run that ended before
    /// reaching the gate returns `false`.
    fn canceller(&self, cause: CancelCause) -> std::thread::JoinHandle<bool> {
        let cancel = self.cancel.clone();
        let (reached, cancelled, finished) = (
            Arc::clone(&self.reached),
            Arc::clone(&self.cancelled),
            Arc::clone(&self.finished),
        );
        std::thread::spawn(move || {
            while !reached.load(Ordering::SeqCst) {
                if finished.load(Ordering::SeqCst) {
                    return false;
                }
                std::thread::yield_now();
            }
            cancel.cancel(cause);
            cancelled.store(true, Ordering::SeqCst);
            true
        })
    }

    /// Run `operation` over this handle with a canceller thread, and return
    /// its result once the canceller has finished.
    fn run<T>(&self, cause: CancelCause, operation: impl FnOnce(&Cancel) -> T) -> T {
        let canceller = self.canceller(cause);
        let outcome = operation(&self.cancel);
        self.finished.store(true, Ordering::SeqCst);
        assert!(
            canceller.join().expect("the canceller finishes"),
            "the operation ended before charge {}",
            self.charges.load(Ordering::SeqCst)
        );
        outcome
    }

    /// The operation stopped at the held charge: that charge was the first
    /// and only one to see the cancellation.
    fn assert_stopped_at(&self, at: u64) {
        assert_eq!(self.charges.load(Ordering::SeqCst), at, "charges made");
        assert_eq!(
            self.after_cancel.load(Ordering::SeqCst),
            1,
            "charges made after the cancellation"
        );
    }
}

/// How many charges `operation` makes over a handle nobody cancels.
fn charges_of<T>(operation: impl FnOnce(&Cancel) -> T) -> u64 {
    let gate = gated(None);
    let _ = operation(&gate.cancel);
    gate.charges.load(Ordering::SeqCst)
}

/// FR-276-AC-1 (TC-757 step 1): each operation, called with a handle already
/// cancelled with `Requested`, returns its cancelled failure and no output.
#[trace("TC-757", "FR-276-AC-1")]
#[test]
fn a_handle_cancelled_before_the_call_stops_every_operation() {
    let chain = fixture_chain();
    let cancelled = Cancel::new();
    cancelled.cancel(CancelCause::Requested);
    let limits = SpineLimits::default();

    assert_requested(parse(
        &request(FIXTURE.as_bytes()),
        limits.source,
        &cancelled,
    ));
    assert_requested(select(
        &chain.parsed,
        &BTreeMap::new(),
        limits.model,
        &cancelled,
    ));
    assert_requested(check(
        &chain.parsed,
        "fixture".to_owned(),
        &chain.models,
        &DependencyInput::default(),
        limits,
        &cancelled,
    ));
    assert_requested(package(
        &chain.checked,
        PackageLimits::default(),
        &cancelled,
    ));
    assert_eq!(
        call_seven(
            &chain.checked,
            default_accounting(DEFAULT_WORK_UNITS),
            &cancelled
        )
        .unwrap_err(),
        CallFailure::Cancelled(CancelCause::Requested)
    );
}

/// `outcome` is `Cancelled(Requested)` and carries no output.
fn assert_requested<T>(outcome: Result<Staged<T>, FrontEndFailure>) {
    match outcome {
        Err(StageFailure::Cancelled(CancelCause::Requested)) => {}
        Err(other) => panic!("expected Cancelled(Requested), got {other:?}"),
        Ok(_) => panic!("a cancelled operation returned an output"),
    }
}

/// The text of each source `emitted` carries, in order.
fn references(emitted: &EmittedUnit) -> Vec<Vec<u8>> {
    emitted
        .sources()
        .iter()
        .map(|source| source.text().as_bytes().to_vec())
        .collect()
}

/// A source of `count` independent declarations, each `Integer`-valued.
fn declarations(count: usize) -> String {
    let mut source = String::from(HEADER);
    for index in 0..count {
        writeln!(source, "function f{index} using v(): Integer pure {{ 7 }}")
            .expect("a string write");
    }
    source
}

/// Limits raised to fit a source of `bytes`, so no default ceiling stops it.
fn wide(bytes: usize) -> SpineLimits {
    let mut limits = SpineLimits::default();
    limits.source.source_bytes = bytes;
    limits.source.tokens = usize::MAX / 2;
    limits.source.nodes = usize::MAX / 2;
    limits.checking = quire_semantic_value::checking::CheckingLimits::new(u64::MAX)
        .with_input_bytes(u64::MAX)
        .with_work_budget(u64::MAX);
    limits
}

/// Declarations in the cancelled-check test, as the AC names them.
const DECLARATIONS: usize = 200_000;

/// The unit of `count` declarations, parsed and with its models selected,
/// under limits raised to fit it.
fn parsed_declarations(count: usize) -> (ParsedUnitSource, AdmittedModels, SpineLimits) {
    let source = declarations(count);
    let limits = wide(source.len());
    let live = Cancel::new();
    let parsed = parse(&request(source.as_bytes()), limits.source, &live)
        .expect("the generated unit parses")
        .into_value();
    let models = select(&parsed, &BTreeMap::new(), limits.model, &live)
        .expect("a unit with no model selects nothing")
        .into_value();
    (parsed, models, limits)
}

/// `check` over `parsed`, cancelled with `cause` from a second thread at
/// charge `at`: the check returns `Cancelled(cause)` and no package, and
/// exactly one charge saw the cancellation.
fn assert_check_cancelled_at(
    (parsed, models, limits): &(ParsedUnitSource, AdmittedModels, SpineLimits),
    at: u64,
    cause: CancelCause,
) {
    let gate = gated(Some(at));
    let outcome = gate.run(cause, |cancel| {
        check(
            parsed,
            "fixture".to_owned(),
            models,
            &DependencyInput::default(),
            *limits,
            cancel,
        )
    });
    match outcome {
        Err(StageFailure::Cancelled(seen)) => assert_eq!(seen, cause),
        Err(other) => panic!("expected Cancelled({cause:?}), got {other:?}"),
        Ok(_) => panic!("a cancelled check returned a package"),
    }
    gate.assert_stopped_at(at);
}

/// FR-276-AC-2 (TC-757 step 2): a `check` over a generated unit of
/// declarations, cancelled with `Deadline` from a second thread at its
/// 1,000th charge, returns `Cancelled(Deadline)` and no package, and
/// exactly one charge sees the cancellation.
///
/// The oracle counts polls of the handle, so it sees every charge only if
/// every charge site polls. The sites that poll are: the kernel `Meter`
/// charge, S1's leaf commit and parser step, I1's normalization `Meter`
/// charge, S3's contract meter (one `DeclarationCheck` charge per
/// declaration), S3's measure pass (one per node), S3's `Typer` node charge,
/// the type-environment work budget and E4's entry and per-node write. The
/// large-body, `select` and `package` tests below fail when their site does
/// not poll, because each asserts the uncancelled run made at least as many
/// charges as the input has nodes. Not polled: `model` dispatch linking,
/// which no spine operation calls.
#[trace("TC-757", "FR-276-AC-2")]
#[test]
fn a_check_cancelled_from_another_thread_stops_within_one_charge() {
    let unit = parsed_declarations(DECLARATIONS);
    let total = charges_of(|cancel| {
        check(
            &unit.0,
            "fixture".to_owned(),
            &unit.1,
            &DependencyInput::default(),
            unit.2,
            cancel,
        )
    });
    assert!(total > 1_000, "the check made only {total} charges");
    assert_check_cancelled_at(&unit, 1_000, CancelCause::Deadline);
}

/// `terms` ones added in one function body.
fn sum(terms: usize) -> String {
    let mut source = String::from(HEADER);
    source.push_str("function big using v(): Integer pure { 1");
    for _ in 1..terms {
        source.push_str(" + 1");
    }
    source.push_str(" }\n");
    source
}

/// FR-276-AC-2 (TC-757 step 2) for a cancel that lands inside one large
/// function body: the S3 node charge and the measure pass charge every node,
/// so the cancellation stops the check within one charge of landing in the
/// middle of the body, not at the next declaration.
#[trace("TC-757", "FR-276-AC-2")]
#[test]
fn a_cancel_inside_one_large_body_stops_within_one_charge() {
    const TERMS: usize = 3_000;
    let source = sum(TERMS);
    let limits = wide(source.len());
    let live = Cancel::new();
    let parsed = parse(&request(source.as_bytes()), limits.source, &live)
        .expect("the sum parses")
        .into_value();
    let models = select(&parsed, &BTreeMap::new(), limits.model, &live)
        .expect("a unit with no model selects nothing")
        .into_value();
    let unit = (parsed, models, limits);
    let total = charges_of(|cancel| {
        check(
            &unit.0,
            "fixture".to_owned(),
            &unit.1,
            &DependencyInput::default(),
            unit.2,
            cancel,
        )
    });
    // Every term is a node that the measure pass and the typer each charge:
    // a site that did not poll would leave `total` below the node count.
    assert!(
        total >= u64::try_from(2 * TERMS).expect("a small count"),
        "the check of one {TERMS}-term body made only {total} charges"
    );
    assert_check_cancelled_at(&unit, total / 2, CancelCause::Requested);
}

/// FR-276-AC-2 for `select`: the I1 normalization meter polls the handle at
/// every charge, so a cancel landing in the middle of the model's
/// normalization stops it within one charge.
#[trace("TC-757", "FR-276-AC-2")]
#[test]
fn a_cancel_inside_select_stops_within_one_charge() {
    let packages = qsl_semantics::model::intake::package_input([MODEL_DOCUMENT]).unwrap();
    let limits = SpineLimits::default();
    let live = Cancel::new();
    let parsed = parse(
        &request(spine_model::unit().as_bytes()),
        limits.source,
        &live,
    )
    .expect("the model fixture parses")
    .into_value();
    let run = |cancel: &Cancel| select(&parsed, &packages, limits.model, cancel);
    let total = charges_of(run);
    assert!(total > 2, "select made only {total} charges");
    let at = total / 2;
    let gate = gated(Some(at));
    let outcome = gate.run(CancelCause::Requested, run);
    assert!(matches!(
        outcome,
        Err(StageFailure::Cancelled(CancelCause::Requested))
    ));
    gate.assert_stopped_at(at);
}

/// FR-260-AC-4 (FR-056-AC-2): a model document one byte over the caller's
/// `intake.input_bytes` refuses `resource_exhausted`/`intake-limit-exceeded`
/// through `select` (FR-255 Behavior 1 and FR-260 item 5: `IntakeLimits` is a
/// caller budget, not a stage's own limits type), naming that setting,
/// its bound and the document's size, never a missing package.
#[trace("TC-732", "FR-260-AC-4", "FR-255-AC-1", "FR-506-AC-5")]
#[test]
fn select_names_the_intake_limit_for_an_oversize_document() {
    let packages = qsl_semantics::model::intake::package_input([MODEL_DOCUMENT]).unwrap();
    let mut limits = SpineLimits::default();
    let size = u64::try_from(MODEL_DOCUMENT.len()).expect("a small document");
    limits.model.intake = qsl_foundation::IntakeLimits::default().with_input_bytes(size - 1);
    let parsed = parse(
        &request(spine_model::unit().as_bytes()),
        limits.source,
        &Cancel::new(),
    )
    .expect("the model fixture parses")
    .into_value();
    let mut exact = limits.model;
    exact.intake = qsl_foundation::IntakeLimits::default().with_input_bytes(size);
    assert!(select(&parsed, &packages, exact, &Cancel::new()).is_ok());
    let Err(StageFailure::Refused(refusal)) =
        select(&parsed, &packages, limits.model, &Cancel::new())
    else {
        panic!("a document over the intake limit is a caller-budget refusal");
    };
    assert_eq!(refusal.code(), Code::ResourceExhausted);
    assert_eq!(refusal.cause(), Some("intake-limit-exceeded"));
    assert_eq!(
        refusal.fields(),
        Some(BTreeMap::from([
            ("kind", "input-bytes-exceeded".to_owned()),
            ("bound", (size - 1).to_string()),
            ("actual", size.to_string()),
            ("setting", "intake.input_bytes".to_owned()),
        ]))
    );
    let mut zero = limits.model;
    zero.intake = qsl_foundation::IntakeLimits::default().with_input_bytes(0);
    let Err(StageFailure::Refused(refusal)) = select(&parsed, &packages, zero, &Cancel::new())
    else {
        panic!("a nonempty document exceeds the zero intake limit");
    };
    assert_eq!(refusal.code(), Code::ResourceExhausted);
    assert_eq!(
        refusal
            .fields()
            .and_then(|fields| fields.get("setting").cloned()),
        Some("intake.input_bytes".to_owned())
    );
    assert_eq!(
        refusal
            .fields()
            .and_then(|fields| fields.get("actual").cloned()),
        Some(size.to_string())
    );
}

/// FR-276-AC-2 for `package`: the emitter polls the handle at entry and at
/// every node it writes.
#[trace("TC-757", "FR-276-AC-2")]
#[test]
fn a_cancel_inside_package_stops_within_one_charge() {
    let chain = fixture_chain();
    let run = |cancel: &Cancel| package(&chain.checked, PackageLimits::default(), cancel);
    let total = charges_of(run);
    assert!(total > 2, "package made only {total} charges");
    let at = total / 2;
    let gate = gated(Some(at));
    let outcome = gate.run(CancelCause::Deadline, run);
    assert!(matches!(
        outcome,
        Err(StageFailure::Cancelled(CancelCause::Deadline))
    ));
    gate.assert_stopped_at(at);
}

/// `levels` functions, each calling the one before it twice, so calling the
/// last evaluates `2^levels` leaves.
fn doubling(levels: usize) -> String {
    let mut source = String::from(HEADER);
    source.push_str("function d0 using v(): Integer pure { 1 }\n");
    for level in 1..levels {
        let below = level - 1;
        writeln!(
            source,
            "function d{level} using v(): Integer pure {{ d{below}() + d{below}() }}"
        )
        .expect("a string write");
    }
    source
}

/// FR-276-AC-3 (TC-757 step 3): an `execute` whose evaluation would charge
/// millions of work units, run with every accounting limit at `u64::MAX` and
/// cancelled from a second thread after 1,000 charges, returns
/// `CallFailure::Cancelled(Requested)` and no value.
#[trace("TC-757", "FR-276-AC-3")]
#[test]
fn an_execute_cancelled_from_another_thread_returns_no_value() {
    let source = doubling(24);
    let chain = chain_of(
        source.as_bytes(),
        &BTreeMap::new(),
        &DependencyInput::default(),
        SpineLimits::default(),
    )
    .unwrap_or_else(|failure| panic!("the doubling unit compiles: {failure:?}"));

    let gate = gated(Some(1_000));
    let function = QualifiedName::unqualified("d23").expect("an identifier");
    let outcome = gate.run(CancelCause::Requested, |cancel| {
        execute(
            &chain.checked,
            &ExecuteRequest {
                function: &function,
                arguments: &[],
                objects: &ObjectEnvironment::default(),
                supplied: SuppliedLimits::default(),
            },
            default_accounting(u64::MAX),
            cancel,
        )
    });

    assert_eq!(
        outcome.map(|_| ()).unwrap_err(),
        CallFailure::Cancelled(CancelCause::Requested)
    );
    gate.assert_stopped_at(1_000);
}

/// FR-276-AC-5 (TC-757 step 5): a cancelled `execute` is category
/// incomplete, exit 22. (A cancelled `StageFailure` of each cause is checked
/// beside the type, in `qsl-foundation`.)
#[trace("TC-757", "FR-276-AC-5")]
#[test]
fn a_cancelled_call_is_incomplete_and_exits_22() {
    for cause in [CancelCause::Requested, CancelCause::Deadline] {
        let category = CallFailure::Cancelled(cause).category();
        assert_eq!(category, Category::Incomplete);
        assert_eq!(category.exit_code(), 22);
    }
}

/// FR-100-AC-12 (TC-452 step 5): `spine::run` over `seven`, cancelled mid-run
/// (`Requested` at the last front-end charge, `Deadline` at the first S6a
/// charge) returns `RunRefusal::Cancelled` with the handle's cause, never a
/// fault, and `from_run` writes it incomplete, exit 22, no stage.
#[trace("TC-452", "FR-100-AC-12")]
#[test]
fn a_cancel_tripped_mid_run_is_cancelled_with_its_cause_never_a_fault() {
    let run_under = |cancel: &Cancel| {
        crate::spine::run(
            identity(),
            "fixture".to_owned(),
            "unit.native",
            FIXTURE.as_bytes(),
            &BTreeMap::new(),
            &DependencyInput::default(),
            SpineLimits::default(),
            &crate::spine::Call {
                function: "seven".to_owned(),
                arguments: Vec::new(),
                accounting: default_accounting(DEFAULT_WORK_UNITS),
                supplied: SuppliedLimits::default(),
            },
            &ObjectEnvironment::default(),
            cancel,
        )
    };
    let front_end = charges_of(|cancel| {
        let limits = SpineLimits::default();
        let parsed = parse(&request(FIXTURE.as_bytes()), limits.source, cancel)?.into_value();
        let models = select(&parsed, &BTreeMap::new(), limits.model, cancel)?.into_value();
        let checked = check(
            &parsed,
            "fixture".to_owned(),
            &models,
            &DependencyInput::default(),
            limits,
            cancel,
        )?
        .into_value();
        package(&checked, PackageLimits::default(), cancel).map(|_| ())
    });
    let total = charges_of(run_under);
    assert!(
        total > front_end,
        "S6a charges at least once: {front_end} of {total}"
    );

    for (at, cause, spelling) in [
        (front_end, CancelCause::Requested, "requested"),
        (front_end + 1, CancelCause::Deadline, "deadline"),
    ] {
        let gate = gated(Some(at));
        let result = gate.run(cause, run_under);
        gate.assert_stopped_at(at);
        let refusal = result.as_ref().err().map(AsRef::as_ref);
        assert!(
            matches!(refusal, Some(RunRefusal::Cancelled(seen)) if *seen == cause),
            "charge {at}: {refusal:?}"
        );
        assert_eq!(
            result.as_ref().unwrap_err().category(),
            Category::Incomplete
        );
        assert_eq!(result.as_ref().unwrap_err().stage(), None);
        let document = OutcomeDocument::from_run(&result);
        assert_eq!(document.category().exit_code(), 22);
        let json: serde_json::Value =
            serde_json::from_slice(&document.to_bytes().expect("encodes")).expect("is JSON");
        assert_eq!(json["category"], "incomplete");
        assert_eq!(json["last_stage"], serde_json::Value::Null);
        assert_eq!(json["result"], serde_json::Value::Null);
        assert_eq!(json["diagnostics"].as_array().map(Vec::len), Some(1));
        assert_eq!(json["diagnostics"][0]["code"], "cancelled");
        assert_eq!(json["diagnostics"][0]["cause"], spelling);
    }
}

/// FR-278-AC-1 (TC-759 step 1): the four operations composed over the
/// compile fixture, over a domain-package request and over a library
/// request each emit a package QSL's I2 reader reads back, and the same
/// composition twice emits the same bytes.
#[trace("TC-759", "FR-278-AC-1")]
#[test]
fn the_composition_emits_packages_the_i2_reader_reads_back() {
    // The compile fixture.
    let fixture = fixture_chain();
    read_back(&fixture, &BTreeMap::new(), "fixture");

    // A domain-package request: the unit's `model` names the document by
    // its `sha256-jcs` digest.
    let packages = qsl_semantics::model::intake::package_input([MODEL_DOCUMENT]).unwrap();
    let model = chain_of(
        spine_model::unit().as_bytes(),
        &packages,
        &DependencyInput::default(),
        SpineLimits::default(),
    )
    .unwrap_or_else(|failure| panic!("the model fixture compiles: {failure:?}"));
    assert_eq!(model.models.models().len(), 1);
    read_back(&model, &packages, "fixture");
    let again = chain_of(
        spine_model::unit().as_bytes(),
        &packages,
        &DependencyInput::default(),
        SpineLimits::default(),
    )
    .expect("the model fixture compiles again");
    assert_eq!(model.emitted.package(), again.emitted.package());

    // A library request: the unit imports a library the request supplies.
    let geometry = geometry();
    let dependencies = DependencyInput::new(vec![geometry.clone()]).expect("admissible");
    let unit = format!(
        "{HEADER}import \"test/geometry\" as g;\n\
         function h using v(): Boolean pure {{ true }}\n"
    );
    let importing = chain_of(
        unit.as_bytes(),
        &BTreeMap::new(),
        &dependencies,
        SpineLimits::default(),
    )
    .unwrap_or_else(|failure| panic!("the importing unit compiles: {failure:?}"));
    assert_eq!(importing.checked.libraries().len(), 1);
    read_back(&importing, &BTreeMap::new(), "fixture");
}

/// FR-147-AC-10 (TC-552): a supplied library's malformed internal-list
/// value is checked at that library's source even when the unit does not
/// import it.
#[trace("TC-552", "FR-147-AC-10")]
#[test]
fn an_unimported_library_internal_list_defect_refuses_the_compile_at_its_owner() {
    let dependencies = DependencyInput::new([geometry()])
        .expect("the library input is admissible")
        .with_refinement_internals([(
            "test/geometry".to_owned(),
            serde_json::json!([{"refinement":"NoSuchRefinement","internal":[]}]),
        )])
        .expect("the value belongs to the supplied library");
    let limits = SpineLimits::default();
    let cancel = Cancel::new();
    let parsed = parse(&request(FIXTURE.as_bytes()), limits.source, &cancel)
        .expect("the root unit parses")
        .into_value();
    let models = select(&parsed, &BTreeMap::new(), limits.model, &cancel)
        .expect("the root unit has no model selections")
        .into_value();
    let failure = check_with_refinement_internals(
        &parsed,
        "fixture".to_owned(),
        None,
        &models,
        &dependencies,
        limits,
        &cancel,
    )
    .expect_err("the unimported library value is still checked");
    let refusal = refusal_or_fault(failure).expect("this is a typed refusal");
    let CompileRefusal::Dependency { path, refusal } = *refusal else {
        panic!("the refusal names its supplied library owner: {refusal:?}");
    };
    assert_eq!(path[0].as_str(), "test/geometry");
    let CompileRefusal::Check { region, refusals } = *refusal else {
        panic!("the library's S3 list defect is located: {refusal:?}");
    };
    assert!(region.is_none(), "a value pointer is not a source span");
    let [refusal] = refusals.as_slice() else {
        panic!("one typed library-value refusal: {refusals:?}");
    };
    let qsl_semantics::check::CheckCause::Refinement(refusal) = &refusal.cause else {
        panic!("the internal-list defect is typed: {refusal:?}");
    };
    let qsl_semantics::check::RefinementCause::InternalList(defect) = &refusal.cause else {
        panic!("the refusal identifies the list locus: {:?}", refusal.cause);
    };
    assert_eq!(defect.pointer, "/0/refinement");
}

/// `chain`'s emitted bytes through the I2 reader, as `identity` (the name the package was checked under).
fn read_back(chain: &Chain, packages: &qsl_semantics::model::intake::PackageInput, identity: &str) {
    let emission = emit_checked(chain.checked.package()).expect("the package emits");
    assert_eq!(emission.package(), chain.emitted.package());
    read_import_view(
        chain.checked.package(),
        &emission,
        LibraryName::new(identity).expect("a non-empty identity"),
        packages,
        &mut AdmittedPackages::default(),
        qsl_package::V2ReadLimits::default(),
    )
    .unwrap_or_else(|refusal| panic!("the I2 reader refused {identity}: {refusal}"));
}

/// FR-278-AC-2 (TC-759 step 2): a syntax error is refused by `parse` with
/// `invalid_syntax`; a `model` declaration naming a document the request does
/// not supply is refused by `select` at the `model` declaration; an `inv`
/// function over an unconstrained integer is refused by `check` with
/// `ill_typed`. Each refusal names its stage.
#[trace("TC-759", "FR-278-AC-2")]
#[test]
fn each_operation_refuses_at_the_stage_that_owns_the_defect() {
    let limits = SpineLimits::default();
    let live = Cancel::new();

    let syntax = refusal(
        parse(
            &request(format!("{HEADER}function f using v(: Boolean pure {{ true }}\n").as_bytes()),
            limits.source,
            &live,
        )
        .expect_err("a syntax error does not parse"),
    );
    assert_eq!(syntax.code().as_str(), "invalid_syntax");
    assert_eq!(syntax.stage(), SpineStage::Source);

    let parsed = parse(
        &request(spine_model::unit().as_bytes()),
        limits.source,
        &live,
    )
    .expect("the model fixture parses")
    .into_value();
    let missing = refusal(
        select(&parsed, &BTreeMap::new(), limits.model, &live)
            .expect_err("no document is supplied"),
    );
    assert_eq!(missing.stage(), SpineStage::Intake);
    let region = missing.region().expect("the refusal is located");
    let (start, end) = (
        usize::try_from(region.start()).expect("a start offset"),
        usize::try_from(region.end()).expect("an end offset"),
    );
    let model_fixture = spine_model::unit();
    assert!(
        model_fixture[start..end].starts_with("model M"),
        "located at {:?}",
        &model_fixture[start..end]
    );

    let ill_typed = format!("{HEADER}function f using v(): Boolean pure {{ 1 + true }}\n");
    let parsed = parse(&request(ill_typed.as_bytes()), limits.source, &live)
        .expect("the ill-typed unit parses")
        .into_value();
    let models = select(&parsed, &BTreeMap::new(), limits.model, &live)
        .expect("a unit with no model selects nothing")
        .into_value();
    let refused = refusal(
        check(
            &parsed,
            "fixture".to_owned(),
            &models,
            &DependencyInput::default(),
            limits,
            &live,
        )
        .expect_err("the unit is ill typed"),
    );
    assert_eq!(refused.code().as_str(), "ill_typed");
    assert_eq!(refused.stage(), SpineStage::Check);
}

/// FR-362-AC-2 / TC-1026: the production parser uses the complete-V1 source
/// grammar and refuses a composed-only profile version tail.
#[trace("TC-1026", "FR-362-AC-2")]
#[test]
fn the_producer_refuses_a_profile_version_tail_as_invalid_syntax() {
    let limits = SpineLimits::default();
    let cancel = Cancel::new();
    let source = "language \"ix:native\" edition \"1-draft\";\nprofile v = \"quire.value.complete/v1\" version \"1\";\n";
    let failure = parse(&request(source.as_bytes()), limits.source, &cancel)
        .expect_err("the producer rejects a profile version tail");
    let syntax = refusal(failure);
    assert_eq!(syntax.code().as_str(), "invalid_syntax");
    assert_eq!(syntax.stage(), SpineStage::Source);
}

/// FR-278-AC-3 (TC-759 step 3): the `EmittedUnit` carries a source
/// provision that carries every source the package was compiled from.
/// (Replaying through that provision is checked beside the replay executor.)
#[trace("TC-759", "FR-278-AC-3")]
#[test]
fn the_provision_names_every_source_the_package_was_compiled_from() {
    let geometry = geometry();
    let unit = format!(
        "{HEADER}import \"test/geometry\" as g;\n\
         function h using v(): Boolean pure {{ true }}\n"
    );
    let chain = chain_of(
        unit.as_bytes(),
        &BTreeMap::new(),
        &DependencyInput::new(vec![geometry.clone()]).expect("admissible"),
        SpineLimits::default(),
    )
    .unwrap_or_else(|failure| panic!("the importing unit compiles: {failure:?}"));

    assert_eq!(
        references(&chain.emitted),
        vec![unit.as_bytes().to_vec(), geometry.bytes.clone()]
    );
}

/// FR-275-AC-5 (TC-755 step 7): each operation reports the work of its own
/// stages. `parse` reports S1 work and none for S3, `check` over the same
/// `ParsedUnitSource` reports S3 work and zero for S1 and S2, and `package` over
/// its `CheckedUnit` reports E4 work and zero for S1 to S4. (`execute`
/// returns the evaluation of FR-277, whose counters are the S6a meter's.)
#[trace("TC-755", "FR-275-AC-5")]
#[test]
fn each_operation_reports_the_work_of_its_own_stages() {
    let limits = SpineLimits::default();
    let cancel = Cancel::new();
    let parsed =
        parse(&request(FIXTURE.as_bytes()), limits.source, &cancel).expect("the fixture parses");
    let parse_work = parsed.work();
    assert!(parse_work.s1 > 0, "parse reported no S1 work");
    assert_eq!(parse_work.s3, 0);
    let parsed = parsed.into_value();

    let models = select(&parsed, &BTreeMap::new(), limits.model, &cancel)
        .expect("a unit with no model selects nothing");
    assert_eq!(models.work().s1, 0);
    let models = models.into_value();

    let checked = check(
        &parsed,
        "fixture".to_owned(),
        &models,
        &DependencyInput::default(),
        limits,
        &cancel,
    )
    .expect("the fixture checks");
    let check_work = checked.work();
    assert_eq!((check_work.s1, check_work.s2), (0, 0));
    assert!(check_work.s3 > 0, "check reported no S3 work");
    assert_eq!(check_work.e4, 0);
    let checked = checked.into_value();

    let emitted = package(&checked, PackageLimits::default(), &cancel).expect("the package emits");
    let emit_work = emitted.work();
    assert_eq!(
        (emit_work.s1, emit_work.s2, emit_work.s3, emit_work.s4),
        (0, 0, 0, 0)
    );
    assert!(emit_work.e4 > 0, "package reported no E4 work");
}

/// How one run of an operation under a limit ended.
enum Reached {
    /// It produced its output.
    Output,
    /// It stopped at a limit.
    Limit(LimitExceeded),
    /// It refused `resource_exhausted`: a ceiling of the caller's budget
    /// (ADR-014 section 1), with the cause and the typed fields it carries.
    Budget {
        cause: Option<&'static str>,
        fields: BTreeMap<&'static str, String>,
    },
}

fn reached<T>(outcome: Result<Staged<T>, FrontEndFailure>) -> Reached {
    match outcome {
        Ok(_) => Reached::Output,
        Err(StageFailure::Limit(limit)) => Reached::Limit(limit),
        Err(StageFailure::Refused(refusal)) if refusal.code() == Code::ResourceExhausted => {
            Reached::Budget {
                cause: refusal.cause(),
                fields: refusal
                    .fields()
                    .expect("a caller-budget refusal carries fields"),
            }
        }
        Err(other) => panic!("expected an output or a limit, got {other:?}"),
    }
}

/// FR-277-AC-1 for one limits field: the smallest value of the field at
/// which `run` produces its output is the counter the input reaches. One
/// below it, `run` stops with `LimitExceeded` naming `field` and carrying the
/// configured value; at it, `run` succeeds.
fn assert_field(
    field: Setting,
    kind: FoundationKind,
    high: u64,
    run: impl Fn(u64) -> Reached,
) -> u64 {
    assert_field_with(field, kind, high, |value| value, run)
}

/// The smallest value of `field` at which `run` produces its output, found
/// by bisection from `high`. Every run below it stops at `field` (a stage
/// limit or a caller-budget refusal naming that setting).
fn smallest_admitted(field: Setting, high: u64, run: &impl Fn(u64) -> Reached) -> u64 {
    assert!(
        matches!(run(high), Reached::Output),
        "`{field:?}` at {high} does not admit the input"
    );
    let (mut low, mut top) = (0_u64, high);
    while low < top {
        let middle = low + (top - low) / 2;
        match run(middle) {
            Reached::Output => top = middle,
            Reached::Limit(limit) => {
                assert_eq!(limit.setting(), field, "limit at {middle}");
                low = middle + 1;
            }
            Reached::Budget { fields, .. } => {
                assert_eq!(fields["setting"], field.name(), "limit at {middle}");
                low = middle + 1;
            }
        }
    }
    assert!(low > 0, "`{field:?}` is not reached by the input");
    low
}

/// [`assert_field`] for a field whose configured bound is `bound(value)`
/// rather than the value itself.
fn assert_field_with(
    field: Setting,
    kind: FoundationKind,
    high: u64,
    bound: impl Fn(u64) -> u64,
    run: impl Fn(u64) -> Reached,
) -> u64 {
    let counter = smallest_admitted(field, high, &run);
    match run(counter - 1) {
        Reached::Limit(limit) => {
            assert_eq!(limit.setting(), field);
            assert_eq!(limit.kind(), kind, "`{field:?}`");
            assert_eq!(limit.configured_bound(), bound(counter - 1), "`{field:?}`");
        }
        Reached::Output => panic!("`{field:?}` at {} admits the input", counter - 1),
        Reached::Budget { .. } => panic!("`{field:?}` is a stage limit, not a budget"),
    }
    assert!(
        matches!(run(counter), Reached::Output),
        "`{field:?}` at {counter}"
    );
    counter
}

/// [`assert_field`] for a field of the caller's budget (ADR-014 section 1,
/// FR-255 Behavior 1): one below the counter the input reaches, `run`
/// refuses `resource_exhausted` with `cause`, `setting` of `field`, the
/// effective `limit` and an `actual` past it; at the counter it succeeds.
fn assert_budget_field(
    field: Setting,
    cause: &str,
    high: u64,
    run: impl Fn(u64) -> Reached,
) -> u64 {
    let counter = smallest_admitted(field, high, &run);
    match run(counter - 1) {
        Reached::Budget {
            cause: seen,
            fields,
        } => {
            assert_eq!(seen, Some(cause), "`{field:?}`");
            assert_eq!(fields["setting"], field.name());
            assert_eq!(fields["limit"], (counter - 1).to_string(), "`{field:?}`");
            let actual: u128 = fields["actual"].parse().expect("a count");
            assert!(actual > u128::from(counter - 1), "`{field:?}`");
        }
        Reached::Output => panic!("`{field:?}` at {} admits the input", counter - 1),
        Reached::Limit(_) => panic!("`{field:?}` is a budget, not a stage limit"),
    }
    assert!(
        matches!(run(counter), Reached::Output),
        "`{field:?}` at {counter}"
    );
    counter
}

/// FR-277-AC-1 (TC-758 step 1) for `parse`: each field of the source limits.
#[trace("TC-758", "FR-277-AC-1")]
#[test]
fn parse_names_the_source_limit_field_it_reached() {
    let bytes = FIXTURE.as_bytes();
    let run = |edit: fn(&mut SourceLimits, u64)| {
        move |value: u64| {
            let mut limits = SpineLimits::default().source;
            edit(&mut limits, value);
            reached(parse(&request(bytes), limits, &Cancel::new()))
        }
    };
    assert_field(
        Setting::S1InputBytes,
        FoundationKind::InputBytes,
        10_000,
        run(|limits, value| limits.source_bytes = usize::try_from(value).expect("small")),
    );
    assert_field(
        Setting::S1Tokens,
        FoundationKind::TokenCount,
        10_000,
        run(|limits, value| limits.tokens = usize::try_from(value).expect("small")),
    );
    assert_field(
        Setting::S1Nodes,
        FoundationKind::NodeCount,
        10_000,
        run(|limits, value| limits.nodes = usize::try_from(value).expect("small")),
    );
    // The work bound is the unit's total step budget, in the same unit as
    // the counter; the field sets it per token. The scale is the budget at a
    // field value of 1.
    let scale =
        match run(|limits, value| limits.work_units = usize::try_from(value).expect("small"))(1) {
            Reached::Limit(limit) => limit.configured_bound(),
            Reached::Output | Reached::Budget { .. } => {
                panic!("one step per token refuses the fixture at a stage limit")
            }
        };
    assert_field_with(
        Setting::S1WorkUnits,
        FoundationKind::WorkBudget,
        256,
        |value| value * scale,
        run(|limits, value| limits.work_units = usize::try_from(value).expect("small")),
    );
    // The byte ceiling reports the source's real length as the counter.
    let length = u128::try_from(bytes.len()).expect("a small length");
    let mut limits = SpineLimits::default().source;
    limits.source_bytes = bytes.len() - 1;
    match reached(parse(&request(bytes), limits, &Cancel::new())) {
        Reached::Limit(limit) => assert_eq!(limit.actual(), length),
        Reached::Output | Reached::Budget { .. } => {
            panic!("one byte short refuses the fixture at a stage limit")
        }
    }
}

/// FR-277-AC-1 (TC-758 step 1) for `select`: each field of the model
/// normalization limits that a charge reaches. `ModelNormalizationLimitsV1`
/// is a caller budget, so `select` refuses `resource_exhausted` naming the
/// field (ADR-014 section 1, FR-255 Behavior 1), not `LimitExceeded`.
#[trace("TC-758", "FR-277-AC-1", "FR-255-AC-1")]
#[test]
fn select_names_the_model_limit_field_it_reached() {
    let packages = qsl_semantics::model::intake::package_input([MODEL_DOCUMENT]).unwrap();
    let live = Cancel::new();
    let parsed = parse(
        &request(spine_model::unit().as_bytes()),
        SpineLimits::default().source,
        &live,
    )
    .expect("the model fixture parses")
    .into_value();
    let run = |edit: fn(&mut ModelNormalizationLimits, u64)| {
        let (parsed, packages) = (&parsed, &packages);
        move |value: u64| {
            let mut limits = ModelNormalizationLimits::default();
            edit(&mut limits, value);
            reached(select(parsed, packages, limits, &Cancel::new()))
        }
    };
    assert_budget_field(
        Setting::ModelDeclarationRecords,
        "insufficient-next-charge",
        100_000,
        run(|limits, value| limits.declaration_records = value),
    );
    assert_budget_field(
        Setting::ModelDerivationFacts,
        "insufficient-next-charge",
        100_000,
        run(|limits, value| limits.derivation_facts = value),
    );
    assert_budget_field(
        Setting::ModelEffectiveDeclarations,
        "insufficient-next-charge",
        100_000,
        run(|limits, value| limits.effective_declarations = value),
    );
    assert_budget_field(
        Setting::ModelAncestorSteps,
        "insufficient-next-charge",
        10_000,
        run(|limits, value| limits.ancestor_steps = value),
    );
    assert_budget_field(
        Setting::ModelWorkUnits,
        "insufficient-next-charge",
        100_000,
        run(|limits, value| limits.work_units = value),
    );
    // `model.dispatch_candidates` and `model.family_steps` are charged only
    // where a dispatch family is linked (FR-302, TC-795), at `check`, and the
    // model fixture declares no dispatched operation, so no `select` run
    // reaches them; TC-795 and the dispatch tests in `qsl-semantics` drive them.
}

/// FR-255 Behavior 1 (QSL-745): a denied normalization charge through
/// `select` retains the stage, accounting contract, prior usage, denied
/// amount and charge point beside the setting, effective limit and actual.
#[trace("TC-758", "FR-255-AC-1")]
#[test]
fn select_retains_the_denied_charge_of_a_normalization_ceiling() {
    let packages = qsl_semantics::model::intake::package_input([MODEL_DOCUMENT]).unwrap();
    let parsed = parse(
        &request(spine_model::unit().as_bytes()),
        SpineLimits::default().source,
        &Cancel::new(),
    )
    .expect("the model fixture parses")
    .into_value();
    let limits = ModelNormalizationLimits {
        work_units: 1,
        ..ModelNormalizationLimits::default()
    };
    let Err(StageFailure::Refused(refusal)) = select(&parsed, &packages, limits, &Cancel::new())
    else {
        panic!("one work unit cannot normalize the model");
    };
    assert_eq!(refusal.code(), Code::ResourceExhausted);
    assert_eq!(refusal.cause(), Some("insufficient-next-charge"));
    let fields = refusal.fields().expect("a denied charge carries fields");
    let keys: Vec<_> = fields.keys().copied().collect();
    assert_eq!(
        keys,
        [
            "accounting_contract",
            "actual",
            "charge_point",
            "consumed",
            "limit",
            "limit_kind",
            "next_charge",
            "setting",
            "stage"
        ]
    );
    assert_eq!(fields["setting"], "model.work_units");
    assert_eq!(fields["limit"], "1");
    assert_eq!(fields["stage"], "model-normalization");
    assert_eq!(fields["accounting_contract"], "ModelNormalizationLimitsV1");
    assert_eq!(fields["limit_kind"], "work_units");
    let consumed: u64 = fields["consumed"].parse().expect("a count");
    let next_charge: u64 = fields["next_charge"].parse().expect("a count");
    assert!(next_charge > 0);
    assert_eq!(fields["actual"], (consumed + next_charge).to_string());
    assert!(fields["charge_point"].starts_with("normalize."));
}

/// FR-277-AC-1 (TC-758 step 1) for `check`: each checking limit.
#[trace("TC-758", "FR-277-AC-1", "FR-506-AC-5")]
#[test]
fn check_names_the_checking_limit_field_it_reached() {
    let chain = fixture_chain();
    let run = |edit: fn(SpineLimits, u64) -> SpineLimits| {
        let chain = &chain;
        move |value: u64| {
            let limits = edit(SpineLimits::default(), value);
            reached(check(
                &chain.parsed,
                "fixture".to_owned(),
                &chain.models,
                &DependencyInput::default(),
                limits,
                &Cancel::new(),
            ))
        }
    };
    assert_field(
        Setting::S3Nodes,
        FoundationKind::NodeCount,
        100_000,
        run(|mut limits, value| {
            limits.checking = CheckingLimits::new(value)
                .with_input_bytes(limits.checking.input_bytes())
                .with_work_budget(limits.checking.work_budget());
            limits
        }),
    );
    let zero_nodes = run(|mut limits, _| {
        limits.checking = CheckingLimits::new(0)
            .with_input_bytes(limits.checking.input_bytes())
            .with_work_budget(limits.checking.work_budget());
        limits
    })(0);
    assert!(matches!(
        zero_nodes,
        Reached::Limit(limit) if limit.setting() == Setting::S3Nodes
    ));
    assert_field(
        Setting::S3InputBytes,
        FoundationKind::InputBytes,
        1_000_000,
        run(|mut limits, value| {
            limits.checking = limits.checking.with_input_bytes(value);
            limits
        }),
    );
    assert_field(
        Setting::S3WorkUnits,
        FoundationKind::WorkBudget,
        1_000_000,
        run(|mut limits, value| {
            limits.checking = limits.checking.with_work_budget(value);
            limits
        }),
    );
}

/// FR-506-AC-5 (TC-1301 row 11): the model's S3 formal nodes admit at their
/// exact count and refuse when the named setting is zero.
#[trace("FR-506-AC-5", "TC-1301")]
#[test]
fn selected_model_formal_node_limit_is_exact() {
    let packages = qsl_semantics::model::intake::package_input([MODEL_DOCUMENT]).unwrap();
    let parsed = parse(
        &request(spine_model::unit().as_bytes()),
        SpineLimits::default().source,
        &Cancel::new(),
    )
    .expect("the selected model unit parses")
    .into_value();
    let models = select(
        &parsed,
        &packages,
        ModelNormalizationLimits::default(),
        &Cancel::new(),
    )
    .expect("the model selects")
    .into_value();
    let s3_nodes = |bound: u64| {
        let mut limits = SpineLimits::default();
        limits.checking = CheckingLimits::new(bound)
            .with_input_bytes(limits.checking.input_bytes())
            .with_work_budget(limits.checking.work_budget());
        reached(check(
            &parsed,
            "fixture".to_owned(),
            &models,
            &DependencyInput::default(),
            limits,
            &Cancel::new(),
        ))
    };
    assert_field(
        Setting::S3Nodes,
        FoundationKind::NodeCount,
        100_000,
        s3_nodes,
    );
    assert!(matches!(
        s3_nodes(0),
        Reached::Limit(limit) if limit.setting() == Setting::S3Nodes
    ));
}

/// FR-255-AC-4 at both entry points, stage-driven: an input that reaches a
/// limit stops with that limit's setting, and the same input passes once the
/// setting is raised, through the settings operation's operand and through a
/// replay request's `stage_limits` entry. Covers `s1.tokens` (parse),
/// `s3.nodes` and `s3.work_units` (check).
#[trace("TC-721", "FR-255-AC-4")]
#[test]
fn a_reached_limit_is_raised_by_the_settings_operation_and_by_a_request() {
    let chain = fixture_chain();
    let entry_points = |name: &'static str| {
        move |value: u64| -> [SpineLimits; 2] {
            let setting = Setting::from_name(name).expect("a table name is a setting");
            let operand = format!("{name}={value}");
            let by_operand = CallerLimits::from_operands([operand.as_str()])
                .expect("a well-formed operand")
                .spine;
            let entries: StageLimits = [(setting, value)].into_iter().collect();
            let by_request = CallerLimits::for_request(
                &entries,
                crate::limits::AccountingLimits::default().0,
                ReplayLimits::default(),
            )
            .spine;
            [by_operand, by_request]
        }
    };
    let check_under = |limits: SpineLimits| {
        reached(check(
            &chain.parsed,
            "fixture".to_owned(),
            &chain.models,
            &DependencyInput::default(),
            limits,
            &Cancel::new(),
        ))
    };
    for entry in 0..2 {
        let tokens = entry_points("s1.tokens");
        assert_field(
            Setting::S1Tokens,
            FoundationKind::TokenCount,
            10_000,
            |value| {
                reached(parse(
                    &request(FIXTURE.as_bytes()),
                    tokens(value)[entry].source,
                    &Cancel::new(),
                ))
            },
        );
        let nodes = entry_points("s3.nodes");
        assert_field(
            Setting::S3Nodes,
            FoundationKind::NodeCount,
            100_000,
            |value| check_under(nodes(value)[entry]),
        );
        let work = entry_points("s3.work_units");
        assert_field(
            Setting::S3WorkUnits,
            FoundationKind::WorkBudget,
            1_000_000,
            |value| check_under(work(value)[entry]),
        );
    }
}

/// FR-255-AC-6, the second half: the effective limits a parsed source and a
/// checked package record equal the table's defaults when nothing is
/// configured.
#[trace("TC-721", "FR-255-AC-6")]
#[test]
fn the_effective_limits_a_checked_package_records_are_the_defaults() {
    use qsl_foundation::SettingLimits;

    let limits = CallerLimits::from_operands([]).expect("no operands").spine;
    let chain = fixture_chain();
    assert_eq!(
        chain.parsed.syntax().effective_limits(),
        SourceLimits::default()
    );
    assert_eq!(
        limits.source.bounds(),
        chain.parsed.syntax().effective_limits().bounds()
    );
    let checked = check(
        &chain.parsed,
        "fixture".to_owned(),
        &chain.models,
        &DependencyInput::default(),
        limits,
        &Cancel::new(),
    )
    .unwrap_or_else(|failure| panic!("the fixture checks: {failure:?}"))
    .into_value();
    assert_eq!(
        checked.package.graph().effective_limits(),
        CheckingLimits::default()
    );
}

/// FR-277-AC-1 (TC-758 step 1) for the type-environment ceilings `check`
/// admits the unit's object types under.
#[trace("TC-758", "FR-277-AC-1")]
#[test]
fn check_names_the_type_environment_limit_field_it_reached() {
    let packages = qsl_semantics::model::intake::package_input([MODEL_DOCUMENT]).unwrap();
    let model = chain_of(
        spine_model::unit().as_bytes(),
        &packages,
        &DependencyInput::default(),
        SpineLimits::default(),
    )
    .unwrap_or_else(|failure| panic!("the model fixture compiles: {failure:?}"));
    let run = |edit: fn(&mut SpineLimits, u64)| {
        let model = &model;
        move |value: u64| {
            let mut limits = SpineLimits::default();
            edit(&mut limits, value);
            reached(check(
                &model.parsed,
                "fixture".to_owned(),
                &model.models,
                &DependencyInput::default(),
                limits,
                &Cancel::new(),
            ))
        }
    };
    assert_field(
        Setting::EnvironmentAncestorSteps,
        FoundationKind::EdgeCount,
        10_000,
        run(|limits, value| limits.assembly.environment.ancestor_steps = value),
    );
    assert_field(
        Setting::EnvironmentWorkUnits,
        FoundationKind::WorkBudget,
        10_000_000,
        run(|limits, value| limits.assembly.environment.work_units = value),
    );
}

/// FR-277-AC-1 (TC-758 step 1) and FR-099-AC-7 for the S4 source
/// resolution `check` runs: a unit importing `test/geometry` reaches one
/// library, one import edge and the library's source bytes, and each
/// `dependency.*` limit one below that stops `check` with `LimitExceeded`
/// naming its field.
#[trace("TC-758", "FR-277-AC-1", "TC-446", "FR-099-AC-7")]
#[test]
fn check_names_the_dependency_limit_field_it_reached() {
    let dependencies = DependencyInput::new(vec![geometry()]).expect("admissible");
    let unit = format!(
        "{HEADER}import \"test/geometry\" as g;\n\
         function h using v(): Boolean pure {{ true }}\n"
    );
    let cancel = Cancel::new();
    let parsed = parse(
        &request(unit.as_bytes()),
        SpineLimits::default().source,
        &cancel,
    )
    .expect("the importing unit parses")
    .into_value();
    let models = select(
        &parsed,
        &BTreeMap::new(),
        SpineLimits::default().model,
        &cancel,
    )
    .expect("the importing unit selects no model")
    .into_value();
    let run = |edit: fn(&mut SpineLimits, usize)| {
        let (parsed, models, dependencies) = (&parsed, &models, &dependencies);
        move |value: u64| {
            let mut limits = SpineLimits::default();
            edit(
                &mut limits,
                usize::try_from(value).expect("a test bound fits usize"),
            );
            reached(check(
                parsed,
                "fixture".to_owned(),
                models,
                dependencies,
                limits,
                &Cancel::new(),
            ))
        }
    };
    assert_field(
        Setting::DependencyLibraries,
        FoundationKind::NodeCount,
        10,
        run(|limits, value| limits.dependencies.libraries = value),
    );
    assert_field(
        Setting::DependencyImportEdges,
        FoundationKind::EdgeCount,
        10,
        run(|limits, value| limits.dependencies.import_edges = value),
    );
    assert_field(
        Setting::DependencySourceBytes,
        FoundationKind::InputBytes,
        1_000_000,
        run(|limits, value| limits.dependencies.source_bytes = value),
    );
}

/// A reached limit is a limit, never a refusal of the input or a fault, and
/// it reads back as the stage's limit refusal for a caller that renders it.
#[trace("TC-758", "FR-277-AC-1")]
#[test]
fn a_reached_limit_is_a_limit_not_a_fault() {
    let mut limits = SpineLimits::default().source;
    limits.tokens = 1;
    let failure = parse(&request(FIXTURE.as_bytes()), limits, &Cancel::new())
        .expect_err("one token does not hold the fixture");
    assert!(matches!(failure, StageFailure::Limit(_)), "{failure:?}");
    let refusal = refusal_or_fault(failure).expect("a limit is not a fault");
    assert_eq!(refusal.code().as_str(), "stage_limit_exceeded");
    assert_eq!(refusal.stage(), SpineStage::Source);
}

/// FR-275 (TC-769 step 3): an internal invariant failure is a
/// `StageFailure::Fault`, not a panic and not a refusal, and exits as an
/// internal failure.
#[trace("TC-769", "FR-285-AC-3")]
#[test]
fn an_internal_invariant_failure_is_a_fault() {
    use qsl_semantics::check::{CheckCause, CheckRefusal, KeyFault};
    let refusal = Box::new(CompileRefusal::Check {
        refusals: vec![CheckRefusal {
            location: quire_semantic_value::location::Location::root(
                quire_semantic_value::location::Origin::Expression,
            ),
            cause: CheckCause::InternalFault(Box::new(KeyFault::UnresolvedDraft)),
        }],
        region: None,
    });
    let failure = failure_of(refusal.into(), &|_| None);
    assert!(matches!(failure, StageFailure::Fault(_)), "{failure:?}");
    assert!(refusal_or_fault(failure).is_err());
}

/// TC-796 step 4 (FR-303-AC-4), at the spine's failure handling: a checker refusal that names
/// a node rebound from declaration `d1` to `d2` leaves the front end as
/// `runtime_invariant`/`established-invariant-broken` in category internal
/// failure naming the node and both declarations, and no refusal or checked
/// package is returned for the unit.
#[trace("TC-796", "FR-303-AC-4")]
#[test]
fn a_rebound_model_node_leaves_the_front_end_as_an_internal_failure_with_no_package() {
    use qsl_semantics::check::{CheckCause, CheckRefusal, CorrespondenceConflict, KeyFault};
    use quire_checked::key::DeclarationKey;
    let node = quire_exact::NodeKey::new(
        "fixture",
        vec![quire_exact::PathSegment::Text("node".into())],
        7,
    )
    .expect("a fixture key");
    let (d1, d2) = (
        DeclarationKey::fixture("ix://test/orders/Invoice"),
        DeclarationKey::fixture("ix://test/orders/Order"),
    );
    let refusal = Box::new(CompileRefusal::Check {
        refusals: vec![CheckRefusal {
            location: quire_semantic_value::location::Location::root(
                quire_semantic_value::location::Origin::Expression,
            ),
            cause: CheckCause::InternalFault(Box::new(KeyFault::CorrespondenceConflict(Box::new(
                CorrespondenceConflict::NodeRebound {
                    node: node.clone(),
                    recorded: d1.clone(),
                    offered: d2.clone(),
                },
            )))),
        }],
        region: None,
    });
    let failure = failure_of(refusal.into(), &|_| None);
    let fault = refusal_or_fault(failure)
        .expect_err("a rebound node is a fault: the unit yields no refusal and no package");
    assert_eq!(fault.stage(), "S3");
    assert_eq!(fault.invariant(), "model-correspondence-one-to-one");
    assert_eq!(
        fault.named(),
        [
            ("node", format!("{node:?}")),
            ("recorded", format!("{d1:?}")),
            ("offered", format!("{d2:?}")),
        ]
    );
    assert_eq!(
        fault.catalog_code(),
        quire_semantic_value::catalog::CatalogCode::new(
            "runtime_invariant",
            "established-invariant-broken"
        )
    );
    assert_eq!(
        fault.category(),
        quire_semantic_value::catalog::Category::InternalFailure
    );
}

/// The model fixture with an operation `touch` on `Widget` whose `pre`
/// holds `clauses` (`language`, `text`), and the unit selecting it.
fn widget_with_touch(
    clauses: &[(&str, &str)],
) -> (String, qsl_semantics::model::intake::PackageInput) {
    let mut document: serde_json::Value =
        serde_json::from_slice(MODEL_DOCUMENT).expect("the model fixture is JSON");
    let generated = |identity: &str| {
        serde_json::json!({"generated": {
            "generatorIdentity": identity,
            "generatorVersion": "1.0.0",
            "inputIdentities": [identity],
        }})
    };
    let identity = "ix://acme/orders/Widget/touch";
    let widget = document["types"]
        .as_array_mut()
        .expect("types")
        .iter_mut()
        .find(|ty| ty["identity"] == "ix://acme/orders/Widget")
        .expect("the fixture declares Widget");
    widget["operations"] = serde_json::json!([{
        "identity": identity,
        "name": "touch",
        "params": [],
        "pre": clauses
            .iter()
            .map(|(language, text)| serde_json::json!({
                "language": language,
                "text": text,
                "origin": generated(identity),
            }))
            .collect::<Vec<_>>(),
        "post": [],
        "origin": generated(identity),
        "frame": {"modifies": [], "creates": [], "deletes": []},
    }]);
    offered_model(&document)
}

/// The unit selecting `document`, the model fixture with its digest.
fn offered_model(
    document: &serde_json::Value,
) -> (String, qsl_semantics::model::intake::PackageInput) {
    let bytes = serde_json::to_vec(document).expect("the document serializes");
    let packages = qsl_semantics::model::intake::package_input([bytes.as_slice()]).unwrap();
    (spine_model::unit(), packages)
}

/// The model fixture with the clauses `clauses` (`name`, `language`, `text`)
/// on the type `Widget`, and the unit selecting it.
fn widget_with_type_clauses(
    clauses: &[(&str, &str, &str)],
) -> (String, qsl_semantics::model::intake::PackageInput) {
    let mut document: serde_json::Value =
        serde_json::from_slice(MODEL_DOCUMENT).expect("the model fixture is JSON");
    let widget = document["types"]
        .as_array_mut()
        .expect("types")
        .iter_mut()
        .find(|ty| ty["identity"] == "ix://acme/orders/Widget")
        .expect("the fixture declares Widget");
    widget["clauses"] = clauses
        .iter()
        .map(|(name, language, text)| {
            let identity = format!("ix://acme/orders/Widget/{name}");
            serde_json::json!({
                "identity": identity,
                "clauseId": name,
                "language": language,
                "text": text,
                "origin": {"generated": {
                    "generatorIdentity": identity,
                    "generatorVersion": "1.0.0",
                    "inputIdentities": [identity],
                }},
            })
        })
        .collect();
    offered_model(&document)
}

/// FR-366: the driver parses each type-level `Invariant` clause of a
/// selected model and leaves a clause in another language unparsed; an
/// invariant whose text does not parse refuses at I1 with the clause and its
/// `TypeDefinition.clauses[k]` member.
#[trace("TC-1050", "FR-366-AC-3")]
#[test]
fn select_parses_type_invariants_and_refuses_text_that_does_not_parse() {
    let limits = SpineLimits::default();
    let live = Cancel::new();
    let select_unit = |clauses: &[(&str, &str, &str)]| {
        let (unit, packages) = widget_with_type_clauses(clauses);
        let parsed = parse(&request(unit.as_bytes()), limits.source, &live)
            .expect("the unit parses")
            .into_value();
        select(&parsed, &packages, limits.model, &live)
    };
    let models = select_unit(&[
        ("Stable", "quire", "true"),
        ("Foreign", "acme:rules", "not an expression((("),
    ])
    .expect("a quire invariant parses and another language is kept verbatim")
    .into_value();
    let [model] = models.models() else {
        panic!("one model");
    };
    let [parsed] = model.type_invariants.as_slice() else {
        panic!("one parsed invariant");
    };
    assert_eq!(parsed.clause.node, "ix://acme/orders/Widget/Stable");
    assert_eq!(parsed.position, 0);

    let failure = select_unit(&[("Stable", "quire", "true"), ("Broken", "quire", "true and")])
        .expect_err("the second clause does not parse");
    let refusal = refusal(failure);
    let CompileRefusal::TypeInvariant { clause, member, .. } = &*refusal else {
        panic!("expected a type invariant refusal, got {refusal:?}");
    };
    assert_eq!(clause.node, "ix://acme/orders/Widget/Broken");
    assert_eq!(
        *member,
        qsl_semantics::model::refusal::MemberPath::new(
            qsl_semantics::model::refusal::MemberRoot::TypeDefinition
        )
        .name("clauses")
        .index(1)
    );
    assert_eq!(refusal.code(), qsl_foundation::Code::InvalidSyntax);
}

/// FR-103-AC-8: the driver parses each kept `quire` clause of a selected
/// model's operations and leaves a clause in another language unparsed; a
/// kept clause whose text does not parse refuses at I1 with the operation and
/// its `Operation.pre[k]` member, never as a checked package.
#[trace("FR-103-AC-8", "TC-993")]
#[test]
fn select_parses_kept_operation_clauses_and_refuses_text_that_does_not_parse() {
    let limits = SpineLimits::default();
    let live = Cancel::new();
    let select_unit = |clauses: &[(&str, &str)]| {
        let (unit, packages) = widget_with_touch(clauses);
        let parsed = parse(&request(unit.as_bytes()), limits.source, &live)
            .expect("the unit parses")
            .into_value();
        select(&parsed, &packages, limits.model, &live)
    };
    let models = select_unit(&[("quire", "true"), ("acme:rules", "not an expression (((")])
        .expect("a quire clause parses and another language is kept verbatim")
        .into_value();
    let [model] = models.models() else {
        panic!("one model");
    };
    assert_eq!(model.operation_clauses.len(), 1);
    assert_eq!(model.operation_clauses[0].position, 0);

    let failure = select_unit(&[("quire", "true"), ("quire", "true and")])
        .expect_err("the second clause does not parse");
    let refusal = refusal(failure);
    let CompileRefusal::OperationClause {
        operation, member, ..
    } = &*refusal
    else {
        panic!("expected an operation clause refusal, got {refusal:?}");
    };
    assert_eq!(operation.node, "ix://acme/orders/Widget/touch");
    assert_eq!(
        *member,
        qsl_semantics::model::refusal::MemberPath::new(
            qsl_semantics::model::refusal::MemberRoot::Operation
        )
        .name("pre")
        .index(1)
    );
    assert_eq!(refusal.code(), qsl_foundation::Code::InvalidSyntax);
}

/// A check refusal of the type invariant `Widget/Stable` carrying `cause`.
fn type_invariant_check_refusal(cause: qsl_semantics::check::CheckCause) -> CompileRefusal {
    use qsl_semantics::check::{CheckCause, CheckRefusal, TypeInvariantRefusal};
    CompileRefusal::Check {
        refusals: vec![CheckRefusal {
            location: quire_semantic_value::location::Location::root(
                quire_semantic_value::location::Origin::Expression,
            ),
            cause: CheckCause::TypeInvariant(Box::new(TypeInvariantRefusal {
                clause: quire_checked::key::DeclarationKey {
                    package: "acme/orders".to_owned(),
                    node: "ix://acme/orders/Widget/Stable".to_owned(),
                },
                position: 1,
                cause,
            })),
        }],
        region: None,
    }
}

/// FR-366: a stage limit or an internal fault reached while checking a type
/// invariant is a limit or a fault, not a refusal of the input, and the
/// native result names the clause and its `TypeDefinition.clauses[k]`.
#[trace("TC-1050", "FR-366-AC-3")]
#[test]
fn a_limit_or_fault_inside_a_type_invariant_is_read_through_its_wrapper() {
    use qsl_semantics::check::{
        CheckCause, CheckingLimitKind, CheckingStage, KeyFault, StageLimitCause,
    };
    let limit =
        type_invariant_check_refusal(CheckCause::ResourceExhausted(Box::new(StageLimitCause {
            stage: CheckingStage::Typing,
            kind: CheckingLimitKind::Nodes,
            limit: 10,
            actual: 11,
            region: None,
        })));
    let reached = super::limit_of(&limit, &|_| None).expect("a node ceiling is a limit");
    assert_eq!(reached.setting(), Setting::S3Nodes);

    let fault = type_invariant_check_refusal(CheckCause::InternalFault(Box::new(
        KeyFault::UnkeyableRequirements,
    )));
    assert!(super::fault_of(&fault).is_some());

    let named = type_invariant_check_refusal(CheckCause::NonBooleanRoot);
    let fields = named.fields().expect("the refusal names its clause");
    assert_eq!(fields["clause"], "ix://acme/orders/Widget/Stable");
    assert_eq!(fields["member"], "TypeDefinition.clauses[1]");
    assert!(named.to_string().contains("TypeDefinition.clauses[1]"));
}

/// FR-514-AC-1: building the type report resolves every requirement's
/// region without re-walking the location's ancestry per lookup, so a body
/// whose expression tree is 12,000 terms deep reports in time linear in its
/// size. Re-walking costs tens of seconds here; the shared memo costs about a second.
#[trace("FR-514-AC-1")]
#[test]
fn type_report_resolves_a_deep_body_without_rewalking_each_ancestry() {
    const TERMS: usize = 12_000;
    let sum = vec!["x"; TERMS].join(" + ");
    let text = format!("{HEADER}function f using v(x: Int[0, 9]): Boolean pure {{ {sum} < 5 }}\n");
    let cancel = Cancel::new();
    let source_limits = SourceLimits {
        source_bytes: 1 << 28,
        tokens: 1 << 28,
        nodes: 1 << 28,
        work_units: 1 << 20,
    };
    let identity = identity();
    let parsed = parse(
        &ParseRequest {
            source: &identity,
            path: "unit.native",
            bytes: text.as_bytes(),
        },
        source_limits,
        &cancel,
    )
    .expect("the deep source parses under raised limits")
    .into_value();
    let packages = qsl_semantics::model::intake::package_input([MODEL_DOCUMENT])
        .expect("fixture model package input");
    let models = select(
        &parsed,
        &packages,
        ModelNormalizationLimits::default(),
        &cancel,
    )
    .expect("fixture model admits")
    .into_value();
    let raised = SpineLimits {
        source: source_limits,
        checking: SpineLimits::default()
            .checking
            .with_nodes(1 << 40)
            .with_input_bytes(1 << 40)
            .with_work_budget(1 << 40),
        ..SpineLimits::default()
    };
    let checked = check(
        &parsed,
        "deep-body".to_owned(),
        &models,
        &DependencyInput::default(),
        raised,
        &cancel,
    )
    .expect("the deep body checks under raised limits")
    .into_value();
    let started = std::time::Instant::now();
    let report = super::type_report(checked.package().graph(), parsed.source().reference());
    let elapsed = started.elapsed();
    assert!(
        report.requirements().len() >= TERMS,
        "every term of the body has a requirement row: {}",
        report.requirements().len()
    );
    assert!(
        elapsed < std::time::Duration::from_secs(10),
        "the type report took {elapsed:?}"
    );
}
