// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-093-AC-19 (TC-416 step 11): the emission-to-admission corpus. Every
//! node family of IR's closed checked-package/v2 vocabulary
//! (`CheckedNodeKind::all()`) is classified exactly once: as a family the
//! emitter or typed timed-model writer writes and IR admits (one or more
//! rows), or as a kind QSL does not write ([`NOT_EMITTED_BY_QSL`], each with
//! its reason).
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
    ExpressionForm, FunctionForm, HybridForm, ModelForm, ProtocolForm, RelationForm,
    ScalarTypeForm, StateForm, TemporalForm,
};

use super::super::extent_agreement::{package_declaring, records};
use super::variant_types::fixture as variant_fixture;
use super::*;
use crate::checked_v2::{read_checked_package_v2, V2Read, V2ReadRefusal};
use qsl_foundation::diagnostic::{StageFailure, Staged};

/// FR-108's ConfigVersion unit text and domain package (TC-469), the datum
/// `examples/config-version/spine.rs` writes its spine files from.
#[path = "../../../../examples/config-version/spine_unit.rs"]
pub(super) mod config_version_unit;

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
    /// A model selection whose domain package declares an inline relationship.
    RelationshipModel,
    /// QSpec FR-375's legal row-empty model refinement.
    RefinementV2Empty,
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
    /// A function over a model variant type, FR-360 item 12.
    VariantType,
    /// TC-1021: a function whose body refers to a declared protocol, under a
    /// selected protocol profile.
    ProtocolReference,
    /// TC-1061: a protocol with two roles and a channel, whose queue the
    /// emitter generates.
    ProtocolChannels,
    /// TC-1028: `=` and `!=` over native `String`, `UUID` and `Timestamp`
    /// parameters.
    NativeScalars,
    /// TC-827: [`UNION_NODES`], a union, a construction and a `case`.
    UnionNodes,
    /// TC-524: ADR-018 §6's [`TEMPORAL_CLAUSE`] over the ConfigVersion
    /// domain package.
    TemporalClause,
    /// TC-1266: [`HYPER_CLAUSE`] over the ConfigVersion domain package.
    HyperClause,
    /// TC-1140: [`STATE_GRAPH_CLAIMS`] over the ConfigVersion domain package.
    StateGraphClaim,
    /// TC-1142: [`TERMINAL_DECLARATION`] over the ConfigVersion domain
    /// package.
    Terminal,
    /// TC-1187, TC-1220: [`HYBRID_UNIT`], hybrid models and their claims.
    Hybrid,
}

impl Fixture {
    /// Every fixture; each must back at least one row.
    const ALL: [Self; 28] = [
        Self::Tc416Functions,
        Self::RecursiveF,
        Self::Tree,
        Self::DeclaredTypes,
        Self::SpineCompile,
        Self::SpineModel,
        Self::ModelPopulation,
        Self::RelationshipModel,
        Self::RefinementV2Empty,
        Self::SystemsInterface,
        Self::ConfigVersion,
        Self::ExtentRecords,
        Self::OrderedEnum,
        Self::GoldenCases,
        Self::Float32Add,
        Self::QuantityAndT,
        Self::FormsUnit,
        Self::CyclicEquality,
        Self::VariantType,
        Self::ProtocolReference,
        Self::ProtocolChannels,
        Self::NativeScalars,
        Self::UnionNodes,
        Self::TemporalClause,
        Self::HyperClause,
        Self::StateGraphClaim,
        Self::Terminal,
        Self::Hybrid,
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

/// TC-1028: native `String`, `UUID` and `Timestamp` equality, the families
/// IR-800 admits.
const NATIVE_SCALARS: &str = "function eqs using v(a: String, b: String): Boolean pure { a = b }\n\
     function neu using v(a: UUID, b: UUID): Boolean pure { a != b }\n\
     function eqt using v(a: Timestamp, b: Timestamp): Boolean pure { a = b }\n";

/// TC-827: a union, a construction of one of its members and a `case` over
/// it, the families FR-320 emits.
const UNION_NODES: &str =
    "union Shape { Empty, Circle(Integer), Rect(Integer, Integer) }\n\
     function area using v(s: Shape): Integer pure { case s { Empty: 0; Circle(r): r; Rect(w, h): w * h; } }\n\
     function rect using v(): Shape pure { Shape::Rect(2, 3) }\n";

/// TC-524: ADR-018 §6's liveness clause under weak fairness, after the
/// infinite-trace profile `t` and the ConfigVersion model selection: the
/// `temporal`/`temporal_clause`, `temporal`/`formula` and
/// `temporal`/`fairness` families FR-337 emits.
const TEMPORAL_CLAUSE: &str = "profile t = \"quire.temporal.infinite-trace/v1\";\n\
     model Config = \"example/config-version\" version \"1.0.0\" digest \"sha256-jcs:{}\";\n\
     temporal ReachesTwo using t over (c: Config::ConfigVersion) clock \"model-steps\" on origin {\n\
     fair weak each Config::ConfigVersion::attemptUpdate;\n\
     always eventually holds(c.versionNumber = 2)\n\
     }\n";

/// FR-488: a hyper clause over `behaviours`, with a fairness constraint, a
/// `match` conjunct and a `holds` atom, emitting the `claim`/`hyperproperty`
/// family and its `temporal`/`temporal_clause` body wrapper.
const HYPER_CLAUSE: &str = "profile t = \"quire.temporal.infinite-trace/v1\";\n\
     model Config = \"example/config-version\" version \"1.0.0\" digest \"sha256-jcs:{}\";\n\
     hyper Same using t over behaviours (c: Config::ConfigVersion) \
     forall trace a of Config fair weak each Config::ConfigVersion::attemptUpdate; \
     forall trace b of Config \
     match { a.step.op = b.step.op } \
     { always holds(c.versionNumber @ a = c.versionNumber @ b) }\n";

/// FR-410: ADR-022 section 7's three forms over the ConfigVersion domain
/// package, a claim with no `over`, and an `always possible` claim written
/// with and without `from`, emitting the `claim`/`state_graph` family.
const STATE_GRAPH_CLAIMS: &str =
    "model Config = \"example/config-version\" version \"1.0.0\" digest \"sha256-jcs:{}\";\n\
     possible ReachesTwo using v over (c: Config::ConfigVersion) { c.versionNumber = 2 }\n\
     always possible CanStillWin using v over (c: Config::ConfigVersion) \
     from (c.versionNumber != 0) { c.versionNumber = 2 }\n\
     always possible Open using v over (c: Config::ConfigVersion) { c.versionNumber = 2 }\n\
     unique path InOneWay using v over (c: Config::ConfigVersion) \
     from (c.versionNumber = 0) to (c.versionNumber = 2);\n\
     possible Anything using v { true }\n";

/// A model `terminal when` declaration over the ConfigVersion domain
/// package, emitting the `temporal`/`terminal` family.
const TERMINAL_DECLARATION: &str =
    "model Config = \"example/config-version\" version \"1.0.0\" digest \"sha256-jcs:{}\";\n\
     terminal Done using v on Config when { self.versionNumber = 3 };\n";

/// Structural equality over TC-416's recursive `Tree`, the STD-129 case: a
/// recursive compared type that reaches no `Text`, so it has no leaves
/// (FR-093-AC-11).
const CYCLIC_EQUALITY: &str = "record Tree { kids: Sequence<Tree>[0, 3]; }\n\
    function same using v(a: Tree, b: Tree): Boolean pure { a = b }\n";

/// The FR-360 domain package, whose variant types the `VariantType` fixture
/// names.
static VARIANT_DOCUMENT: std::sync::LazyLock<Vec<u8>> = std::sync::LazyLock::new(|| {
    serde_json::to_vec(&variant_fixture::document()).expect("the fixture serializes")
});

/// The domain package declaring the systems interface `Sensor`.
const SYSTEMS_DOCUMENT: &[u8] =
    include_bytes!("../../../../tests/fixtures/systems-interface.semantic-ir.json");

/// The domain package used to prove relationship model nodes are emitted and
/// admitted through IR's checked-package/v2 reader.
const RELATIONSHIP_MODEL_DOCUMENT: &[u8] =
    include_bytes!("../../../../tests/fixtures/spine-model-relationship.semantic-ir.json");

/// The complete-V1 header shared by every model source unit below.
const COMPLETE_V1_HEADER: &str = "language \"ix:native\" edition \"1-draft\";\n\
     profile v = \"quire.model.complete/v1\";\n";

/// FR-447's use case: `Braking2` with its two initial entries, and the
/// `Slows` and `Recent` claims over it, after [`COMPLETE_V1_HEADER`].
const HYBRID_UNIT: &str = "profile p = \"quire.temporal.infinite-trace/v1\";\n\
     profile t = \"quire.temporal.timed/v1\";\n\
     dimension Time;\n\
     unit s : Time = rational(1, 1);\n\
     unit ms : Time = rational(1, 1000) * s;\n\
     hybrid Braking2 using p {\n\
       initial {\n\
         mode Moving when { v = 20 and a >= 1 and a <= 2 and t = 0 };\n\
         mode Stopped when { v = 0 and t = 0 };\n\
       }\n\
       mode Moving invariant { v >= 0 and v <= 20 and a >= 1 and a <= 2 }\n\
         flow { v' = -a; a' = 0; t' = 1; }\n\
         transition to Stopped when { v = 0 } reset { v = 0 and a >= 1 };\n\
       mode Stopped invariant { v = 0 }\n\
         flow { v' = 0; a' = 0; t' = 1; }\n\
     }\n\
     temporal Slows using t over (h: Braking2) clock \"model-time\"\n\
         on each (q: Braking2) when (q.v > 10) {\n\
       fair Moving::Stopped;\n\
       capture v0: Rational[0, 20; 1, 1] = q.v;\n\
         eventually [0 s, 10500 ms] holds(h.v < v0 and h.v <= 10)\n\
     }\n\
     temporal Recent using p over (h: Braking2) clock \"model-steps\" on origin {\n\
       once [0, 2] holds(h.v = 20)\n\
     }\n";

/// The relationship model source after [`COMPLETE_V1_HEADER`].
const RELATIONSHIP_MODEL_UNIT: &str =
    "model M = \"acme/orders\" version \"1.0.0\" digest \"sha256-jcs:{}\";\n\
     function keep using v(g: M::Widget): M::Widget pure { g }\n";

/// The systems-interface model source after [`COMPLETE_V1_HEADER`].
const SYSTEMS_INTERFACE_UNIT: &str =
    "model S = \"acme/systems\" version \"1.0.0\" digest \"sha256-jcs:{}\";\n\
     function keep using v(s: S::Sensor): S::Sensor pure { s }\n";

/// One package of a fixture, checked, linked and emitted, with the domain
/// package documents its read needs as evidence.
struct Emitted {
    package: CheckedPackage,
    emission: Emission,
    documents: Vec<Vec<u8>>,
}

impl Emitted {
    /// `package` emitted placing each occurrence at its form span.
    fn placed(package: CheckedPackage, documents: Vec<&'static [u8]>) -> Self {
        Self::placed_owned(package, documents.into_iter().map(<[u8]>::to_vec).collect())
    }

    /// `package` emitted against dynamically assembled model documents.
    fn placed_owned(package: CheckedPackage, documents: Vec<Vec<u8>>) -> Self {
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
            documents: documents.into_iter().map(<[u8]>::to_vec).collect(),
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
            Self::RelationshipModel => {
                let document = RELATIONSHIP_MODEL_DOCUMENT;
                let unit = format!(
                    "{}{}",
                    COMPLETE_V1_HEADER,
                    RELATIONSHIP_MODEL_UNIT.replace("{}", &document_digest(document)),
                );
                Emitted::placed(modelled(&unit, document, Vec::new()), vec![document])
            }
            Self::RefinementV2Empty => {
                let abstract_document = state_machine_document("example/refinement-abstract");
                let concrete_document = state_machine_document("example/refinement-concrete");
                let abstract_digest = document_digest(&abstract_document);
                let concrete_digest = document_digest(&concrete_document);
                let unit = format!(
                    "{COMPLETE_V1_HEADER}profile inf = \"quire.temporal.infinite-trace/v1\";\n\
                     model A = \"example/refinement-abstract\" version \"1.0.0\" digest \"sha256-jcs:{abstract_digest}\";\n\
                     model C = \"example/refinement-concrete\" version \"1.0.0\" digest \"sha256-jcs:{concrete_digest}\";\n\
                     refinement R using inf abstract A concrete C {{}}\n"
                );
                let documents = vec![abstract_document, concrete_document];
                Emitted::placed_owned(
                    modelled_with_documents(&unit, &documents, Vec::new()),
                    documents,
                )
            }
            Self::SystemsInterface => {
                let unit = format!(
                    "{}{}",
                    COMPLETE_V1_HEADER,
                    SYSTEMS_INTERFACE_UNIT.replace("{}", &document_digest(SYSTEMS_DOCUMENT)),
                );
                Emitted::placed(
                    modelled(&unit, SYSTEMS_DOCUMENT, Vec::new()),
                    vec![SYSTEMS_DOCUMENT],
                )
            }
            Self::TemporalClause => {
                let document = config_version_unit::DOMAIN_PACKAGE.as_bytes();
                let unit = format!(
                    "{COMPLETE_V1_HEADER}{}",
                    TEMPORAL_CLAUSE.replace("{}", &document_digest(document))
                );
                Emitted::placed(modelled(&unit, document, Vec::new()), vec![document])
            }
            Self::HyperClause => {
                let document = config_version_unit::DOMAIN_PACKAGE.as_bytes();
                let unit = format!(
                    "{COMPLETE_V1_HEADER}{}",
                    HYPER_CLAUSE.replace("{}", &document_digest(document))
                );
                Emitted::placed(modelled(&unit, document, Vec::new()), vec![document])
            }
            Self::StateGraphClaim => {
                let document = config_version_unit::DOMAIN_PACKAGE.as_bytes();
                let unit = format!(
                    "{COMPLETE_V1_HEADER}{}",
                    STATE_GRAPH_CLAIMS.replace("{}", &document_digest(document))
                );
                Emitted::placed(modelled(&unit, document, Vec::new()), vec![document])
            }
            Self::Terminal => {
                let document = config_version_unit::DOMAIN_PACKAGE.as_bytes();
                let unit = format!(
                    "{COMPLETE_V1_HEADER}{}",
                    TERMINAL_DECLARATION.replace("{}", &document_digest(document))
                );
                Emitted::placed(modelled(&unit, document, Vec::new()), vec![document])
            }
            Self::Hybrid => text(HYBRID_UNIT),
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
            Self::NativeScalars => text(NATIVE_SCALARS),
            Self::UnionNodes => text(UNION_NODES),
            Self::VariantType => {
                let document = VARIANT_DOCUMENT.as_slice();
                let unit = format!(
                    "{COMPLETE_V1_HEADER}model M = {package:?} version \"1\" \
                     digest \"sha256-jcs:{digest}\";\n\
                     function keep using v(t: M::Tier): M::Tier pure {{ t }}\n",
                    package = variant_fixture::PACKAGE,
                    digest = document_digest(document),
                );
                Emitted::placed(modelled(&unit, document, Vec::new()), vec![document])
            }
            Self::ProtocolChannels => Emitted::placed(
                modelled(
                    &super::protocol_reference::channel_unit(),
                    SPINE_MODEL_DOCUMENT,
                    Vec::new(),
                ),
                vec![SPINE_MODEL_DOCUMENT],
            ),
            Self::ProtocolReference => Emitted::placed(
                modelled(
                    &super::protocol_reference::reference_unit(),
                    SPINE_MODEL_DOCUMENT,
                    Vec::new(),
                ),
                vec![SPINE_MODEL_DOCUMENT],
            ),
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

/// The spine fixture's object package with one minimal, independent state
/// machine added, under `package`'s identity.
fn state_machine_document(package: &str) -> Vec<u8> {
    with_state_machine(SPINE_MODEL_DOCUMENT, "acme/orders", package)
}

/// The document `base`, its identity `from` renamed `package`, with one
/// minimal, independent state machine added.
fn with_state_machine(base: &[u8], from: &str, package: &str) -> Vec<u8> {
    let base = String::from_utf8(base.to_vec()).expect("fixture UTF-8");
    let base = base.replace(from, package);
    let mut document: serde_json::Value =
        serde_json::from_str(&base).expect("the spine model document is JSON");
    let identity = format!("ix://{package}/Lifecycle");
    let state_identity = format!("ix://{package}/Lifecycle/Idle");
    let operation_identity = format!("ix://{package}/Lifecycle/tick");
    let transition_identity = format!("ix://{package}/Lifecycle/tick_transition");
    let origin = |identity: &str| {
        json!({"generated": {
            "generatorIdentity": identity,
            "generatorVersion": "1.0.0",
            "inputIdentities": [identity]
        }})
    };
    document["constructs"]
        .as_array_mut()
        .expect("constructs")
        .push(json!({
            "kind": {"module": package, "name": "state_machine"},
            "moduleVersion": "1.0.0",
            "manifestDigest": "sha256:0000000000000000000000000000000000000000000000000000000000000000",
            "construct": {
                "identity": "none", "shape": "record",
                "members": {"states": "required", "transitions": "required"},
                "meaning": qsl_semantics::model::intake::meaning::STATE_MACHINE
            }
        }));
    document["types"]
        .as_array_mut()
        .expect("types")
        .push(json!({
            "identity": identity,
            "displayName": "Lifecycle",
            "kind": {"module": package, "name": "state_machine"},
            "roles": [],
            "origin": origin(&identity),
            "constraints": [],
            "extensions": [],
            "unknownPolicy": "reject",
            "supertypes": [],
            "fields": [],
            "clauses": [],
            "operations": [{
                "identity": operation_identity,
                "name": "tick",
                "params": [], "pre": [], "post": [],
                "frame": {"modifies": [], "creates": [], "deletes": []},
                "origin": origin(&operation_identity)
            }],
            "states": [{"identity": state_identity, "name": "Idle", "origin": origin(&state_identity)}],
            "transitions": [{
                "identity": transition_identity,
                "from": state_identity,
                "to": state_identity,
                "trigger": operation_identity,
                "emits": [],
                "origin": origin(&transition_identity)
            }]
        }));
    serde_json::to_vec(&document).expect("the state-machine model serializes")
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

fn modelled_with_documents(
    unit: &str,
    documents: &[Vec<u8>],
    functions: Vec<FunctionDeclaration>,
) -> CheckedPackage {
    let packages = qsl_semantics::model::intake::package_input(documents.iter().map(Vec::as_slice));
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
        row(K::ScalarType(S::String), F::NativeScalars),
        row(K::ScalarType(S::Uuid), F::NativeScalars),
        row(K::ScalarType(S::Timestamp), F::NativeScalars),
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
        row(K::CompositeType(C::Union), F::UnionNodes),
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
        row(K::Value(ValueForm::UnionValue), F::UnionNodes),
        row(K::Value(ValueForm::Parameter), F::Tc416Functions),
        row(K::Expression(E::Call), F::Tc416Functions),
        row(K::Expression(E::Unary), F::ConfigVersion),
        row(K::Expression(E::Binary), F::Tc416Functions),
        row(K::Expression(E::Case), F::UnionNodes),
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
        row(K::Model(ModelForm::ModelRoot), F::SpineModel),
        row(K::Model(ModelForm::SystemsInterface), F::SystemsInterface),
        row(K::Model(ModelForm::VariantType), F::VariantType),
        row(K::Model(ModelForm::RecordValueType), F::VariantType),
        row(
            K::Relation(RelationForm::Relationship),
            F::RelationshipModel,
        ),
        row(K::Relation(RelationForm::Refinement), F::RefinementV2Empty),
        row(K::State(StateForm::StateClause), F::ConfigVersion),
        row(K::State(StateForm::Frame), F::ConfigVersion),
        row(K::State(StateForm::OperationAnchor), F::ConfigVersion),
        row(
            K::Protocol(ProtocolForm::ProtocolClause),
            F::ProtocolReference,
        ),
        row(
            K::Protocol(ProtocolForm::ProtocolReference),
            F::ProtocolReference,
        ),
        row(K::Protocol(ProtocolForm::Role), F::ProtocolReference),
        row(K::Protocol(ProtocolForm::Channel), F::ProtocolChannels),
        row(K::Protocol(ProtocolForm::Queue), F::ProtocolChannels),
        row(K::Protocol(ProtocolForm::Control), F::ProtocolReference),
        row(K::Protocol(ProtocolForm::Obligation), F::ProtocolReference),
        row(K::Temporal(TemporalForm::TemporalClause), F::TemporalClause),
        row(K::Temporal(TemporalForm::Formula), F::TemporalClause),
        row(K::Temporal(TemporalForm::Fairness), F::TemporalClause),
        row(K::Claim(ClaimForm::Hyperproperty), F::HyperClause),
        row(K::Claim(ClaimForm::StateGraph), F::StateGraphClaim),
        row(K::Temporal(TemporalForm::Terminal), F::Terminal),
        row(K::Hybrid(HybridForm::Model), F::Hybrid),
        row(K::Hybrid(HybridForm::Mode), F::Hybrid),
        row(K::Hybrid(HybridForm::Transition), F::Hybrid),
    ]
}

/// Every IR node kind neither QSL's emitter nor typed timed-model writer writes.
const NOT_EMITTED_BY_QSL: &[(CheckedNodeKind, &str)] = {
    use CheckedNodeKind as K;
    const NO_RECORD_FORM: &str =
        "lowering's record_form maps only object types, systems interfaces, record value types and variant types to model nodes";
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
            K::Value(ValueForm::OptionValue),
            "lowering builds no option literal (UnbuiltLiteral)",
        ),
        (
            K::Expression(ExpressionForm::Reference),
            "a name read lowers to a reference term, not a node",
        ),
        (
            K::Model(ModelForm::StateMachine),
            "a refinement binds its model's model_root node, not a state-machine declaration node (FR-375)",
        ),
        (K::Model(ModelForm::ModelImport), NO_RECORD_FORM),
        (K::Model(ModelForm::ValueType), NO_RECORD_FORM),
        (K::Model(ModelForm::EventType), NO_RECORD_FORM),
        (K::Model(ModelForm::Process), NO_RECORD_FORM),
        (K::Model(ModelForm::PersistenceInterface), NO_RECORD_FORM),
        (K::Model(ModelForm::Namespace), NO_RECORD_FORM),
        (K::Model(ModelForm::SystemsPart), NO_RECORD_FORM),
        (K::Model(ModelForm::SystemsPort), NO_RECORD_FORM),
        (K::Model(ModelForm::SystemsConnection), NO_RECORD_FORM),
        (K::Model(ModelForm::SystemsAllocation), NO_RECORD_FORM),
        (K::Relation(RelationForm::Population), NO_RECORD_FORM),
        (K::Relation(RelationForm::Membership), NO_RECORD_FORM),
        (K::Relation(RelationForm::CausalRelation), NO_RECORD_FORM),
        (K::State(StateForm::Transition), NO_TAG),
        (K::State(StateForm::Snapshot), NO_TAG),
        (K::Temporal(TemporalForm::Clock), NO_TAG),
        (K::Temporal(TemporalForm::Window), NO_TAG),
        (K::Temporal(TemporalForm::Activation), NO_TAG),
        (K::Temporal(TemporalForm::Deadline), NO_TAG),
        (K::Protocol(ProtocolForm::Compensation), NO_TAG),
        (K::Claim(ClaimForm::VerificationClaim), NO_TAG),
        (K::Claim(ClaimForm::AnalysisClaim), NO_TAG),
        (K::Claim(ClaimForm::SynthesisRequest), NO_TAG),
        (K::Correspondence(CorrespondenceForm::SourceLocus), NO_TAG),
        (
            K::Correspondence(CorrespondenceForm::ModelCorrespondence),
            NO_TAG,
        ),
        (K::Correspondence(CorrespondenceForm::BindingRole), NO_TAG),
        (
            K::Correspondence(CorrespondenceForm::AbstractionRelation),
            NO_TAG,
        ),
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
        evidence
            .insert_domain_package_document(document_digest(document), document.to_vec())
            .expect("a fresh evidence key");
    }
    let pinned: PinnedRequest = qsl_semantics::library::fixtures::single_pin(
        library(),
        emitted.emission.package.package_id(),
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
    // IR does not export `ExtensionForm`, so its kinds cannot be named in
    // `NOT_EMITTED_BY_QSL`; QSL writes no extension node until QSL-978.
    for kind in all
        .iter()
        .filter(|kind| matches!(kind, CheckedNodeKind::Extension(_)))
    {
        classified
            .entry(*kind)
            .or_default()
            .push("not emitted: QSL writes no extension node yet (QSL-978)".to_owned());
    }
    // `CheckedTimedModel::write_nodes` writes the complete closed timed-model
    // family; its emitted-package conformance test reads the result through I2.
    for kind in all
        .iter()
        .filter(|kind| matches!(kind, CheckedNodeKind::TimedModel(_)))
    {
        classified
            .entry(*kind)
            .or_default()
            .push("admitted by the typed timed-model writer (TC-367)".to_owned());
    }
    // QSL assembles dimension equivalences for unit conversion but lowers no
    // node for one.
    classified
        .entry(CheckedNodeKind::ScalarType(
            ScalarTypeForm::DimensionEquivalence,
        ))
        .or_default()
        .push("not emitted: QSL lowers no dimension-equivalence node".to_owned());
    // IR-1059: the tuple value type is an IR kind QSL's emitter does not write.
    classified
        .entry(CheckedNodeKind::Model(ModelForm::TupleValueType))
        .or_default()
        .push("not emitted: QSL lowers no tuple value type node yet".to_owned());
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

/// Two refinements of different names with one subject, profile and body
/// key one node today, though a refinement's name is part of its identity
/// (the planner's ruling on STD-493): the later declaration refuses
/// `unsupported_construct`/`not-yet-implemented`, naming both, and no
/// package is written.
#[trace("FR-375", "TC-310")]
#[test]
fn two_refinements_of_one_meaning_refuse_naming_both() {
    let abstract_document = state_machine_document("example/refinement-abstract");
    let concrete_document = state_machine_document("example/refinement-concrete");
    let unit = format!(
        "{COMPLETE_V1_HEADER}profile inf = \"quire.temporal.infinite-trace/v1\";\n\
         model A = \"example/refinement-abstract\" version \"1.0.0\" digest \"sha256-jcs:{}\";\n\
         model C = \"example/refinement-concrete\" version \"1.0.0\" digest \"sha256-jcs:{}\";\n\
         refinement R using inf abstract A concrete C {{}}\n\
         refinement R2 using inf abstract A concrete C {{}}\n",
        document_digest(&abstract_document),
        document_digest(&concrete_document),
    );
    let package =
        modelled_with_documents(&unit, &[abstract_document, concrete_document], Vec::new());
    let refusal = emit_checked(&package).expect_err("the second declaration refuses");
    assert!(
        matches!(
            &refusal,
            EmitRefusal::SameMeaningRefinements { first, second }
                if first == "R" && second == "R2"
        ),
        "{refusal:?}"
    );
    assert_eq!(refusal.code(), Code::UnsupportedConstruct);
    assert_eq!(refusal.cause(), Some("not-yet-implemented"));
}

/// Two refinements of different meanings are two nodes and both emit: `R`
/// refines `A` by `C` and `R2` refines `C` by `A`, so their subjects differ
/// and no declaration refuses as [`EmitRefusal::SameMeaningRefinements`].
#[trace("FR-375", "TC-310")]
#[test]
fn two_refinements_of_different_meanings_both_emit() {
    let abstract_document = state_machine_document("example/refinement-abstract");
    let concrete_document = state_machine_document("example/refinement-concrete");
    let unit = format!(
        "{COMPLETE_V1_HEADER}profile inf = \"quire.temporal.infinite-trace/v1\";\n\
         model A = \"example/refinement-abstract\" version \"1.0.0\" digest \"sha256-jcs:{}\";\n\
         model C = \"example/refinement-concrete\" version \"1.0.0\" digest \"sha256-jcs:{}\";\n\
         refinement R using inf abstract A concrete C {{}}\n\
         refinement R2 using inf abstract C concrete A {{}}\n",
        document_digest(&abstract_document),
        document_digest(&concrete_document),
    );
    let package =
        modelled_with_documents(&unit, &[abstract_document, concrete_document], Vec::new());
    let emission = emit_checked(&package).expect("both refinements emit");
    let wire = wire(&emission);
    let refinements: Vec<&Value> = nodes(&wire)
        .iter()
        .filter(|node| node["node_tag"] == "relation" && node["semantic_form"] == "refinement")
        .collect();
    assert_eq!(refinements.len(), 2, "{refinements:?}");
    assert_ne!(refinements[0]["node_id"], refinements[1]["node_id"]);
}

/// The row-empty refinement `R` of the state-machine model `A` by `C`, and
/// the two model documents.
fn row_empty_refinement() -> (String, Vec<Vec<u8>>) {
    let abstract_document = state_machine_document("example/refinement-abstract");
    let concrete_document = state_machine_document("example/refinement-concrete");
    let abstract_digest = document_digest(&abstract_document);
    let concrete_digest = document_digest(&concrete_document);
    let unit = format!(
        "{COMPLETE_V1_HEADER}profile inf = \"quire.temporal.infinite-trace/v1\";\n\
         model A = \"example/refinement-abstract\" version \"1.0.0\" digest \"sha256-jcs:{abstract_digest}\";\n\
         model C = \"example/refinement-concrete\" version \"1.0.0\" digest \"sha256-jcs:{concrete_digest}\";\n\
         refinement R using inf abstract A concrete C {{}}\n"
    );
    (unit, vec![abstract_document, concrete_document])
}

/// QSpec FR-375-AC-20's refusal half: the reader recomputes the refinement
/// node's key from the closed `quire.refinement-node/v1` preimage, so
/// changing only its `semantic_type` root, or only one body row, while
/// keeping its node id (the package id and dependencies recomputed) refuses
/// `invalid_package`/`stale-node-key` at that node.
#[trace("FR-375-AC-20", "TC-310")]
#[test]
fn a_refinement_changed_under_its_node_id_is_a_stale_node_key() {
    let (unit, documents) = row_empty_refinement();
    let package = modelled_with_documents(&unit, &documents, Vec::new());
    let emission = emit_checked(&package).expect("the row-empty refinement emits");
    let wire = wire(&emission);
    let position = nodes(&wire)
        .iter()
        .position(|node| node["node_tag"] == "relation" && node["semantic_form"] == "refinement")
        .expect("one refinement relation node");
    let refinement = &wire["semantic_graph"]["nodes"][position];
    let abstract_root = refinement["body"]["abstract"].clone();
    let verdict = |envelope: &Value| {
        let mut evidence = read_evidence(&emission);
        for document in &documents {
            evidence
                .insert_domain_package_document(document_digest(document), document.to_vec())
                .expect("a fresh evidence key");
        }
        match quire_contract_model::read_checked_package(
            &jcs(envelope),
            quire_contract_model::CheckedPackageReadLimits::bounded(),
            &evidence,
        ) {
            quire_contract_model::CheckedPackageDispatchResult::AdmittedV2(_) => {
                "admitted".to_owned()
            }
            quire_contract_model::CheckedPackageDispatchResult::Refused(refusal) => format!(
                "{:?} at {} ({:?})",
                refusal.code,
                refusal.path.as_ref().map_or("-", |path| path.as_str()),
                refusal.cause
            ),
            other => format!("{other:?}"),
        }
    };
    assert_eq!(verdict(&wire), "admitted");
    let expected =
        format!("InvalidPackage at /semantic_graph/nodes/{position}/node_id (Some(StaleNodeKey))");

    // The concrete root swapped for the abstract one: the subject changes,
    // the dependencies (the abstract root alone) do not.
    let mut retyped = wire.clone();
    retyped["semantic_graph"]["nodes"][position]["semantic_type"] = abstract_root.clone();
    crate::checked_v2::tests::refresh_frame_identity(&mut retyped);
    assert_ne!(retyped["package_id"], wire["package_id"]);
    assert_eq!(verdict(&retyped), expected);

    // One body member changed: the selected profile. This model has no
    // population or object to write a valid row over, so the profile is the
    // body change that leaves every other member admissible.
    let mut reprofiled = wire.clone();
    reprofiled["semantic_graph"]["nodes"][position]["body"]["profile"]["identity"] =
        json!("quire.temporal.finite-trace/v1");
    crate::checked_v2::tests::refresh_frame_identity(&mut reprofiled);
    assert_eq!(verdict(&reprofiled), expected);
}

/// The declaration occurrence is provenance rather than refinement identity:
/// moving the authored declaration changes its source-map region while the
/// refinement node and package identities stay fixed.
#[trace("FR-375", "QSL-1102")]
#[trace("FR-375-AC-20")]
#[test]
fn a_row_empty_refinement_identity_excludes_its_source_region() {
    let (unit, documents) = row_empty_refinement();
    let compile = |source: &str| {
        let package = modelled_with_documents(source, &documents, Vec::new());
        let emission = emit_checked(&package).expect("the row-empty refinement emits");
        let admitted = read(&Emitted {
            package,
            emission: emission.clone(),
            documents: documents.clone(),
        });
        assert!(
            admitted.is_ok(),
            "the emitted refinement reads back: {admitted:?}"
        );
        emission
    };
    let first = compile(&unit);
    let shifted = compile(&format!("// provenance-only prefix\n{unit}"));
    let find_node = |emission: &Emission| {
        nodes(&wire(emission))
            .iter()
            .find(|node| node["node_tag"] == "relation" && node["semantic_form"] == "refinement")
            .expect("one refinement relation node")
            .clone()
    };
    let first_node = find_node(&first);
    let shifted_node = find_node(&shifted);
    // QSpec FR-375-AC-7/AC-11: the header's `semantic_type` is the CONCRETE
    // model's `model_root`, `body.abstract` (and the dependencies) the
    // ABSTRACT model's.
    let first_wire = wire(&first);
    let root_of = |identity: &str| {
        let roots: Vec<&Value> = nodes(&first_wire)
            .iter()
            .filter(|node| {
                node["semantic_form"] == "model_root" && node["owner"]["identity"] == identity
            })
            .collect();
        assert_eq!(roots.len(), 1, "one root for {identity}");
        roots[0]["node_id"].clone()
    };
    let abstract_root = root_of("example/refinement-abstract");
    let concrete_root = root_of("example/refinement-concrete");
    assert_ne!(abstract_root, concrete_root);
    assert_eq!(first_node["semantic_type"], concrete_root);
    assert_eq!(first_node["body"]["abstract"], abstract_root);
    assert_eq!(first_node["dependencies"], json!([abstract_root]));
    // QSpec FR-375-AC-20: the typed body writes exactly the nine closed
    // members, every row array present and empty, and no optional member.
    let mut members: Vec<&str> = first_node["body"]
        .as_object()
        .expect("an object body")
        .keys()
        .map(String::as_str)
        .collect();
    members.sort_unstable();
    assert_eq!(
        members,
        [
            "abstract",
            "assume",
            "ensure",
            "histories",
            "objects",
            "populations",
            "profile",
            "steps",
            "term"
        ]
    );
    for array in [
        "assume",
        "ensure",
        "histories",
        "objects",
        "populations",
        "steps",
    ] {
        assert_eq!(first_node["body"][array], json!([]), "{array}");
    }
    assert_eq!(first_node["node_id"], shifted_node["node_id"]);
    assert_eq!(first.package.package_id(), shifted.package.package_id());
    let declaration_region = |emission: &Emission, node: &Value| {
        wire(emission)["source_map"]
            .as_array()
            .expect("source map")
            .iter()
            .find(|entry| entry["node_id"] == node["node_id"] && entry["role"] == "declaration")
            .expect("declaration source entry")["regions"][0]
            .clone()
    };
    let first_region = declaration_region(&first, &first_node);
    let shifted_region = declaration_region(&shifted, &shifted_node);
    assert_ne!(first_region["start"], shifted_region["start"]);
    assert_eq!(
        first_region["end"].as_u64().unwrap() - first_region["start"].as_u64().unwrap(),
        shifted_region["end"].as_u64().unwrap() - shifted_region["start"].as_u64().unwrap()
    );
}

/// A row-empty refinement of the model `A` by `C`, with the protocol clause
/// `Local` over `C::Widget`: a static role, an `unknown`-delivery channel
/// and `finish`.
fn catalog_refinement() -> (String, Vec<Vec<u8>>) {
    let abstract_document = state_machine_document("example/refinement-abstract");
    let concrete_document = state_machine_document("example/refinement-concrete");
    let unit = format!(
        "{COMPLETE_V1_HEADER}profile inf = \"quire.temporal.infinite-trace/v1\";\n\
         profile p = \"quire.protocol.finite-global/v1\";\n\
         model A = \"example/refinement-abstract\" version \"1.0.0\" digest \"sha256-jcs:{}\";\n\
         model C = \"example/refinement-concrete\" version \"1.0.0\" digest \"sha256-jcs:{}\";\n\
         protocol Local using p over (input: C::Widget) on origin {{\n\
         role w on C::Widget;\n\
         channel ch from w to w carries Boolean ordering unordered delivery unknown \
         capacity 2 overflow block;\n\
         run sequence Main {{ }}\n\
         finish Done as (d: Boolean) {{ true }};\n\
         }}\n\
         refinement R using inf abstract A concrete C {{}}\n",
        document_digest(&abstract_document),
        document_digest(&concrete_document),
    );
    (unit, vec![abstract_document, concrete_document])
}

/// The refinement relation node of `emission`.
fn refinement_node(emission: &Emission) -> Value {
    nodes(&wire(emission))
        .iter()
        .find(|node| node["node_tag"] == "relation" && node["semantic_form"] == "refinement")
        .expect("one refinement relation node")
        .clone()
}

/// QSpec FR-375: a protocol clause over the concrete model gives a
/// row-empty model refinement its `class_catalog`, in the closed body and
/// so in the node identity. The concrete side lists the concrete model's
/// operation classes and the clause's classes, the abstract side the
/// abstract model's operation classes, each in UTF-8 byte order; every
/// class node and binder type is a local node of the package and a
/// dependency of the refinement.
#[trace("FR-375-AC-23", "TC-310")]
#[test]
fn a_concrete_protocol_gives_a_model_refinement_its_class_catalog() {
    let (unit, documents) = catalog_refinement();
    let package = modelled_with_documents(&unit, &documents, Vec::new());
    let emission = emit_checked(&package).expect("the refinement with a catalog emits");
    let node = refinement_node(&emission);
    let catalog = &node["body"]["class_catalog"];
    let spelled = |side: &str| -> Vec<&str> {
        catalog[side]
            .as_array()
            .unwrap_or_else(|| panic!("the {side} side: {catalog}"))
            .iter()
            .map(|class| class["class"].as_str().expect("a class spelling"))
            .collect()
    };
    assert_eq!(
        spelled("concrete"),
        [
            "Lifecycle::tick",
            "Local::Done",
            "duplicate Local::ch",
            "lose Local::ch",
        ]
    );
    assert_eq!(spelled("abstract"), ["Lifecycle::tick"]);

    let wire = wire(&emission);
    let by_id = |id: &Value| {
        nodes(&wire)
            .iter()
            .find(|node| &node["node_id"] == id)
            .unwrap_or_else(|| panic!("{id} is a node of the package"))
            .clone()
    };
    let dependencies = node["dependencies"].as_array().expect("dependencies");
    for class in catalog["concrete"]
        .as_array()
        .into_iter()
        .chain(catalog["abstract"].as_array())
        .flatten()
    {
        let used = std::iter::once(&class["node"])
            .chain(class["binder_types"].as_array().into_iter().flatten());
        for id in used {
            by_id(id);
            assert!(dependencies.contains(id), "{id} is a dependency");
        }
    }
    // The operation class names the state machine's model node; its binder
    // record is the receiver alone, `tick` having no parameter or result.
    let tick = &catalog["concrete"][0];
    assert_eq!(by_id(&tick["node"])["semantic_form"], "state_machine");
    assert_eq!(tick["binder_types"].as_array().map(Vec::len), Some(1));
    // The finish class names the clause's `obligation` node, typed by its
    // closing binder; a channel class has no binder.
    let done = &catalog["concrete"][1];
    assert_eq!(by_id(&done["node"])["semantic_form"], "obligation");
    assert_eq!(done["binder_types"].as_array().map(Vec::len), Some(1));
    assert_eq!(catalog["concrete"][2]["binder_types"], json!([]));
    assert_eq!(
        by_id(&catalog["concrete"][2]["node"])["semantic_form"],
        "channel"
    );
    let mut sorted = dependencies.clone();
    sorted.sort_by(|left, right| left["digest"].as_str().cmp(&right["digest"].as_str()));
    assert_eq!(&sorted, dependencies, "dependencies ascend");

    // With the clause over the abstract model instead, the concrete side
    // has no protocol class, so the refinement has no catalog and another
    // node.
    let without = unit
        .replace("over (input: C::Widget)", "over (input: A::Widget)")
        .replace("role w on C::Widget", "role w on A::Widget");
    let other = modelled_with_documents(&without, &documents, Vec::new());
    let other = refinement_node(&emit_checked(&other).expect("the refinement emits"));
    assert!(other["body"].get("class_catalog").is_none(), "{other}");
    assert_ne!(other["node_id"], node["node_id"]);
}

/// QSpec FR-375-AC-27: an `on each` clause over the concrete model writes
/// `activate <P>` naming the clause node, with the trigger's binder record
/// (one field for a Boolean trigger).
#[trace("FR-375-AC-27", "TC-310")]
#[test]
fn an_each_clause_writes_its_activate_class() {
    let (unit, documents) = catalog_refinement();
    let unit = unit.replace("on origin", "on each (t: Boolean)");
    let package = modelled_with_documents(&unit, &documents, Vec::new());
    let emission = emit_checked(&package).expect("the refinement emits");
    let node = refinement_node(&emission);
    let activate = node["body"]["class_catalog"]["concrete"]
        .as_array()
        .expect("the concrete side")
        .iter()
        .find(|class| class["class"] == "activate Local")
        .expect("the activate class")
        .clone();
    let wire = wire(&emission);
    let clause = nodes(&wire)
        .iter()
        .find(|node| node["semantic_form"] == "protocol_clause")
        .expect("the clause node")
        .clone();
    assert_eq!(activate["node"], clause["node_id"]);
    let [binder] = activate["binder_types"]
        .as_array()
        .expect("binders")
        .as_slice()
    else {
        panic!("one trigger field: {activate}");
    };
    let boolean = nodes(&wire)
        .iter()
        .find(|node| &node["node_id"] == binder)
        .expect("the binder type node")
        .clone();
    assert_eq!(boolean["semantic_form"], "boolean", "{boolean}");
}

/// QSpec FR-375-AC-23: a clause whose `owner_model` is a model of a third
/// selected declaration is foreign to the refinement: it contributes no
/// class, so the row-empty model refinement has no catalog.
#[trace("FR-375-AC-23", "TC-310")]
#[test]
fn a_clause_over_a_third_model_contributes_no_class() {
    let (unit, mut documents) = catalog_refinement();
    let third = state_machine_document("example/refinement-third");
    let unit = unit
        .replace(
            "protocol Local",
            &format!(
                "model T = \"example/refinement-third\" version \"1.0.0\" \
                 digest \"sha256-jcs:{}\";\nprotocol Local",
                document_digest(&third)
            ),
        )
        .replace("over (input: C::Widget)", "over (input: T::Widget)")
        .replace("role w on C::Widget", "role w on T::Widget");
    documents.push(third);
    let package = modelled_with_documents(&unit, &documents, Vec::new());
    let emission = emit_checked(&package).expect("the refinement emits");
    let node = refinement_node(&emission);
    assert!(node["body"].get("class_catalog").is_none(), "{node}");
    assert!(nodes(&wire(&emission))
        .iter()
        .any(|node| node["semantic_form"] == "protocol_clause"));
}

/// [`catalog_refinement`] with the clause `Local` over the abstract model and
/// the refinement's abstract side that clause: a protocol subject.
fn protocol_subject() -> (String, Vec<Vec<u8>>) {
    let (unit, documents) = catalog_refinement();
    let unit = unit
        .replace("over (input: C::Widget)", "over (input: A::Widget)")
        .replace("role w on C::Widget", "role w on A::Widget")
        .replace("abstract A concrete C", "abstract A::Local concrete C");
    (unit, documents)
}

/// A row-empty model subject `R2` of `A` by `C`, beside a protocol
/// subject `R` over the same models.
const MODEL_SUBJECT_R2: &str = "refinement R2 using inf abstract A concrete C {}\n";

/// `unit` over `documents` with the `refinement_internals` value `internals`,
/// checked and emitted: the package id and every refinement node by name.
fn emit_with_internals(
    unit: &str,
    documents: &[Vec<u8>],
    internals: Option<Value>,
) -> (Value, BTreeMap<String, Value>) {
    let packages = qsl_semantics::model::intake::package_input(documents.iter().map(Vec::as_slice));
    let mut declarations =
        assemble_with_models(unit.as_bytes(), &packages).expect("the unit assembles");
    if let Some(value) = internals {
        declarations = declarations.with_refinement_internals(value);
    }
    let graph = declarations
        .check(CheckingLimits::default())
        .unwrap_or_else(|refusals| panic!("{unit}: {refusals:?}"));
    let names: Vec<String> = graph
        .refinements()
        .iter()
        .map(|refinement| refinement.name().to_owned())
        .collect();
    let emission = emit_checked(&CheckedPackage::link(graph)).expect("the package emits");
    let wire = wire(&emission);
    let refinements: Vec<Value> = nodes(&wire)
        .iter()
        .filter(|node| node["node_tag"] == "relation" && node["semantic_form"] == "refinement")
        .cloned()
        .collect();
    // Each refinement's node is located at its own declaration.
    let by_name = names
        .into_iter()
        .map(|name| {
            let start = unit
                .find(&format!("refinement {name} "))
                .expect("its declaration");
            let node = refinements
                .iter()
                .find(|node| {
                    wire["source_map"].as_array().is_some_and(|entries| {
                        entries.iter().any(|entry| {
                            entry["node_id"] == node["node_id"]
                                && entry["role"] == "declaration"
                                && entry["regions"][0]["start"].as_u64()
                                    == u64::try_from(start).ok()
                        })
                    })
                })
                .unwrap_or_else(|| panic!("the node of {name}"))
                .clone();
            (name, node)
        })
        .collect();
    (wire["package_id"].clone(), by_name)
}

/// QSpec FR-375-AC-12: the `refinement_internals` value is the only host of
/// a protocol subject's internal list: `[{"refinement": "R", "internal":
/// ["A::Local::Done"]}]` writes `abstract_internal` `["Local::Done"]`; the
/// value absent, `[]` or naming only another refinement writes no member.
#[trace("FR-375-AC-12", "TC-310")]
#[test]
fn the_internal_list_value_hosts_abstract_internal() {
    let (unit, documents) = protocol_subject();
    let internal =
        |name: &str, entries: &[&str]| json!([{"refinement": name, "internal": entries}]);
    let (_, listed) =
        emit_with_internals(&unit, &documents, Some(internal("R", &["A::Local::Done"])));
    assert_eq!(
        listed["R"]["body"]["abstract_internal"],
        json!(["Local::Done"])
    );
    for value in [None, Some(json!([]))] {
        let (_, plain) = emit_with_internals(&unit, &documents, value.clone());
        assert!(
            plain["R"]["body"].get("abstract_internal").is_none(),
            "{value:?}"
        );
    }
    // A value naming only another refinement, the model subject `R2`.
    let two = format!("{unit}{MODEL_SUBJECT_R2}");
    let (_, nodes) = emit_with_internals(&two, &documents, Some(internal("R2", &[])));
    assert!(nodes["R"]["body"].get("abstract_internal").is_none());
    assert!(nodes["R2"]["body"].get("abstract_internal").is_none());
}

/// QSpec FR-375-AC-13: a list and none give different refinement node ids
/// and package ids; reordering the value's objects or entries changes
/// neither, nor does renaming the abstract alias with the value's spellings
/// renamed to match.
#[trace("FR-375-AC-13", "TC-310")]
#[test]
fn the_internal_list_enters_the_identity_and_its_order_does_not() {
    let (unit, documents) = protocol_subject();
    let two = format!("{unit}{MODEL_SUBJECT_R2}");
    let (none_id, none) = emit_with_internals(&two, &documents, None);
    let value = |alias: &str, entries: [&str; 2], objects_reversed: bool| {
        let r = json!({"refinement": "R", "internal": entries.map(|entry| entry.replace("A::", &format!("{alias}::")))});
        let r2 = json!({"refinement": "R2", "internal": []});
        if objects_reversed {
            json!([r2, r])
        } else {
            json!([r, r2])
        }
    };
    let entries = ["A::Local::Done", "lose A::Local::ch"];
    let reversed = ["lose A::Local::ch", "A::Local::Done"];
    let (listed_id, listed) =
        emit_with_internals(&two, &documents, Some(value("A", entries, false)));
    assert_ne!(listed["R"]["node_id"], none["R"]["node_id"]);
    assert_ne!(listed_id, none_id);
    assert_eq!(
        listed["R"]["body"]["abstract_internal"],
        json!(["Local::Done", "lose Local::ch"])
    );
    for (order, objects_reversed) in [(reversed, false), (entries, true), (reversed, true)] {
        let (id, nodes) =
            emit_with_internals(&two, &documents, Some(value("A", order, objects_reversed)));
        assert_eq!(id, listed_id, "{order:?} {objects_reversed}");
        assert_eq!(nodes["R"]["node_id"], listed["R"]["node_id"]);
        assert_eq!(nodes["R2"]["node_id"], listed["R2"]["node_id"]);
    }
    // The abstract alias renamed, with the value's spellings to match.
    let renamed = two
        .replace("model A ", "model S ")
        .replace("abstract A ", "abstract S ")
        .replace("A::", "S::");
    let (renamed_id, renamed_nodes) =
        emit_with_internals(&renamed, &documents, Some(value("S", entries, false)));
    assert_eq!(renamed_nodes["R"]["node_id"], listed["R"]["node_id"]);
    assert_eq!(renamed_id, listed_id);
}

/// QSpec FR-375 (AC-11, AC-25): a row-empty protocol subject keeps the
/// abstract model's root as `abstract` and names its clause as `protocol`,
/// a local dependency. Its catalog lists the clause's classes on the
/// abstract side, and the protocol is part of the node's meaning: the model
/// subject over the same models is another node.
#[trace("FR-375-AC-11", "FR-375-AC-25", "FR-375-AC-23", "TC-310")]
#[test]
fn a_row_empty_protocol_subject_names_its_clause() {
    let (unit, documents) = protocol_subject();
    let package = modelled_with_documents(&unit, &documents, Vec::new());
    let emission = emit_checked(&package).expect("the protocol subject emits");
    let node = refinement_node(&emission);
    let wire = wire(&emission);
    let clause = nodes(&wire)
        .iter()
        .find(|node| node["semantic_form"] == "protocol_clause")
        .expect("the clause node")
        .clone();
    let root = nodes(&wire)
        .iter()
        .find(|node| {
            node["semantic_form"] == "model_root"
                && node["owner"]["identity"] == "example/refinement-abstract"
        })
        .expect("the abstract root")
        .clone();
    assert_eq!(node["body"]["abstract"], root["node_id"]);
    assert_eq!(node["body"]["protocol"], clause["node_id"]);
    assert!(node["body"].get("abstract_internal").is_none());
    let dependencies = node["dependencies"].as_array().expect("dependencies");
    assert!(dependencies.contains(&clause["node_id"]));
    assert!(dependencies.contains(&root["node_id"]));
    let abstract_classes: Vec<&str> = node["body"]["class_catalog"]["abstract"]
        .as_array()
        .expect("the abstract side")
        .iter()
        .map(|class| class["class"].as_str().expect("a spelling"))
        .collect();
    assert_eq!(
        abstract_classes,
        ["Local::Done", "duplicate Local::ch", "lose Local::ch"]
    );
    for class in node["body"]["class_catalog"]["abstract"]
        .as_array()
        .into_iter()
        .flatten()
    {
        assert!(dependencies.contains(&class["node"]), "{class}");
    }

    let (model_unit, _) = catalog_refinement();
    let model_subject = modelled_with_documents(
        &model_unit
            .replace("over (input: C::Widget)", "over (input: A::Widget)")
            .replace("role w on C::Widget", "role w on A::Widget"),
        &documents,
        Vec::new(),
    );
    let model_node =
        refinement_node(&emit_checked(&model_subject).expect("the model subject emits"));
    assert!(model_node["body"].get("protocol").is_none());
    assert_ne!(model_node["node_id"], node["node_id"]);
}

/// QSpec FR-375: the protocol subject reads back through IR.
#[trace("FR-375-AC-11", "TC-310")]
#[test]
fn a_row_empty_protocol_subject_reads_back() {
    let (unit, documents) = protocol_subject();
    let package = modelled_with_documents(&unit, &documents, Vec::new());
    let emission = emit_checked(&package).expect("the protocol subject emits");
    let admitted = read(&Emitted {
        package,
        emission,
        documents,
    });
    assert!(
        admitted.is_ok(),
        "the protocol subject reads back: {admitted:?}"
    );
}

/// QSpec FR-375-AC-23, AC-26 with QSL-1186's `parallel` node: a clause over
/// the concrete model whose `run` holds a `parallel` writes its `fork` and
/// `join` classes, both naming the `parallel` control, and the `memory
/// drain` entry of its source-default model (`sc` when none is written).
#[trace("FR-375-AC-23", "FR-375-AC-26", "TC-310")]
#[test]
fn a_parallel_clause_writes_fork_join_and_drain_classes() {
    let (unit, documents) = catalog_refinement();
    let unit = unit.replace(
        "run sequence Main { }",
        "run sequence Main { parallel par { branch a sequence A { } branch b sequence B { } } join all [a, b]; }",
    );
    let package = modelled_with_documents(&unit, &documents, Vec::new());
    let emission = emit_checked(&package).expect("the refinement emits");
    let node = refinement_node(&emission);
    let concrete = node["body"]["class_catalog"]["concrete"]
        .as_array()
        .expect("the concrete side")
        .clone();
    let by_class = |name: &str| {
        concrete
            .iter()
            .find(|class| class["class"] == name)
            .unwrap_or_else(|| panic!("the class {name}: {concrete:?}"))
            .clone()
    };
    let (fork, join) = (by_class("fork Local::par"), by_class("join Local::par"));
    assert_eq!(fork["node"], join["node"]);
    assert_eq!(fork["binder_types"], json!([]));
    let wire = wire(&emission);
    let parallel = nodes(&wire)
        .iter()
        .find(|candidate| candidate["node_id"] == fork["node"])
        .expect("the parallel node")
        .clone();
    assert_eq!(parallel["semantic_form"], "control");
    assert_eq!(
        parallel["body"]["operation"]["member"]["operator"],
        "parallel"
    );
    assert_eq!(
        by_class("memory drain"),
        json!({"class": "memory drain", "memory_model": "sc", "drain": "store-buffer-flush"})
    );
}

/// QSpec FR-375: the emitted catalog reads back through IR.
#[trace("FR-375-AC-23", "TC-310")]
#[test]
fn a_model_refinement_class_catalog_reads_back() {
    let (unit, documents) = catalog_refinement();
    let package = modelled_with_documents(&unit, &documents, Vec::new());
    let emission = emit_checked(&package).expect("the refinement with a catalog emits");
    let admitted = read(&Emitted {
        package,
        emission,
        documents,
    });
    assert!(admitted.is_ok(), "the catalog reads back: {admitted:?}");
}

/// Every node body of every package the admission corpus emits is in QSpec
/// FR-322's stratified grammar, classified as Leaf, Group, Tuple, Member or
/// Body by a walk of the emitted JSON (FR-264-AC-2's walk over the families
/// the corpus covers).
#[trace("TC-738", "FR-264-AC-2", "TC-416")]
#[test]
fn every_emitted_body_of_the_corpus_is_in_the_stratified_grammar() {
    let mut bodies = 0;
    for fixture in Fixture::ALL {
        for emitted in fixture.emit() {
            let written = wire(&emitted.emission);
            for node in nodes(&written) {
                if node["node_tag"] == "relation" && node["semantic_form"] == "refinement" {
                    // A refinement body's top level is the FR-375 closed
                    // shape; its row terms are checked by the V2 reader's
                    // refinement grammar, not the ordinary term stratum.
                    assert_eq!(node["body"]["term"], "refinement");
                    continue;
                }
                assert!(
                    qsl_semantics::check::stratum::is_stratified_body(&node["body"]),
                    "{fixture:?}: a written body is outside the stratified grammar: {}",
                    node["body"]
                );
                bodies += 1;
            }
        }
    }
    assert!(bodies > 0, "the corpus emits nodes");
}

/// QSL-946: the dimension and units the quantity-typed operand functions
/// below are written over.
const QUANTITY_UNITS: &str = "dimension Length;\n\
     unit m : Length = rational(1, 1);\n\
     unit km : Length = rational(1000, 1) * m;\n";

/// One operation over quantity-typed operands, written in genuine source
/// through the `Quantity<u>` type form, and the operation identity its
/// application lowers to.
const QUANTITY_OPERATIONS: &[(&str, &str, Option<&str>)] = &[
    (
        "eq",
        "function op using v(a: Quantity<m>, b: Quantity<m>): Boolean pure { a = b }",
        Some("quire.op.quantity.eq"),
    ),
    (
        "ne",
        "function op using v(a: Quantity<m>, b: Quantity<m>): Boolean pure { a != b }",
        Some("quire.op.quantity.ne"),
    ),
    (
        "lt",
        "function op using v(a: Quantity<m>, b: Quantity<m>): Boolean pure { a < b }",
        Some("quire.op.quantity.lt"),
    ),
    (
        "add",
        "function op using v(a: Quantity<m>, b: Quantity<m>): Quantity<m> pure { a + b }",
        Some("quire.op.quantity.add"),
    ),
    (
        "convert to the same unit",
        "function op using v(q: Quantity<m>): Quantity<m> pure { convert<Quantity<m>>(q) }",
        // A conversion to the operand's own unit is the identity: it lowers
        // to its operand, so no application is written.
        None,
    ),
    (
        "convert to another unit",
        "function op using v(q: Quantity<m>): Quantity<km> pure { convert<Quantity<km>>(q) }",
        Some("quire.op.quantity.convert"),
    ),
];

/// FR-091-AC-44, FR-093 (QSL-946, IR-263): each operation over operands
/// typed `Quantity<u>` in genuine source emits a package IR's strict
/// checked-package/v2 reader admits at its `package_id`, and the package
/// applies the operation's `quire.op.quantity.*` identity.
#[trace("FR-091-AC-44", "FR-093-AC-19", "TC-483", "TC-416")]
#[test]
fn quantity_typed_source_operations_are_admitted_by_the_strict_reader() {
    let mut failures = Vec::new();
    for (name, function, identity) in QUANTITY_OPERATIONS {
        let (_, package) = package_from_text(&format!("{QUANTITY_UNITS}{function}\n"));
        let emitted = Emitted::placed(package, Vec::new());
        let outcome = Outcome {
            families: families(&emitted.package),
            read: read(&emitted),
            emitted,
        };
        let applied = identity.is_none_or(|identity| {
            outcome
                .families
                .values()
                .flatten()
                .any(|node| outcome.applies(node, identity))
        });
        // The identity conversion writes no node: the body is the parameter.
        if identity.is_none()
            && outcome
                .families
                .keys()
                .any(|kind| matches!(kind, CheckedNodeKind::Expression(_)))
        {
            failures.push(format!("{name}: wrote an expression node"));
        }
        if !applied {
            failures.push(format!("{name}: no application of {identity:?}"));
        }
        match &outcome.read {
            Ok(read)
                if read.package.package_id() == outcome.emitted.emission.package.package_id() => {}
            other => failures.push(format!("{name}: not admitted: {other:?}")),
        }
    }
    assert!(failures.is_empty(), "{failures:#?}");
}

/// QSL-974: the dimension and units quantity literals are written over.
const QUANTITY_TIME: &str = "dimension Time;\n\
     unit s : Time = rational(1, 1);\n\
     unit ms : Time = rational(1, 1000) * s;\n\
     function f using v(d: Quantity<ms>): Boolean pure { true }\n";

/// The `(value, type digest)` of each `quantity` literal node of `wire`, and
/// the digest of each unit node by name.
fn quantity_literals(wire: &Value) -> (Vec<(String, Value)>, BTreeMap<String, Value>) {
    let mut literals = Vec::new();
    let mut units = BTreeMap::new();
    for node in nodes(wire) {
        let body = &node["body"];
        if body["term"] == "literal" {
            assert_eq!(node["semantic_form"], "literal");
            // No literal of another kind is typed at a unit node.
            if body["value_kind"] == "quantity" {
                assert_eq!(node["semantic_type"], body["type"]);
                literals.push((
                    body["value"].as_str().expect("a spelled value").to_owned(),
                    body["type"].clone(),
                ));
            } else {
                assert_ne!(body["value_kind"], "rational", "{node}");
            }
        }
        if node["semantic_form"] == "unit" {
            let name = node["declaration"]["qualified_name"][0]
                .as_str()
                .expect("a unit name");
            units.insert(name.to_owned(), node["node_id"].clone());
        }
    }
    (literals, units)
}

/// FR-093-AC-33, FR-094, FR-322-AC-79 (TC-483 step 6): a quantity literal
/// lowers to a `value`/`literal` node of `value_kind` `quantity` typed by its
/// unit's node, equal literals in one unit share one node, a rational is
/// never emitted for one, and IR's strict v2 reader admits each package.
#[trace("FR-093-AC-33", "FR-094", "TC-483")]
#[test]
fn quantity_literals_lower_to_shared_quantity_literal_nodes_the_strict_reader_admits() {
    // (functions beside f, expected (value, unit) literals in order)
    let cases: [(&str, &[(&str, &str)]); 7] = [
        ("function h using v(): Boolean pure { f(3 ms) }", &[("3", "ms")]),
        // `h`, `h2`, `h3` and `h4` together: `h4` shares `h`'s node, and
        // `0.5` and `rational(2, 4)` share one `1/2` node.
        (
            "function h using v(): Boolean pure { f(3 ms) }\n\
             function h2 using v(): Boolean pure { f(0.5 ms) }\n\
             function h3 using v(): Boolean pure { f(rational(2, 4) ms) }\n\
             function h4 using v(): Boolean pure { f(3 ms) }",
            &[("1/2", "ms"), ("3", "ms")],
        ),
        (
            "function n using v(): Boolean pure { f(rational(-3, 1) ms) }",
            &[("-3", "ms")],
        ),
        (
            "function k using v(d: Quantity<s>): Boolean pure { d = rational(1, 1000) s }",
            &[("1/1000", "s")],
        ),
        (
            "function c using v(d: Quantity<ms>): Boolean pure { convert<Quantity<s>>(d) = 3 s }",
            &[("3", "s")],
        ),
        // The same magnitude in two units is two nodes.
        (
            "function a using v(d: Quantity<s>, e: Quantity<ms>): Boolean pure { d = 3 s and e = 3 ms }",
            &[("3", "ms"), ("3", "s")],
        ),
        ("function z using v(): Boolean pure { f(0 ms) }", &[("0", "ms")]),
    ];
    for (functions, expected) in cases {
        let (_, package) = package_from_text(&format!("{QUANTITY_TIME}{functions}\n"));
        let emitted = Emitted::placed(package, Vec::new());
        let written = wire(&emitted.emission);
        let (mut literals, units) = quantity_literals(&written);
        let mut want: Vec<(String, Value)> = expected
            .iter()
            .map(|(value, unit)| ((*value).to_owned(), units[*unit].clone()))
            .collect();
        literals.sort_by_key(|(value, unit)| (value.clone(), unit.to_string()));
        want.sort_by_key(|(value, unit)| (value.clone(), unit.to_string()));
        assert_eq!(literals, want, "{functions}");
        match read(&emitted) {
            Ok(read) if read.package.package_id() == emitted.emission.package.package_id() => {}
            other => panic!("{functions}: not admitted: {other:?}"),
        }
    }
}

/// The node of `wire` keyed `id`.
fn node_of<'a>(wire: &'a Value, id: &Value) -> &'a Value {
    nodes(wire)
        .iter()
        .find(|node| &node["node_id"] == id)
        .unwrap_or_else(|| panic!("no node {id}"))
}

/// The node a `reference` term targets.
fn target<'a>(wire: &'a Value, term: &Value) -> &'a Value {
    assert_eq!(term["term"], "reference", "{term}");
    node_of(wire, &term["target"])
}

/// The root body node of function `name`.
fn function_body<'a>(wire: &'a Value, name: &str) -> &'a Value {
    let function = nodes(wire)
        .iter()
        .find(|node| {
            node["node_tag"] == "function" && node["declaration"]["qualified_name"][0] == name
        })
        .unwrap_or_else(|| panic!("no function {name}"));
    let body = function["body"]["members"]
        .as_array()
        .unwrap()
        .iter()
        .find(|member| member["name"] == "body")
        .expect("a body member");
    target(wire, &body["value"])
}

/// Whether `node` is a `quantity` literal of `value`.
fn is_quantity_literal(node: &Value, value: &str) -> bool {
    node["body"]["term"] == "literal"
        && node["body"]["value_kind"] == "quantity"
        && node["body"]["value"] == value
}

/// FR-093-AC-33 (TC-483 step 6): the call's argument references the literal
/// node; `k`'s equality is over `[ref(d), ref(literal)]`; `c`'s conversion
/// has `rounding` = `exact` over `d` and its equality runs over the
/// conversion node and the `3 s` literal.
#[trace("FR-093-AC-33", "TC-483")]
#[test]
fn quantity_literal_nodes_are_referenced_by_their_operations() {
    let (_, package) = package_from_text(&format!(
        "{QUANTITY_TIME}function h using v(): Boolean pure {{ f(3 ms) }}\n\
         function k using v(d: Quantity<s>): Boolean pure {{ d = rational(1, 1000) s }}\n\
         function c using v(d: Quantity<ms>): Boolean pure {{ convert<Quantity<s>>(d) = 3 s }}\n"
    ));
    let emitted = Emitted::placed(package, Vec::new());
    let written = wire(&emitted.emission);

    let call = function_body(&written, "h");
    assert_eq!(
        call["body"]["operation"]["identity"],
        "quire.op.function.call"
    );
    let arguments = call["body"]["arguments"].as_array().unwrap();
    let literal = target(&written, arguments.last().unwrap());
    assert!(is_quantity_literal(literal, "3"), "{literal}");

    let equal = function_body(&written, "k");
    assert_eq!(
        equal["body"]["operation"]["identity"],
        "quire.op.quantity.eq"
    );
    let arguments = equal["body"]["arguments"].as_array().unwrap();
    assert_eq!(arguments.len(), 2);
    let parameter = target(&written, &arguments[0]);
    assert_eq!(parameter["semantic_form"], "parameter");
    assert!(is_quantity_literal(
        target(&written, &arguments[1]),
        "1/1000"
    ));

    let equal = function_body(&written, "c");
    assert_eq!(
        equal["body"]["operation"]["identity"],
        "quire.op.quantity.eq"
    );
    let arguments = equal["body"]["arguments"].as_array().unwrap();
    assert_eq!(arguments.len(), 2);
    let convert = target(&written, &arguments[0]);
    assert_eq!(
        convert["body"]["operation"]["identity"],
        "quire.op.quantity.convert"
    );
    assert_eq!(
        convert["body"]["operation"]["mode"],
        json!({"kind": "rounding", "value": "exact"})
    );
    let converted = convert["body"]["arguments"].as_array().unwrap();
    assert_eq!(converted.len(), 1);
    assert_eq!(
        target(&written, &converted[0])["semantic_form"],
        "parameter"
    );
    assert!(is_quantity_literal(target(&written, &arguments[1]), "3"));
    assert!(read(&emitted).is_ok());
}

/// Two copies of the config-version model, each with a state machine, as
/// the abstract `A` and concrete `C` of a refinement `R` written by `rows`.
/// `ConfigVersion` keeps only its `versionNumber` field (`parent` is an
/// `Option<Reference<_>>`, whose row is a nested reference row the default
/// build does not support), and its operations are `operations`, each
/// shaped as `attemptUpdate` is.
pub(super) fn rows_refinement(rows: &str, operations: &[&str]) -> (String, Vec<Vec<u8>>) {
    rows_refinement_with(rows, operations, RowsModel::default())
}

/// How [`rows_refinement_with`] edits the config-version model.
#[derive(Clone, Copy, Default)]
struct RowsModel {
    /// Each operation takes one required Boolean parameter `flag`.
    flag: bool,
    /// `ConfigVersion` also has `epoch`, a copy of `versionNumber`, so an
    /// object row can map two fields.
    epoch: bool,
}

/// [`rows_refinement`] over the model `model` edits.
fn rows_refinement_with(
    rows: &str,
    operations: &[&str],
    model: RowsModel,
) -> (String, Vec<Vec<u8>>) {
    let RowsModel { flag, epoch } = model;
    let mut document: Value =
        serde_json::from_str(config_version_unit::DOMAIN_PACKAGE).expect("the model is JSON");
    for object in document["types"].as_array_mut().expect("types") {
        if let Some(fields) = object.get_mut("fields").and_then(Value::as_array_mut) {
            fields.retain(|field| field["name"] != "parent");
            let copy = fields
                .iter()
                .find(|field| field["name"] == "versionNumber")
                .filter(|_| epoch)
                .map(|field| {
                    let text = serde_json::to_string(field).expect("the field serializes");
                    serde_json::from_str::<Value>(&text.replace("versionNumber", "epoch"))
                        .expect("the copy parses")
                });
            fields.extend(copy);
        }
        if let Some(declared) = object.get_mut("operations").and_then(Value::as_array_mut) {
            let Some(template) = declared.first().cloned() else {
                continue;
            };
            *declared = operations
                .iter()
                .map(|name| {
                    let mut operation = template.clone();
                    operation["name"] = json!(name);
                    operation["identity"] =
                        json!(format!("ix://example/config-version/ConfigVersion/{name}"));
                    if flag {
                        let identity =
                            format!("ix://example/config-version/ConfigVersion/{name}/flag");
                        operation["params"] = json!([{
                            "identity": identity,
                            "name": "flag",
                            "typeRef": "ix://quire/native/Boolean",
                            "presence": "required",
                            "nullable": false,
                            "defaultKind": "none",
                            "multiplicity": {"lower": 1, "upper": 1, "ordered": false, "unique": true},
                            "origin": {"generated": {
                                "generatorIdentity": identity,
                                "generatorVersion": "1.0.0",
                                "inputIdentities": [identity],
                            }},
                        }]);
                    }
                    operation
                })
                .collect();
        }
    }
    let base = serde_json::to_vec(&document).expect("the model serializes");
    let abstract_document =
        with_state_machine(&base, "example/config-version", "example/rows-abstract");
    let concrete_document =
        with_state_machine(&base, "example/config-version", "example/rows-concrete");
    let unit = format!(
        "{COMPLETE_V1_HEADER}profile inf = \"quire.temporal.infinite-trace/v1\";\n\
         model A = \"example/rows-abstract\" version \"1.0.0\" digest \"sha256-jcs:{}\";\n\
         model C = \"example/rows-concrete\" version \"1.0.0\" digest \"sha256-jcs:{}\";\n\
         refinement R using inf abstract A concrete C {{\n{rows}\n}}\n",
        document_digest(&abstract_document),
        document_digest(&concrete_document),
    );
    (unit, vec![abstract_document, concrete_document])
}

/// The population and object rows of every [`rows_refinement`].
pub(super) const MAPS: &str = "population A::config_history from C::config_history;\n\
    object A::ConfigVersion from C::ConfigVersion { versionNumber = self.versionNumber; }";

/// `rows` over the operations `operations`, checked and emitted, with the
/// unit's text.
fn emit_rows(rows: &str, operations: &[&str]) -> (Emitted, String) {
    let (unit, documents) = rows_refinement(rows, operations);
    let package = modelled_with_documents(&unit, &documents, Vec::new());
    let emission = emit_checked(&package).expect("the refinement with rows emits");
    (
        Emitted {
            package,
            emission,
            documents,
        },
        unit,
    )
}

/// The `[start, end)` of `needle` in `text`.
fn span_of(text: &str, needle: &str) -> (u64, u64) {
    let start = text
        .find(needle)
        .unwrap_or_else(|| panic!("{needle} in {text}"));
    let start = u64::try_from(start).expect("an offset");
    (
        start,
        start + u64::try_from(needle.len()).expect("a length"),
    )
}

/// QSpec FR-375 rows (FR-375-AC-21, AC-24): a population row names both
/// population nodes and an object row both object-type nodes and each field
/// by name with its expression over the concrete object (`self`, FR-341's
/// level-0 `parameter`); the node depends on each, and IR's reader admits
/// the package. A model with no operation needs no step row, and with
/// every abstract field mapped the owner is proof-eligible: one `generated`
/// occurrence, at its declaration.
#[trace("FR-375-AC-21", "FR-375-AC-24", "FR-322-AC-88", "TC-310")]
#[test]
fn population_and_object_rows_read_back() {
    let (emitted, _) = emit_rows(MAPS, &[]);
    let node = refinement_node(&emitted.emission);
    let body = &node["body"];
    let population = &body["populations"][0];
    let object = &body["objects"][0];
    let field = &object["fields"][0];
    assert_eq!(field["name"], "versionNumber");
    assert_eq!(field["expression"]["term"], "reference");
    let dependencies = node["dependencies"].as_array().expect("dependencies");
    for key in [
        &population["abstract"],
        &population["concrete"],
        &object["abstract_type"],
        &object["concrete_type"],
        &field["expression"]["target"],
    ] {
        assert!(dependencies.contains(key), "{key} in {dependencies:?}");
    }
    assert_eq!(
        node["occurrences"],
        json!([{"role": "declaration", "ordinal": 0}, {"role": "generated", "ordinal": 0}])
    );
    assert!(wire(&emitted.emission)
        .get("refinement_step_rows")
        .is_none());
    let admitted = read(&emitted);
    assert!(admitted.is_ok(), "the rows read back: {admitted:?}");
}

/// QSpec FR-375-AC-22, FR-322-AC-88 and AC-128 (AC-87 does not apply: QSL
/// derives no node per step row, the rows are members of the owner's
/// body): each step row has
/// one `refinement_step_rows` entry at its own source row, keyed by the
/// owner and its canonical ordinal, and a proof-eligible owner a
/// `generated` occurrence per row (ordinal `i + 1` at row `i`'s region)
/// beside the one at its declaration. Writing the rows in the other order
/// moves the regions but not the `steps` order, the node id or the
/// package id.
#[trace("FR-375-AC-22", "FR-322-AC-88", "FR-322-AC-128", "TC-310", "TC-233")]
#[test]
fn step_rows_carry_their_source_regions_by_canonical_ordinal() {
    let operations = ["attemptUpdate", "rollback"];
    let update = "step C::ConfigVersion::attemptUpdate -> A::ConfigVersion::attemptUpdate(self);";
    let rollback = "step C::ConfigVersion::rollback -> stutter;";
    let (forward, forward_unit) = emit_rows(&format!("{MAPS}\n{update}\n{rollback}"), &operations);
    let (reversed, reversed_unit) =
        emit_rows(&format!("{MAPS}\n{rollback}\n{update}"), &operations);
    let (forward_node, reversed_node) = (
        refinement_node(&forward.emission),
        refinement_node(&reversed.emission),
    );
    assert_eq!(forward_node["node_id"], reversed_node["node_id"]);
    assert_eq!(forward_node["body"], reversed_node["body"]);
    assert_eq!(
        wire(&forward.emission)["package_id"],
        wire(&reversed.emission)["package_id"]
    );
    let classes: Vec<&str> = forward_node["body"]["steps"]
        .as_array()
        .expect("steps")
        .iter()
        .map(|step| step["source_class"].as_str().expect("a class"))
        .collect();
    assert_eq!(
        classes,
        ["ConfigVersion::attemptUpdate", "ConfigVersion::rollback"]
    );
    for (emitted, unit, node) in [
        (&forward, &forward_unit, &forward_node),
        (&reversed, &reversed_unit, &reversed_node),
    ] {
        let written = wire(&emitted.emission);
        let rows = written["refinement_step_rows"]
            .as_array()
            .expect("a step-row sidecar");
        let at = |row: &str| span_of(unit, row);
        let expected = [at(update), at(rollback)];
        assert_eq!(rows.len(), 2);
        for (ordinal, (row, (start, end))) in rows.iter().zip(expected).enumerate() {
            assert_eq!(row["node_id"], node["node_id"]);
            assert_eq!(row["ordinal"], ordinal);
            assert_eq!(
                (row["region"]["start"].clone(), row["region"]["end"].clone()),
                (json!(start), json!(end))
            );
        }
        let generated: Vec<(u64, u64, u64)> = written["source_map"]
            .as_array()
            .expect("source map")
            .iter()
            .filter(|entry| entry["node_id"] == node["node_id"] && entry["role"] == "generated")
            .map(|entry| {
                (
                    entry["ordinal"].as_u64().expect("an ordinal"),
                    entry["regions"][0]["start"].as_u64().expect("a start"),
                    entry["regions"][0]["end"].as_u64().expect("an end"),
                )
            })
            .collect();
        let declaration = span_of(unit, "refinement R");
        assert_eq!(generated.len(), 3, "{generated:?}");
        assert_eq!(generated[0].1, declaration.0);
        assert_eq!((generated[1].1, generated[1].2), expected[0]);
        assert_eq!((generated[2].1, generated[2].2), expected[1]);
    }
}

/// The refusals of [`rows_refinement`] over `rows`, which must not check.
fn refused_rows(rows: &str, operations: &[&str]) -> Vec<qsl_semantics::check::CheckCause> {
    let (unit, documents) = rows_refinement(rows, operations);
    let packages = qsl_semantics::model::intake::package_input(documents.iter().map(Vec::as_slice));
    assemble_with_models(unit.as_bytes(), &packages)
        .expect("the unit assembles")
        .check(CheckingLimits::default())
        .map(|_| ())
        .expect_err("the refinement is not emitted")
        .into_iter()
        .map(|refusal| refusal.cause)
        .collect()
}

/// A row whose expression reads a whole concrete population or builds a
/// `seq` has no node form (STD-493), like a `history` row: the
/// refinement refuses `RefinementNotEmitted` naming it, never a fault or a
/// malformed package.
#[trace("FR-375-AC-21", "TC-310")]
#[test]
fn whole_population_and_seq_rows_are_not_emitted() {
    for field in [
        "versionNumber = count(s in C::config_history where s.versionNumber = 1);",
        "versionNumber = if size(seq(i in 0 .. 1: i)) = 2 then self.versionNumber \
         else self.versionNumber;",
    ] {
        let rows = format!(
            "population A::config_history from C::config_history;\n\
             object A::ConfigVersion from C::ConfigVersion {{ {field} }}"
        );
        let refusals = refused_rows(&rows, &[]);
        assert!(
            matches!(
                &refusals[..],
                [qsl_semantics::check::CheckCause::RefinementNotEmitted { name, .. }] if name == "R"
            ),
            "{field}: {refusals:?}"
        );
    }
}

/// QSpec FR-341 row binders (SR-1735002 FND-002): over an operation with a
/// Boolean parameter `flag`, a step row binds `self` at level 0 and `flag`
/// at level 1, each a `parameter` node, whether the row passes `flag`,
/// `not flag` or leaves it open (`_`).
#[trace("FR-375-AC-22", "TC-310")]
#[test]
fn a_row_parameter_above_self_binds_at_level_one() {
    for arguments in ["(self, flag)", "(self, not flag)", "(self, _)"] {
        let step = format!(
            "step C::ConfigVersion::attemptUpdate -> A::ConfigVersion::attemptUpdate{arguments};"
        );
        let (unit, documents) = rows_refinement_with(
            &format!("{MAPS}\n{step}"),
            &["attemptUpdate"],
            RowsModel {
                flag: true,
                ..RowsModel::default()
            },
        );
        let package = modelled_with_documents(&unit, &documents, Vec::new());
        let emission = emit_checked(&package).expect("the refinement emits");
        let mut parameters: Vec<(String, String)> = wire(&emission)["semantic_graph"]["nodes"]
            .as_array()
            .expect("nodes")
            .iter()
            .filter(|node| node["semantic_form"] == "parameter")
            .map(|parameter| {
                let text = |name: &str| {
                    super::protocol_graph::member(&parameter["body"], name)["value"]
                        .as_str()
                        .expect("a literal")
                        .to_owned()
                };
                (text("name"), text("level"))
            })
            .filter(|(name, _)| name == "self" || name == "flag")
            .collect();
        parameters.sort();
        parameters.dedup();
        assert_eq!(
            parameters,
            [
                ("flag".to_owned(), "1".to_owned()),
                ("self".to_owned(), "0".to_owned())
            ],
            "{arguments}"
        );
        // The read-back waits on IR-1077 (`operation_rows_read_back`).
    }
}

/// QSpec FR-322-AC-129: an owner that is not proof-eligible (an `any`
/// step) still carries every step row's region, and no `generated`
/// occurrence.
#[trace("FR-322-AC-129", "TC-233", "TC-310")]
#[test]
fn an_ineligible_owner_has_step_rows_and_no_generated_occurrence() {
    let row = "step C::ConfigVersion::attemptUpdate -> any;";
    let (emitted, unit) = emit_rows(&format!("{MAPS}\n{row}"), &["attemptUpdate"]);
    let node = refinement_node(&emitted.emission);
    assert_eq!(
        node["occurrences"],
        json!([{"role": "declaration", "ordinal": 0}])
    );
    assert_eq!(node["body"]["steps"][0]["target"], json!({"kind": "any"}));
    let written = wire(&emitted.emission);
    let (start, end) = span_of(&unit, row);
    assert_eq!(written["refinement_step_rows"][0]["region"]["start"], start);
    assert_eq!(written["refinement_step_rows"][0]["region"]["end"], end);
}

/// QSpec FR-375 `assume` and `ensure`: each names its operation's
/// `operation_anchor` with the written kind and granularity, and an
/// operation's step row names that anchor as its `source_node` (no class
/// catalog), and an abstract target's operation its own anchor.
#[trace("FR-375-AC-21", "TC-310")]
#[test]
fn fairness_and_operation_rows_name_operation_anchors() {
    let (emitted, _) = emit_rows(
        &format!(
            "{MAPS}\n\
             step C::ConfigVersion::attemptUpdate -> A::ConfigVersion::attemptUpdate(self);\n\
             assume fair weak each C::ConfigVersion::attemptUpdate;\n\
             ensure fair strong whole A::ConfigVersion::attemptUpdate;"
        ),
        &["attemptUpdate"],
    );
    let written = wire(&emitted.emission);
    let node = refinement_node(&emitted.emission);
    let anchor = |key: &Value| {
        nodes(&written)
            .iter()
            .find(|candidate| candidate["node_id"] == *key)
            .unwrap_or_else(|| panic!("a node {key}"))["semantic_form"]
            .clone()
    };
    let body = &node["body"];
    let assume = &body["assume"][0];
    assert_eq!(
        (&assume["kind"], &assume["granularity"]),
        (&json!("weak"), &json!("each"))
    );
    assert_eq!(anchor(&assume["operation"]), "operation_anchor");
    let ensure = &body["ensure"][0];
    assert_eq!(
        (&ensure["kind"], &ensure["granularity"]),
        (&json!("strong"), &json!("whole"))
    );
    assert_eq!(anchor(&ensure["operation"]), "operation_anchor");
    let step = &body["steps"][0];
    assert_eq!(step["source_node"], assume["operation"]);
    assert_eq!(step["target"]["operation"], ensure["operation"]);
    assert_eq!(anchor(&step["target"]["receiver"]["target"]), "parameter");
}

/// The step, `assume` and `ensure` rows read back once IR resolves an
/// `operation_anchor` against its declaring type (IR-1077).
#[trace("FR-375-AC-21", "TC-310")]
#[test]
#[ignore = "IR-1077: IR requires a row's operation_anchor context to be the model root"]
fn operation_rows_read_back() {
    let (emitted, _) = emit_rows(
        &format!(
            "{MAPS}\n\
             step C::ConfigVersion::attemptUpdate -> A::ConfigVersion::attemptUpdate(self);\n\
             assume fair weak each C::ConfigVersion::attemptUpdate;"
        ),
        &["attemptUpdate"],
    );
    let admitted = read(&emitted);
    assert!(admitted.is_ok(), "the rows read back: {admitted:?}");
}

/// A protocol subject `A::Local` refined by the concrete protocol `Impl`,
/// whose step rows are `rows`.
fn protocol_rows(rows: &[&str]) -> (String, Vec<Vec<u8>>) {
    protocol_rows_running("sequence Main { }", rows)
}

/// [`protocol_rows`] whose concrete protocol `Impl` runs `run`.
pub(super) fn protocol_rows_running(run: &str, rows: &[&str]) -> (String, Vec<Vec<u8>>) {
    let (unit, documents) = protocol_subject();
    let unit = unit.replace(
        "refinement R using inf abstract A::Local concrete C {}",
        &format!(
            "protocol Impl using p over (input: C::Widget) on origin {{\n\
             role w on C::Widget;\n\
             channel ch from w to w carries Boolean ordering unordered delivery unknown \
             capacity 2 overflow block;\n\
             run {run}\n\
             finish Done as (d: Boolean) {{ true }};\n\
             }}\n\
             refinement R using inf abstract A::Local concrete C {{\n{}\n}}",
            rows.join("\n")
        ),
    );
    (unit, documents)
}

/// The protocol-subject rows of [`protocol_rows`]: the finish, then the
/// channel's two classes.
pub(super) const PROTOCOL_ROWS: [&str; 3] = [
    "step lose C::Impl::ch -> stutter;",
    "step C::Impl::Done -> stutter;",
    "step duplicate C::Impl::ch -> stutter;",
];

/// QSpec FR-375-AC-19 (the declaration-naming rows): a protocol subject's
/// `steps` are ordered by their node's digest, rows of one node by their
/// alias-free class spelling (`duplicate Impl::ch` before `lose Impl::ch`),
/// each naming its class catalog node, whatever order they are written in.
#[trace("FR-375-AC-19", "TC-310")]
#[test]
fn protocol_step_rows_order_by_node_then_class_spelling() {
    let steps = |rows: &[&str]| {
        let (unit, documents) = protocol_rows(rows);
        let package = modelled_with_documents(&unit, &documents, Vec::new());
        let emission = emit_checked(&package).expect("the protocol rows emit");
        let node = refinement_node(&emission);
        let catalog = node["body"]["class_catalog"]["concrete"].clone();
        (node["body"]["steps"].clone(), catalog)
    };
    let (written, catalog) = steps(&PROTOCOL_ROWS);
    let steps_of = |steps: &Value| -> Vec<(String, String)> {
        steps
            .as_array()
            .expect("steps")
            .iter()
            .map(|step| {
                (
                    step["source_node"]["digest"]
                        .as_str()
                        .expect("a node")
                        .to_owned(),
                    step["source_class"].as_str().expect("a class").to_owned(),
                )
            })
            .collect()
    };
    let ordered = steps_of(&written);
    let mut sorted = ordered.clone();
    sorted.sort();
    assert_eq!(ordered, sorted);
    let classes: Vec<&str> = ordered.iter().map(|(_, class)| class.as_str()).collect();
    let duplicate = classes
        .iter()
        .position(|class| *class == "duplicate Impl::ch");
    let lose = classes.iter().position(|class| *class == "lose Impl::ch");
    assert!(duplicate < lose && lose.is_some(), "{classes:?}");
    for step in written.as_array().expect("steps") {
        let entry = catalog
            .as_array()
            .expect("a concrete catalog")
            .iter()
            .find(|entry| entry["class"] == step["source_class"])
            .unwrap_or_else(|| panic!("{} is catalogued", step["source_class"]));
        assert_eq!(entry["node"], step["source_node"]);
    }
    let reversed: Vec<&str> = PROTOCOL_ROWS.iter().rev().copied().collect();
    assert_eq!(steps(&reversed).0, written);
}

/// A protocol subject's step rows read back through IR's catalog rebuild.
#[trace("FR-375-AC-19", "TC-310")]
#[test]
fn protocol_step_rows_read_back() {
    let (unit, documents) = protocol_rows(&PROTOCOL_ROWS);
    let package = modelled_with_documents(&unit, &documents, Vec::new());
    let emission = emit_checked(&package).expect("the protocol rows emit");
    let admitted = read(&Emitted {
        package,
        emission,
        documents,
    });
    assert!(admitted.is_ok(), "the rows read back: {admitted:?}");
}

/// [`PROTOCOL_ROWS`] with a `tso` parallel `Par` in `Impl`'s run: its
/// `fork`/`join` rows and the `memory drain` and `memory flush` rows.
const MEMORY_RUN: &str = "sequence Main { parallel Par memory tso { branch a sequence Ay { } \
     branch b sequence Bee { } } join all [a, b]; }";
const MEMORY_ROWS: [&str; 7] = [
    "step memory flush -> stutter;",
    "step lose C::Impl::ch -> stutter;",
    "step fork C::Impl::Par -> stutter;",
    "step memory drain -> stutter;",
    "step C::Impl::Done -> stutter;",
    "step join C::Impl::Par -> stutter;",
    "step duplicate C::Impl::ch -> stutter;",
];

/// The checked package and emission of [`protocol_rows_running`].
fn memory_emission(run: &str, rows: &[&str]) -> Emitted {
    let (unit, documents) = protocol_rows_running(run, rows);
    let package = modelled_with_documents(&unit, &documents, Vec::new());
    let emission = emit_checked(&package).expect("the memory rows emit");
    Emitted {
        package,
        emission,
        documents,
    }
}

/// QSpec FR-375-AC-19 (the memory rows, QSL-1181): the `memory`
/// rows come after every declaration-naming row, `drain` before `flush`,
/// neither with a `source_node`; writing the rows in another order changes
/// neither the node id nor the package id.
#[trace("FR-375-AC-19", "TC-310")]
#[test]
fn memory_rows_go_last_drain_before_flush() {
    let forward = memory_emission(MEMORY_RUN, &MEMORY_ROWS);
    let reversed: Vec<&str> = MEMORY_ROWS.iter().rev().copied().collect();
    let reversed = memory_emission(MEMORY_RUN, &reversed);
    let node = refinement_node(&forward.emission);
    let steps = node["body"]["steps"].as_array().expect("steps");
    let tail: Vec<&Value> = steps[steps.len() - 2..].iter().collect();
    assert_eq!(
        tail.iter()
            .map(|step| step["source_class"].as_str().expect("a class"))
            .collect::<Vec<_>>(),
        ["memory drain", "memory flush"]
    );
    for step in &tail {
        assert!(step.get("source_node").is_none(), "{step}");
    }
    for step in &steps[..steps.len() - 2] {
        assert!(step.get("source_node").is_some(), "{step}");
    }
    assert_eq!(
        refinement_node(&reversed.emission)["node_id"],
        node["node_id"]
    );
    assert_eq!(
        wire(&reversed.emission)["package_id"],
        wire(&forward.emission)["package_id"]
    );
}

/// QSpec FR-375-AC-23 and AC-26: over `parallel` controls defaulting to
/// `sc`, `tso` and `ra` (and a second `sc`), the concrete catalog holds
/// `memory drain` once per model in the order `ra`, `sc`, `tso`, each with
/// `drain: store-buffer-flush` and no `node` or `binder_types`, then
/// `memory flush` once for `tso`.
#[trace("FR-375-AC-23", "FR-375-AC-26", "TC-310")]
#[test]
fn memory_catalog_entries_are_one_drain_per_model_then_flush() {
    let parallel = |name: &str, memory: &str| {
        format!(
            "parallel {name} memory {memory} {{ branch a sequence {name}A {{ }} \
             branch b sequence {name}B {{ }} }} join all [a, b];"
        )
    };
    let run = format!(
        "sequence Main {{ {} {} {} {} }}",
        parallel("Sc", "sc"),
        parallel("Tso", "tso"),
        parallel("Ra", "ra"),
        parallel("Again", "sc"),
    );
    let mut rows: Vec<String> = PROTOCOL_ROWS.iter().map(|row| (*row).to_owned()).collect();
    for name in ["Sc", "Tso", "Ra", "Again"] {
        rows.push(format!("step fork C::Impl::{name} -> stutter;"));
        rows.push(format!("step join C::Impl::{name} -> stutter;"));
    }
    rows.push("step memory drain -> stutter;".to_owned());
    rows.push("step memory flush -> stutter;".to_owned());
    let rows: Vec<&str> = rows.iter().map(String::as_str).collect();
    let emitted = memory_emission(&run, &rows);
    let node = refinement_node(&emitted.emission);
    let memory: Vec<&Value> = node["body"]["class_catalog"]["concrete"]
        .as_array()
        .expect("a concrete catalog")
        .iter()
        .filter(|entry| {
            entry["class"]
                .as_str()
                .is_some_and(|class| class.starts_with("memory "))
        })
        .collect();
    assert_eq!(
        memory,
        [
            &json!({"class": "memory drain", "memory_model": "ra", "drain": "store-buffer-flush"}),
            &json!({"class": "memory drain", "memory_model": "sc", "drain": "store-buffer-flush"}),
            &json!({"class": "memory drain", "memory_model": "tso", "drain": "store-buffer-flush"}),
            &json!({"class": "memory flush", "memory_model": "tso"}),
        ]
    );
}

/// The memory rows and catalog entries read back through IR's catalog rebuild.
#[trace("FR-375-AC-19", "FR-375-AC-26", "TC-310")]
#[test]
fn memory_rows_read_back() {
    let emitted = memory_emission(MEMORY_RUN, &MEMORY_ROWS);
    let admitted = read(&emitted);
    assert!(admitted.is_ok(), "the memory rows read back: {admitted:?}");
}

/// IR's verdict on `envelope`, a rewrite of `emitted`'s package, with the
/// refinement node at `position` rekeyed from its closed
/// `quire.refinement-node/v1` preimage and the frame identity refreshed.
fn refinement_verdict(emitted: &Emitted, envelope: &Value, position: usize) -> String {
    let mut envelope = envelope.clone();
    let node = &envelope["semantic_graph"]["nodes"][position];
    let old = node["node_id"]["digest"]
        .as_str()
        .expect("a digest")
        .to_owned();
    let preimage = json!({
        "version": "quire.refinement-node/v1",
        "semantic_type": node["semantic_type"],
        "body": node["body"],
    });
    let new = quire_canonical::sha256(&preimage, quire_canonical::Limits::new(u64::MAX))
        .expect("the preimage encodes")
        .to_string();
    let text = serde_json::to_string(&envelope)
        .expect("the envelope serializes")
        .replace(&old, &new);
    envelope = serde_json::from_str(&text).expect("the envelope parses");
    crate::checked_v2::tests::refresh_frame_identity(&mut envelope);
    let mut evidence = read_evidence(&emitted.emission);
    for document in &emitted.documents {
        evidence
            .insert_domain_package_document(document_digest(document), document.to_vec())
            .expect("a fresh evidence key");
    }
    match quire_contract_model::read_checked_package(
        &jcs(&envelope),
        quire_contract_model::CheckedPackageReadLimits::bounded(),
        &evidence,
    ) {
        quire_contract_model::CheckedPackageDispatchResult::AdmittedV2(_) => "admitted".to_owned(),
        quire_contract_model::CheckedPackageDispatchResult::Refused(refusal) => format!(
            "{:?} at {} ({:?})",
            refusal.code,
            refusal.path.as_ref().map_or("-", |path| path.as_str()),
            refusal.cause
        ),
        other => format!("{other:?}"),
    }
}

/// QSpec FR-375-AC-21's refusal half over QSL's admitted population and
/// object rows, every identity recomputed so each join is reached: a
/// wrong-kind object-row type key and a wrong-kind `semantic_type` refuse
/// `invalid_model_binding`/`malformed-declaration` at that member, and an
/// absent local `abstract` key refuses `missing_declaration`/`missing-name`.
#[trace("FR-375-AC-21", "TC-310")]
#[test]
fn a_refinement_row_or_header_key_of_the_wrong_kind_or_absent_refuses_at_its_member() {
    let (emitted, _) = emit_rows(MAPS, &[]);
    let wire = wire(&emitted.emission);
    let position = nodes(&wire)
        .iter()
        .position(|node| node["node_tag"] == "relation" && node["semantic_form"] == "refinement")
        .expect("one refinement relation node");
    assert_eq!(refinement_verdict(&emitted, &wire, position), "admitted");
    let at = |member: &str| format!("/semantic_graph/nodes/{position}/{member}");
    let population =
        wire["semantic_graph"]["nodes"][position]["body"]["populations"][0]["concrete"].clone();

    // The object type, now named by no member, leaves the dependencies.
    let mut object_type = wire.clone();
    let node = &mut object_type["semantic_graph"]["nodes"][position];
    let concrete_type = node["body"]["objects"][0]["concrete_type"].clone();
    node["body"]["objects"][0]["concrete_type"] = population.clone();
    node["dependencies"]
        .as_array_mut()
        .expect("dependencies")
        .retain(|dependency| *dependency != concrete_type);
    let verdict = refinement_verdict(&emitted, &object_type, position);
    assert!(
        verdict.starts_with("InvalidModelBinding at ")
            && verdict.contains(&at("body/objects/0/concrete_type"))
            && verdict.ends_with("(Some(MalformedDeclaration))"),
        "{verdict}"
    );

    let mut semantic_type = wire.clone();
    semantic_type["semantic_graph"]["nodes"][position]["semantic_type"] = population;
    let verdict = refinement_verdict(&emitted, &semantic_type, position);
    assert!(
        verdict.starts_with("InvalidModelBinding at ")
            && verdict.contains(&at("semantic_type"))
            && verdict.ends_with("(Some(MalformedDeclaration))"),
        "{verdict}"
    );

    let mut absent = wire.clone();
    let root = absent["semantic_graph"]["nodes"][position]["body"]["abstract"]["digest"]
        .as_str()
        .expect("a local abstract key")
        .to_owned();
    let missing = "0".repeat(64);
    let node = &mut absent["semantic_graph"]["nodes"][position];
    node["body"]["abstract"]["digest"] = json!(missing);
    for dependency in node["dependencies"].as_array_mut().expect("dependencies") {
        if dependency["digest"] == root {
            dependency["digest"] = json!(missing);
        }
    }
    node["dependencies"]
        .as_array_mut()
        .expect("dependencies")
        .sort_by(|left, right| left["digest"].as_str().cmp(&right["digest"].as_str()));
    let verdict = refinement_verdict(&emitted, &absent, position);
    assert!(
        verdict.starts_with("MissingDeclaration at ")
            && verdict.contains(&at("body/abstract"))
            && verdict.ends_with("(Some(MissingName))"),
        "{verdict}"
    );

    // An external `abstract` key naming a package that is no direct
    // dependency of this one.
    let mut external = wire.clone();
    let node = &mut external["semantic_graph"]["nodes"][position];
    let local = node["body"]["abstract"].clone();
    node["body"]["abstract"] = json!({
        "package": {"domain": "quire.package", "algorithm": "sha256", "digest": "1".repeat(64)},
        "node": local,
    });
    node["dependencies"]
        .as_array_mut()
        .expect("dependencies")
        .retain(|dependency| *dependency != local);
    let verdict = refinement_verdict(&emitted, &external, position);
    assert!(
        verdict.starts_with("MissingDeclaration at ")
            && verdict.contains(&at("body/abstract"))
            && verdict.ends_with("(Some(MissingSelection))"),
        "{verdict}"
    );
}

/// QSpec FR-375-AC-21's ordering half: with every identity recomputed, an
/// object row whose two fields are written out of name order refuses
/// `invalid_semantic_graph` at the first offending entry.
#[trace("FR-375-AC-21", "TC-310")]
#[test]
fn an_out_of_order_row_refuses_at_its_first_offending_entry() {
    let (unit, documents) = rows_refinement_with(
        "population A::config_history from C::config_history;\n\
         object A::ConfigVersion from C::ConfigVersion { versionNumber = self.versionNumber; epoch = self.epoch; }",
        &[],
        RowsModel {
            epoch: true,
            ..RowsModel::default()
        },
    );
    let package = modelled_with_documents(&unit, &documents, Vec::new());
    let emission = emit_checked(&package).expect("the two-field refinement emits");
    let emitted = Emitted {
        package,
        emission,
        documents,
    };
    let wire = wire(&emitted.emission);
    let position = nodes(&wire)
        .iter()
        .position(|node| node["node_tag"] == "relation" && node["semantic_form"] == "refinement")
        .expect("one refinement relation node");
    assert_eq!(refinement_verdict(&emitted, &wire, position), "admitted");
    let fields = &wire["semantic_graph"]["nodes"][position]["body"]["objects"][0]["fields"];
    let names: Vec<&str> = fields
        .as_array()
        .expect("fields")
        .iter()
        .map(|field| field["name"].as_str().expect("a name"))
        .collect();
    assert_eq!(names, ["epoch", "versionNumber"]);

    let mut swapped = wire.clone();
    swapped["semantic_graph"]["nodes"][position]["body"]["objects"][0]["fields"]
        .as_array_mut()
        .expect("fields")
        .swap(0, 1);
    let verdict = refinement_verdict(&emitted, &swapped, position);
    assert!(
        verdict.starts_with("InvalidSemanticGraph at ")
            && verdict.contains(&format!(
                "/semantic_graph/nodes/{position}/body/objects/0/fields/1"
            )),
        "{verdict}"
    );
}
