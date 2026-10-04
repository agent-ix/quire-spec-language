// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-093-AC-19 (TC-416 step 11): the emission-to-admission corpus. Every
//! node family of IR's closed checked-package/v2 vocabulary
//! (`CheckedNodeKind::all()`) is classified exactly once: as a family the
//! emitter writes and IR admits (one or more rows), or as a kind QSL's
//! lowering never writes ([`NOT_EMITTED_BY_QSL`], each with its reason).
//!
//! Every fixture is emitted and read back through QSL's I2 read
//! (`read_checked_package_v2`, which runs IR's checked-package/v2 reader).
//! An admitted row asserts IR admits a node of its family at the emitted
//! `package_id`. The families each fixture hands the emitter, decoded
//! through IR's `CheckedNodeKind`, must all be row families, so a fixture
//! that starts emitting an unclassified or not-emitted kind fails. A new IR
//! kind fails until it is classified.
//!
//! Any refusal, and any omission, fails the test: it is a bug, not a row.

use qsl_forms::TypeFormHead;
use quire_contract_model::{
    BoundedDomainForm, CheckedNodeKind, ClaimForm, CompositeTypeForm, CorrespondenceForm,
    ExpressionForm, FunctionForm, ModelForm, ProtocolForm, RelationForm, ScalarTypeForm, StateForm,
    TemporalForm,
};

use super::super::extent_agreement::{package_declaring, records};
use super::*;
use crate::checked_v2::{read_checked_package_v2, V2Read, V2ReadRefusal};
use qsl_foundation::diagnostic::{StageFailure, Staged};

/// FR-108's ConfigVersion unit text and domain package (TC-469), the datum
/// `examples/config-version/spine.rs` writes its spine files from.
#[path = "../../../../examples/config-version/spine_unit.rs"]
mod config_version_unit;

/// An emit fixture the corpus emits. Each reuses an emit-test fixture or
/// helper; [`FORMS_UNIT`], [`CYCLIC_EQUALITY`] and the systems-interface
/// document are the only new units, for families no other fixture writes.
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
    /// TC-442's spine-model unit with a `Population<M::Gadget>[3]`
    /// parameter, the form FR-093-AC-8 lowers (source text spells no
    /// `Population` type, so the function is built as a form).
    ModelPopulation,
    /// A `Reference` to the systems interface `S::Sensor` of
    /// `tests/fixtures/systems-interface.semantic-ir.json`.
    SystemsInterface,
    /// TC-469: FR-108's ConfigVersion unit against its domain package.
    ConfigVersion,
    /// TC-440: the extent-agreement records.
    ExtentRecords,
    /// TC-416 step 10: the ordered enum compared with `<`.
    OrderedEnum,
    /// TC-416 step 7: one package per application identity the golden
    /// conformance test emits.
    GoldenCases,
    /// The golden conformance test's `fadd` over `Float32[nearest-even]`.
    Float32Add,
    /// TC-160 step 8's `q(a: Length)` beside `t`, over a `metre` unit the
    /// fixture unit's own source declares; `a * a`'s `metre^2` compound unit
    /// names the `metre` unit node, which is typed by the `Length`
    /// dimension node.
    QuantityAndT,
    /// [`FORMS_UNIT`].
    FormsUnit,
    /// [`CYCLIC_EQUALITY`].
    CyclicEquality,
}

impl Fixture {
    /// Every fixture; each must back at least one row.
    const ALL: [Self; 16] = [
        Self::Tc416Functions,
        Self::RecursiveF,
        Self::Tree,
        Self::DeclaredTypes,
        Self::SpineCompile,
        Self::SpineModel,
        Self::ModelPopulation,
        Self::SystemsInterface,
        Self::ConfigVersion,
        Self::ExtentRecords,
        Self::OrderedEnum,
        Self::GoldenCases,
        Self::Float32Add,
        Self::QuantityAndT,
        Self::FormsUnit,
        Self::CyclicEquality,
    ];
}

/// Quantifiers over a set, a bag and an ordered set, a predicate, a decimal
/// comparison, and record and tuple values: families no other emit fixture
/// writes.
const FORMS_UNIT: &str =
    "function qs using v(s: Set<Int[0, 9]>[0, 5]): Boolean pure { forall(x in s: x < 5) }\n\
     function qb using v(s: Bag<Int[0, 9]>[0, 5]): Boolean pure { exists(x in s: x < 5) }\n\
     function qo using v(s: OrderedSet<Int[0, 9]>[0, 5]): Boolean pure { exists(x in s: x < 5) }\n\
     predicate Positive using v(x: Int[0, 9]): Boolean { x > 0 }\n\
     function d using v(x: Decimal[-100, 100; 0, 2; nearest-even], \
     y: Decimal[-100, 100; 0, 2; nearest-even]): Boolean pure { x < y }\n\
     record P { x: Int[0, 9]; }\n\
     tuple T(Int[0, 9], Int[0, 9]);\n\
     function mk using v(a: Int[0, 9]): P pure { P { x: a } }\n\
     function mt using v(a: Int[0, 9]): T pure { T(a, a) }\n";

/// Structural equality over TC-416's recursive `Tree`, the STD-129 case: a
/// recursive compared type that reaches no `Text`, so it has no leaves
/// (FR-093-AC-11).
const CYCLIC_EQUALITY: &str = "record Tree { kids: Sequence<Tree>[0, 3]; }\n\
    function same using v(a: Tree, b: Tree): Boolean pure { a = b }\n";

/// The domain package declaring the systems interface `Sensor`.
const SYSTEMS_DOCUMENT: &[u8] =
    include_bytes!("../../../../tests/fixtures/systems-interface.semantic-ir.json");

/// One package of a fixture, checked, linked and emitted, with the domain
/// package documents its read needs as evidence.
struct Emitted {
    package: CheckedPackage,
    emission: Emission,
    documents: Vec<&'static [u8]>,
}

impl Emitted {
    /// `package` emitted placing each occurrence at its form span.
    fn placed(package: CheckedPackage, documents: Vec<&'static [u8]>) -> Self {
        Self {
            emission: emit_checked(&package).expect("the fixture emits"),
            package,
            documents,
        }
    }

    /// `package` emitted placing every occurrence at the whole fixture
    /// unit, for packages built from forms without spans.
    fn whole(package: CheckedPackage, documents: Vec<&'static [u8]>) -> Self {
        Self {
            emission: emit(&package),
            package,
            documents,
        }
    }
}

impl Fixture {
    /// The fixture's packages, each checked, linked and emitted.
    fn emit(self) -> Vec<Emitted> {
        let whole = |package| Emitted::whole(package, Vec::new());
        let text = |declarations| Emitted::placed(package_from_text(declarations).1, Vec::new());
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
            Self::SpineCompile => Emitted::placed(spine_compile_package(), Vec::new()),
            Self::SpineModel => Emitted::placed(
                modelled(&spine_model_without(&[]), SPINE_MODEL_DOCUMENT, Vec::new()),
                vec![SPINE_MODEL_DOCUMENT],
            ),
            Self::ModelPopulation => {
                let population = TypeForm::new(TypeFormHead::Population, SPAN)
                    .with_arguments(vec![TypeForm::name("M::Gadget", SPAN)])
                    .with_bounds(vec!["3".to_owned()]);
                let pop = FunctionDeclaration::new(
                    "pop",
                    vec![("p".to_owned(), population)],
                    boolean(),
                    None,
                    Expression::boolean(true),
                );
                Emitted::whole(
                    modelled(&spine_model_without(&[]), SPINE_MODEL_DOCUMENT, vec![pop]),
                    vec![SPINE_MODEL_DOCUMENT],
                )
            }
            Self::SystemsInterface => {
                let unit = format!(
                    "language \"ix:native\" edition \"1-draft\";\n\
                     profile v = \"quire.value.complete/v1\";\n\
                     model S = \"acme/systems\" version \"1.0.0\" digest \"sha256-jcs:{}\";\n\
                     function keep using v(s: S::Sensor): S::Sensor pure {{ s }}\n",
                    document_digest(SYSTEMS_DOCUMENT)
                );
                Emitted::placed(
                    modelled(&unit, SYSTEMS_DOCUMENT, Vec::new()),
                    vec![SYSTEMS_DOCUMENT],
                )
            }
            Self::ConfigVersion => {
                let document = config_version_unit::DOMAIN_PACKAGE.as_bytes();
                let unit = config_version_unit::unit_text(&document_digest(document));
                Emitted::placed(modelled(&unit, document, Vec::new()), vec![document])
            }
            Self::ExtentRecords => whole(package_declaring(
                TypeEnvironment::new(records(), []).expect("FR-143 admits the records"),
            )),
            Self::OrderedEnum => text(ORDERED_COMPARISON),
            Self::Float32Add => whole(golden::float_add_of(BuiltinType::Float32, "nearest-even")),
            Self::QuantityAndT => {
                let (units, metre) = source_metre_units();
                whole(q_and_t_package_over(units, metre))
            }
            Self::FormsUnit => text(FORMS_UNIT),
            Self::CyclicEquality => text(CYCLIC_EQUALITY),
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

/// `unit` assembled against `document`, with `functions` added, checked
/// and linked.
fn modelled(
    unit: &str,
    document: &'static [u8],
    functions: Vec<FunctionDeclaration>,
) -> CheckedPackage {
    let packages = qsl_semantics::model::intake::package_input([document]);
    let mut declarations =
        assemble_with_models(unit.as_bytes(), &packages).expect("the unit assembles");
    declarations.functions.extend(functions);
    CheckedPackage::link(
        declarations
            .check(CheckingLimits::default())
            .expect("the package checks"),
    )
}

/// One family and the fixture that emits it: IR's reader admits the
/// fixture, at its emitted `package_id`, with a node of the family, and
/// when `operation` is named, a node of the family that applies it.
#[derive(Clone, Copy, Debug)]
struct Row {
    family: CheckedNodeKind,
    fixture: Fixture,
    operation: Option<&'static str>,
}

/// Every node family the emitter writes, one row each. The cyclic compared
/// type is a second row for `expression`/`binary`: an admitted case of that
/// family, not a family of its own.
fn rows() -> Vec<Row> {
    use BoundedDomainForm as B;
    use CheckedNodeKind as K;
    use CompositeTypeForm as C;
    use ExpressionForm as E;
    use Fixture as F;
    use ScalarTypeForm as S;
    let row = |family, fixture| Row {
        family,
        fixture,
        operation: None,
    };
    vec![
        row(K::ScalarType(S::Boolean), F::Tc416Functions),
        row(K::ScalarType(S::Integer), F::Tc416Functions),
        row(K::ScalarType(S::Rational), F::GoldenCases),
        row(K::ScalarType(S::Decimal), F::FormsUnit),
        row(K::ScalarType(S::Float32), F::Float32Add),
        row(K::ScalarType(S::Float64), F::GoldenCases),
        row(K::ScalarType(S::Text), F::Tc416Functions),
        row(K::ScalarType(S::Enum), F::OrderedEnum),
        row(K::ScalarType(S::Dimension), F::QuantityAndT),
        row(K::ScalarType(S::Unit), F::QuantityAndT),
        row(K::ScalarType(S::CompoundUnit), F::QuantityAndT),
        row(K::CompositeType(C::Option), F::ConfigVersion),
        row(K::CompositeType(C::Sequence), F::Tree),
        row(K::CompositeType(C::Set), F::FormsUnit),
        row(K::CompositeType(C::Bag), F::FormsUnit),
        row(K::CompositeType(C::OrderedSet), F::FormsUnit),
        row(K::CompositeType(C::Record), F::Tree),
        row(K::CompositeType(C::Tuple), F::DeclaredTypes),
        row(K::CompositeType(C::Reference), F::SpineModel),
        row(K::BoundedDomain(B::IntegerRange), F::RecursiveF),
        row(K::BoundedDomain(B::RationalRange), F::GoldenCases),
        row(K::BoundedDomain(B::DecimalRange), F::FormsUnit),
        row(K::BoundedDomain(B::FloatRounding), F::GoldenCases),
        row(K::BoundedDomain(B::TextBounds), F::GoldenCases),
        row(K::BoundedDomain(B::CollectionBounds), F::ExtentRecords),
        row(K::BoundedDomain(B::ModelPopulation), F::ModelPopulation),
        row(K::Value(ValueForm::Literal), F::Tc416Functions),
        row(K::Value(ValueForm::EnumValue), F::OrderedEnum),
        row(K::Value(ValueForm::RecordValue), F::FormsUnit),
        row(K::Value(ValueForm::TupleValue), F::FormsUnit),
        row(K::Value(ValueForm::Parameter), F::Tc416Functions),
        row(K::Expression(E::Call), F::Tc416Functions),
        row(K::Expression(E::Unary), F::ConfigVersion),
        row(K::Expression(E::Binary), F::Tc416Functions),
        Row {
            family: K::Expression(E::Binary),
            fixture: F::CyclicEquality,
            operation: Some("quire.op.structural.eq"),
        },
        row(K::Expression(E::Conditional), F::Tc416Functions),
        row(K::Expression(E::Let), F::Tc416Functions),
        row(K::Expression(E::Quantify), F::FormsUnit),
        row(K::Expression(E::Collection), F::GoldenCases),
        row(K::Expression(E::Conversion), F::RecursiveF),
        row(K::Expression(E::Query), F::SpineCompile),
        row(K::Expression(E::PreRead), F::ConfigVersion),
        row(K::Expression(E::PresenceRead), F::ConfigVersion),
        row(K::Expression(E::ValueRead), F::ConfigVersion),
        row(K::Expression(E::Deref), F::SpineModel),
        row(K::Expression(E::Reachability), F::ConfigVersion),
        row(K::Function(FunctionForm::PureFunction), F::Tc416Functions),
        row(K::Function(FunctionForm::Predicate), F::FormsUnit),
        row(K::Function(FunctionForm::RecursiveFunction), F::RecursiveF),
        row(K::Model(ModelForm::ObjectType), F::SpineModel),
        row(K::Model(ModelForm::SystemsInterface), F::SystemsInterface),
        row(K::State(StateForm::StateClause), F::ConfigVersion),
        row(K::State(StateForm::Frame), F::ConfigVersion),
        row(K::State(StateForm::OperationAnchor), F::ConfigVersion),
    ]
}

/// Every IR node kind QSL's lowering never writes, each with the reason.
const NOT_EMITTED_BY_QSL: &[(CheckedNodeKind, &str)] = {
    use CheckedNodeKind as K;
    const NO_RECORD_FORM: &str =
        "lowering's record_form maps only object types and systems interfaces to model nodes";
    const NO_TAG: &str = "lowering builds no node of this tag";
    &[
        (
            K::CompositeType(CompositeTypeForm::Alias),
            "an alias resolves to its target type's node",
        ),
        (
            K::Value(ValueForm::CollectionValue),
            "a collection literal lowers to an expression/collection node, and a constant \
             Value::Collection is refused as UnbuiltLiteral",
        ),
        (
            K::CompositeType(CompositeTypeForm::Union),
            "lowering builds no union type; the checker has no union type form",
        ),
        (
            K::Value(ValueForm::UnionValue),
            "lowering builds no union value; the checker has no union type form",
        ),
        (
            K::Expression(ExpressionForm::Case),
            "lowering builds no case expression; the value expression grammar has none",
        ),
        (
            K::Value(ValueForm::OptionValue),
            "lowering builds no option literal (UnbuiltLiteral)",
        ),
        (
            K::Expression(ExpressionForm::Reference),
            "a name read lowers to a reference term, not a node",
        ),
        (K::Model(ModelForm::ModelImport), NO_RECORD_FORM),
        (K::Model(ModelForm::ValueType), NO_RECORD_FORM),
        (K::Model(ModelForm::VariantType), NO_RECORD_FORM),
        (K::Model(ModelForm::RecordValueType), NO_RECORD_FORM),
        (K::Model(ModelForm::EventType), NO_RECORD_FORM),
        (K::Model(ModelForm::StateMachine), NO_RECORD_FORM),
        (K::Model(ModelForm::Process), NO_RECORD_FORM),
        (K::Model(ModelForm::PersistenceInterface), NO_RECORD_FORM),
        (K::Model(ModelForm::Namespace), NO_RECORD_FORM),
        (K::Model(ModelForm::SystemsPart), NO_RECORD_FORM),
        (K::Model(ModelForm::SystemsPort), NO_RECORD_FORM),
        (K::Model(ModelForm::SystemsConnection), NO_RECORD_FORM),
        (K::Model(ModelForm::SystemsAllocation), NO_RECORD_FORM),
        (
            K::Relation(RelationForm::Relationship),
            "record_form maps a relationship, but no checked construct names one: model \
             nodes are built for object types (references, frame owners, creates and \
             deletes), and intake refuses a relationship in a frame",
        ),
        (K::Relation(RelationForm::Population), NO_RECORD_FORM),
        (K::Relation(RelationForm::Membership), NO_RECORD_FORM),
        (K::Relation(RelationForm::CausalRelation), NO_RECORD_FORM),
        (K::State(StateForm::Transition), NO_TAG),
        (K::State(StateForm::Snapshot), NO_TAG),
        (K::Temporal(TemporalForm::TemporalClause), NO_TAG),
        (K::Temporal(TemporalForm::Formula), NO_TAG),
        (K::Temporal(TemporalForm::Fairness), NO_TAG),
        (K::Temporal(TemporalForm::Clock), NO_TAG),
        (K::Temporal(TemporalForm::Window), NO_TAG),
        (K::Temporal(TemporalForm::Activation), NO_TAG),
        (K::Temporal(TemporalForm::Deadline), NO_TAG),
        (K::Protocol(ProtocolForm::ProtocolClause), NO_TAG),
        (K::Protocol(ProtocolForm::Role), NO_TAG),
        (K::Protocol(ProtocolForm::Channel), NO_TAG),
        (K::Protocol(ProtocolForm::Queue), NO_TAG),
        (K::Protocol(ProtocolForm::Control), NO_TAG),
        (K::Protocol(ProtocolForm::Obligation), NO_TAG),
        (K::Protocol(ProtocolForm::Compensation), NO_TAG),
        (K::Claim(ClaimForm::VerificationClaim), NO_TAG),
        (K::Claim(ClaimForm::AnalysisClaim), NO_TAG),
        (K::Claim(ClaimForm::Hyperproperty), NO_TAG),
        (K::Claim(ClaimForm::SynthesisRequest), NO_TAG),
        (K::Correspondence(CorrespondenceForm::SourceLocus), NO_TAG),
        (
            K::Correspondence(CorrespondenceForm::ModelCorrespondence),
            NO_TAG,
        ),
        (K::Correspondence(CorrespondenceForm::BindingRole), NO_TAG),
        (
            K::Correspondence(CorrespondenceForm::ProfileCorrespondence),
            NO_TAG,
        ),
    ]
};

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
            package_id: emitted.emission.package.package_id(),
        },
    );
    read_checked_package_v2(
        emitted.emission.package.bytes(),
        library(),
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

impl Outcome {
    /// The checked node `node`.
    fn node(&self, node: &CheckedNodeId) -> Option<&SemanticNode> {
        self.emitted
            .package
            .graph()
            .semantic_graph()
            .nodes()
            .find(|candidate| node_id(candidate.key()) == *node)
    }

    /// Whether `node` is an application of the operation `identity`.
    fn applies(&self, node: &CheckedNodeId, identity: &str) -> bool {
        self.node(node).is_some_and(|candidate| {
            matches!(
                candidate.body(),
                qsl_semantics::check::BodyTerm::Application(qsl_semantics::check::ApplicationTerm { operation, .. })
                    if operation.identity() == identity
            )
        })
    }
}

/// The checks of one row against its fixture's outcomes; each failure is a
/// message naming the row.
fn check_row(row: &Row, outcomes: &[Outcome], failures: &mut Vec<String>) {
    let Row {
        family,
        fixture,
        operation,
    } = *row;
    let admitted = outcomes.iter().any(|outcome| {
        outcome.read.as_ref().is_ok_and(|read| {
            read.admitted
                .graph()
                .nodes
                .iter()
                .zip(read.admitted.node_kinds())
                .any(|(node, kind)| {
                    *kind == family
                        && operation.is_none_or(|identity| outcome.applies(&node.node_id, identity))
                })
        })
    });
    if !admitted {
        let applying = operation.map_or(String::new(), |identity| format!(" applying {identity}"));
        failures.push(format!(
            "{family:?} ({fixture:?}): IR admits no node of the family{applying}"
        ));
    }
}

/// The checks of one fixture's packages: each is admitted at its own
/// `package_id`, and none omits a node.
fn check_fixture(fixture: Fixture, rows: &[Row], outcomes: &[Outcome], failures: &mut Vec<String>) {
    if !rows.iter().any(|row| row.fixture == fixture) {
        failures.push(format!("{fixture:?}: backs no row"));
    }
    for outcome in outcomes {
        let emitted_at = outcome.emitted.emission.package.package_id();
        match &outcome.read {
            Ok(read) if read.package.package_id() == emitted_at => {}
            Ok(read) => failures.push(format!(
                "{fixture:?}: admitted at {:?}, emitted at {emitted_at:?}",
                read.package.package_id(),
            )),
            Err(failure) => failures.push(format!("{fixture:?}: not admitted: {failure:?}")),
        }
        for omission in &outcome.emitted.emission.omitted {
            failures.push(format!("{fixture:?}: unexpected omission {omission:?}"));
        }
    }
}

/// FR-093-AC-19 (TC-416 step 11): every kind of IR's closed node-kind list
/// is classified exactly once, as admitted or not written by QSL. Each
/// row's fixture is admitted by IR's checked-package/v2 reader, through
/// QSL's I2 read, at its emitted `package_id`, with a node of the row's
/// family (the cyclic compared type's structural equality included), and
/// no fixture emits a family without a row.
#[trace("FR-093-AC-19", "TC-416")]
#[test]
fn every_emitted_node_family_is_admitted_at_its_package_id() {
    let rows = rows();

    // Every IR kind is classified exactly once. A family with several rows,
    // such as `expression`/`binary` with the cyclic compared type, is one
    // admitted classification.
    let mut classified: BTreeMap<CheckedNodeKind, Vec<String>> = BTreeMap::new();
    let admitted: BTreeSet<CheckedNodeKind> = rows.iter().map(|row| row.family).collect();
    for family in admitted {
        classified
            .entry(family)
            .or_default()
            .push("admitted".to_owned());
    }
    for (kind, reason) in NOT_EMITTED_BY_QSL {
        classified
            .entry(*kind)
            .or_default()
            .push(format!("not emitted: {reason}"));
    }
    let all = CheckedNodeKind::all();
    let unclassified: Vec<_> = all
        .iter()
        .filter(|kind| !classified.contains_key(kind))
        .collect();
    let ambiguous: Vec<_> = classified
        .iter()
        .filter(|(_, classes)| classes.len() != 1)
        .collect();
    let unknown: Vec<_> = classified
        .keys()
        .filter(|kind| !all.contains(kind))
        .collect();
    assert!(
        unclassified.is_empty() && ambiguous.is_empty() && unknown.is_empty(),
        "IR kinds with no classification: {unclassified:?}; classified more than once: \
         {ambiguous:?}; not IR kinds: {unknown:?}"
    );

    let outcomes: BTreeMap<Fixture, Vec<Outcome>> = Fixture::ALL
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

    // No fixture emits a family without a row.
    let named: BTreeSet<CheckedNodeKind> = rows.iter().map(|row| row.family).collect();
    let without_row: BTreeSet<_> = outcomes
        .values()
        .flatten()
        .flat_map(|outcome| outcome.families.keys())
        .filter(|family| !named.contains(family))
        .collect();
    assert!(
        without_row.is_empty(),
        "emitted families with no row: {without_row:?}"
    );

    let mut failures = Vec::new();
    for (fixture, outcomes) in &outcomes {
        check_fixture(*fixture, &rows, outcomes, &mut failures);
    }
    for row in &rows {
        match outcomes.get(&row.fixture) {
            Some(outcomes) => check_row(row, outcomes, &mut failures),
            None => failures.push(format!("{row:?}: its fixture is not in Fixture::ALL")),
        }
    }
    assert!(failures.is_empty(), "{failures:#?}");
}
