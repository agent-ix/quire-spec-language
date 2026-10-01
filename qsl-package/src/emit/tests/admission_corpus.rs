// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-093-AC-19 (TC-416 step 11): the emission-to-admission corpus. One row
//! per node family the emitter writes, each naming an emit fixture. Every
//! fixture is emitted and read back through QSL's I2 read
//! (`read_checked_package_v2`, which runs IR's checked-package/v2 reader),
//! and each row asserts its family is admitted at the emitted `package_id`.
//!
//! The table cannot go stale silently. The families the rows' fixtures hand
//! the emitter, decoded from each checked node's (`node_tag`,
//! `semantic_form`) through IR's own `CheckedNodeKind`, must equal the
//! families the rows name. A fixture that starts emitting a family no row
//! names fails the test, and so does a row whose family its fixture does not
//! emit.
//!
//! Two known gaps are rows of their own, each naming its reason
//! ([`KnownGap`]). Any other refusal, or any other omission, fails the test:
//! it is a bug, not a row.

use quire_contract_ir::{
    BoundedDomainForm, CheckedNodeKind, CheckedPackageRefusalCause, CompositeTypeForm,
    ExpressionForm, FunctionForm, ModelForm, ScalarTypeForm, StateForm,
};

use super::super::extent_agreement::{package_declaring, records};
use super::*;
use crate::checked_v2::{read_checked_package_v2, V2Read, V2ReadRefusal};
use qsl_foundation::diagnostic::{StageFailure, Staged};

/// FR-108's ConfigVersion unit text and domain package (TC-469), the datum
/// `examples/config-version/spine.rs` writes its spine files from.
#[path = "../../../../examples/config-version/spine_unit.rs"]
mod config_version_unit;

/// An emit fixture the corpus emits. Each is an existing fixture of the
/// emit tests, except [`Fixture::CollectionKinds`] and
/// [`Fixture::CyclicEquality`], which no other emit test holds.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
enum Fixture {
    /// TC-416 step 1: `both`, `nb`, `h`, `f` and `t`.
    Tc416Functions,
    /// TC-416 step 5: the recursive `f` of FR-092's G4 to G6.
    RecursiveF,
    /// TC-416 step 6: `record Tree { kids: Sequence<Tree>[0, 3]; }`.
    Tree,
    /// FR-092's D1 and D5: `record Point` and `tuple Pair`.
    DeclaredTypes,
    /// TC-435: `tests/fixtures/spine-compile.native`.
    SpineCompile,
    /// TC-442: `tests/fixtures/spine-model.native` against its domain
    /// package.
    SpineModel,
    /// TC-469: FR-108's ConfigVersion unit against its domain package.
    ConfigVersion,
    /// TC-440: the extent-agreement records.
    ExtentRecords,
    /// TC-416 step 10: the ordered enum compared with `<`.
    OrderedEnum,
    /// TC-416 step 7: one package per application identity the golden
    /// conformance test emits.
    GoldenCases,
    /// TC-160 step 8: `q(a: Length)` beside `t`; `a * a`'s `metre^2`
    /// compound unit names the `metre` unit node.
    QuantityAndT,
    /// [`COLLECTION_KINDS`].
    CollectionKinds,
    /// [`CYCLIC_EQUALITY`].
    CyclicEquality,
}

/// Quantifiers over a set, a bag and an ordered set, and a predicate: the
/// families no other emit fixture writes.
const COLLECTION_KINDS: &str =
    "function qs using v(s: Set<Int[0, 9]>[0, 5]): Boolean pure { forall(x in s: x < 5) }\n\
     function qb using v(s: Bag<Int[0, 9]>[0, 5]): Boolean pure { exists(x in s: x < 5) }\n\
     function qo using v(s: OrderedSet<Int[0, 9]>[0, 5]): Boolean pure { exists(x in s: x < 5) }\n\
     predicate Positive using v(x: Int[0, 9]): Boolean { x > 0 }\n";

/// Structural equality over TC-416's recursive `Tree`, the STD-129 case: a
/// recursive compared type that reaches no `Text`, so it has no leaves
/// (FR-093-AC-11).
const CYCLIC_EQUALITY: &str = "record Tree { kids: Sequence<Tree>[0, 3]; }\n\
    function same using v(a: Tree, b: Tree): Boolean pure { a = b }\n";

/// One package of a fixture, checked, linked and emitted, with the domain
/// package documents its read needs as evidence.
struct Emitted {
    package: CheckedPackage,
    emission: Emission,
    documents: Vec<&'static [u8]>,
}

impl Fixture {
    /// The fixture's packages, each checked, linked and emitted.
    fn emit(self) -> Vec<Emitted> {
        let placed = |package: CheckedPackage| Emitted {
            emission: emit_checked(&package).expect("the fixture emits"),
            package,
            documents: Vec::new(),
        };
        let whole = |package: CheckedPackage| Emitted {
            emission: emit(&package),
            package,
            documents: Vec::new(),
        };
        let one = match self {
            Self::GoldenCases => {
                return golden::cases()
                    .into_iter()
                    .map(|case| whole(case.package))
                    .collect();
            }
            Self::Tc416Functions => whole(package(vec![both(), nb(), h(), f(), t()])),
            Self::RecursiveF => whole(recursive_f()),
            Self::Tree => whole(tree()),
            Self::DeclaredTypes => whole(declared_types(Vec::new(), Vec::new())),
            Self::SpineCompile => placed(spine_compile_package()),
            Self::SpineModel => modelled(&spine_model_without(&[]), SPINE_MODEL_DOCUMENT),
            Self::ConfigVersion => {
                let document = config_version_unit::DOMAIN_PACKAGE.as_bytes();
                let unit = config_version_unit::unit_text(&document_digest(document));
                modelled(&unit, document)
            }
            Self::ExtentRecords => whole(package_declaring(
                TypeEnvironment::new(records(), []).expect("FR-143 admits the records"),
            )),
            Self::OrderedEnum => placed(package_from_text(ORDERED_COMPARISON).1),
            Self::QuantityAndT => whole(q_and_t_package()),
            Self::CollectionKinds => placed(package_from_text(COLLECTION_KINDS).1),
            Self::CyclicEquality => placed(package_from_text(CYCLIC_EQUALITY).1),
        };
        vec![one]
    }
}

/// `document`'s `sha256-jcs` digest, as `intake::package_input` keys it.
fn document_digest(document: &[u8]) -> String {
    let packages = qsl_semantics::model::intake::package_input([document]);
    let [(digest, _)] = packages.iter().collect::<Vec<_>>()[..] else {
        panic!("one supplied document");
    };
    qsl_semantics::model::key::hex(digest)
}

/// `unit` assembled against `document`, checked, linked and emitted.
fn modelled(unit: &str, document: &'static [u8]) -> Emitted {
    let packages = qsl_semantics::model::intake::package_input([document]);
    let package = CheckedPackage::link(
        assemble_with_models(unit.as_bytes(), &packages)
            .expect("the unit assembles")
            .check(CheckingLimits::default())
            .expect("the package checks"),
    );
    Emitted {
        emission: emit_checked(&package).expect("the package emits"),
        package,
        documents: vec![document],
    }
}

/// Why a row's family is not admitted today. These are the only two
/// refusals the corpus holds as rows; each stays until QSpec or QSL rules.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum KnownGap {
    /// STD-129: QSpec FR-322 says nothing about an equality over a cyclic
    /// compared type, and IR refuses one, at the equality node, as
    /// `operation-law-missing` (or `operator-ineligible` after IR #238).
    /// The emitter writes the node; IR's reader refuses the package.
    CyclicComparedType,
    /// QSL-247/IR-450: lowering names a declared unit node by key and does
    /// not build it, so the emitter omits a compound unit over one
    /// (`NamesAbsentNode`), with everything that names it, and IR never
    /// sees a `scalar_type`/`compound_unit` node.
    UndeclaredUnit,
}

/// What a row asserts about its family.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Expect {
    /// IR's reader admits the fixture, at its emitted `package_id`, with a
    /// node of the family.
    Admitted,
    /// The family hits a known gap.
    Known(KnownGap),
}

/// One family, the fixture that emits it, and what the read must show.
#[derive(Clone, Copy, Debug)]
struct Row {
    family: CheckedNodeKind,
    fixture: Fixture,
    expect: Expect,
}

/// Every node family the emitter writes over the corpus, one row each. The
/// cyclic compared type is a second row for `expression`/`binary`: it is a
/// case of an admitted family, not a family of its own.
fn rows() -> Vec<Row> {
    use CheckedNodeKind as K;
    use Expect::{Admitted, Known};
    use Fixture as F;
    let row = |family, fixture, expect| Row {
        family,
        fixture,
        expect,
    };
    vec![
        row(
            K::ScalarType(ScalarTypeForm::Boolean),
            F::Tc416Functions,
            Admitted,
        ),
        row(
            K::ScalarType(ScalarTypeForm::Integer),
            F::Tc416Functions,
            Admitted,
        ),
        row(
            K::ScalarType(ScalarTypeForm::Text),
            F::Tc416Functions,
            Admitted,
        ),
        row(
            K::ScalarType(ScalarTypeForm::Rational),
            F::GoldenCases,
            Admitted,
        ),
        row(
            K::ScalarType(ScalarTypeForm::Float64),
            F::GoldenCases,
            Admitted,
        ),
        row(
            K::ScalarType(ScalarTypeForm::Enum),
            F::OrderedEnum,
            Admitted,
        ),
        row(
            K::ScalarType(ScalarTypeForm::CompoundUnit),
            F::QuantityAndT,
            Known(KnownGap::UndeclaredUnit),
        ),
        row(
            K::CompositeType(CompositeTypeForm::Sequence),
            F::Tree,
            Admitted,
        ),
        row(
            K::CompositeType(CompositeTypeForm::Set),
            F::CollectionKinds,
            Admitted,
        ),
        row(
            K::CompositeType(CompositeTypeForm::Bag),
            F::CollectionKinds,
            Admitted,
        ),
        row(
            K::CompositeType(CompositeTypeForm::OrderedSet),
            F::CollectionKinds,
            Admitted,
        ),
        row(
            K::CompositeType(CompositeTypeForm::Record),
            F::Tree,
            Admitted,
        ),
        row(
            K::CompositeType(CompositeTypeForm::Tuple),
            F::DeclaredTypes,
            Admitted,
        ),
        row(
            K::CompositeType(CompositeTypeForm::Option),
            F::ConfigVersion,
            Admitted,
        ),
        row(
            K::CompositeType(CompositeTypeForm::Reference),
            F::SpineModel,
            Admitted,
        ),
        row(
            K::BoundedDomain(BoundedDomainForm::IntegerRange),
            F::RecursiveF,
            Admitted,
        ),
        row(
            K::BoundedDomain(BoundedDomainForm::RationalRange),
            F::GoldenCases,
            Admitted,
        ),
        row(
            K::BoundedDomain(BoundedDomainForm::FloatRounding),
            F::GoldenCases,
            Admitted,
        ),
        row(
            K::BoundedDomain(BoundedDomainForm::TextBounds),
            F::GoldenCases,
            Admitted,
        ),
        row(
            K::BoundedDomain(BoundedDomainForm::CollectionBounds),
            F::ExtentRecords,
            Admitted,
        ),
        row(K::Value(ValueForm::Literal), F::Tc416Functions, Admitted),
        row(K::Value(ValueForm::EnumValue), F::OrderedEnum, Admitted),
        row(K::Value(ValueForm::Parameter), F::Tc416Functions, Admitted),
        row(
            K::Expression(ExpressionForm::Call),
            F::Tc416Functions,
            Admitted,
        ),
        row(
            K::Expression(ExpressionForm::Unary),
            F::ConfigVersion,
            Admitted,
        ),
        row(
            K::Expression(ExpressionForm::Binary),
            F::Tc416Functions,
            Admitted,
        ),
        row(
            K::Expression(ExpressionForm::Binary),
            F::CyclicEquality,
            Known(KnownGap::CyclicComparedType),
        ),
        row(
            K::Expression(ExpressionForm::Conditional),
            F::Tc416Functions,
            Admitted,
        ),
        row(
            K::Expression(ExpressionForm::Let),
            F::Tc416Functions,
            Admitted,
        ),
        row(
            K::Expression(ExpressionForm::Quantify),
            F::CollectionKinds,
            Admitted,
        ),
        row(
            K::Expression(ExpressionForm::Collection),
            F::GoldenCases,
            Admitted,
        ),
        row(
            K::Expression(ExpressionForm::Conversion),
            F::RecursiveF,
            Admitted,
        ),
        row(
            K::Expression(ExpressionForm::Query),
            F::SpineCompile,
            Admitted,
        ),
        row(
            K::Expression(ExpressionForm::PreRead),
            F::ConfigVersion,
            Admitted,
        ),
        row(
            K::Expression(ExpressionForm::PresenceRead),
            F::ConfigVersion,
            Admitted,
        ),
        row(
            K::Expression(ExpressionForm::ValueRead),
            F::ConfigVersion,
            Admitted,
        ),
        row(
            K::Expression(ExpressionForm::Deref),
            F::SpineModel,
            Admitted,
        ),
        row(
            K::Expression(ExpressionForm::Reachability),
            F::ConfigVersion,
            Admitted,
        ),
        row(
            K::Function(FunctionForm::PureFunction),
            F::Tc416Functions,
            Admitted,
        ),
        row(
            K::Function(FunctionForm::Predicate),
            F::CollectionKinds,
            Admitted,
        ),
        row(
            K::Function(FunctionForm::RecursiveFunction),
            F::RecursiveF,
            Admitted,
        ),
        row(K::Model(ModelForm::ObjectType), F::SpineModel, Admitted),
        row(K::State(StateForm::StateClause), F::ConfigVersion, Admitted),
        row(K::State(StateForm::Frame), F::ConfigVersion, Admitted),
        row(
            K::State(StateForm::OperationAnchor),
            F::ConfigVersion,
            Admitted,
        ),
    ]
}

/// Each checked node's family, decoded through IR's `CheckedNodeKind`, with
/// the nodes of that family.
fn families(package: &CheckedPackage) -> BTreeMap<CheckedNodeKind, Vec<CheckedNodeId>> {
    let mut families: BTreeMap<CheckedNodeKind, Vec<CheckedNodeId>> = BTreeMap::new();
    for node in package.graph().semantic_graph().nodes() {
        let kind = CheckedNodeKind::decode(ir_tag(node.node_tag()), node.semantic_form())
            .unwrap_or_else(|| {
                panic!(
                    "{:?}/{} is no IR v2 family",
                    node.node_tag(),
                    node.semantic_form()
                )
            });
        families.entry(kind).or_default().push(node_id(node.key()));
    }
    families
}

/// QSL's I2 read of `emitted`, pinned at its own `package_id`, with the
/// lock's required features and the fixture's domain package documents as
/// evidence.
fn read(emitted: &Emitted) -> Result<V2Read, StageFailure<V2ReadRefusal>> {
    let mut evidence = read_evidence(&emitted.emission);
    for document in &emitted.documents {
        evidence.insert_domain_package_document(document_digest(document), document.to_vec());
    }
    let pinned: PinnedRequest = qsl_semantics::library::fixtures::single_pin(
        library(),
        Selection {
            version: "1".to_owned(),
            package_id: emitted.emission.package.package_id(),
        },
    );
    read_checked_package_v2(
        emitted.emission.package.bytes(),
        library(),
        "1".to_owned(),
        V2ReadLimits::default(),
        &evidence,
        &pinned,
    )
    .map(Staged::into_value)
}

/// One emitted package of a fixture and what QSL's I2 read made of it.
struct Outcome {
    emitted: Emitted,
    families: BTreeMap<CheckedNodeKind, Vec<CheckedNodeId>>,
    read: Result<V2Read, StageFailure<V2ReadRefusal>>,
}

/// The family of `node` in `outcome`'s checked graph.
fn family_of(outcome: &Outcome, node: &CheckedNodeId) -> Option<CheckedNodeKind> {
    outcome
        .families
        .iter()
        .find(|(_, nodes)| nodes.contains(node))
        .map(|(family, _)| *family)
}

/// Whether `node` is an application of `quire.op.structural.eq` in
/// `outcome`'s checked graph.
fn is_structural_equality(outcome: &Outcome, node: &CheckedNodeId) -> bool {
    outcome
        .emitted
        .package
        .graph()
        .semantic_graph()
        .nodes()
        .find(|candidate| node_id(candidate.key()) == *node)
        .is_some_and(|candidate| {
            matches!(
                candidate.body(),
                SemanticTerm::Application { operation, .. }
                    if operation.identity() == "quire.op.structural.eq"
            )
        })
}

/// The checks of one row against its fixture's outcomes; each failure is a
/// message naming the row.
fn check_row(row: &Row, outcomes: &[Outcome], failures: &mut Vec<String>) {
    let Row {
        family,
        fixture,
        expect,
    } = *row;
    let mut fail = |message: String| failures.push(format!("{family:?} ({fixture:?}): {message}"));
    match expect {
        Expect::Admitted => {
            let admitted = outcomes.iter().any(|outcome| {
                outcome
                    .read
                    .as_ref()
                    .is_ok_and(|read| read.admitted.node_kinds().contains(&family))
            });
            if !admitted {
                fail("IR admits no node of the family".to_owned());
            }
        }
        Expect::Known(KnownGap::UndeclaredUnit) => {
            if !outcomes
                .iter()
                .any(|outcome| outcome.families.contains_key(&family))
            {
                fail("the fixture builds no node of the family".to_owned());
            }
            for outcome in outcomes {
                for node in outcome.families.get(&family).into_iter().flatten() {
                    let omitted = outcome.emitted.emission.omitted.iter().any(|omission| {
                        omission.node == *node
                            && matches!(omission.cause, OmissionCause::NamesAbsentNode(_))
                    });
                    if !omitted {
                        fail(format!("{node:?} is not omitted for an absent unit"));
                    }
                }
            }
        }
        Expect::Known(KnownGap::CyclicComparedType) => {
            for outcome in outcomes {
                match &outcome.read {
                    Err(StageFailure::Refused(V2ReadRefusal::Envelope { refusal, .. }))
                        if matches!(
                            refusal.cause,
                            Some(
                                CheckedPackageRefusalCause::OperationLawMissing
                                    | CheckedPackageRefusalCause::OperatorIneligible
                            )
                        ) && refusal.locus.as_ref().is_some_and(|node| {
                            family_of(outcome, node) == Some(family)
                                && is_structural_equality(outcome, node)
                        }) => {}
                    Err(other) => fail(format!("refused for another reason: {other:?}")),
                    Ok(_) => fail("IR admits it: retire the known gap".to_owned()),
                }
            }
        }
    }
}

/// The checks of one fixture's packages: each is admitted at its own
/// `package_id` unless a row says the fixture hits the cyclic compared type
/// gap, and each omission roots in a row's undeclared unit gap: a node of
/// that row's family, or another node naming the same absent unit, omitted
/// for the absent unit, and the nodes naming either.
fn check_fixture(fixture: Fixture, rows: &[Row], outcomes: &[Outcome], failures: &mut Vec<String>) {
    let refused = rows.iter().any(|row| {
        row.fixture == fixture && row.expect == Expect::Known(KnownGap::CyclicComparedType)
    });
    let unit_gaps: BTreeSet<CheckedNodeKind> = rows
        .iter()
        .filter(|row| {
            row.fixture == fixture && row.expect == Expect::Known(KnownGap::UndeclaredUnit)
        })
        .map(|row| row.family)
        .collect();
    for outcome in outcomes {
        if !refused {
            let emitted_at = outcome.emitted.emission.package.package_id();
            match &outcome.read {
                Ok(read) if read.package.package_id() == emitted_at => {}
                Ok(read) => failures.push(format!(
                    "{fixture:?}: admitted at {:?}, emitted at {emitted_at:?}",
                    read.package.package_id(),
                )),
                Err(failure) => failures.push(format!("{fixture:?}: not admitted: {failure:?}")),
            }
        }
        let omitted = &outcome.emitted.emission.omitted;
        // The absent units the gap families' nodes name.
        let absent_units: BTreeSet<&CheckedNodeId> = omitted
            .iter()
            .filter(|omission| {
                family_of(outcome, &omission.node).is_some_and(|family| unit_gaps.contains(&family))
            })
            .filter_map(|omission| match &omission.cause {
                OmissionCause::NamesAbsentNode(unit) => Some(unit),
                _ => None,
            })
            .collect();
        for omission in omitted {
            let accounted = match &omission.cause {
                OmissionCause::NamesOmittedNode(_) => true,
                OmissionCause::NamesAbsentNode(absent) => absent_units.contains(absent),
                OmissionCause::UnsupportedForm { .. }
                | OmissionCause::DeclarationOccurrenceMismatch
                | OmissionCause::UnlockedOwner => false,
            };
            if !accounted {
                failures.push(format!("{fixture:?}: unexpected omission {omission:?}"));
            }
        }
    }
}

/// FR-093-AC-19 (TC-416 step 11): every node family the emitter writes over
/// the emit fixtures is admitted by IR's checked-package/v2 reader, through
/// QSL's I2 read, at the emitted `package_id`, except the two known gaps,
/// each a row naming its reason. The rows' families equal the families the
/// fixtures emit, so a new family without a row fails here.
#[trace("FR-093-AC-19", "TC-416")]
#[test]
fn every_emitted_node_family_is_admitted_at_its_package_id() {
    let rows = rows();
    let fixtures: BTreeSet<Fixture> = rows.iter().map(|row| row.fixture).collect();
    let outcomes: BTreeMap<Fixture, Vec<Outcome>> = fixtures
        .iter()
        .map(|&fixture| {
            let outcomes = fixture
                .emit()
                .into_iter()
                .map(|emitted| Outcome {
                    families: families(&emitted.package),
                    read: read(&emitted),
                    emitted,
                })
                .collect();
            (fixture, outcomes)
        })
        .collect();

    // The table covers exactly the families the fixtures emit.
    let emitted: BTreeSet<CheckedNodeKind> = outcomes
        .values()
        .flatten()
        .flat_map(|outcome| outcome.families.keys().copied())
        .collect();
    let named: BTreeSet<CheckedNodeKind> = rows.iter().map(|row| row.family).collect();
    let without_row: Vec<_> = emitted.difference(&named).collect();
    let never_emitted: Vec<_> = named.difference(&emitted).collect();
    assert!(
        without_row.is_empty() && never_emitted.is_empty(),
        "emitted families with no row: {without_row:?}; rows naming no emitted family: \
         {never_emitted:?}"
    );

    // One row per family; the cyclic compared type is an extra case.
    let mut primary: BTreeMap<CheckedNodeKind, usize> = BTreeMap::new();
    for row in &rows {
        if row.expect != Expect::Known(KnownGap::CyclicComparedType) {
            *primary.entry(row.family).or_default() += 1;
        }
    }
    let duplicated: Vec<_> = named
        .iter()
        .filter(|family| primary.get(family) != Some(&1))
        .collect();
    assert!(
        duplicated.is_empty(),
        "families without exactly one row: {duplicated:?}"
    );

    let mut failures = Vec::new();
    for (fixture, outcomes) in &outcomes {
        check_fixture(*fixture, &rows, outcomes, &mut failures);
    }
    for row in &rows {
        check_row(row, &outcomes[&row.fixture], &mut failures);
    }
    assert!(failures.is_empty(), "{failures:#?}");
}
