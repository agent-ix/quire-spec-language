// SPDX-License-Identifier: AGPL-3.0-or-later
//! QSpec FR-322 "Aggregate order" (FR-322-AC-260, FR-322-AC-265): the
//! declared order of the reference aggregates a timed model's nodes list.
//!
//! The six aggregates of a `timed_model` root, the constraints of each part
//! of a `clock_guard` and the bounds of a `time_invariant` list their
//! members strictly ascending by the member's **order key**: the minimum
//! (`source.identity`, `start`, `end`) over every region of every
//! `source_map` entry of the member node, then, between members that share
//! that triple, a key of the member's structure. A `clock_place` is keyed
//! by its place path; any other member by the member key FR-322 lists for
//! its form, in which a term is keyed by its structure (its form and the
//! shape of its body, a reference replaced by the key of its target). No
//! digest and no region of a shared term enters a key.
//!
//! [`order_timed_aggregates`] writes that order; [`first_out_of_order`]
//! finds the first member a reader refuses, which the conformance tests
//! replay against QSpec's published `AO` and `CL` vectors.

use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::str::FromStr;

use quire_contract_model::{
    CheckedNodeId, CheckedNodeTag, CheckedSemanticNodeV2, CheckedSourceMapEntry, CheckedTerm,
    LiteralKind, TimedModelForm,
};
use quire_exact::{Integer, Rational};

/// The deepest term a structural key follows: a key of a term nested
/// deeper is refused rather than walked without bound.
const TERM_DEPTH: usize = 256;

/// The `timed_model` root's reference aggregates, by binding index.
const ROOT_AGGREGATES: [usize; 6] = [3, 4, 5, 6, 7, 8];

/// One element of a key, compared element by element. `Close` sorts below
/// every other element, so a list that is a prefix of another is below it.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
enum Token {
    Close,
    Open,
    Number(Rational),
    Text(Box<str>),
    Boolean(bool),
}

/// Why an order key cannot be built.
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
pub enum OrderFault {
    /// A reference names no node of the graph.
    #[error("an ordered timed member references a node the graph does not hold")]
    Dangling,
    /// A member node has no source region.
    #[error("an ordered timed member has no source region")]
    Unlocated,
    /// A node's body is not the closed form its key reads.
    #[error("an ordered timed member's body is not its closed form")]
    Malformed,
    /// A term nests deeper than 256 levels.
    #[error("an ordered timed member's term nests too deeply to key")]
    TooDeep,
    /// Two members of one aggregate have one order key.
    #[error("two members of a timed aggregate have one order key")]
    EqualKeys,
}

/// The region triple of an order key.
type Triple<'g> = (&'g str, u64, u64);

/// The graph an order key is read from.
struct Graph<'g> {
    nodes: BTreeMap<&'g CheckedNodeId, &'g CheckedSemanticNodeV2>,
    triples: BTreeMap<&'g CheckedNodeId, Triple<'g>>,
}

impl<'g> Graph<'g> {
    fn new(nodes: &'g [CheckedSemanticNodeV2], source_map: &'g [CheckedSourceMapEntry]) -> Self {
        let mut triples: BTreeMap<&CheckedNodeId, Triple<'_>> = BTreeMap::new();
        for entry in source_map {
            for region in &entry.regions {
                let triple = (region.source.identity.as_ref(), region.start, region.end);
                triples
                    .entry(&entry.node_id)
                    .and_modify(|held| *held = (*held).min(triple))
                    .or_insert(triple);
            }
        }
        Self {
            nodes: nodes.iter().map(|node| (&node.node_id, node)).collect(),
            triples,
        }
    }

    fn node(&self, id: &CheckedNodeId) -> Result<&'g CheckedSemanticNodeV2, OrderFault> {
        self.nodes.get(id).copied().ok_or(OrderFault::Dangling)
    }

    fn triple(&self, id: &CheckedNodeId) -> Result<Triple<'g>, OrderFault> {
        self.triples.get(id).copied().ok_or(OrderFault::Unlocated)
    }

    /// The order of `left` before `right`: by region triple, then by the
    /// member key.
    fn compare(&self, left: &CheckedNodeId, right: &CheckedNodeId) -> Result<Ordering, OrderFault> {
        match self.triple(left)?.cmp(&self.triple(right)?) {
            Ordering::Equal => Ok(self.member_key(left)?.cmp(&self.member_key(right)?)),
            other => Ok(other),
        }
    }

    /// FR-322-AC-265's member key of the timed node `id`.
    // Intake proof: conformance_the_clause_member_key_vectors_replay and
    // conformance_the_aggregate_order_vectors_replay.
    #[ix_trace_rs::string_edge]
    fn member_key(&self, id: &CheckedNodeId) -> Result<Vec<Token>, OrderFault> {
        let node = self.node(id)?;
        let form = TimedModelForm::from_wire(&node.semantic_form).ok_or(OrderFault::Malformed)?;
        let body = &node.body;
        let mut key = Vec::new();
        match form {
            TimedModelForm::Place => self.path_key(binding(body, "path")?, &mut key)?,
            TimedModelForm::Constraint => match text(binding(body, "kind")?)? {
                "place" => {
                    key.push(number(0));
                    self.place_path(binding(body, "place")?, &mut key)?;
                    self.term(binding(body, "owner")?, &mut key, 0)?;
                    self.term(binding(body, "slots")?, &mut key, 0)?;
                    comparison(binding(body, "comparison")?, &mut key)?;
                    key.push(Token::Number(rational(binding(body, "bound")?)?));
                }
                "parameter" => {
                    key.push(number(1));
                    key.push(Token::Text(
                        self.operation_name(binding(body, "operation")?)?,
                    ));
                    key.push(Token::Text(text(binding(body, "parameter")?)?.into()));
                    comparison(binding(body, "comparison")?, &mut key)?;
                    key.push(Token::Number(rational(binding(body, "bound")?)?));
                }
                _ => return Err(OrderFault::Malformed),
            },
            TimedModelForm::Reset => match text(binding(body, "kind")?)? {
                "place" => {
                    key.push(number(0));
                    self.place_path(binding(body, "place")?, &mut key)?;
                    key.push(Token::Text(
                        self.operation_name(binding(body, "operation")?)?,
                    ));
                    self.term(binding(body, "condition")?, &mut key, 0)?;
                    self.term(binding(body, "binders")?, &mut key, 0)?;
                    self.term(binding(body, "owner")?, &mut key, 0)?;
                    self.term(binding(body, "slots")?, &mut key, 0)?;
                    key.push(Token::Number(rational(binding(body, "value")?)?));
                }
                "parameter" => {
                    key.push(number(1));
                    key.push(Token::Text(
                        self.operation_name(binding(body, "operation")?)?,
                    ));
                    key.push(Token::Text(text(binding(body, "parameter")?)?.into()));
                    key.push(Token::Number(rational(binding(body, "value")?)?));
                }
                _ => return Err(OrderFault::Malformed),
            },
            TimedModelForm::Invariant => {
                key.push(Token::Open);
                for bound in references(binding(body, "bounds")?)? {
                    key.push(Token::Open);
                    key.extend(self.member_key(bound)?);
                    key.push(Token::Close);
                }
                key.push(Token::Close);
                self.term(binding(body, "predicate")?, &mut key, 0)?;
            }
            TimedModelForm::Guard => {
                key.push(Token::Text(
                    self.operation_name(binding(body, "operation")?)?,
                ));
            }
            TimedModelForm::Urgency => {
                key.push(Token::Text(text(binding(body, "kind")?)?.into()));
                if let Ok(name) = binding(body, "operation_name") {
                    key.push(Token::Text(text(name)?.into()));
                }
            }
            TimedModelForm::Model | TimedModelForm::Source => return Err(OrderFault::Malformed),
        }
        Ok(key)
    }

    /// The place path key of the `clock_place` `reference` names.
    fn place_path(&self, reference: &CheckedTerm, key: &mut Vec<Token>) -> Result<(), OrderFault> {
        let place = self.node(target(reference)?)?;
        self.path_key(binding(&place.body, "path")?, key)
    }

    /// The place path key of `path`: each step by its kind, then a field
    /// by its name, a tuple by its index, a union by its case, a slot by
    /// its collection, minimum and maximum, `unbounded` above every
    /// integer. Neither the selector nor `resolved_path` is read.
    // Intake proof: conformance_the_clause_member_key_vectors_replay and
    // conformance_the_aggregate_order_vectors_replay.
    #[ix_trace_rs::string_edge]
    fn path_key(&self, path: &CheckedTerm, key: &mut Vec<Token>) -> Result<(), OrderFault> {
        key.push(Token::Open);
        for step in members_of(path)? {
            key.push(Token::Open);
            match text(binding(step, "kind")?)? {
                "field" => {
                    key.push(number(0));
                    key.push(Token::Text(text(binding(step, "name")?)?.into()));
                }
                "tuple" => {
                    key.push(number(1));
                    key.push(Token::Number(rational(binding(step, "index")?)?));
                }
                "union" => {
                    key.push(number(2));
                    key.push(Token::Text(text(binding(step, "case")?)?.into()));
                }
                "slot" => {
                    key.push(number(3));
                    key.push(Token::Text(text(binding(step, "collection")?)?.into()));
                    key.push(Token::Number(rational(binding(step, "minimum")?)?));
                    let maximum = binding(step, "maximum")?;
                    if text(maximum).is_ok_and(|word| word == "unbounded") {
                        key.push(number(1));
                    } else {
                        key.push(number(0));
                        key.push(Token::Number(rational(maximum)?));
                    }
                }
                _ => return Err(OrderFault::Malformed),
            }
            key.push(Token::Close);
        }
        key.push(Token::Close);
        Ok(())
    }

    /// The operation name of the `operation_anchor` `reference` names.
    // Intake proof: conformance_the_clause_member_key_vectors_replay and
    // conformance_the_aggregate_order_vectors_replay.
    #[ix_trace_rs::string_edge]
    fn operation_name(&self, reference: &CheckedTerm) -> Result<Box<str>, OrderFault> {
        let anchor = self.node(target(reference)?)?;
        let members = match &anchor.body {
            CheckedTerm::Aggregate { members } => members.as_slice(),
            CheckedTerm::OperationAnchorBody(body) => body.members.as_slice(),
            _ => return Err(OrderFault::Malformed),
        };
        members
            .iter()
            .find_map(|member| match member {
                CheckedTerm::Binding { name, value } if name.as_ref() == "operation" => {
                    text(value).ok()
                }
                _ => None,
            })
            .map(Box::from)
            .ok_or(OrderFault::Malformed)
    }

    /// The structural key of `term`: a reference by the key of its target
    /// node (its form, then its body), a literal by its kind and value, an
    /// aggregate by its members in order. No digest, region or literal
    /// type is read.
    fn term(
        &self,
        term: &CheckedTerm,
        key: &mut Vec<Token>,
        depth: usize,
    ) -> Result<(), OrderFault> {
        if depth > TERM_DEPTH {
            return Err(OrderFault::TooDeep);
        }
        match term {
            CheckedTerm::Reference { target } => {
                let node = self.node(target)?;
                key.push(Token::Open);
                key.push(Token::Text(node.semantic_form.clone()));
                self.term(&node.body, key, depth + 1)?;
                key.push(Token::Close);
            }
            CheckedTerm::Literal {
                value_kind, value, ..
            } => {
                key.push(Token::Text(value_kind.as_wire().into()));
                key.push(literal_token(*value_kind, value));
            }
            CheckedTerm::Aggregate { members } => {
                key.push(Token::Open);
                for member in members {
                    self.term(member, key, depth + 1)?;
                }
                key.push(Token::Close);
            }
            CheckedTerm::Binding { name, value } => {
                key.push(Token::Text(name.as_ref().into()));
                self.term(value, key, depth + 1)?;
            }
            CheckedTerm::Application {
                operation,
                arguments,
                ..
            } => {
                let identity = serde_json::to_value(operation)
                    .ok()
                    .and_then(|record| record["identity"].as_str().map(Box::from))
                    .unwrap_or_default();
                key.push(Token::Text(identity));
                key.push(Token::Open);
                for argument in arguments {
                    self.term(argument, key, depth + 1)?;
                }
                key.push(Token::Close);
            }
            CheckedTerm::OperationAnchorBody(body) => {
                key.push(Token::Open);
                for member in &body.members {
                    self.term(member, key, depth + 1)?;
                }
                key.push(Token::Close);
            }
            CheckedTerm::DependencyReference { .. }
            | CheckedTerm::Frame { .. }
            | CheckedTerm::AbstractionRelation { .. }
            | CheckedTerm::Refinement { .. }
            | CheckedTerm::ExtensionForm { .. }
            | CheckedTerm::NominalBody(_) => return Err(OrderFault::Malformed),
        }
        Ok(())
    }
}

/// Each ordered aggregate of the timed nodes of `nodes`: the node's graph
/// index, the pointer of the aggregate's members below the node, the index
/// of its first ordered member, and the member references.
fn aggregates(nodes: &[CheckedSemanticNodeV2]) -> Vec<(usize, String, usize, Vec<&CheckedNodeId>)> {
    let mut found = Vec::new();
    for (index, node) in nodes.iter().enumerate() {
        if CheckedNodeTag::from_wire(&node.node_tag) != Some(CheckedNodeTag::TimedModel) {
            continue;
        }
        let Ok(members) = members_of(&node.body) else {
            continue;
        };
        match TimedModelForm::from_wire(&node.semantic_form) {
            Some(TimedModelForm::Model) => {
                for at in ROOT_AGGREGATES {
                    if let Some(Ok(list)) = members.get(at).map(bound_references) {
                        found.push((index, format!("body/members/{at}/value/members"), 0, list));
                    }
                }
            }
            Some(TimedModelForm::Guard) => {
                let parts = members
                    .get(1)
                    .map(bound_value)
                    .and_then(|parts| parts.and_then(members_of).ok());
                for (part, entry) in parts.unwrap_or_default().iter().enumerate() {
                    let Ok(entries) = members_of(entry) else {
                        continue;
                    };
                    let list = entries
                        .iter()
                        .skip(1)
                        .filter_map(|member| target(member).ok())
                        .collect();
                    found.push((
                        index,
                        format!("body/members/1/value/members/{part}/members"),
                        1,
                        list,
                    ));
                }
            }
            Some(TimedModelForm::Invariant) => {
                if let Some(Ok(list)) = members.get(1).map(bound_references) {
                    found.push((index, "body/members/1/value/members".to_owned(), 0, list));
                }
            }
            _ => {}
        }
    }
    found
}

/// The pointer of the first member of an ordered timed aggregate that is
/// not strictly above its predecessor (QSpec FR-322 "Aggregate order"),
/// `None` when every aggregate is in order.
///
/// # Errors
///
/// An [`OrderFault`] when a member's key cannot be built.
#[cfg(test)]
pub(crate) fn first_out_of_order(
    nodes: &[CheckedSemanticNodeV2],
    source_map: &[CheckedSourceMapEntry],
) -> Result<Option<String>, OrderFault> {
    let graph = Graph::new(nodes, source_map);
    for (index, pointer, first, list) in aggregates(nodes) {
        for (at, pair) in list.windows(2).enumerate() {
            if graph.compare(pair[0], pair[1])? != Ordering::Less {
                return Ok(Some(format!(
                    "/semantic_graph/nodes/{index}/{pointer}/{}",
                    first + at + 1
                )));
            }
        }
    }
    Ok(None)
}

/// Put every ordered aggregate of the timed nodes of `nodes` in declared
/// order. Returns whether any member moved; the caller rekeys the nodes it
/// changed.
///
/// # Errors
///
/// An [`OrderFault`] when a key cannot be built, or two members of one
/// aggregate have one key, which is a producer defect.
pub(crate) fn order_timed_aggregates(
    nodes: &mut [CheckedSemanticNodeV2],
    source_map: &[CheckedSourceMapEntry],
) -> Result<bool, OrderFault> {
    let mut orders: Vec<(usize, String, usize, Vec<CheckedNodeId>)> = Vec::new();
    {
        let graph = Graph::new(nodes, source_map);
        for (index, pointer, first, list) in aggregates(nodes) {
            let mut sorted: Vec<&CheckedNodeId> = list.clone();
            let mut fault = None;
            sorted.sort_by(|left, right| match graph.compare(left, right) {
                Ok(order) => order,
                Err(error) => {
                    fault.get_or_insert(error);
                    Ordering::Equal
                }
            });
            if let Some(fault) = fault {
                return Err(fault);
            }
            for pair in sorted.windows(2) {
                if graph.compare(pair[0], pair[1])? != Ordering::Less {
                    return Err(OrderFault::EqualKeys);
                }
            }
            if sorted != list {
                orders.push((index, pointer, first, sorted.into_iter().cloned().collect()));
            }
        }
    }
    let moved = !orders.is_empty();
    for (index, pointer, first, sorted) in orders {
        let Some(node) = nodes.get_mut(index) else {
            return Err(OrderFault::Malformed);
        };
        let list = aggregate_at(&mut node.body, &pointer).ok_or(OrderFault::Malformed)?;
        let ordered = list
            .iter()
            .take(first)
            .cloned()
            .chain(sorted.into_iter().map(CheckedTerm::reference))
            .collect();
        *list = ordered;
    }
    Ok(moved)
}

/// The member list at `pointer` (`body/members/{b}/value/members` or
/// `body/members/1/value/members/{p}/members`) below a node.
fn aggregate_at<'t>(body: &'t mut CheckedTerm, pointer: &str) -> Option<&'t mut Vec<CheckedTerm>> {
    let mut steps = pointer.split('/').skip(1);
    let mut at = body;
    while let Some(step) = steps.next() {
        at = match (step, at) {
            ("members", CheckedTerm::Aggregate { members }) => match steps.next() {
                None => return Some(members),
                Some(index) => members.get_mut(index.parse::<usize>().ok()?)?,
            },
            ("value", CheckedTerm::Binding { value, .. }) => value,
            _ => return None,
        };
    }
    None
}

fn number(value: u64) -> Token {
    Token::Number(Rational::from_integer(Integer::from(value)))
}

fn comparison(term: &CheckedTerm, key: &mut Vec<Token>) -> Result<(), OrderFault> {
    // The comparisons are declared in their rank.
    let rank = crate::checked_v2::ClockComparison::from_wire(text(term)?)
        .ok_or(OrderFault::Malformed)? as u64;
    key.push(number(rank));
    Ok(())
}

/// A literal's value: a number as an exact rational, text by its bytes.
fn literal_token(kind: LiteralKind, value: &serde_json::Value) -> Token {
    if let Some(flag) = value.as_bool() {
        return Token::Boolean(flag);
    }
    let spelled = match value {
        serde_json::Value::String(text) => text.clone(),
        other => other.to_string(),
    };
    match kind {
        LiteralKind::Text => Token::Text(spelled.into()),
        _ => parse_rational(&spelled).map_or_else(|| Token::Text(spelled.into()), Token::Number),
    }
}

fn parse_rational(spelled: &str) -> Option<Rational> {
    let (numerator, denominator) = spelled.split_once('/').unwrap_or((spelled, "1"));
    Rational::new(
        Integer::from_str(numerator).ok()?,
        Integer::from_str(denominator).ok()?,
    )
    .ok()
}

fn rational(term: &CheckedTerm) -> Result<Rational, OrderFault> {
    match term {
        CheckedTerm::Literal { value, .. } => match value {
            serde_json::Value::String(text) => parse_rational(text),
            serde_json::Value::Number(number) => parse_rational(&number.to_string()),
            _ => None,
        }
        .ok_or(OrderFault::Malformed),
        _ => Err(OrderFault::Malformed),
    }
}

fn members_of(term: &CheckedTerm) -> Result<&[CheckedTerm], OrderFault> {
    match term {
        CheckedTerm::Aggregate { members } => Ok(members),
        _ => Err(OrderFault::Malformed),
    }
}

fn bound_value(term: &CheckedTerm) -> Result<&CheckedTerm, OrderFault> {
    match term {
        CheckedTerm::Binding { value, .. } => Ok(value),
        _ => Err(OrderFault::Malformed),
    }
}

fn bound_references(term: &CheckedTerm) -> Result<Vec<&CheckedNodeId>, OrderFault> {
    references(bound_value(term)?)
}

fn references(term: &CheckedTerm) -> Result<Vec<&CheckedNodeId>, OrderFault> {
    members_of(term)?.iter().map(target).collect()
}

fn binding<'t>(term: &'t CheckedTerm, name: &str) -> Result<&'t CheckedTerm, OrderFault> {
    members_of(term)?
        .iter()
        .find_map(|member| match member {
            CheckedTerm::Binding { name: held, value } if held.as_ref() == name => Some(&**value),
            _ => None,
        })
        .ok_or(OrderFault::Malformed)
}

fn target(term: &CheckedTerm) -> Result<&CheckedNodeId, OrderFault> {
    match term {
        CheckedTerm::Reference { target } => Ok(target),
        _ => Err(OrderFault::Malformed),
    }
}

fn text(term: &CheckedTerm) -> Result<&str, OrderFault> {
    match term {
        CheckedTerm::Literal {
            value_kind: LiteralKind::Text,
            value,
            ..
        } => value.as_str().ok_or(OrderFault::Malformed),
        _ => Err(OrderFault::Malformed),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ix_trace_rs::trace;
    use serde_json::Value;

    /// The QSpec checked-package V2 proposal directory; the conformance
    /// tests fail explicitly without a QSpec checkout.
    fn proposals() -> std::path::PathBuf {
        let directory = std::env::var_os("QSPEC_DIR")
            .expect("QSPEC_DIR must point to an available QSpec checkout");
        std::path::Path::new(&directory).join("proposals/checked-package-v2")
    }

    fn read(path: &std::path::Path) -> Value {
        serde_json::from_slice(
            &std::fs::read(path).unwrap_or_else(|failure| panic!("{}: {failure}", path.display())),
        )
        .expect("published JSON")
    }

    fn graph(
        fixture: &str,
        patch: Option<&Value>,
    ) -> (Vec<CheckedSemanticNodeV2>, Vec<CheckedSourceMapEntry>) {
        crate::checked_v2::tests::admitted_fixture_graph(fixture, patch)
    }

    /// Replay each case of `cases` on its fixture. IR's reader enforces
    /// declared order: an ordered case is admitted and its typed graph is in
    /// order; a refused case is refused `invalid_semantic_graph` at the
    /// case's locus.
    fn replay(cases: &[Value], selected: impl Fn(&Value) -> bool) {
        let mut replayed = 0;
        for case in cases.iter().filter(|case| selected(case)) {
            let id = &case["id"];
            let outcome = crate::checked_v2::tests::read_fixture_graph(
                case["fixture"].as_str().expect("a fixture"),
                case.get("patch"),
            );
            match (case["expected"].as_str(), outcome) {
                (Some("admitted"), Ok((nodes, source_map))) => {
                    let found = first_out_of_order(&nodes, &source_map)
                        .unwrap_or_else(|fault| panic!("{id}: {fault}"));
                    assert_eq!(found, None, "{id} is in declared order");
                }
                (Some("admitted"), Err(refusal)) => panic!("{id} is admitted: {refusal:?}"),
                (_, Err((code, locus))) => {
                    assert_eq!(
                        code,
                        quire_contract_model::CheckedPackageRefusalCode::InvalidSemanticGraph,
                        "{id}"
                    );
                    assert_eq!(
                        locus.as_deref(),
                        case["locus"].as_str(),
                        "{id} refuses at its locus"
                    );
                }
                (_, Ok(_)) => panic!("{id} is refused"),
            }
            replayed += 1;
        }
        assert!(replayed > 0, "the vectors publish order cases");
    }

    /// QSpec FR-322-AC-260 (TC-367 CK-30): the slot vectors' `AO-01`..`AO-10`
    /// are in declared order exactly where published, the first member out
    /// of order at each refused case's locus: by minimum source region, then
    /// by the place path key for places of one region.
    #[trace("QSpec-FR-322-AC-260", "QSpec-TC-367")]
    #[test]
    fn conformance_the_aggregate_order_vectors_replay() {
        let directory = proposals();
        let vectors = read(&directory.join("timed-slot-selector-vectors.json"));
        let cases = vectors["aggregate_order"]["cases"]
            .as_array()
            .expect("AO cases");
        replay(cases, |_| true);
    }

    /// QSpec FR-322-AC-265 (TC-367 CK-36): the clause vectors' order cases
    /// (`CL-16`, `CL-29`..`CL-53`) and the published clause package replay:
    /// members of one region are ordered by their structural member key,
    /// never by a digest or a shared term's region.
    #[trace("QSpec-FR-322-AC-265", "QSpec-TC-367")]
    #[test]
    fn conformance_the_clause_member_key_vectors_replay() {
        let directory = proposals();
        let vectors = read(&directory.join("timed-clause-node-vectors.json"));
        let cases = vectors["cases"].as_array().expect("CL cases");
        replay(cases, |case| {
            case["expected"] == "admitted" || case["expected_code"] == "invalid_semantic_graph"
        });
    }

    /// QSpec FR-322 "Aggregate order": the producer writes the declared
    /// order whatever order the members were collected in. The clause
    /// package with every ordered list reversed is put back in the
    /// published order.
    #[trace("QSpec-FR-322-AC-260", "QSpec-FR-322-AC-265")]
    #[test]
    fn the_producer_writes_the_declared_order() {
        for fixture in [
            "fixtures/positive-timed-clause-nodes.json",
            "fixtures/positive-timed-slot-nodes.json",
        ] {
            let (published, source_map) = graph(fixture, None);
            let mut nodes = published.clone();
            for (index, pointer, first, _) in aggregates(&published) {
                let list = aggregate_at(&mut nodes[index].body, &pointer).expect("an aggregate");
                list[first..].reverse();
            }
            assert_ne!(nodes, published, "{fixture}: some list moved");
            assert_eq!(
                order_timed_aggregates(&mut nodes, &source_map),
                Ok(true),
                "{fixture}"
            );
            assert_eq!(
                nodes, published,
                "{fixture}: the published order is restored"
            );
            assert_eq!(
                order_timed_aggregates(&mut nodes, &source_map),
                Ok(false),
                "{fixture}: an ordered package does not move"
            );
        }
    }
}
