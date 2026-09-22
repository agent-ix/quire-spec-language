// SPDX-License-Identifier: AGPL-3.0-or-later
//! The complete-V1 `Production` inventory against the accepted EBNF grammar.
use std::collections::BTreeMap;

use ix_trace_rs::trace;
use quire_spec_language::complete::{
    self, CompleteCause, CompleteCode, HostCause, Limits, Production,
};
use quire_spec_language::{SourceIdentity, Span};

// Independently transcribed from the accepted complete-V1 EBNF. This must not
// be generated from the implementation grammar: an omitted implementation
// variant must make this test fail to compile or compare unequal.
const ACCEPTED_PRODUCTIONS: &[Production] = &[
    Production::CompleteUnit,
    Production::Header,
    Production::Profile,
    Production::ImportDeclaration,
    Production::Model,
    Production::Declaration,
    Production::TypeReference,
    Production::QualifiedName,
    Production::ModelName,
    Production::TypeName,
    Production::OperationName,
    Production::ParameterType,
    Production::RoundingMode,
    Production::TextProfile,
    Production::DimensionDeclaration,
    Production::DimensionTerm,
    Production::UnitDeclaration,
    Production::EnumDeclaration,
    Production::EnumMember,
    Production::RecordDeclaration,
    Production::Field,
    Production::TupleDeclaration,
    Production::AliasDeclaration,
    Production::FunctionDeclaration,
    Production::Predicate,
    Production::Parameter,
    Production::StateClause,
    Production::Block,
    Production::Expression,
    Production::Implication,
    Production::Disjunction,
    Production::Conjunction,
    Production::Comparison,
    Production::Sum,
    Production::Product,
    Production::Unary,
    Production::Postfix,
    Production::Primary,
    Production::ExactNumber,
    Production::FloatValue,
    Production::Hex32,
    Production::Hex64,
    Production::HexDigit,
    Production::EnumValue,
    Production::CollectionValue,
    Production::RecordValue,
    Production::FieldValue,
    Production::TupleValue,
    Production::CollectionCall,
    Production::SignedInteger,
    Production::TemporalClause,
    Production::Activation,
    Production::Capture,
    Production::Interval,
    Production::TemporalExpression,
    Production::TemporalImplication,
    Production::TemporalDisjunction,
    Production::TemporalConjunction,
    Production::TemporalRelation,
    Production::TemporalUnary,
    Production::TemporalPrimary,
    Production::ProtocolClause,
    Production::Role,
    Production::RoleLifetime,
    Production::Relationship,
    Production::Channel,
    Production::Ordering,
    Production::DeliveryPolicy,
    Production::Capacity,
    Production::OverflowPolicy,
    Production::ProtocolRequirement,
    Production::NodeReference,
    Production::Compensation,
    Production::Control,
    Production::Sequence,
    Production::Visibility,
    Production::Choice,
    Production::Case,
    Production::Parallel,
    Production::JoinPolicy,
    Production::Branch,
    Production::Repetition,
    Production::AwaitControl,
    Production::Related,
    Production::EventNode,
    Production::Check,
    Production::Commit,
    Production::Finish,
    Production::RelationClause,
    Production::ExecutionBinding,
    Production::HyperClause,
    Production::TraceDomain,
    Production::Quantifier,
    Production::HybridDeclaration,
    Production::HybridMode,
    Production::Equation,
    Production::SynthesisDeclaration,
    Production::VerificationPlan,
    Production::VerificationStep,
];

fn source(declarations: &str) -> String {
    format!(
        "language \"ix:native\" edition \"1-draft\";\nprofile Complete = \"quire.value.complete/v1\" version \"1\" digest \"sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\";\n{declarations}"
    )
}

fn parse(id: &str, text: &str) -> complete::ParsedSource {
    complete::parse(
        SourceIdentity {
            identity: format!("test:{id}"),
            revision: "1".into(),
        },
        format!("{id}.native"),
        text.as_bytes(),
        Limits::default(),
    )
    .unwrap()
}

fn corpus() -> Vec<(&'static str, String)> {
    vec![
        (
            "values",
            source(
                concat!(
                    "import \"acme/base\" version \"1\" digest \"sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb\" as Base;\n",
                    "model M = \"acme/model\" version \"1\" digest \"sha256:cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc\";\n",
                    "dimension Length = M::Length^2 / M::Time;\n",
                    "unit meter: M::Length = rational(1, 1) * M::meter + decimal(0, 0);\n",
                    "ordered enum Color { Red = \"red\", Blue, }\n",
                    "record Reading { amount: Decimal[-2, 4; 18, 4; nearest-even]; label: Option<Text[0, 32; nfc]>?; }\n",
                    "tuple Pair(Integer, Reference<M::Thing>);\n",
                    "type Readings = Sequence<Reading>[0, 100];\n",
                    "function choose using Complete (x: Integer): Integer pure decreases (x) { if true then x else 0 }\n",
                    "function FloatBits using Complete (): Float32[nearest-away] pure { float32(bits: 0x7fc00001) }\n",
                    "function WideBits using Complete (): Float64[nearest-even] pure { float64(bits: 0x7ff8000000000001) }\n",
                    "function enumValue using Complete (): M::Color pure { M::Color::Red }\n",
                    "function tupleValue using Complete (): M::Pair pure { M::Pair(1, 2) }\n",
                    "function values using Complete (): Sequence<Integer>[0, 10] pure { sequence[1, 2, 3] }\n",
                    "function recordValue using Complete (): Reading pure { Reading{amount: decimal(-12, 2), label: \"x\"} }\n",
                    "function folded using Complete (): Integer pure { fold<M::Integer>(acc, item in sequence[1, 2]: acc + item, identity: 0) }\n",
                    "predicate Good using Complete (x: Int[-10, 10]): Boolean { convert<Integer>(x) >= 0 and contains(set[1, 2], x) }\n",
                    "invariant Stable using Complete on M::Thing at current { allInstances<M::Thing>(self)[0].ready }\n",
                    "pre Before using Complete on M::Thing::act { true }\n",
                    "relation Same using Complete over (left: M::Execution, right: M::Execution) { true }\n",
                    "hyper Secret using Complete over traces in M::Execution bounded 2 forall trace a exists trace b { true }\n",
                    "hybrid Plant using Complete { mode Idle invariant { true } flow { M::x' = 0; } transition to Idle when { true } reset { true }; }\n",
                    "synthesis Find using Complete grammar M::Grammar domain M::Domain satisfies { true };\n",
                    "verify Portfolio using Complete { check Safety claim M::Claim method M::Method domain M::Domain depends [Earlier] bound 10; }",
                ),
            ),
        ),
        (
            "temporal",
            source(
                concat!(
                    "temporal Watch using Complete over (sample: M::Sample) clock \"ticks\" on each (trigger: M::Trigger) when (true) {\n",
                    "capture remembered: Integer = 0;\n",
                    "always[0,*] (holds(true) until[0,1] holds(false))\n",
                    "}",
                ),
            ),
        ),
        (
            "protocol",
            source(
                concat!(
                    "protocol Purchase using Complete over (input: M::Input) on origin {\n",
                    "role Buyer on M::Actor;\n",
                    "role Workers each M::Actor from Buyer max 2 lifetime workflow;\n",
                    "relationship Order = M::Order;\n",
                    "channel Messages from Buyer to Workers carries M::Message ordering unordered delivery at-least-once capacity 10 overflow reject;\n",
                    "channel Ordered from Workers to Buyer carries M::Message ordering fifo by (key: Integer) { true } delivery exactly-once-premise capacity symbolic(Cap) overflow block;\n",
                    "requires temporal Watch;\n",
                    "compensate Undo for Main::Tried as (failure: M::Failure) by Buyer on M::Actor::undo using Complete clock \"ticks\" {\n",
                    "activate first (trigger: M::Trigger) when { true } { }\n",
                    "within [0,10]; attempts 2 of M::Attempt;\n",
                    "retry (current_attempt: M::Attempt, prior: M::Attempt) { true };\n",
                    "commit never; recover (recovery: M::Recovery) { true }; }\n",
                    "run sequence Main {\n",
                    "send Sent via Messages as (sent: M::Message) { true };\n",
                    "receive Got via Messages of Main::Sent as (got: M::Message) related by Order (sent, got) { true };\n",
                    "attempt Tried by Buyer on M::Actor::act contracts [] as (tried: M::Attempt) { true };\n",
                    "effect Done of Main::Tried as (done: M::Effect) { true };\n",
                    "event Notice by Buyer for Undo as (notice: M::Notice) { true };\n",
                    "check Static using Complete { true };\n",
                    "commit Saved by Buyer as (saved: M::Save) { true };\n",
                    "choice Choose by Buyer visible () { case yes when { true } sequence Yes { } case no when { false } sequence No { } }\n",
                    "parallel Both { branch left sequence Left { } branch right sequence Right { } } join any [left, right] outstanding continue;\n",
                    "repeat Loop by Buyer visible (true) max 2 invariant { true } variant { 1 } while { false } sequence Body { } exhausted sequence Exhausted { }\n",
                    "await Wait after Main::Sent using Complete clock \"ticks\" within [0,1] match receive Reply via Messages of Main::Sent as (reply: M::Message) { true }; then sequence Then { } timeout sequence Timeout { }\n",
                    "}\n",
                    "finish End as (outcome: Boolean) { true };\n",
                    "}",
                ),
            ),
        ),
    ]
}

#[trace("TC-180", "FR-339-AC-1", "FR-339-AC-2")]
#[test]
fn every_named_production_has_positive_boundary_and_located_negative_vectors() {
    assert_eq!(Production::all(), ACCEPTED_PRODUCTIONS);
    let mut occurrences = BTreeMap::new();
    for (id, text) in corpus() {
        let parsed = parse(id, &text);
        assert!(
            parsed.diagnostics().is_empty(),
            "{id}: {:?}",
            parsed.diagnostics()
        );
        for node in parsed.cst().nodes() {
            let span = node.span();
            assert!(span.start <= span.end && span.end <= text.len());
            assert!(text.is_char_boundary(span.start) && text.is_char_boundary(span.end));
            assert_eq!(
                parsed.cst().render_node(node).unwrap(),
                text.as_bytes()[span.start..span.end],
                "half-open CST boundary for {:?}",
                node.production()
            );
            occurrences
                .entry(node.production())
                .or_insert_with(|| (id, text.clone(), span));
        }
    }

    let missing: Vec<_> = Production::all()
        .iter()
        .filter(|production| !occurrences.contains_key(production))
        .collect();
    assert!(missing.is_empty(), "uncovered productions: {missing:?}");

    for production in Production::all() {
        let (id, source, span) = occurrences.get(production).unwrap();
        assert!(span.start < source.len(), "{production:?} must own a token");
        let mut invalid = source.clone().into_bytes();
        invalid[span.start] = b'@';
        let invalid = String::from_utf8(invalid).unwrap();
        let parsed = parse(id, &invalid);
        assert!(!parsed.is_admissible(), "{production:?} mutation admitted");
        assert_eq!(
            parsed.diagnostics()[0].span.start.byte,
            span.start,
            "{production:?} negative vector must retain its exact locus"
        );

        let boundary = span.end - 1;
        let mut invalid = source.clone().into_bytes();
        invalid[boundary] = b'@';
        let invalid = String::from_utf8(invalid).unwrap();
        let parsed = parse(id, &invalid);
        assert!(
            !parsed.is_admissible(),
            "{production:?} right-boundary mutation admitted"
        );
        assert!(
            parsed.diagnostics().iter().any(|diagnostic| {
                diagnostic.span.start.byte <= boundary && boundary < diagnostic.span.end.byte
            }),
            "{production:?} boundary vector must retain a diagnostic covering its locus"
        );
    }
}

#[trace("TC-180", "FR-339-AC-1", "FR-339-AC-2")]
#[test]
fn grammar_combinators_enforce_authored_boundaries_at_the_failing_token() {
    struct Vector {
        name: &'static str,
        valid: &'static str,
        invalid: &'static str,
        failure_prefix: &'static str,
        failure_token: &'static str,
    }

    let vectors = [
        Vector {
            name: "choice-alternatives",
            valid: concat!(
                "record RoundingChoices { ",
                "exactValue: Decimal[-2, 4; 18, 4; exact]; ",
                "zero: Decimal[-2, 4; 18, 4; toward-zero]; ",
                "positive: Decimal[-2, 4; 18, 4; toward-positive]; ",
                "negative: Decimal[-2, 4; 18, 4; toward-negative]; ",
                "even: Decimal[-2, 4; 18, 4; nearest-even]; ",
                "away: Decimal[-2, 4; 18, 4; nearest-away]; ",
                "}",
            ),
            invalid: "record RoundingChoice { amount: Decimal[-2, 4; 18, 4; sideways]; }",
            failure_prefix: "amount: Decimal[-2, 4; 18, 4; ",
            failure_token: "sideways",
        },
        Vector {
            name: "optional-present-absent-and-repeat-zero-many",
            valid: concat!(
                "dimension Plain; ",
                "dimension Derived = M::Length^2 * M::Time / M::Length;",
            ),
            invalid: "dimension Broken = ;",
            failure_prefix: "dimension Broken = ",
            failure_token: ";",
        },
        Vector {
            name: "repeat-minimum-one",
            valid: "record Minimum { alpha: Integer; beta: Integer; }",
            invalid: "record Empty { }",
            failure_prefix: "record Empty { ",
            failure_token: "}",
        },
        Vector {
            name: "hex32-one-short",
            valid: "function F32 using Complete (): Float32[exact] pure { float32(bits: 0x12345678) }",
            invalid: "function F32 using Complete (): Float32[exact] pure { float32(bits: 0x1234567) }",
            failure_prefix: "float32(bits: 0x1234567",
            failure_token: ")",
        },
        Vector {
            name: "hex32-one-long",
            valid: "function F32 using Complete (): Float32[exact] pure { float32(bits: 0x12345678) }",
            invalid: "function F32 using Complete (): Float32[exact] pure { float32(bits: 0x123456789) }",
            failure_prefix: "float32(bits: 0x12345678",
            failure_token: "9",
        },
        Vector {
            name: "hex64-one-short",
            valid: "function F64 using Complete (): Float64[exact] pure { float64(bits: 0x123456789abcdef0) }",
            invalid: "function F64 using Complete (): Float64[exact] pure { float64(bits: 0x123456789abcdef) }",
            failure_prefix: "float64(bits: 0x123456789abcdef",
            failure_token: ")",
        },
        Vector {
            name: "hex64-one-long",
            valid: "function F64 using Complete (): Float64[exact] pure { float64(bits: 0x123456789abcdef0) }",
            invalid: "function F64 using Complete (): Float64[exact] pure { float64(bits: 0x123456789abcdef01) }",
            failure_prefix: "float64(bits: 0x123456789abcdef0",
            failure_token: "1",
        },
        Vector {
            name: "required-colon",
            valid: "record Required { datum: Integer; }",
            invalid: "record Required { datum Integer; }",
            failure_prefix: "record Required { datum ",
            failure_token: "Integer",
        },
        Vector {
            name: "required-semicolon",
            valid: "record Required { datum: Integer; }",
            invalid: "record Required { datum: Integer }",
            failure_prefix: "record Required { datum: Integer ",
            failure_token: "}",
        },
    ];

    for vector in vectors {
        let valid = source(vector.valid);
        let parsed = parse(vector.name, &valid);
        assert!(
            parsed.is_admissible(),
            "{} positive vector: {:?}",
            vector.name,
            parsed.diagnostics()
        );

        let invalid = source(vector.invalid);
        let expected_start = invalid
            .find(vector.failure_prefix)
            .expect("the table's failure prefix must occur")
            + vector.failure_prefix.len();
        assert_eq!(
            &invalid[expected_start..expected_start + vector.failure_token.len()],
            vector.failure_token,
            "{} table locus",
            vector.name
        );
        let parsed = parse(vector.name, &invalid);
        assert!(!parsed.is_admissible(), "{} negative vector", vector.name);
        let diagnostic = &parsed.diagnostics()[0];
        assert_eq!(
            diagnostic.span.start.byte, expected_start,
            "{} diagnostic start",
            vector.name
        );
        assert_eq!(
            diagnostic.span.end.byte,
            expected_start + vector.failure_token.len(),
            "{} diagnostic end",
            vector.name
        );
    }
}

#[trace("TC-180", "FR-339-AC-1", "FR-339-AC-2")]
#[test]
fn exact_ebnf_name_boundaries_refuse_overbroad_qualified_names() {
    let relationship = source(
        "protocol P using Complete over (input: M::Input) on origin { role R on M::Actor; relationship Link = Local; run sequence Main { } finish End as (outcome: Boolean) { true }; }",
    );
    let parsed = parse("relationship-model-name", &relationship);
    let at = relationship.find("Local;").unwrap() + "Local".len();
    assert!(!parsed.is_admissible());
    assert_eq!(parsed.diagnostics()[0].span.start.byte, at);

    let valid = corpus()
        .into_iter()
        .find(|(id, _)| *id == "protocol")
        .unwrap()
        .1;
    let invalid = valid.replace("M::Actor::undo", "M::Actor");
    let parsed = parse("compensation-operation-name", &invalid);
    let at = invalid.find("M::Actor using").unwrap() + "M::Actor ".len();
    assert!(!parsed.is_admissible());
    assert_eq!(parsed.diagnostics()[0].span.start.byte, at);

    let invalid = valid.replace("attempts 2 of M::Attempt", "attempts 2 of Integer");
    let parsed = parse("compensation-attempt-type-name", &invalid);
    let at = invalid.find("attempts 2 of Integer").unwrap() + "attempts 2 of ".len();
    assert!(!parsed.is_admissible());
    assert_eq!(parsed.diagnostics()[0].span.start.byte, at);

    let invalid = valid.replace(
        "attempt Tried by Buyer on M::Actor::act",
        "attempt Tried by Buyer on Local",
    );
    let parsed = parse("attempt-operation-name", &invalid);
    let at = invalid.find("Local contracts").unwrap() + "Local ".len();
    assert!(!parsed.is_admissible());
    assert_eq!(parsed.diagnostics()[0].span.start.byte, at);
}

#[trace("TC-180", "FR-339-AC-4")]
#[test]
fn undeclared_extension_refuses_at_the_extension_token() {
    let text = source("widget Surprise { }");
    let parsed = parse("unknown", &text);
    let start = text.find("widget").unwrap();
    assert!(!parsed.is_admissible());
    assert_eq!(parsed.diagnostics()[0].span.start.byte, start);
    assert_eq!(parsed.cst().render(), text.as_bytes());
    assert_eq!(
        parsed.cst().recoveries()[0].span,
        Span {
            start,
            end: start + "widget".len(),
        }
    );
}

#[trace("TC-180", "FR-339-AC-1")]
#[test]
fn contextual_complete_words_remain_eligible_identifiers() {
    let contextual = [
        "exact",
        "set",
        "bag",
        "orderedSet",
        "unknown",
        "helper",
        "rec",
        "cast",
        "Tuple",
    ];
    let declarations = contextual
        .iter()
        .map(|word| format!("record {word} {{ {word}: M::{word}; }}"))
        .collect::<Vec<_>>()
        .join("\n");
    let text = source(&declarations);
    let parsed = parse("contextual-identifiers", &text);
    assert!(parsed.is_admissible(), "{:?}", parsed.diagnostics());
    assert_eq!(parsed.cst().render(), text.as_bytes());
}

#[trace("TC-180", "FR-131-AC-2", "FR-339-AC-4")]
#[test]
fn profile_identity_is_syntactic_and_retained_for_package_resolution() {
    let valid = source("record R { datum: Integer; }");
    let unknown = valid.replace("quire.value.complete/v1", "quire.value.unknown/v1");
    let parsed = parse("unknown-profile", &unknown);
    let start = unknown.find("\"quire.value.unknown/v1\"").unwrap();
    assert!(parsed.is_admissible(), "{:?}", parsed.diagnostics());
    assert_eq!(parsed.selections().profiles[0].identity_span.start, start);
    assert_eq!(
        parsed.selections().profiles[0].definition.identity(),
        "quire.value.unknown/v1"
    );
    assert_eq!(parsed.cst().render(), unknown.as_bytes());
}

#[trace("TC-180", "FR-131-AC-2", "FR-339-AC-1")]
#[test]
fn reserved_member_spellings_do_not_create_phantom_profile_selections() {
    let text = source(
        "predicate Member using Complete (): Boolean { M::profile = \"x\" and true and true and true }",
    );
    let parsed = parse("member-profile", &text);
    assert!(parsed.is_admissible(), "{:?}", parsed.diagnostics());
    assert_eq!(parsed.selections().profiles.len(), 1);
    assert_eq!(parsed.selections().profiles[0].alias, "Complete");
}

fn first_diagnostic(id: &str, bytes: &[u8]) -> (CompleteCode, CompleteCause) {
    let identity = SourceIdentity {
        identity: format!("test:{id}"),
        revision: "1".into(),
    };
    match complete::parse(identity, format!("{id}.native"), bytes, Limits::default()) {
        Ok(parsed) => {
            for diagnostic in parsed.diagnostics() {
                assert!(diagnostic.cause.is_cause_of(diagnostic.code), "{id}");
            }
            let first = parsed.diagnostics().first().expect(id);
            (first.code, first.cause)
        }
        Err(refusal) => {
            assert!(refusal.cause.is_cause_of(refusal.code), "{id}");
            (refusal.code, refusal.cause)
        }
    }
}

#[trace("TC-047", "FR-047-AC-3")]
#[test]
fn complete_source_diagnostics_carry_their_catalogued_typed_cause() {
    let profile = |identity: &str, version: &str, digest: &str| {
        format!(
            "language \"ix:native\" edition \"1-draft\";\nprofile Complete = \"{identity}\" version \"{version}\" digest \"{digest}\";\nrecord Reading {{ datum: Integer; }}"
        )
    };
    let digest = format!("sha256:{}", "a".repeat(64));
    let cases: [(&str, Vec<u8>, CompleteCode, CompleteCause); 10] = [
        (
            "unexpected-token",
            source("record Broken { value: Integer }").into_bytes(),
            CompleteCode::InvalidSyntax,
            CompleteCause::UnexpectedToken,
        ),
        (
            "unexpected-end",
            b"language \"ix:native\" edition \"1-draft\"".to_vec(),
            CompleteCode::InvalidSyntax,
            CompleteCause::UnexpectedEnd,
        ),
        (
            "invalid-token",
            source("record Broken { value: Integer; } $").into_bytes(),
            CompleteCode::InvalidSyntax,
            CompleteCause::InvalidToken,
        ),
        (
            "invalid-escape",
            source("ordered enum Color { Red = \"r\\q\" }").into_bytes(),
            CompleteCode::InvalidSyntax,
            CompleteCause::InvalidEscape,
        ),
        (
            "unknown-language",
            b"language \"ix:other\" edition \"1-draft\";".to_vec(),
            CompleteCode::UnknownLanguage,
            CompleteCause::UnsupportedSelection,
        ),
        (
            "invalid-identifier",
            profile("", "1", &digest).into_bytes(),
            CompleteCode::InvalidIdentifier,
            CompleteCause::Host(HostCause::SelectionIdentity),
        ),
        (
            "invalid-digest",
            profile("quire.value.complete/v1", "1", "sha256:AA").into_bytes(),
            CompleteCode::InvalidDigest,
            CompleteCause::Host(HostCause::SelectionDigest),
        ),
        (
            "invalid-utf8",
            vec![0xff],
            CompleteCode::InvalidUtf8,
            CompleteCause::Host(HostCause::InvalidUtf8),
        ),
        (
            "nul",
            b"language\0".to_vec(),
            CompleteCode::InvalidSyntax,
            CompleteCause::InvalidToken,
        ),
        (
            "unnamed",
            Vec::new(),
            CompleteCode::InvalidSourceIdentity,
            CompleteCause::Host(HostCause::UnnamedSource),
        ),
    ];
    for (id, bytes, code, cause) in cases {
        let observed = if id == "unnamed" {
            let refusal = complete::parse(
                SourceIdentity {
                    identity: String::new(),
                    revision: "1".into(),
                },
                "unnamed.native",
                &bytes,
                Limits::default(),
            )
            .unwrap_err();
            (refusal.code, refusal.cause)
        } else {
            first_diagnostic(id, &bytes)
        };
        assert_eq!(observed, (code, cause), "{id}");
        assert!(cause.is_cause_of(code), "{id}");
    }
    assert_eq!(CompleteCause::InvalidEscape.as_str(), "invalid-escape");
    assert_eq!(
        CompleteCause::Host(HostCause::SelectionDigest).as_str(),
        "invalid-digest"
    );
    assert!(!CompleteCause::UnexpectedEnd.is_cause_of(CompleteCode::ResourceExhausted));
}
