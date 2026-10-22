// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-147 (TC-552), FR-135-AC-5 and FR-135-AC-9: the S3 check of a
//! refinement whose abstract side is a protocol subject, and of the internal
//! list (`refinement_internals`) that names its internal abstract step
//! classes. The abstract model is ADR-020 §8's counter and the protocols are
//! clauses of the unit over it.
//!
//! Not covered here, for want of the checker they need: the engine's
//! decision of the steps and the liveness settlement (FR-148, FR-147-AC-3),
//! the requirement record and the node identity (S4, FR-147-AC-1 and AC-9),
//! and the supplied-library value (FR-147-AC-10).

use ix_trace_rs::trace;
use qsl_foundation::Code;
use qsl_semantics::check::{
    AbstractSide, CheckCause, CheckRefusal, CheckedGraph, CheckedRefinement, RefinementCause,
    RefinementSupport, StepTarget,
};
use quire_semantic_value::checking::CheckingLimits;
use serde_json::{json, Value};

use super::refinement_declarations::{assemble, counters, mutated, CAS, SPEC};

/// The protocol `Twice` over the abstract counter model `alias`: attempts
/// `a1` and `a2` of `inc`, a `parallel` `par` of two empty branches, a
/// channel `ch` whose delivery is `unknown` (so `duplicate` and `lose` are
/// both classes), a static role `w` and `finish`.
fn twice(alias: &str) -> String {
    format!(
        "protocol Twice using v over (c: {alias}::Counter) on origin {{\n\
         role w on {alias}::Counter;\n\
         channel ch from w to w carries Boolean ordering unordered delivery unknown \
         capacity 2 overflow block;\n\
         run sequence Main {{\n\
         attempt a1 by w on {alias}::Counter::inc contracts [] as (r1: Boolean) {{ true }};\n\
         attempt a2 by w on {alias}::Counter::inc contracts [] as (r2: Boolean) {{ true }};\n\
         parallel par {{ branch pl sequence PL {{ }} branch pr sequence PR {{ }} }} \
         join all [pl, pr];\n\
         }}\n\
         finish Done as (d: Boolean) {{ true }};\n\
         }}\n"
    )
}

/// ADR-020 §8's compare-and-set counter refining the protocol `protocol`
/// over the abstract model `alias`: `commitA` and `commitB` map to the
/// abstract increment, every other step to a stutter.
fn refinement(alias: &str, protocol: &str) -> String {
    format!(
        "refinement CasTwice using inf abstract {alias}::{protocol} concrete Impl {{\n\
         population {alias}::counters from Impl::counters;\n\
         object {alias}::Counter from Impl::Counter {{ value = self.value; }}\n\
         step Impl::Counter::commitA -> {alias}::Counter::inc(self);\n\
         step Impl::Counter::commitB -> {alias}::Counter::inc(self);\n\
         step Impl::Counter::beginA -> stutter;\n\
         step Impl::Counter::beginB -> stutter;\n\
         step Impl::Counter::retryA -> stutter;\n\
         step Impl::Counter::retryB -> stutter;\n\
         step Impl::Counter::peek -> stutter;\n\
         }}\n"
    )
}

fn cas_twice() -> String {
    format!("{}{}", twice("Spec"), refinement("Spec", "Twice"))
}

/// ADR-020 §8's `CasRefinesCounter`, whose abstract side is a model.
fn cas_refines_counter() -> String {
    refinement("Spec", "Twice")
        .replace("CasTwice", "CasRefinesCounter")
        .replace("abstract Spec::Twice", "abstract Spec")
}

/// One refusal of a check, and the text it is located at when it has a
/// source region (a defect of the internal list has none).
struct Found {
    refusal: CheckRefusal,
    at: Option<String>,
}

impl Found {
    fn code_and_cause(&self) -> (Code, Option<&'static str>) {
        (self.refusal.cause.code(), self.refusal.cause.cause())
    }

    fn refinement(&self) -> &RefinementCause {
        let CheckCause::Refinement(refusal) = &self.refusal.cause else {
            panic!("a refinement refusal: {:?}", self.refusal);
        };
        &refusal.cause
    }

    /// The pointer of an internal-list defect, and the earlier value it
    /// repeats.
    fn pointer(&self) -> (&str, Option<&str>) {
        let RefinementCause::InternalList(defect) = self.refinement() else {
            panic!("an internal-list defect: {:?}", self.refinement());
        };
        (&defect.pointer, defect.first.as_deref())
    }

    /// The feature an unsupported refusal names.
    fn feature(&self) -> &'static str {
        let RefinementCause::Unsupported { feature, .. } = self.refinement() else {
            panic!("a not-yet-implemented refusal: {:?}", self.refinement());
        };
        feature.name()
    }
}

const UNBUILT: RefinementSupport = RefinementSupport {
    protocol_abstract_side: false,
    refinement_internal_list: false,
};

fn check_over(
    alias: &str,
    body: &str,
    internals: Option<Value>,
    support: RefinementSupport,
) -> Result<CheckedGraph, Vec<Found>> {
    let (spec, cas) = counters();
    let models: [(&str, &str, &[u8]); 2] = [(alias, SPEC, &spec), ("Impl", CAS, &cas)];
    let (text, declarations) = assemble("", &models, body);
    let mut declarations =
        declarations.unwrap_or_else(|refusal| panic!("{body}: the unit assembles: {refusal:?}"));
    if let Some(internals) = internals {
        declarations = declarations.with_refinement_internals(internals);
    }
    let declarations = declarations
        .with_refinement_support(support)
        .with_unwritten_refinements();
    let regions = declarations.regions();
    declarations
        .check(CheckingLimits::default())
        .map_err(|refusals| {
            refusals
                .into_iter()
                .map(|refusal| Found {
                    at: regions.refusal_region(&refusal).map(|region| {
                        let (start, end) = (
                            usize::try_from(region.start()).expect("a start"),
                            usize::try_from(region.end()).expect("an end"),
                        );
                        text[start..end].to_owned()
                    }),
                    refusal,
                })
                .collect()
        })
}

fn check(
    body: &str,
    internals: Option<Value>,
    support: RefinementSupport,
) -> Result<CheckedGraph, Vec<Found>> {
    check_over("Spec", body, internals, support)
}

fn checked(body: &str, internals: Option<Value>) -> CheckedGraph {
    check(body, internals, RefinementSupport::BUILD).unwrap_or_else(|found| {
        panic!(
            "{body}: {:?}",
            found.iter().map(|f| &f.refusal).collect::<Vec<_>>()
        )
    })
}

fn refused(body: &str, internals: Option<Value>, support: RefinementSupport) -> Vec<Found> {
    match check(body, internals, support) {
        Ok(_) => panic!("{body}: expected a refusal"),
        Err(found) => found,
    }
}

fn only(found: &[Found]) -> &Found {
    let [only] = found else {
        panic!(
            "one refusal: {:?}",
            found.iter().map(|f| &f.refusal).collect::<Vec<_>>()
        );
    };
    only
}

fn the_refinement(graph: &CheckedGraph) -> &CheckedRefinement {
    let [refinement] = graph.refinements() else {
        panic!("one refinement: {:?}", graph.refinements());
    };
    refinement
}

fn spellings(refinement: &CheckedRefinement) -> Vec<&str> {
    refinement
        .abstract_internal()
        .iter()
        .map(|class| class.spelling())
        .collect()
}

fn internal(entries: &[&str]) -> Option<Value> {
    Some(json!([{"refinement": "CasTwice", "internal": entries}]))
}

/// A refusal's catalog code and cause.
type Catalog = (Code, Option<&'static str>);

/// The value, the refusal it gets, the pointer it is located at and the
/// earlier pointer a repetition repeats.
type Vector = (Value, Catalog, &'static str, Option<&'static str>);

const MISSING: Catalog = (Code::MissingDeclaration, Some("missing-name"));
const MALFORMED: Catalog = (Code::InvalidModelBinding, Some("malformed-declaration"));
const CONFLICTING: Catalog = (Code::InvalidModelBinding, Some("conflicting-binding"));
const UNSUPPORTED: Catalog = (Code::UnsupportedConstruct, Some("not-yet-implemented"));

/// Part of FR-147-AC-1 (TC-552 step 1; tagged TC-only while the requirement
/// record is unbuilt): `CasTwice` checks with the protocol side
/// naming `Twice` and the alias-free `abstract_internal = [Twice::Done]`; a
/// row `commitA -> node Spec::Twice::a1(self)` checks, naming node `a1`. The
/// requirement record is S4's and not tested.
#[trace("TC-552")]
#[test]
fn a_refinement_over_a_protocol_checks_with_its_internal_list() {
    let graph = checked(&cas_twice(), internal(&["Spec::Twice::Done"]));
    let refinement = the_refinement(&graph);
    assert_eq!(
        refinement.abstract_side(),
        &AbstractSide::Protocol {
            clause: "Twice".to_owned()
        }
    );
    assert_eq!(refinement.abstract_model(), "Spec");
    assert_eq!(spellings(refinement), ["Twice::Done"]);

    let row = mutated(
        &cas_twice(),
        "step Impl::Counter::commitA -> Spec::Counter::inc(self);",
        "step Impl::Counter::commitA -> node Spec::Twice::a1(self);",
    );
    let graph = checked(&row, None);
    let nodes: Vec<_> = the_refinement(&graph)
        .steps()
        .iter()
        .filter_map(|step| match &step.target {
            StepTarget::Node {
                protocol,
                node,
                arguments,
            } => Some((protocol.as_str(), node.as_str(), arguments.len())),
            _ => None,
        })
        .collect();
    assert_eq!(nodes, [("Twice", "a1", 1)]);
}

/// FR-147-AC-2 (TC-552 step 2): the refusal vectors of the protocol side.
#[trace("TC-552", "FR-147-AC-2")]
#[test]
fn each_refusal_of_the_protocol_side_is_located() {
    let base = cas_twice();

    // A protocol the abstract package declares none of.
    let found = refused(
        &mutated(&base, "abstract Spec::Twice", "abstract Spec::Thrice"),
        None,
        RefinementSupport::BUILD,
    );
    let thrice = only(&found);
    assert_eq!(thrice.code_and_cause(), MISSING);
    assert_eq!(thrice.at.as_deref(), Some("Thrice"));

    // A clause whose roles act on the other model.
    let found = refused(
        &mutated(&base, "abstract Spec::Twice", "abstract Impl::Twice"),
        None,
        RefinementSupport::BUILD,
    );
    let wrong_model = found
        .iter()
        .find(|f| f.at.as_deref() == Some("Twice") && f.code_and_cause() == MALFORMED);
    assert!(wrong_model.is_some(), "{:?}", found.len());

    // An entry no node resolves.
    let found = refused(
        &base,
        internal(&["Spec::Twice::zz"]),
        RefinementSupport::BUILD,
    );
    let zz = only(&found);
    assert_eq!(zz.code_and_cause(), MISSING);
    assert_eq!(zz.pointer(), ("/0/internal/0", None));

    // A node the list names and a row maps: neither wins.
    let row = mutated(
        &base,
        "step Impl::Counter::commitA -> Spec::Counter::inc(self);",
        "step Impl::Counter::commitA -> node Spec::Twice::a1(self);",
    );
    let found = refused(
        &row,
        internal(&["Spec::Twice::a1"]),
        RefinementSupport::BUILD,
    );
    let conflict = only(&found);
    assert_eq!(conflict.code_and_cause(), CONFLICTING);
    assert_eq!(
        conflict.at.as_deref(),
        Some("step Impl::Counter::commitA -> node Spec::Twice::a1(self);")
    );
    assert_eq!(
        conflict.refinement(),
        &RefinementCause::ListEntryConflict {
            class: "Twice::a1".to_owned(),
            entry: "/0/internal/0".to_owned(),
        }
    );

    // A node row under a model subject, whatever the support set.
    let model_row = mutated(
        &cas_refines_counter(),
        "step Impl::Counter::commitA -> Spec::Counter::inc(self);",
        "step Impl::Counter::commitA -> node Spec::Twice::a1(self);",
    );
    for support in [RefinementSupport::BUILD, UNBUILT] {
        let found = refused(&format!("{}{model_row}", twice("Spec")), None, support);
        let row = only(&found);
        assert_eq!(row.code_and_cause(), MALFORMED);
        assert_eq!(row.at.as_deref(), Some("Spec::Twice::a1"), "at the node");
    }

    // Only an attempt, event, send, receive or finish node is a right side.
    let par = mutated(
        &base,
        "step Impl::Counter::commitA -> Spec::Counter::inc(self);",
        "step Impl::Counter::commitA -> node Spec::Twice::par(self);",
    );
    let found = refused(&par, None, RefinementSupport::BUILD);
    let not_a_node = only(&found);
    assert_eq!(not_a_node.code_and_cause(), MISSING);
    assert_eq!(not_a_node.at.as_deref(), Some("Spec::Twice::par"));
}

/// FR-147-AC-4 (TC-552 step 4): with `Done` out of the list the refinement
/// checks with `Done` visible and `abstract_internal` empty; with the list
/// naming it, internal; with the list naming it and a row mapping to it,
/// refused.
#[trace("TC-552", "FR-147-AC-4")]
#[test]
fn a_node_the_list_does_not_name_is_visible() {
    let base = cas_twice();
    let graph = checked(&base, internal(&[]));
    assert!(the_refinement(&graph).abstract_internal().is_empty());
    let graph = checked(&base, None);
    assert!(the_refinement(&graph).abstract_internal().is_empty());

    let graph = checked(&base, internal(&["Spec::Twice::Done"]));
    assert_eq!(spellings(the_refinement(&graph)), ["Twice::Done"]);

    let row = mutated(
        &base,
        "step Impl::Counter::retryA -> stutter;",
        "step Impl::Counter::retryA -> node Spec::Twice::Done(self);",
    );
    let found = refused(
        &row,
        internal(&["Spec::Twice::Done"]),
        RefinementSupport::BUILD,
    );
    assert_eq!(only(&found).code_and_cause(), CONFLICTING);
    // The same row without the list entry checks: the class is visible.
    checked(&row, None);
}

/// FR-147-AC-5 (TC-552 step 5): the order of the objects and entries is
/// free, an object for another refinement or an empty array is the empty
/// list, and the resolved order is that of the alias-free spellings, which
/// the lower-case alias `spec` would reverse if the alias were part of it.
#[trace("TC-552", "FR-147-AC-5")]
#[test]
fn the_list_resolves_in_the_byte_order_of_its_alias_free_spellings() {
    let resolved = |value: Value| -> Vec<String> {
        let graph = checked(&cas_twice(), Some(value));
        spellings(the_refinement(&graph))
            .into_iter()
            .map(str::to_owned)
            .collect()
    };
    let both = json!(["Spec::Twice::Done", "lose Spec::Twice::ch"]);
    let swapped = json!(["lose Spec::Twice::ch", "Spec::Twice::Done"]);
    let expected = ["Twice::Done", "lose Twice::ch"];
    assert_eq!(
        resolved(json!([{"refinement": "CasTwice", "internal": both}])),
        expected
    );
    assert_eq!(
        resolved(json!([{"refinement": "CasTwice", "internal": swapped}])),
        expected
    );
    // The same two entries in two objects of two refinements, swapped.
    let other = cas_refines_counter();
    let two_refinements = format!("{}{}{other}", twice("Spec"), refinement("Spec", "Twice"));
    for value in [
        json!([
            {"refinement": "CasRefinesCounter", "internal": []},
            {"refinement": "CasTwice", "internal": both},
        ]),
        json!([
            {"refinement": "CasTwice", "internal": both},
            {"refinement": "CasRefinesCounter", "internal": []},
        ]),
    ] {
        let graph = checked(&two_refinements, Some(value));
        let found = graph
            .refinements()
            .iter()
            .find(|r| r.name() == "CasTwice")
            .expect("CasTwice");
        assert_eq!(spellings(found), expected);
    }
    // Absent, `[]` and an object for another refinement are the empty list.
    for value in [
        None,
        Some(json!([])),
        Some(json!([{"refinement": "CasRefinesCounter", "internal": []}])),
    ] {
        let graph = checked(&two_refinements, value);
        for refinement in graph.refinements() {
            assert!(refinement.abstract_internal().is_empty());
        }
    }

    // The alias `spec`: the alias-free order puts `Twice::Done` first, and
    // the alias-including spellings would put `lose spec::Twice::ch` first.
    let unit = format!("{}{}", twice("spec"), refinement("spec", "Twice"));
    for entries in [
        json!(["spec::Twice::Done", "lose spec::Twice::ch"]),
        json!(["lose spec::Twice::ch", "spec::Twice::Done"]),
    ] {
        let value = json!([{"refinement": "CasTwice", "internal": entries}]);
        let graph = check_over("spec", &unit, Some(value), RefinementSupport::BUILD)
            .unwrap_or_else(|_| panic!("the lower-case alias checks"));
        assert_eq!(spellings(the_refinement(&graph)), expected);
    }
}

/// FR-147-AC-6 (TC-552 step 6): each defect of the value is refused with the
/// pointer of the offending value, and no list.
#[trace("TC-552", "FR-147-AC-6")]
#[test]
fn each_defect_of_the_value_is_refused_at_its_pointer() {
    let value = |entries: Value| json!([{"refinement": "CasTwice", "internal": entries}]);
    let vectors: Vec<Vector> = vec![
        (json!([{"refinement": "CasTwice"}]), MALFORMED, "/0", None),
        (
            json!([{"refinement": "CasTwice", "internal": "Spec::Twice::Done"}]),
            MALFORMED,
            "/0/internal",
            None,
        ),
        (value(json!([7])), MALFORMED, "/0/internal/0", None),
        (
            json!([{"refinement": 7, "internal": []}]),
            MALFORMED,
            "/0/refinement",
            None,
        ),
        (
            json!([{"refinement": "CasTwice", "internal": [], "visible": []}]),
            MALFORMED,
            "/0",
            None,
        ),
        (json!({"refinement": "CasTwice"}), MALFORMED, "", None),
        (json!([7]), MALFORMED, "/0", None),
        (value(json!([""])), MALFORMED, "/0/internal/0", None),
        (value(json!(["frob x"])), MALFORMED, "/0/internal/0", None),
        (
            value(json!(["Spec::Twice"])),
            MALFORMED,
            "/0/internal/0",
            None,
        ),
        (
            value(json!([" Spec::Twice::Done"])),
            MALFORMED,
            "/0/internal/0",
            None,
        ),
        (
            json!([{"refinement": "Nope", "internal": []}]),
            MISSING,
            "/0/refinement",
            None,
        ),
        (
            value(json!(["Other::Twice::Done"])),
            MISSING,
            "/0/internal/0",
            None,
        ),
        (
            value(json!(["Spec::Other::Done"])),
            MISSING,
            "/0/internal/0",
            None,
        ),
        (
            value(json!(["Spec::Twice::zz"])),
            MISSING,
            "/0/internal/0",
            None,
        ),
        (
            value(json!(["spawn Spec::Twice::r"])),
            MISSING,
            "/0/internal/0",
            None,
        ),
        (
            value(json!(["lose Spec::Twice::nochan"])),
            MISSING,
            "/0/internal/0",
            None,
        ),
        (
            value(json!(["Spec::Twice::par"])),
            MISSING,
            "/0/internal/0",
            None,
        ),
        (
            value(json!(["fork Spec::Twice::a1"])),
            MISSING,
            "/0/internal/0",
            None,
        ),
        (
            json!([
                {"refinement": "CasTwice", "internal": []},
                {"refinement": "CasTwice", "internal": []},
            ]),
            CONFLICTING,
            "/1",
            Some("/0"),
        ),
        (
            value(json!(["Spec::Twice::Done", "Spec::Twice::Done"])),
            CONFLICTING,
            "/0/internal/1",
            Some("/0/internal/0"),
        ),
    ];
    for (given, expected, pointer, first) in vectors {
        let found = refused(&cas_twice(), Some(given.clone()), RefinementSupport::BUILD);
        let defect = only(&found);
        assert_eq!(defect.code_and_cause(), expected, "{given}");
        assert_eq!(defect.pointer(), (pointer, first), "{given}");
        assert_eq!(
            defect.at, None,
            "a defect of the value has no source region"
        );
    }
}

/// FR-147-AC-7 (TC-552 step 7): a build whose support set omits the features
/// refuses a non-empty resolved list once, at the model alias after
/// `abstract`, and a protocol side once more at the protocol; an empty list
/// is not refused.
#[trace("TC-552", "FR-147-AC-7")]
#[test]
fn a_build_without_the_features_refuses_a_non_empty_list_once() {
    let model_unit = format!("{}{}", twice("Spec"), cas_refines_counter());
    let list_for_model =
        Some(json!([{"refinement": "CasRefinesCounter", "internal": ["Spec::Twice::Done"]}]));
    let support = RefinementSupport {
        protocol_abstract_side: true,
        refinement_internal_list: false,
    };
    let found = refused(&model_unit, list_for_model, support);
    let list = only(&found);
    assert_eq!(list.code_and_cause(), UNSUPPORTED);
    assert_eq!(list.feature(), "refinement-internal-list");
    assert_eq!(list.at.as_deref(), Some("Spec"));

    // The protocol subject in a build omitting both: two refusals, each once.
    let found = refused(&cas_twice(), internal(&["Spec::Twice::Done"]), UNBUILT);
    assert_eq!(
        found.len(),
        2,
        "{:?}",
        found.iter().map(|f| &f.at).collect::<Vec<_>>()
    );
    let mut named: Vec<_> = found
        .iter()
        .map(|f| (f.feature(), f.at.as_deref().expect("a region")))
        .collect();
    named.sort_unstable();
    assert_eq!(
        named,
        [
            ("protocol-abstract-side", "Twice"),
            ("refinement-internal-list", "Spec")
        ]
    );
    for refusal in &found {
        assert_eq!(refusal.code_and_cause(), UNSUPPORTED);
    }

    // An empty list is not refused for the list.
    let found = refused(&cas_twice(), internal(&[]), UNBUILT);
    assert_eq!(only(&found).feature(), "protocol-abstract-side");
}

/// FR-135-AC-5: a protocol abstract side in a build that omits
/// `protocol-abstract-side` refuses `not-yet-implemented` at the protocol
/// name, with no other diagnostic about the side, and the model subject is
/// unaffected.
#[trace("TC-540", "FR-135-AC-5")]
#[test]
fn a_build_without_the_protocol_side_refuses_it_at_the_protocol_name() {
    let support = RefinementSupport {
        protocol_abstract_side: false,
        refinement_internal_list: true,
    };
    let found = refused(&cas_twice(), None, support);
    let side = only(&found);
    assert_eq!(side.code_and_cause(), UNSUPPORTED);
    assert_eq!(side.feature(), "protocol-abstract-side");
    assert_eq!(side.at.as_deref(), Some("Twice"));
    let RefinementCause::Unsupported {
        profile, subject, ..
    } = side.refinement()
    else {
        panic!("unsupported");
    };
    assert_eq!(profile, "quire.temporal.infinite-trace/v1");
    assert_eq!(*subject, None);
    // A name the unit does not declare is not reported either.
    let found = refused(
        &mutated(
            &cas_twice(),
            "abstract Spec::Twice",
            "abstract Spec::Thrice",
        ),
        None,
        support,
    );
    assert_eq!(only(&found).feature(), "protocol-abstract-side");

    // The rest of the declaration is still checked and reported beside it.
    let missing_row = mutated(&cas_twice(), "step Impl::Counter::peek -> stutter;", "");
    let found = refused(&missing_row, None, support);
    assert_eq!(found.len(), 2);
    assert_eq!(found[0].feature(), "protocol-abstract-side");
    assert!(matches!(
        found[1].refinement(),
        RefinementCause::MissingRow { .. }
    ));

    let unit = format!("{}{}", twice("Spec"), cas_refines_counter());
    check(&unit, None, UNBUILT).unwrap_or_else(|_| panic!("a model subject checks"));
}

/// FR-147-AC-11 (TC-552 step 11): in a build omitting both features the
/// resolution defects are reported whatever the support set says and the
/// not-yet-built refusal is reported only for a resolved non-empty list.
#[trace("TC-552", "FR-147-AC-11")]
#[test]
fn resolution_defects_are_reported_before_the_unbuilt_feature() {
    // An unresolved entry on the protocol subject: `missing-name`, no
    // `refinement-internal-list` refusal, and the protocol side's own.
    let found = refused(&cas_twice(), internal(&["Spec::Twice::zz"]), UNBUILT);
    let mut pairs: Vec<_> = found
        .iter()
        .map(|f| match f.refinement() {
            RefinementCause::Unsupported { feature, .. } => (feature.name(), None),
            RefinementCause::InternalList(defect) => ("list", Some(defect.pointer.as_str())),
            other => panic!("{other:?}"),
        })
        .collect();
    pairs.sort_unstable();
    assert_eq!(
        pairs,
        [
            ("list", Some("/0/internal/0")),
            ("protocol-abstract-side", None)
        ]
    );

    // A value attributable to no refinement reports its defect alone.
    let model_unit = format!("{}{}", twice("Spec"), cas_refines_counter());
    for value in [json!([{}]), json!([{"refinement": 7, "internal": []}])] {
        let found = refused(&model_unit, Some(value), UNBUILT);
        assert!(
            found
                .iter()
                .all(|f| matches!(f.refinement(), RefinementCause::InternalList(_))),
            "{:?}",
            found.iter().map(|f| f.refinement()).collect::<Vec<_>>()
        );
    }

    // On a model subject only the shape, the spelling, the repetition and
    // the refinement name are checked: an entry that names a protocol is
    // not resolved, and the well-formed list is reported as not built.
    let model =
        |entries: Value| Some(json!([{"refinement": "CasRefinesCounter", "internal": entries}]));
    let found = refused(
        &model_unit,
        model(json!(["Other::Twice::Done", "Spec::Twice::zz"])),
        UNBUILT,
    );
    assert_eq!(only(&found).feature(), "refinement-internal-list");
    let found = refused(&model_unit, model(json!(["frob x"])), UNBUILT);
    assert_eq!(only(&found).code_and_cause(), MALFORMED);
    let found = refused(
        &model_unit,
        model(json!(["Spec::Twice::Done", "Spec::Twice::Done"])),
        UNBUILT,
    );
    assert_eq!(only(&found).code_and_cause(), CONFLICTING);
}

/// FR-147-AC-12 (TC-552 step 12): in a build that has the feature, a
/// non-empty list on a model subject is `malformed-declaration` at the
/// object, with no `not-yet-implemented`; an empty list does not refuse.
#[trace("TC-552", "FR-147-AC-12")]
#[test]
fn a_non_empty_list_on_a_model_subject_is_malformed_once_the_feature_is_built() {
    let unit = format!("{}{}", twice("Spec"), cas_refines_counter());
    let list =
        |entries: Value| Some(json!([{"refinement": "CasRefinesCounter", "internal": entries}]));
    let found = refused(
        &unit,
        list(json!(["Spec::Twice::Done"])),
        RefinementSupport::BUILD,
    );
    let object = only(&found);
    assert_eq!(object.code_and_cause(), MALFORMED);
    assert_eq!(object.pointer(), ("/0", None));
    assert_eq!(object.at, None);
    checked(&unit, list(json!([])));
}

/// FR-147-AC-13 (TC-552 step 13): the refinement checks to the same value
/// whether the protocol is declared before or after it, and a protocol
/// refused by its own checks reports its own refusals alone.
#[trace("TC-552", "FR-147-AC-13")]
#[test]
fn the_protocol_is_checked_before_the_refinement_whatever_the_source_order() {
    let before = checked(&cas_twice(), internal(&["Spec::Twice::Done"]));
    let after = checked(
        &format!("{}{}", refinement("Spec", "Twice"), twice("Spec")),
        internal(&["Spec::Twice::Done"]),
    );
    let (before, after) = (the_refinement(&before), the_refinement(&after));
    assert_eq!(before.abstract_side(), after.abstract_side());
    assert_eq!(before.abstract_internal(), after.abstract_internal());
    assert_eq!(
        format!("{:?}", before.steps()),
        format!("{:?}", after.steps())
    );
    assert_eq!(
        format!("{:?}", before.objects()),
        format!("{:?}", after.objects())
    );
    assert_eq!(
        format!("{:?}", before.populations()),
        format!("{:?}", after.populations())
    );

    // `Twice` refused by its own checks: a role on a type that is no object
    // type of any model. The refusals are those of the clause checked alone.
    let broken = mutated(
        &twice("Spec"),
        "role w on Spec::Counter;",
        "role w on Spec::Nope;",
    );
    let alone = refused(&broken, None, RefinementSupport::BUILD);
    assert!(!alone.is_empty());
    let found = refused(
        &format!("{broken}{}", refinement("Spec", "Twice")),
        internal(&["Spec::Twice::Done"]),
        RefinementSupport::BUILD,
    );
    let debug = |found: &[Found]| -> Vec<String> {
        found.iter().map(|f| format!("{:?}", f.refusal)).collect()
    };
    assert_eq!(debug(&found), debug(&alone));
    assert!(found.iter().all(|f| f.refinement_or_none().is_none()));
}

impl Found {
    fn refinement_or_none(&self) -> Option<&RefinementCause> {
        match &self.refusal.cause {
            CheckCause::Refinement(refusal) => Some(&refusal.cause),
            _ => None,
        }
    }
}

/// The protocol `Full`: one node or channel or role of the classes an
/// internal list can name over the counter model.
fn full() -> String {
    "protocol Full using v over (c: Spec::Counter) on origin {\n\
     role w on Spec::Counter;\n\
     role pool each Spec::Counter from counters max 2 lifetime until (true);\n\
     channel ch from w to w carries Boolean ordering unordered delivery unknown \
     capacity 2 overflow block;\n\
     run sequence Main {\n\
     attempt att by w on Spec::Counter::inc contracts [] as (r1: Boolean) { true };\n\
     event ev by w as (e1: Boolean) { true };\n\
     fence fe ordering seq_cst;\n\
     parallel par memory tso {\n\
     branch l sequence L { send tx via ch as (m1: Boolean) { true }; }\n\
     branch r receive rx via ch of l::L::tx as (m2: Boolean) { true };\n\
     } join all [l, r];\n\
     }\n\
     finish Done as (d: Boolean) { true };\n\
     }\n"
    .to_owned()
}

/// FR-147-AC-8 (TC-552 step 8; the `timeout`, `cattempt`, `cend` and
/// `activate` classes are `the_timeout_cattempt_cend_and_activate_classes_resolve`):
/// each class spells and resolves over `Full` and makes exactly its own class
/// internal, in the byte order of the alias-free spellings.
#[trace("TC-552", "FR-147-AC-8")]
#[test]
fn each_class_spells_and_resolves_over_a_protocol_that_has_it() {
    let unit = format!("{}{}", full(), refinement("Spec", "Full"));
    let entries = [
        "Spec::Full::att",
        "Spec::Full::ev",
        "Spec::Full::tx",
        "Spec::Full::rx",
        "Spec::Full::Done",
        "Spec::Full::fe",
        "fork Spec::Full::par",
        "join Spec::Full::par",
        "duplicate Spec::Full::ch",
        "lose Spec::Full::ch",
        "spawn Spec::Full::pool",
        "retire Spec::Full::pool",
        "memory flush",
    ];
    let graph = checked(&unit, internal(&entries));
    let mut expected: Vec<String> = entries
        .iter()
        .map(|entry| entry.replace("Spec::", ""))
        .collect();
    expected.sort();
    let resolved: Vec<String> = spellings(the_refinement(&graph))
        .into_iter()
        .map(str::to_owned)
        .collect();
    assert_eq!(resolved, expected);

    // The bare path of a `parallel` node is no class, a static role is no
    // replicated one, and the memory kind is the model's.
    for entry in [
        "Spec::Full::par",
        "spawn Spec::Full::w",
        "retire Spec::Full::w",
        "memory nope",
        "activate Spec::Full",
    ] {
        let found = refused(&unit, internal(&[entry]), RefinementSupport::BUILD);
        assert_eq!(only(&found).code_and_cause(), MISSING, "{entry}");
    }
}

/// The protocol `Rich`: an `await` (the `timeout` class), an `on each`
/// trigger (the `activate` class) and a compensation template `Undo` (the
/// `cattempt` and `cend` classes) over the counter model, whose record
/// `Try` is the template's attempt type.
fn rich() -> String {
    "protocol Rich using v over (c: Spec::Counter) on each (trg0: Boolean) {\n\
     role w on Spec::Counter;\n\
     compensate Undo for Main::Applied as (failure: Boolean) by w \
     on Spec::Counter::inc using v clock \"ticks\" {\n\
     activate first (trigger: Boolean) when { true } { }\n\
     within [0,10]; attempts 2 of Spec::Try;\n\
     retry (current_attempt: Spec::Try, prior: Spec::Try) { true };\n\
     commit Main::Committed; recover (recovery: Boolean) { true }; }\n\
     run sequence Main {\n\
     attempt Tried by w on Spec::Counter::inc contracts [] as (tried: Boolean) { true };\n\
     effect Applied of Tried as (applied: Boolean) { true };\n\
     commit Committed by w as (committed: Boolean) { true };\n\
     await Wait after Tried using v clock \"wall\" within [0,1] \
     match event Seen by w as (seen: Boolean) { true }; \
     then sequence Then { } timeout sequence Late { }\n\
     }\n\
     finish Done as (d: Boolean) { true };\n\
     }\n"
    .to_owned()
}

/// FR-147-AC-8 (TC-552 step 8): the `timeout` class of an `await` node, the
/// `cattempt` and `cend` classes of a compensation template and the
/// `activate` class of an `on each` trigger spell and resolve; an `on origin`
/// protocol has no trigger. With `Full`'s test, every class of the criterion.
#[trace("TC-552", "FR-147-AC-8")]
#[test]
fn the_timeout_cattempt_cend_and_activate_classes_resolve() {
    let unit = format!("{}{}", rich(), refinement("Spec", "Rich"));
    let entries = [
        "Spec::Rich::Wait",
        "activate Spec::Rich",
        "Spec::Rich::Undo",
        "cend Spec::Rich::Undo",
    ];
    let graph = checked(&unit, internal(&entries));
    let resolved: Vec<&str> = spellings(the_refinement(&graph));
    assert_eq!(
        resolved,
        [
            "Rich::Undo",
            "Rich::Wait",
            "activate Rich",
            "cend Rich::Undo"
        ]
    );
    // `cend` names a compensation template only.
    for entry in ["cend Spec::Rich::Wait", "cend Spec::Rich::Tried"] {
        let found = refused(&unit, internal(&[entry]), RefinementSupport::BUILD);
        assert_eq!(only(&found).code_and_cause(), MISSING, "{entry}");
    }
    // An `on origin` protocol has no trigger to activate.
    let found = refused(
        &format!("{}{}", twice("Spec"), refinement("Spec", "Twice")),
        internal(&["activate Spec::Twice"]),
        RefinementSupport::BUILD,
    );
    assert_eq!(only(&found).code_and_cause(), MISSING);
}

/// FR-135-AC-9 (TC-540): a non-empty resolved list refuses
/// `unsupported_construct`/`not-yet-implemented` naming
/// `refinement-internal-list`, once for the refinement, at the first token of
/// the name after `abstract`, with no `subject` member and the profile
/// identity; the pair is the one FR-341-AC-5 classifies `unsupported`
/// (checked on the pair in qsl-replay). A list with a resolution defect
/// reports the defect and not this pair.
#[trace("TC-540", "FR-135-AC-9")]
#[test]
fn the_list_refusal_names_the_feature_the_refinement_and_no_subject() {
    let support = RefinementSupport {
        protocol_abstract_side: true,
        refinement_internal_list: false,
    };
    let found = refused(&cas_twice(), internal(&["Spec::Twice::Done"]), support);
    let list = only(&found);
    assert_eq!(list.code_and_cause(), UNSUPPORTED);
    assert_eq!(list.at.as_deref(), Some("Spec"));
    let CheckCause::Refinement(refusal) = &list.refusal.cause else {
        panic!("a refinement refusal");
    };
    assert_eq!(refusal.refinement, "CasTwice");
    let RefinementCause::Unsupported {
        feature,
        profile,
        subject,
    } = &refusal.cause
    else {
        panic!("not yet implemented: {:?}", refusal.cause);
    };
    assert_eq!(feature.name(), "refinement-internal-list");
    assert_eq!(profile, "quire.temporal.infinite-trace/v1");
    assert_eq!(*subject, None);

    let found = refused(&cas_twice(), internal(&["Spec::Twice::zz"]), support);
    assert_eq!(only(&found).code_and_cause(), MISSING);
}

/// FR-147-AC-11 and FR-147 "Precedence": the defects of the value are
/// reported beside the refusals of the rest of the stage, which would
/// otherwise end it first.
#[trace("TC-552", "FR-147-AC-11")]
#[test]
fn the_defects_of_the_value_are_reported_beside_another_refinements_refusal() {
    let unit = format!(
        "{}{}",
        twice("Spec"),
        mutated(
            &cas_refines_counter(),
            "step Impl::Counter::peek -> stutter;",
            ""
        )
    );
    let value = Some(json!([{"refinement": "CasRefinesCounter", "internal": [7]}]));
    let found = refused(&unit, value, RefinementSupport::BUILD);
    let mut kinds: Vec<_> = found
        .iter()
        .map(|f| match f.refinement() {
            RefinementCause::MissingRow { .. } => "missing row",
            RefinementCause::InternalList(_) => "list defect",
            other => panic!("{other:?}"),
        })
        .collect();
    kinds.sort_unstable();
    assert_eq!(kinds, ["list defect", "missing row"]);
}

/// A shape defect of an object that names a refinement is that refinement's
/// alone: its entries are not resolved, so nothing else is reported for it.
#[trace("TC-552", "FR-147-AC-6")]
#[test]
fn a_shape_defect_of_a_named_object_is_reported_once() {
    let object = |name: &str, entries: Value| {
        Some(json!([{"refinement": name, "internal": entries, "visible": []}]))
    };
    let found = refused(
        &cas_twice(),
        object("CasTwice", json!(["Spec::Twice::zz"])),
        RefinementSupport::BUILD,
    );
    let defect = only(&found);
    assert_eq!(defect.code_and_cause(), MALFORMED);
    assert_eq!(defect.pointer(), ("/0", None));
    let unit = format!("{}{}", twice("Spec"), cas_refines_counter());
    let found = refused(
        &unit,
        object("CasRefinesCounter", json!(["Spec::Twice::Done"])),
        RefinementSupport::BUILD,
    );
    assert_eq!(only(&found).pointer(), ("/0", None));
}

/// A clause refused by its controls check reports its own refusals alone:
/// the refinement does not resolve a list entry against what the clause
/// failed to check.
#[trace("TC-552", "FR-147-AC-13")]
#[test]
fn a_clause_refused_by_its_controls_check_gives_no_name_error() {
    let broken = mutated(&twice("Spec"), "from w to w", "from w to nobody");
    let alone = refused(&broken, None, RefinementSupport::BUILD);
    assert!(!alone.is_empty());
    let found = refused(
        &format!("{broken}{}", refinement("Spec", "Twice")),
        internal(&["lose Spec::Twice::ch"]),
        RefinementSupport::BUILD,
    );
    assert_eq!(found.len(), alone.len());
    assert!(found.iter().all(|f| f.refinement_or_none().is_none()));
}

/// FR-147 "Abstract side": the protocol's state takes no row, so an object
/// row's field row naming a protocol binder is refused
/// `invalid_model_binding`/`malformed-declaration`.
#[trace("TC-552")]
#[test]
fn a_field_row_naming_a_protocol_binder_is_refused() {
    let row = mutated(
        &cas_twice(),
        "{ value = self.value; }",
        "{ value = self.value; r1 = self.value; }",
    );
    let found = refused(&row, None, RefinementSupport::BUILD);
    let field = only(&found);
    assert_eq!(
        field.code_and_cause(),
        MALFORMED,
        "{:?}",
        field.refinement()
    );
}

/// `CasTwice` with `commitA`'s row replaced by `target`.
fn node_row(target: &str) -> String {
    mutated(
        &cas_twice(),
        "step Impl::Counter::commitA -> Spec::Counter::inc(self);",
        &format!("step Impl::Counter::commitA -> {target};"),
    )
}

/// FR-147-AC-14 (TC-552 step 14), admitted rows: `a1` applies `inc`, which
/// has no parameter and no result, so its binder record is the receiver
/// alone. A list left out and `_` leave it open.
#[trace("TC-552", "FR-147-AC-14")]
#[test]
fn a_node_row_whose_arguments_fit_the_binder_record_checks() {
    for (target, open) in [
        ("node Spec::Twice::a1(self)", 0),
        ("node Spec::Twice::a1(_)", 1),
        ("node Spec::Twice::a1", 1),
    ] {
        let graph = checked(&node_row(target), None);
        let arguments: Vec<_> = the_refinement(&graph)
            .steps()
            .iter()
            .filter_map(|step| match &step.target {
                StepTarget::Node {
                    node, arguments, ..
                } if node == "a1" => Some(arguments),
                _ => None,
            })
            .collect();
        let [arguments] = arguments.as_slice() else {
            panic!("{target}: one node row");
        };
        assert_eq!(arguments.len(), 1, "{target}: one field");
        assert_eq!(
            arguments
                .iter()
                .filter(|argument| argument.is_none())
                .count(),
            open,
            "{target}"
        );
    }
}

/// FR-147-AC-14 (TC-552 step 14), the count: a list whose length differs
/// from the field count refuses `ill_typed`/`operator-ineligible` at the
/// node, retaining both counts.
#[trace("TC-552", "FR-147-AC-14")]
#[test]
fn a_node_row_with_the_wrong_argument_count_is_refused_at_the_node() {
    for (target, actual) in [
        ("node Spec::Twice::a1(self, 1, 2)", 3),
        ("node Spec::Twice::a1(self, _)", 2),
    ] {
        let found = refused(&node_row(target), None, RefinementSupport::BUILD);
        let count = only(&found);
        assert_eq!(
            count.code_and_cause(),
            (Code::IllTyped, Some("operator-ineligible")),
            "{target}"
        );
        assert_eq!(count.at.as_deref(), Some("Spec::Twice::a1"), "{target}");
        assert_eq!(
            count.refinement(),
            &RefinementCause::NodeArgumentCount {
                node: "Spec::Twice::a1".to_owned(),
                expected: 1,
                actual,
            },
            "{target}"
        );
    }
}

/// FR-147-AC-14 (TC-552 step 14), the type: an argument that is not its
/// field's type refuses `ill_typed`/`type-mismatch` at the argument,
/// retaining the field's type and the argument's.
#[trace("TC-552", "FR-147-AC-14")]
#[test]
fn a_node_row_argument_of_the_wrong_type_is_refused_at_the_argument() {
    let found = refused(
        &node_row("node Spec::Twice::a1(true)"),
        None,
        RefinementSupport::BUILD,
    );
    let mismatch = only(&found);
    assert_eq!(
        mismatch.code_and_cause(),
        (Code::IllTyped, Some("type-mismatch"))
    );
    assert_eq!(mismatch.at.as_deref(), Some("true"));
    let RefinementCause::NodeArgumentType {
        node,
        position,
        expected,
        actual,
    } = mismatch.refinement()
    else {
        panic!("a node argument type refusal: {:?}", mismatch.refinement());
    };
    assert_eq!((node.as_str(), *position), ("Spec::Twice::a1", 0));
    assert!(
        matches!(**expected, quire_exact::ValueType::Reference(_)),
        "{expected:?}"
    );
    assert_eq!(**actual, quire_exact::ValueType::Boolean);
}
