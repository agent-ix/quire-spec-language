// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-1022, TC-1023 and TC-195 N10/N11 (FR-081-AC-9, FR-081-AC-10): the
//! retained one-edge facts expand to the path-form facts of an independent
//! enumeration over the original declarations, a mis-wired link is refused,
//! and an identity binds its whole ancestry.

use std::collections::{BTreeMap, BTreeSet};

use ix_trace_rs::trace;
use qsl_semantics::model::accounting::ModelNormalizationLimits;
use qsl_semantics::model::domain_package::{
    DomainPackage, DomainPackageRecord, DomainPackageRef, FieldMemberRecord, Multiplicity,
    ObjectTypeRecord, ValueTypeRef,
};
use qsl_semantics::model::key::{
    DeclarationKey, EffectiveId, RULE_INHERIT, RULE_QUALIFY, RULE_REDEFINE, RULE_SUBSET,
};
use qsl_semantics::model::links::{ExpandError, Links};
use qsl_semantics::model::normalize::{
    normalize, EffectiveView, ModelRefusalCause, NormalizeOutcome, ViewEntry,
};

const PREFIX: &str = "ix://test/orders/";

/// One object type of a fixture and its direct supertypes.
#[derive(Clone)]
struct TypeDecl {
    name: &'static str,
    parents: Vec<&'static str>,
}

/// One field of a fixture.
#[derive(Clone)]
struct FieldDecl {
    name: &'static str,
    owner: &'static str,
    redefines: Option<&'static str>,
    subsets: Vec<&'static str>,
}

#[derive(Clone)]
struct Fixture {
    types: Vec<TypeDecl>,
    fields: Vec<FieldDecl>,
}

fn key(name: &str) -> DeclarationKey {
    DeclarationKey::fixture(format!("{PREFIX}{name}"))
}

fn ty(name: &'static str, parents: &[&'static str]) -> TypeDecl {
    TypeDecl {
        name,
        parents: parents.to_vec(),
    }
}

fn fld(
    name: &'static str,
    owner: &'static str,
    redefines: Option<&'static str>,
    subsets: &[&'static str],
) -> FieldDecl {
    FieldDecl {
        name,
        owner,
        redefines,
        subsets: subsets.to_vec(),
    }
}

impl Fixture {
    fn package(&self) -> DomainPackage {
        let mut records: Vec<DomainPackageRecord> = self
            .types
            .iter()
            .map(|decl| {
                DomainPackageRecord::ObjectType(ObjectTypeRecord {
                    key: key(decl.name),
                    interface_features: None,
                    abstract_type: false,
                    supertypes: decl.parents.iter().map(|parent| key(parent)).collect(),
                })
            })
            .collect();
        records.extend(self.fields.iter().map(|decl| {
            DomainPackageRecord::FieldMember(FieldMemberRecord {
                key: key(decl.name),
                owner: key(decl.owner),
                value_type: ValueTypeRef::Package(key(self.types[0].name)),
                multiplicity: Multiplicity {
                    lower: 0,
                    upper: Some(1),
                    ordered: false,
                    unique: true,
                },
                presence: quire_exact::Presence::Required,
                subsets: decl.subsets.iter().map(|target| key(target)).collect(),
                redefines: decl.redefines.map(key),
            })
        }));
        DomainPackage::new(DomainPackageRef::fixture("n01"), records)
    }

    fn view(&self) -> EffectiveView {
        match normalize(&self.package(), ModelNormalizationLimits::default()) {
            NormalizeOutcome::Completed(view) => view,
            other => panic!("the fixture normalizes, got {other:?}"),
        }
    }

    fn parents_of(&self, name: &str) -> Vec<String> {
        let mut parents: Vec<String> = self
            .types
            .iter()
            .find(|decl| decl.name == name)
            .map(|decl| decl.parents.iter().map(|parent| full(parent)).collect())
            .unwrap_or_default();
        parents.sort();
        parents
    }
}

fn full(name: &str) -> String {
    format!("{PREFIX}{name}")
}

fn four_type_chain() -> Fixture {
    Fixture {
        types: vec![
            ty("T0", &[]),
            ty("T1", &["T0"]),
            ty("T2", &["T1"]),
            ty("T3", &["T2"]),
        ],
        fields: vec![fld("T0/f", "T0", None, &[])],
    }
}

fn diamond() -> Fixture {
    Fixture {
        types: vec![
            ty("A", &[]),
            ty("B", &["A"]),
            ty("C", &["A"]),
            ty("D", &["B", "C"]),
        ],
        fields: vec![fld("A/f", "A", None, &[])],
    }
}

/// A diamond whose descendant `D/g` redefines `A/f`, a second redefiner
/// `C/h` on another branch, and a subsetting field `B/s`.
fn hidden_redefinition() -> Fixture {
    Fixture {
        types: vec![
            ty("A", &[]),
            ty("B", &["A"]),
            ty("C", &["A"]),
            ty("D", &["B", "C"]),
        ],
        fields: vec![
            fld("A/f", "A", None, &[]),
            fld("A/k", "A", None, &[]),
            fld("B/s", "B", None, &["A/k"]),
            fld("C/h", "C", Some("A/f"), &[]),
            fld("D/g", "D", Some("A/f"), &[]),
        ],
    }
}

// ---- the independent enumeration over the original declarations ----------

type Path = Vec<String>;

/// Every supertype path from `from`, in ascending-key depth-first order,
/// each as the list of types after `from`.
fn paths_from(fixture: &Fixture, from: &str) -> Vec<Path> {
    let mut out = Vec::new();
    for parent in fixture.parents_of(from) {
        let bare = parent.trim_start_matches(PREFIX).to_owned();
        out.push(vec![parent.clone()]);
        for rest in paths_from(fixture, &bare) {
            let mut path = vec![parent.clone()];
            path.extend(rest);
            out.push(path);
        }
    }
    out
}

/// The least path from `from` to `owner`: empty when they are the same type.
fn least_path(fixture: &Fixture, from: &str, owner: &str) -> Option<Path> {
    if from == owner {
        return Some(Vec::new());
    }
    paths_from(fixture, from)
        .into_iter()
        .find(|path| path.last().map(String::as_str) == Some(&full(owner)))
}

fn reaches(fixture: &Fixture, from: &str, owner: &str) -> bool {
    least_path(fixture, from, owner).is_some()
}

/// One path-form fact: `(phase, rule rank, rule name, inputs)`.
type Expected = (u8, u8, &'static str, Vec<String>);

fn sorted(mut facts: Vec<Expected>) -> Vec<(&'static str, Vec<String>)> {
    facts.sort_by(|a, b| (a.0, &a.3, a.1).cmp(&(b.0, &b.3, b.1)));
    facts
        .into_iter()
        .map(|(_, _, rule, inputs)| (rule, inputs))
        .collect()
}

fn expected_type(fixture: &Fixture, name: &str) -> Vec<(&'static str, Vec<String>)> {
    let mut facts: Vec<Expected> = vec![(2, 2, "qualify", vec![full(name)])];
    facts.extend(
        paths_from(fixture, name)
            .into_iter()
            .map(|path| (3, 2, "inherit", path)),
    );
    sorted(facts)
}

fn expected_member(
    fixture: &Fixture,
    at: &str,
    member: &FieldDecl,
) -> Vec<(&'static str, Vec<String>)> {
    let mut facts: Vec<Expected> = Vec::new();
    if member.owner == at {
        facts.push((2, 2, "qualify", vec![full(member.name)]));
    } else {
        for path in paths_from(fixture, at) {
            if path.last().map(String::as_str) == Some(&full(member.owner)) {
                let mut inputs = path;
                inputs.push(full(member.name));
                facts.push((3, 2, "inherit", inputs));
            }
        }
    }
    let path_to = |owner: &str| -> Path {
        least_path(fixture, at, owner).expect("the owner reaches the type")
    };
    // This member's own `subsets` and `redefines`.
    for target in &member.subsets {
        let mut inputs = path_to(member.owner);
        inputs.extend([full(member.name), full(target)]);
        facts.push((4, 1, "subset", inputs));
    }
    if let Some(target) = member.redefines {
        let mut inputs = path_to(member.owner);
        inputs.extend([full(member.name), full(target)]);
        facts.push((4, 0, "redefine", inputs));
    }
    // Every redefiner of this member whose owner reaches the type.
    for redefiner in &fixture.fields {
        if redefiner.redefines == Some(member.name) && reaches(fixture, at, redefiner.owner) {
            let mut inputs = path_to(redefiner.owner);
            inputs.extend([full(redefiner.name), full(member.name)]);
            facts.push((4, 0, "redefine", inputs));
        }
    }
    sorted(facts)
}

fn rule_name(fact: &qsl_semantics::model::links::PathFact) -> &'static str {
    if fact.rule == RULE_QUALIFY {
        "qualify"
    } else if fact.rule == RULE_INHERIT {
        "inherit"
    } else if fact.rule == RULE_SUBSET {
        "subset"
    } else if fact.rule == RULE_REDEFINE {
        "redefine"
    } else {
        panic!("a fact cites a rule outside the four")
    }
}

// ---- the test oracle's own expansion through the parent identities --------

fn entry_by_id<'v>(view: &'v EffectiveView, id: &EffectiveId) -> &'v ViewEntry {
    view.declarations()
        .iter()
        .find(|entry| entry.effective_id == *id)
        .expect("a linked parent is in the view")
}

/// The inherit paths of `entry` read straight off the links.
fn oracle_inherit(view: &EffectiveView, entry: &ViewEntry) -> Vec<Path> {
    let is_member = entry.preimage.owner_effective_type.is_some();
    let mut out = Vec::new();
    for fact in &entry.preimage.derivation {
        if fact.rule != RULE_INHERIT {
            continue;
        }
        let parent = entry_by_id(view, &fact.parent.expect("an inherit fact links"));
        let head = full_name(&fact.inputs[0]);
        let below = oracle_inherit(view, parent);
        if !is_member {
            out.push(vec![head.clone()]);
            for rest in below {
                let mut path = vec![head.clone()];
                path.extend(rest);
                out.push(path);
            }
        } else if below.is_empty() {
            out.push(vec![head]);
        } else {
            for rest in below {
                let mut path = vec![head.clone()];
                path.extend(rest);
                out.push(path);
            }
        }
    }
    out
}

fn full_name(key: &DeclarationKey) -> String {
    key.node.clone()
}

// ---- tests ----------------------------------------------------------------

fn check_against_enumeration(fixture: &Fixture) {
    let view = fixture.view();
    let links = view.links().expect("a normalized view's links are exact");
    for entry in view.declarations() {
        let original = entry
            .preimage
            .original
            .node
            .trim_start_matches(PREFIX)
            .to_owned();
        let expected = match entry.preimage.owner_effective_type {
            None => expected_type(fixture, &original),
            Some(owner) => {
                let owner_entry = entry_by_id(&view, &owner);
                let at = owner_entry
                    .preimage
                    .original
                    .node
                    .trim_start_matches(PREFIX);
                let member = fixture
                    .fields
                    .iter()
                    .find(|decl| decl.name == original)
                    .expect("a member is a fixture field");
                expected_member(fixture, at, member)
            }
        };
        let expanded = links
            .expand(&entry.effective_id, 1_000_000)
            .expect("the expansion fits");
        let actual: Vec<(&str, Vec<String>)> = expanded
            .iter()
            .map(|fact| {
                (
                    rule_name(fact),
                    fact.inputs.iter().map(|input| input.node.clone()).collect(),
                )
            })
            .collect();
        assert_eq!(actual, expected, "expansion of {original}");
        assert!(
            expanded
                .iter()
                .enumerate()
                .all(|(position, fact)| fact.ordinal == position),
            "zero-based ordinals of {original}"
        );

        // The inherit facts also equal the paths read off the links by this
        // test, without the production expansion.
        let read: Vec<Path> = oracle_inherit(&view, entry);
        let enumerated: Vec<Path> = expected
            .iter()
            .filter(|(rule, _)| *rule == "inherit")
            .map(|(_, inputs)| match entry.preimage.owner_effective_type {
                None => inputs.clone(),
                Some(_) => inputs[..inputs.len() - 1].to_vec(),
            })
            .collect();
        assert_eq!(
            read, enumerated,
            "inherit paths of {original} through the links"
        );
    }
}

#[trace("QSpec-TC-195", "QSpec-FR-081-AC-9")]
#[test]
fn links_expand_to_independent_enumeration() {
    check_against_enumeration(&four_type_chain());
    check_against_enumeration(&diamond());
    check_against_enumeration(&hidden_redefinition());
}

/// TC-1022 step 4: the diamond stores one inherit fact per direct edge.
#[trace("QSpec-TC-195", "QSpec-FR-081-AC-9")]
#[test]
fn the_diamond_holds_two_inherit_facts_not_three() {
    let view = diamond().view();
    let facts = |owner: Option<&str>, name: &str| -> Vec<&'static str> {
        view.declarations()
            .iter()
            .find(|entry| {
                entry.preimage.original == key(name)
                    && match owner {
                        None => entry.preimage.owner_effective_type.is_none(),
                        Some(owner) => entry.preimage.owner_effective_type.is_some_and(|id| {
                            entry_by_id(&view, &id).preimage.original == key(owner)
                        }),
                    }
            })
            .expect("the declaration")
            .preimage
            .derivation
            .iter()
            .map(|fact| {
                if fact.rule == RULE_QUALIFY {
                    "qualify"
                } else {
                    "inherit"
                }
            })
            .collect()
    };
    assert_eq!(facts(None, "D"), ["qualify", "inherit", "inherit"]);
    assert_eq!(facts(Some("D"), "A/f"), ["inherit", "inherit"]);
    assert_eq!(facts(Some("A"), "A/f"), ["qualify"]);
}

/// The reader's own ceiling: an expansion past `max_facts` is refused.
#[trace("QSpec-TC-195", "QSpec-FR-081-AC-9")]
#[test]
fn an_expansion_past_the_readers_ceiling_is_refused() {
    let view = diamond().view();
    let links = view.links().expect("exact links");
    let d = view
        .declarations()
        .iter()
        .find(|entry| entry.preimage.original == key("D"))
        .expect("D");
    assert!(links.expand(&d.effective_id, 5).is_ok());
    assert_eq!(
        links.expand(&d.effective_id, 4),
        Err(ExpandError::Exceeds { max_facts: 4 })
    );
}

fn entries(view: &EffectiveView) -> Vec<ViewEntry> {
    view.declarations().to_vec()
}

fn position(entries: &[ViewEntry], owner: Option<&str>, name: &str) -> usize {
    entries
        .iter()
        .position(|entry| {
            entry.preimage.original == key(name)
                && match owner {
                    None => entry.preimage.owner_effective_type.is_none(),
                    Some(owner) => entry.preimage.owner_effective_type.is_some_and(|id| {
                        entries
                            .iter()
                            .find(|candidate| candidate.effective_id == id)
                            .is_some_and(|candidate| candidate.preimage.original == key(owner))
                    }),
                }
        })
        .unwrap_or_else(|| panic!("declaration {name}"))
}

fn refusal_cause(entries: &[ViewEntry]) -> ModelRefusalCause {
    match Links::new(entries) {
        Ok(_) => panic!("the mutation is accepted"),
        Err(refusal) => refusal.cause,
    }
}

/// TC-1022 step 5 and TC-195 N10: every mutation of a link is refused by the
/// strict reader.
#[trace("QSpec-TC-195", "QSpec-FR-081-AC-9")]
#[test]
fn the_strict_reader_refuses_every_mis_wired_link() {
    let view = diamond().view();
    assert!(Links::new(view.declarations()).is_ok());
    let base = entries(&view);
    let id = |owner: Option<&str>, name: &str| base[position(&base, owner, name)].effective_id;

    // A parent that names a member where a type is required.
    let mut mutated = base.clone();
    let at = position(&mutated, None, "B");
    mutated[at].preimage.derivation[1].parent = Some(id(Some("A"), "A/f"));
    assert!(matches!(
        refusal_cause(&mutated),
        ModelRefusalCause::MisWiredLink { .. }
    ));

    // A parent whose original differs from the fact's input key.
    let mut mutated = base.clone();
    let at = position(&mutated, None, "D");
    mutated[at].preimage.derivation[1].parent = Some(id(None, "C"));
    assert!(matches!(
        refusal_cause(&mutated),
        ModelRefusalCause::MisWiredLink { .. }
    ));

    // A parent whose owner is not the named supertype.
    let mut mutated = base.clone();
    let at = position(&mutated, Some("D"), "A/f");
    mutated[at].preimage.derivation[0].parent = Some(id(Some("C"), "A/f"));
    assert!(matches!(
        refusal_cause(&mutated),
        ModelRefusalCause::MisWiredLink { position: 0, .. }
    ));

    // A repeated parent on one declaration.
    let mut mutated = base.clone();
    let at = position(&mutated, Some("D"), "A/f");
    mutated[at].preimage.derivation[1].parent = mutated[at].preimage.derivation[0].parent;
    assert!(matches!(
        refusal_cause(&mutated),
        ModelRefusalCause::DuplicateParent {
            earlier: 0,
            later: 1,
            ..
        }
    ));

    // A missing parent preimage.
    let mut mutated = base.clone();
    let missing = position(&mutated, Some("B"), "A/f");
    mutated.remove(missing);
    assert!(matches!(
        refusal_cause(&mutated),
        ModelRefusalCause::MisWiredLink { .. }
    ));

    // A link on a qualify fact, and an inherit fact with no link.
    let mut mutated = base.clone();
    let at = position(&mutated, None, "A");
    mutated[at].preimage.derivation[0].parent = Some(id(None, "A"));
    assert!(matches!(
        refusal_cause(&mutated),
        ModelRefusalCause::MisWiredLink { position: 0, .. }
    ));
    let mut mutated = base.clone();
    let at = position(&mutated, None, "B");
    mutated[at].preimage.derivation[1].parent = None;
    assert!(matches!(
        refusal_cause(&mutated),
        ModelRefusalCause::MisWiredLink { .. }
    ));
}

/// N10: facts out of order, and a repeated rule over equal inputs.
#[trace("QSpec-TC-195", "QSpec-FR-081-AC-9")]
#[test]
fn n10_unsorted_derivation_is_refused() {
    let view = diamond().view();
    let mut entry = view
        .declarations()
        .iter()
        .find(|entry| {
            entry.preimage.original == key("B") && entry.preimage.owner_effective_type.is_none()
        })
        .expect("B")
        .clone();
    entry.preimage.derivation.swap(0, 1);
    let (cause, _) = entry
        .preimage
        .validate_derivation()
        .expect_err("a moved fact breaks the ordinals");
    assert!(matches!(
        cause,
        ModelRefusalCause::UnsortedDerivation { .. }
    ));
}

fn identities(view: &EffectiveView) -> BTreeMap<String, (EffectiveId, String)> {
    view.declarations()
        .iter()
        .map(|entry| {
            let owner = entry
                .preimage
                .owner_effective_type
                .map(|id| entry_by_id(view, &id).preimage.original.node.clone())
                .unwrap_or_default();
            (
                format!("{owner}|{}", entry.preimage.original.node),
                (entry.effective_id, format!("{:?}", entry.preimage)),
            )
        })
        .collect()
}

fn changed(base: &Fixture, variant: &Fixture) -> BTreeSet<String> {
    let before = identities(&base.view());
    let after = identities(&variant.view());
    before
        .keys()
        .chain(after.keys())
        .filter(|label| before.get(*label).map(|v| v.0) != after.get(*label).map(|v| v.0))
        .cloned()
        .collect()
}

fn five_type_chain() -> Fixture {
    Fixture {
        types: vec![
            ty("S", &[]),
            ty("T0", &[]),
            ty("T1", &["T0"]),
            ty("T1b", &["T0"]),
            ty("T2", &["T1"]),
            ty("T3", &["T2"]),
            ty("T4", &["T3"]),
        ],
        fields: vec![
            fld("T0/f", "T0", None, &[]),
            fld("T0/h", "T0", None, &[]),
            fld("T4/g", "T4", Some("T0/f"), &[]),
            fld("T4/s", "T4", None, &["T0/h"]),
        ],
    }
}

fn labels(owner_member: &[(&str, &str)]) -> BTreeSet<String> {
    owner_member
        .iter()
        .map(|(owner, node)| format!("{}|{}", full(owner), full(node)))
        .collect()
}

/// TC-1023: an identity binds its whole ancestry through the parent link.
#[trace("QSpec-TC-195", "QSpec-FR-081-AC-10")]
#[test]
fn identity_binds_ancestry_and_is_unique() {
    let base = five_type_chain();

    // The middle edge T2 -> T1 replaced by T2 -> T1b: T2 and everything below
    // it that reaches it change; T0, T1, T1b and S do not.
    let mut rewired = base.clone();
    rewired.types[4] = ty("T2", &["T1b"]);
    let moved = changed(&base, &rewired);
    let mut expected = BTreeSet::new();
    for decl in ["T2", "T3", "T4"] {
        expected.insert(format!("|{}", full(decl)));
    }
    for owner in ["T2", "T3", "T4"] {
        for field in ["T0/f", "T0/h"] {
            expected.insert(format!("{}|{}", full(owner), full(field)));
        }
    }
    for field in ["T4/g", "T4/s"] {
        expected.insert(format!("{}|{}", full("T4"), full(field)));
    }
    assert_eq!(moved, expected, "an edge replaced in the middle");

    // The redefinition target changed from f to h.
    let mut retargeted = base.clone();
    retargeted.fields[2] = fld("T4/g", "T4", Some("T0/h"), &[]);
    assert_eq!(
        changed(&base, &retargeted),
        labels(&[("T4", "T4/g"), ("T4", "T0/f"), ("T4", "T0/h")])
    );

    // One subset target changed from h to f.
    let mut resubset = base.clone();
    resubset.fields[3] = fld("T4/s", "T4", None, &["T0/f"]);
    assert_eq!(changed(&base, &resubset), labels(&[("T4", "T4/s")]));

    // The root's own key changed: every type below it and every member of
    // those types changes; the unrelated type does not.
    let mut renamed = base.clone();
    for decl in &mut renamed.types {
        if decl.name == "T0" {
            decl.name = "U0";
        }
        for parent in &mut decl.parents {
            if *parent == "T0" {
                *parent = "U0";
            }
        }
    }
    for decl in &mut renamed.fields {
        if decl.owner == "T0" {
            decl.owner = "U0";
        }
    }
    let root_moved = changed(&base, &renamed);
    assert!(
        !root_moved.iter().any(|label| label.ends_with(&full("S"))),
        "the unrelated type keeps its identity"
    );
    for decl in ["T1", "T1b", "T2", "T3", "T4"] {
        assert!(
            root_moved.contains(&format!("|{}", full(decl))),
            "{decl} reaches the root"
        );
    }

    // No two distinct declarations of any variant share an identity or a
    // preimage; one identity has exactly one preimage.
    let mut seen: BTreeMap<EffectiveId, String> = BTreeMap::new();
    let mut preimages: BTreeSet<String> = BTreeSet::new();
    for fixture in [&base, &rewired, &retargeted, &resubset, &renamed] {
        for (_, (id, preimage)) in identities(&fixture.view()) {
            match seen.get(&id) {
                Some(known) => assert_eq!(known, &preimage, "one identity, one preimage"),
                None => {
                    assert!(
                        preimages.insert(preimage.clone()),
                        "one preimage, one identity"
                    );
                    seen.insert(id, preimage);
                }
            }
        }
    }
}

/// Two declarations of equal declared shape and distinct original keys have
/// distinct identities.
#[trace("QSpec-TC-195", "QSpec-FR-081-AC-2")]
#[test]
fn equal_shape_distinct_keys_differ() {
    let view = Fixture {
        types: vec![ty("P", &[]), ty("Q", &[])],
        fields: vec![fld("P/x", "P", None, &[]), fld("Q/x", "Q", None, &[])],
    }
    .view();
    let ids: BTreeSet<EffectiveId> = view
        .declarations()
        .iter()
        .map(|entry| entry.effective_id)
        .collect();
    assert_eq!(ids.len(), view.declarations().len());
}
