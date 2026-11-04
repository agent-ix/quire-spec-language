// SPDX-License-Identifier: AGPL-3.0-or-later
//! The assembler's dimension and unit declarations (FR-091 "Dimension and
//! unit declarations"): every source error is reported, the unit-graph
//! topology checks run only when no other check found an error, and each
//! dimension and unit is then keyed with `nominal_key` and admitted through
//! `UnitGraph::admit`. The assembler stops at the admitted graph, which
//! keeps each node's preimage bytes; lowering builds the unit and dimension
//! nodes from them (FR-094).

use std::collections::{BTreeMap, BTreeSet};

use qsl_forms::{DimensionForm, ExactNumberForm, ExactNumberKind, TermOperator, UnitForm};
use qsl_foundation::diagnostic::LimitExceeded;
use qsl_foundation::{Setting, Span};
use quire_exact::{Integer, NodeKey, Rational};

use super::{AssemblyCause, AssemblyError, AssemblyLimits, TopologyFault};
use crate::check::lowering::AdmittedModel;
use crate::check::lowering::{quantity_origin, strongly_connected, QuantityKind};
use crate::check::node_key::SourceOwner;
use crate::model::domain_package::PackageDimensionRecord;
use crate::value::semantic_node::{ModelSubject, NodeOwner, NominalRefusal, OwnerSelection};
use crate::value::unit::{admit_unit_graph, DimensionPreimage, UnitPreimage};
use quire_checked::key::DeclarationKey;
use quire_semantic_value::semantic_node::{InvalidSemanticGraph, SemanticGraphCause};
use quire_semantic_value::unit::UnitGraph;

/// The admitted graph and the span of each declared name by its key.
pub(super) struct Assembled {
    pub(super) graph: UnitGraph,
    pub(super) spans: BTreeMap<NodeKey, Span>,
    /// Each dimension's key and normalized base map (base-dimension key to
    /// exponent), by its declared name.
    pub(super) dimensions: BTreeMap<String, (NodeKey, BTreeMap<NodeKey, Integer>)>,
    /// Each unit's dimension key, composed scale and composed offset to its
    /// canonical root, by its declared name.
    pub(super) units: BTreeMap<String, ComposedUnit>,
}

/// The source document owners and their encounter order for one assembly.
pub(super) struct SourceDocuments<'a> {
    pub(super) owners: &'a BTreeMap<String, SourceOwner>,
    pub(super) order: &'a [SourceOwner],
}

/// What the dimension and unit declarations fix before the unit-graph
/// topology is decided: each base dimension's key, each derived dimension's
/// name, and each unit's dimension with its composed edge when its target
/// chain is sound (FR-091 "Unit-graph topology": the base equivalence pairs
/// are decided before the first dimension turn).
#[derive(Debug, Default)]
pub(super) struct Provisional {
    /// Each base dimension's key, by its declared name.
    pub(super) bases: BTreeMap<String, NodeKey>,
    /// The declared name of each derived dimension.
    pub(super) derived: BTreeSet<String>,
    /// Each unit's dimension name and, when no row 1, 2, 3 or 6 defect
    /// breaks its target chain, its composed scale and offset to the root.
    pub(super) units: BTreeMap<String, (String, Option<(Rational, Rational)>)>,
}

/// Why the declarations admit no unit graph.
pub(super) struct Refused {
    /// Every error found.
    pub(super) errors: Vec<AssemblyError>,
    /// Whether the errors are unit-graph topology defects, found only once
    /// every name and number resolved.
    pub(super) topology: bool,
    /// What the declarations fix regardless.
    pub(super) provisional: Provisional,
}

/// A unit's dimension node and its composed edge to the canonical root of
/// that dimension: `root = scale × value + offset`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ComposedUnit {
    /// The unit's dimension node.
    pub(crate) dimension: NodeKey,
    /// The composed scale to the root.
    pub(crate) scale: Rational,
    /// The composed offset to the root.
    pub(crate) offset: Rational,
}

fn error(cause: AssemblyCause, span: Span) -> AssemblyError {
    AssemblyError { cause, span }
}

/// The first declaration each name binds, and every duplicate refused.
fn index_names<'a>(
    names: impl Iterator<Item = (&'a str, Span)>,
    errors: &mut Vec<AssemblyError>,
) -> BTreeMap<&'a str, usize> {
    let mut spans: BTreeMap<&str, Vec<Span>> = BTreeMap::new();
    let mut first = BTreeMap::new();
    for (index, (name, span)) in names.enumerate() {
        first.entry(name).or_insert(index);
        spans.entry(name).or_default().push(span);
    }
    for (name, spans) in spans {
        if let [_, second, ..] = spans.as_slice() {
            errors.push(error(
                AssemblyCause::DuplicateQuantityName {
                    name: name.to_owned(),
                    spans: spans.clone(),
                },
                *second,
            ));
        }
    }
    first
}

/// The reduced value `number` writes (ADR-013 R-07), or the error it is:
/// a zero denominator, or a decimal scale above the bound.
fn reduce(number: &ExactNumberForm, limits: AssemblyLimits) -> Result<Rational, AssemblyError> {
    let denominator = match number.kind {
        ExactNumberKind::Rational => number.second.clone(),
        ExactNumberKind::Decimal => {
            let scale = number
                .second
                .to_u64()
                .filter(|scale| *scale <= limits.decimal_scale);
            match scale {
                Some(scale) => Integer::power_of_ten(scale),
                None => {
                    let actual = number.second.to_u64().map_or(u128::MAX, u128::from);
                    return Err(error(
                        AssemblyCause::DecimalScaleLimit(LimitExceeded::new(
                            Setting::S3DecimalScale,
                            limits.decimal_scale,
                            actual,
                        )),
                        number.span,
                    ));
                }
            }
        }
    };
    Rational::new(number.first.clone(), denominator)
        .map_err(|_| error(AssemblyCause::ZeroDenominator, number.span))
}

/// A base dimension a normalized map names: a base dimension of the unit,
/// by its index, or an imported one, by its key.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
enum BaseRef {
    Source(usize),
    Imported(NodeKey),
}

/// The base-dimension exponents of every dimension, indexed as `dimensions`,
/// once no cycle remains: a base dimension is itself with exponent one, a
/// derived dimension is its terms' maps, multiplied by their exponents
/// (negated after `/`) and added, with zero exponents dropped. A term naming
/// an imported dimension uses that dimension's admitted normalized map.
fn normalized(
    dimensions: &[DimensionForm],
    by_name: &BTreeMap<&str, usize>,
    imported: &Imported,
    edges: &[Vec<usize>],
) -> Vec<BTreeMap<BaseRef, Integer>> {
    let mut maps: Vec<BTreeMap<BaseRef, Integer>> = vec![BTreeMap::new(); dimensions.len()];
    for component in strongly_connected(edges) {
        for index in component {
            let form = &dimensions[index];
            if form.terms.is_empty() {
                maps[index].insert(BaseRef::Source(index), Integer::one());
                continue;
            }
            let mut sum: BTreeMap<BaseRef, Integer> = BTreeMap::new();
            for term in &form.terms {
                let named: Vec<(BaseRef, Integer)> = match by_name.get(term.name.name.as_str()) {
                    Some(&named) => maps[named]
                        .iter()
                        .map(|(base, power)| (base.clone(), power.clone()))
                        .collect(),
                    None => match imported.dimensions.get(term.name.name.as_str()) {
                        Some(dimension) => dimension
                            .map
                            .iter()
                            .map(|(base, power)| (BaseRef::Imported(base.clone()), power.clone()))
                            .collect(),
                        None => continue,
                    },
                };
                let exponent = term
                    .exponent
                    .as_ref()
                    .map_or_else(Integer::one, |(exponent, _)| exponent.clone());
                let exponent = match term.operator {
                    Some(TermOperator::Divide) => exponent.neg(),
                    Some(TermOperator::Multiply) | None => exponent,
                };
                for (base, power) in named {
                    let entry = sum.entry(base).or_insert_with(Integer::zero);
                    *entry = entry.add(&power.mul(&exponent));
                }
            }
            sum.retain(|_, exponent| !exponent.is_zero());
            maps[index] = sum;
        }
    }
    maps
}

/// A dimension of an admitted domain package, which a dimension term of
/// the unit names as `Alias::Name` (FR-142: an imported node, whose
/// definition and units count as admitted).
#[derive(Clone, Debug)]
pub(super) struct ImportedDimension {
    /// Its node key.
    pub(super) key: NodeKey,
    /// Its FCD identity.
    pub(super) identity: String,
    /// Its normalized base map over imported base keys.
    pub(super) map: BTreeMap<NodeKey, Integer>,
    /// Whether its package declares a unit of it.
    pub(super) has_unit: bool,
    /// Its model selection's span, where a refusal of it is located.
    pub(super) selection: Span,
    /// The package dimensions its terms name, by `Alias::Name`.
    pub(super) terms: Vec<String>,
    /// Its place in the selections' order, then in its package's order.
    pub(super) order: usize,
}

/// The dimensions of the unit's admitted domain packages, by `Alias::Name`,
/// and each package base dimension's preimage by its key.
#[derive(Default)]
pub(super) struct Imported {
    pub(super) dimensions: BTreeMap<String, ImportedDimension>,
    bases: BTreeMap<NodeKey, DimensionPreimage>,
    owners: Vec<NodeOwner>,
}

impl Imported {
    /// The dimensions of `models`, each selected at the matching span of
    /// `selections`.
    pub(super) fn new(models: &[(AdmittedModel, Span)]) -> Self {
        let mut imported = Self::default();
        for (model, selection) in models {
            let Some(alias) = model.alias() else { continue };
            for dimension in &model.dimensions {
                let Some((key, map, preimage)) =
                    package_dimension(model, dimension, &mut Vec::new())
                else {
                    continue;
                };
                if dimension.base {
                    imported.owners.push(preimage.owner().clone());
                    imported.bases.insert(key.clone(), preimage);
                }
                imported.dimensions.insert(
                    format!("{alias}::{}", dimension.name),
                    ImportedDimension {
                        key,
                        identity: dimension.key.node.clone(),
                        map,
                        has_unit: model
                            .units
                            .iter()
                            .any(|unit| unit.dimension == dimension.key),
                        selection: *selection,
                        terms: dimension
                            .terms
                            .iter()
                            .filter_map(|(term, _)| {
                                let named = model.dimensions.iter().find(|d| &d.key == term)?;
                                Some(format!("{alias}::{}", named.name))
                            })
                            .collect(),
                        order: imported.dimensions.len(),
                    },
                );
            }
        }
        imported
    }
}

/// A package dimension's key, normalized base map and preimage: a base
/// dimension is keyed over its name, a derived one over its normalized
/// terms, each term's map multiplied by its exponent. `None` for a term
/// naming no dimension of the package, or a cycle, which intake refuses.
pub(crate) fn package_dimension(
    model: &AdmittedModel,
    dimension: &PackageDimensionRecord,
    visiting: &mut Vec<DeclarationKey>,
) -> Option<(NodeKey, BTreeMap<NodeKey, Integer>, DimensionPreimage)> {
    if visiting.contains(&dimension.key) {
        return None;
    }
    visiting.push(dimension.key.clone());
    let owner = NodeOwner::Model(ModelSubject {
        identity: model.selection().identity.clone(),
        node: dimension.key.node.clone(),
    });
    let mut map: BTreeMap<NodeKey, Integer> = BTreeMap::new();
    for (term, exponent) in &dimension.terms {
        let named = model.dimensions.iter().find(|d| &d.key == term)?;
        let (_, term_map, _) = package_dimension(model, named, visiting)?;
        for (base, power) in term_map {
            let entry = map.entry(base).or_insert_with(Integer::zero);
            *entry = entry.add(&power.mul(exponent));
        }
    }
    visiting.pop();
    map.retain(|_, exponent| !exponent.is_zero());
    let terms: Vec<(NodeKey, Integer)> = map.clone().into_iter().collect();
    let preimage = DimensionPreimage::new(owner, vec![dimension.name.clone()], terms).ok()?;
    let key = preimage.key().ok()?;
    if dimension.terms.is_empty() {
        map.insert(key.clone(), Integer::one());
    }
    Some((key, map, preimage))
}

/// What `dimensions` and `units` fix before the topology is decided.
#[allow(clippy::too_many_arguments)] // the declarations as resolved so far
fn provisional(
    dimensions: &[DimensionForm],
    units: &[UnitForm],
    unit_dimension: &[Option<usize>],
    unit_edges: &[Vec<usize>],
    unknown_target: &[bool],
    scales: &[Option<Rational>],
    offsets: &[Option<Rational>],
    owner: &SourceOwner,
    owners: &BTreeMap<String, SourceOwner>,
    package_name: &str,
) -> Provisional {
    let mut fixed = Provisional::default();
    for form in dimensions {
        let name = form.name.name.clone();
        if !form.terms.is_empty() {
            fixed.derived.insert(name);
            continue;
        }
        let key = DimensionPreimage::new(
            owners.get(&name).unwrap_or(owner).node_owner(package_name),
            vec![name.clone()],
            Vec::new(),
        )
        .ok()
        .and_then(|preimage| preimage.key().ok());
        if let Some(key) = key {
            fixed.bases.entry(name).or_insert(key);
        }
    }
    for (index, form) in units.iter().enumerate() {
        let composed = compose(
            index,
            unit_dimension,
            unit_edges,
            unknown_target,
            scales,
            offsets,
        );
        fixed
            .units
            .entry(form.name.name.clone())
            .or_insert((form.dimension.name.clone(), composed));
    }
    fixed
}

/// Unit `index`'s composed edge to its root along its target chain, or
/// `None` when a zero or unreduced scale, an unresolved target, a target of
/// another dimension or a cycle breaks the chain.
fn compose(
    index: usize,
    unit_dimension: &[Option<usize>],
    unit_edges: &[Vec<usize>],
    unknown_target: &[bool],
    scales: &[Option<Rational>],
    offsets: &[Option<Rational>],
) -> Option<(Rational, Rational)> {
    let dimension = (*unit_dimension.get(index)?)?;
    let mut chain = vec![index];
    let mut at = index;
    while let Some(&target) = unit_edges.get(at)?.first() {
        if chain.contains(&target) || *unit_dimension.get(target)? != Some(dimension) {
            return None;
        }
        chain.push(target);
        at = target;
    }
    if *unknown_target.get(at)? {
        return None;
    }
    // Fold from the root: `root = s_t × (s × v + o) + o_t`.
    let mut scale = Rational::from_integer(Integer::one());
    let mut offset = Rational::from_integer(Integer::zero());
    for unit in chain.iter().rev() {
        let unit_scale = scales.get(*unit)?.as_ref()?;
        let unit_offset = offsets.get(*unit)?.as_ref()?;
        if unit_scale.is_zero() {
            return None;
        }
        offset = scale.mul(unit_offset).add(&offset);
        scale = scale.mul(unit_scale);
    }
    Some((scale, offset))
}

/// A node of FR-142's turn list: a local dimension, or a dimension of an
/// admitted domain package by `Alias::Name`.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
enum Turn {
    Local(usize),
    Imported(String),
}

/// What the unit-graph topology check reads, once every name that types a
/// unit and every number resolved.
struct Walk<'a> {
    dimensions: &'a [DimensionForm],
    units: &'a [UnitForm],
    dimension_names: &'a [&'a str],
    dimension_spans: &'a [Span],
    /// The rank of each dimension's source document in request order.
    dimension_documents: &'a [usize],
    /// The rank of each unit's source document in request order.
    unit_documents: &'a [usize],
    unit_names: &'a [&'a str],
    unit_spans: &'a [Span],
    dimension_index: &'a BTreeMap<&'a str, usize>,
    /// Each dimension's local term edges, in term order.
    dimension_edges: &'a [Vec<usize>],
    /// Whether a dimension has a term naming no dimension.
    unknown_term: &'a [bool],
    unit_dimension: &'a [usize],
    unit_edges: &'a [Vec<usize>],
    /// Whether a unit's target names no unit.
    unknown_target: &'a [bool],
    scales: &'a [Rational],
    offsets: &'a [Rational],
    imported: &'a Imported,
    maps: &'a [BTreeMap<BaseRef, Integer>],
}

impl Walk<'_> {
    fn topology(&self, fault: TopologyFault, declarations: Vec<&str>, span: Span) -> AssemblyError {
        error(
            AssemblyCause::UnitGraphTopology {
                fault,
                declarations: declarations.into_iter().map(str::to_owned).collect(),
            },
            span,
        )
    }

    /// FR-142's turn list: a local dimension takes its turn at its own
    /// declaration; a dimension of an admitted package takes its turn the
    /// first time a declaration reaches it, before that declaration and
    /// before the nodes reached through its terms; the package dimensions
    /// no declaration reached follow, in the order of their selections.
    fn turns(&self) -> Vec<Turn> {
        enum Declaration {
            Dimension(usize),
            Unit(usize),
        }
        let mut declarations: Vec<((usize, usize), Declaration)> = self
            .dimension_spans
            .iter()
            .enumerate()
            .map(|(index, span)| {
                (
                    (self.dimension_documents[index], span.start),
                    Declaration::Dimension(index),
                )
            })
            .chain(self.unit_spans.iter().enumerate().map(|(index, span)| {
                (
                    (self.unit_documents[index], span.start),
                    Declaration::Unit(index),
                )
            }))
            .collect();
        // Source order: the documents as the request lists them, then the
        // byte offset within a document.
        declarations.sort_by_key(|(position, _)| *position);

        let mut turns = Vec::new();
        let mut reached: BTreeSet<Turn> = BTreeSet::new();
        // The nodes a declaration mentions, in mention order, expanded once
        // depth-first through their own terms.
        let mut mention = |roots: Vec<Turn>, turns: &mut Vec<Turn>| {
            let mut stack: Vec<Turn> = roots.into_iter().rev().collect();
            while let Some(node) = stack.pop() {
                if !reached.insert(node.clone()) {
                    continue;
                }
                let children: Vec<Turn> = match &node {
                    Turn::Local(index) => self.dimensions[*index]
                        .terms
                        .iter()
                        .filter_map(|term| self.named(&term.name.name))
                        .collect(),
                    Turn::Imported(name) => {
                        let Some(dimension) = self.imported.dimensions.get(name.as_str()) else {
                            continue;
                        };
                        turns.push(node.clone());
                        dimension
                            .terms
                            .iter()
                            .map(|term| Turn::Imported(term.clone()))
                            .collect()
                    }
                };
                stack.extend(children.into_iter().rev());
            }
        };
        for (_, declaration) in &declarations {
            match declaration {
                Declaration::Dimension(index) => {
                    let terms = self.dimensions[*index]
                        .terms
                        .iter()
                        .filter_map(|term| self.named(&term.name.name))
                        .collect();
                    mention(terms, &mut turns);
                    turns.push(Turn::Local(*index));
                }
                Declaration::Unit(index) => {
                    let mut roots = vec![Turn::Local(self.unit_dimension[*index])];
                    if let Some(&target) = self.unit_edges[*index].first() {
                        roots.push(Turn::Local(self.unit_dimension[target]));
                    }
                    mention(roots, &mut turns);
                }
            }
        }
        let mut rest: Vec<(&String, usize)> = self
            .imported
            .dimensions
            .iter()
            .map(|(name, dimension)| (name, dimension.order))
            .collect();
        rest.sort_by_key(|(_, order)| *order);
        for (name, _) in rest {
            let node = Turn::Imported(name.clone());
            if reached.insert(node.clone()) {
                turns.push(node);
            }
        }
        turns
    }

    /// The node a dimension name denotes, when it names one.
    fn named(&self, name: &str) -> Option<Turn> {
        match self.dimension_index.get(name) {
            Some(&index) => Some(Turn::Local(index)),
            None => self
                .imported
                .dimensions
                .contains_key(name)
                .then(|| Turn::Imported(name.to_owned())),
        }
    }

    /// The first defect of the first node in turn-list order that has one.
    fn first_defect(&self) -> Option<AssemblyError> {
        let components = strongly_connected(self.dimension_edges);
        let mut component_of = vec![0; self.dimensions.len()];
        let mut cyclic = vec![false; self.dimensions.len()];
        for (number, component) in components.iter().enumerate() {
            let loops = component.len() > 1
                || component
                    .first()
                    .is_some_and(|member| self.dimension_edges[*member].contains(member));
            for &member in component {
                component_of[member] = number;
                cyclic[member] = loops;
            }
        }
        // A dimension that has a cycle, an unknown term, or a term that
        // reaches one has no normalized map.
        let mut mapless = vec![false; self.dimensions.len()];
        for component in &components {
            let broken = component.iter().any(|&member| {
                cyclic[member]
                    || self.unknown_term[member]
                    || self.dimension_edges[member]
                        .iter()
                        .any(|&term| mapless[term])
            });
            for &member in component {
                mapless[member] = broken;
            }
        }
        let unit_components = strongly_connected(self.unit_edges);
        let mut unit_cyclic = vec![false; self.units.len()];
        for component in &unit_components {
            let loops = component.len() > 1
                || component
                    .first()
                    .is_some_and(|member| self.unit_edges[*member].contains(member));
            if loops {
                for &member in component {
                    unit_cyclic[member] = true;
                }
            }
        }
        let context = Context {
            components: &components,
            component_of: &component_of,
            cyclic: &cyclic,
            mapless: &mapless,
            unit_cyclic: &unit_cyclic,
        };
        self.turns().into_iter().find_map(|turn| match turn {
            Turn::Local(index) => self.local_turn(index, &context),
            Turn::Imported(name) => self.imported_turn(&name, &context),
        })
    }

    /// The first local unit, in source order, whose dimension has a normalized
    /// map carrying `base`.
    fn carrier(&self, base: &BaseRef, context: &Context<'_>) -> Option<usize> {
        (0..self.units.len()).find(|&unit| {
            let dimension = self.unit_dimension[unit];
            !context.mapless[dimension] && self.maps[dimension].contains_key(base)
        })
    }

    fn imported_turn(&self, name: &str, context: &Context<'_>) -> Option<AssemblyError> {
        let dimension = self.imported.dimensions.get(name)?;
        let base = dimension.map.len() == 1 && dimension.map.contains_key(&dimension.key);
        if !base || dimension.has_unit {
            return None;
        }
        let unit = self.carrier(&BaseRef::Imported(dimension.key.clone()), context)?;
        Some(error(
            AssemblyCause::UnanchoredBaseDimension {
                dimension: dimension.identity.clone(),
                unit: self.unit_names[unit].to_owned(),
            },
            dimension.selection,
        ))
    }

    fn local_turn(&self, index: usize, context: &Context<'_>) -> Option<AssemblyError> {
        let form = &self.dimensions[index];
        // Step (-1): the dimension's own terms.
        for term in &form.terms {
            if self.named(&term.name.name).is_none() {
                return Some(error(
                    AssemblyCause::UnresolvedQuantityName {
                        name: term.name.name.clone(),
                    },
                    term.name.span,
                ));
            }
        }
        if context.cyclic[index] {
            return Some(error(
                AssemblyCause::QuantityCycle {
                    edges: self.dimension_cycle(index, context),
                },
                self.dimension_spans[index],
            ));
        }
        let local: Vec<usize> = (0..self.units.len())
            .filter(|&unit| self.unit_dimension[unit] == index)
            .collect();
        // Step (0): a base dimension no unit anchors.
        if form.terms.is_empty() && local.is_empty() {
            let unit = self.carrier(&BaseRef::Source(index), context)?;
            return Some(error(
                AssemblyCause::UnanchoredBaseDimension {
                    dimension: self.dimension_names[index].to_owned(),
                    unit: self.unit_names[unit].to_owned(),
                },
                self.dimension_spans[index],
            ));
        }
        // Rows 1 to 3, unit by unit in source order.
        for &unit in &local {
            if self.scales[unit].is_zero() {
                return Some(self.topology(
                    TopologyFault::ZeroScale,
                    vec![self.unit_names[unit]],
                    self.unit_spans[unit],
                ));
            }
            if self.unknown_target[unit] {
                let target = self.units[unit].target.as_ref()?;
                return Some(error(
                    AssemblyCause::UnresolvedQuantityName {
                        name: target.name.clone(),
                    },
                    target.span,
                ));
            }
            if let Some(&target) = self.unit_edges[unit].first() {
                if self.unit_dimension[target] != index {
                    return Some(self.topology(
                        TopologyFault::CrossDimensionTarget,
                        vec![self.unit_names[unit], self.unit_names[target]],
                        self.unit_spans[unit],
                    ));
                }
            }
        }
        // Rows 4 and 5: the targetless units.
        let roots: Vec<usize> = local
            .iter()
            .copied()
            .filter(|&unit| self.units[unit].target.is_none())
            .collect();
        match roots.as_slice() {
            [] => {}
            [root] => {
                let one = Rational::from_integer(Integer::one());
                if self.scales[*root] != one || !self.offsets[*root].is_zero() {
                    return Some(self.topology(
                        TopologyFault::NonIdentityRoot,
                        vec![self.unit_names[*root]],
                        self.unit_spans[*root],
                    ));
                }
            }
            [_, second, ..] => {
                let mut names = vec![self.dimension_names[index]];
                names.extend(roots.iter().map(|root| self.unit_names[*root]));
                return Some(self.topology(
                    TopologyFault::TwoRoots,
                    names,
                    self.unit_spans[*second],
                ));
            }
        }
        // Row 6: the cycle that holds the least cyclic unit.
        let least = local
            .iter()
            .copied()
            .find(|&unit| context.unit_cyclic[unit])?;
        let mut edges = Vec::new();
        let mut at = least;
        while let Some(&target) = self.unit_edges[at].first() {
            edges.push((
                self.unit_names[at].to_owned(),
                self.unit_names[target].to_owned(),
            ));
            at = target;
            if at == least {
                break;
            }
        }
        Some(error(
            AssemblyCause::QuantityCycle { edges },
            self.unit_spans[least],
        ))
    }

    /// The term edges of the cycle `member` belongs to, in order from the
    /// cycle's dimension declared first.
    fn dimension_cycle(&self, member: usize, context: &Context<'_>) -> Vec<(String, String)> {
        let component = &context.components[context.component_of[member]];
        let Some(&first) = component.iter().min() else {
            return Vec::new();
        };
        let mut edges = Vec::new();
        let mut seen: BTreeSet<(usize, usize)> = BTreeSet::new();
        let mut visited = BTreeSet::from([first]);
        let mut stack = vec![(first, 0_usize)];
        while let Some((node, next)) = stack.pop() {
            let Some(&target) = self.dimension_edges[node].get(next) else {
                continue;
            };
            stack.push((node, next + 1));
            if context.component_of[target] != context.component_of[member] {
                continue;
            }
            if seen.insert((node, target)) {
                edges.push((
                    self.dimension_names[node].to_owned(),
                    self.dimension_names[target].to_owned(),
                ));
            }
            if visited.insert(target) {
                stack.push((target, 0));
            }
        }
        edges
    }
}

/// What the dimension graph and the unit graph decide before the walk.
struct Context<'a> {
    components: &'a [Vec<usize>],
    component_of: &'a [usize],
    cyclic: &'a [bool],
    mapless: &'a [bool],
    unit_cyclic: &'a [bool],
}

/// The dimensions and units of one unit, admitted (FR-091 "Dimension and
/// unit declarations"), or every error found.
pub(super) fn assemble(
    dimensions: &[DimensionForm],
    units: &[UnitForm],
    imported: &Imported,
    owner: &SourceOwner,
    sources: &SourceDocuments<'_>,
    package_name: &str,
    limits: AssemblyLimits,
) -> Result<Assembled, Refused> {
    let owners = sources.owners;
    let source_order = sources.order;
    if dimensions.is_empty() && units.is_empty() {
        return Ok(Assembled {
            graph: UnitGraph::default(),
            spans: BTreeMap::new(),
            dimensions: BTreeMap::new(),
            units: BTreeMap::new(),
        });
    }
    let mut errors = Vec::new();
    let dimension_names: Vec<&str> = dimensions.iter().map(|d| d.name.name.as_str()).collect();
    let dimension_spans: Vec<Span> = dimensions.iter().map(|d| d.name.span).collect();
    let unit_names: Vec<&str> = units.iter().map(|u| u.name.name.as_str()).collect();
    let unit_spans: Vec<Span> = units.iter().map(|u| u.name.span).collect();
    let dimension_index = index_names(
        dimension_names
            .iter()
            .copied()
            .zip(dimension_spans.iter().copied()),
        &mut errors,
    );
    let unit_index = index_names(
        unit_names.iter().copied().zip(unit_spans.iter().copied()),
        &mut errors,
    );

    // The errors that come before the unit-graph topology (FR-091-AC-34):
    // a duplicate name, a unit typed by no dimension, and an exact number
    // that is not a value. A dimension term or a unit target that names
    // nothing is a topology defect, found at its node's turn.
    let unresolved = |name: &str, span: Span| {
        error(
            AssemblyCause::UnresolvedQuantityName {
                name: name.to_owned(),
            },
            span,
        )
    };
    let mut dimension_edges = vec![Vec::new(); dimensions.len()];
    let mut unknown_term = vec![false; dimensions.len()];
    for (index, form) in dimensions.iter().enumerate() {
        for term in &form.terms {
            match dimension_index.get(term.name.name.as_str()) {
                Some(&named) => dimension_edges[index].push(named),
                None if imported.dimensions.contains_key(term.name.name.as_str()) => {}
                None => unknown_term[index] = true,
            }
        }
    }
    let mut unit_dimension = Vec::with_capacity(units.len());
    let mut unit_edges = vec![Vec::new(); units.len()];
    let mut unknown_target = vec![false; units.len()];
    for (index, form) in units.iter().enumerate() {
        let dimension = dimension_index.get(form.dimension.name.as_str()).copied();
        if dimension.is_none() {
            errors.push(unresolved(&form.dimension.name, form.dimension.span));
        }
        unit_dimension.push(dimension);
        if let Some(target) = &form.target {
            match unit_index.get(target.name.as_str()) {
                Some(&named) => unit_edges[index].push(named),
                None => unknown_target[index] = true,
            }
        }
    }

    let mut scales = Vec::with_capacity(units.len());
    let mut offsets = Vec::with_capacity(units.len());
    for form in units {
        let mut kept = |value: Result<Rational, AssemblyError>| match value {
            Ok(value) => Some(value),
            Err(refusal) => {
                errors.push(refusal);
                None
            }
        };
        scales.push(kept(reduce(&form.scale, limits)));
        offsets.push(match &form.offset {
            Some(offset) => kept(reduce(offset, limits)),
            None => Some(Rational::from_integer(Integer::zero())),
        });
    }
    let fixed = provisional(
        dimensions,
        units,
        &unit_dimension,
        &unit_edges,
        &unknown_target,
        &scales,
        &offsets,
        owner,
        owners,
        package_name,
    );
    if !errors.is_empty() {
        return Err(Refused {
            errors,
            topology: false,
            provisional: fixed,
        });
    }
    let scales: Vec<Rational> = scales.into_iter().flatten().collect();
    let offsets: Vec<Rational> = offsets.into_iter().flatten().collect();
    let unit_dimension: Vec<usize> = unit_dimension.into_iter().flatten().collect();

    // Unit-graph topology (FR-091-AC-35): FR-142's turn list, the first
    // defect of the first node that has one, and no other.
    let maps = normalized(dimensions, &dimension_index, imported, &dimension_edges);
    let document = |kind: QuantityKind, name: &str| {
        owners
            .get(&quantity_origin(kind, name))
            .unwrap_or(owner)
            .clone()
    };
    let dimension_owners: Vec<SourceOwner> = dimension_names
        .iter()
        .map(|name| document(QuantityKind::Dimension, name))
        .collect();
    let unit_owners: Vec<SourceOwner> = unit_names
        .iter()
        .map(|name| document(QuantityKind::Unit, name))
        .collect();
    let rank = |found: &SourceOwner| {
        source_order
            .iter()
            .position(|source| source == found)
            .unwrap_or(0)
    };
    let dimension_documents: Vec<usize> = dimension_owners.iter().map(rank).collect();
    let unit_documents: Vec<usize> = unit_owners.iter().map(rank).collect();
    let walk = Walk {
        dimensions,
        units,
        dimension_names: &dimension_names,
        dimension_spans: &dimension_spans,
        dimension_documents: &dimension_documents,
        unit_documents: &unit_documents,
        unit_names: &unit_names,
        unit_spans: &unit_spans,
        dimension_index: &dimension_index,
        dimension_edges: &dimension_edges,
        unknown_term: &unknown_term,
        unit_dimension: &unit_dimension,
        unit_edges: &unit_edges,
        unknown_target: &unknown_target,
        scales: &scales,
        offsets: &offsets,
        imported,
        maps: &maps,
    };
    if let Some(defect) = walk.first_defect() {
        return Err(Refused {
            errors: vec![defect],
            topology: true,
            provisional: fixed,
        });
    }
    let unit_dimension: Vec<Option<usize>> = unit_dimension.into_iter().map(Some).collect();

    // Keys: each base dimension, then each derived one, then each unit
    // after its target.
    let first_span = dimension_spans
        .iter()
        .chain(&unit_spans)
        .min_by_key(|span| span.start)
        .copied()
        .unwrap_or(Span { start: 0, end: 0 });
    let fault = |refusal: NominalRefusal| Refused {
        errors: vec![error(AssemblyCause::nominal(refusal), first_span)],
        topology: false,
        provisional: Provisional::default(),
    };
    let node_owners: BTreeSet<_> = owners
        .values()
        .map(|owner| owner.node_owner(package_name))
        .chain(std::iter::once(owner.node_owner(package_name)))
        .collect();
    let mut dimension_keys: Vec<Option<NodeKey>> = vec![None; dimensions.len()];
    let mut dimension_nodes = Vec::with_capacity(dimensions.len());
    for derived in [false, true] {
        for (index, form) in dimensions.iter().enumerate() {
            if form.terms.is_empty() == derived {
                continue;
            }
            let mut terms = Vec::with_capacity(maps[index].len());
            if derived {
                for (base, exponent) in &maps[index] {
                    let key = match base {
                        BaseRef::Source(base) => dimension_keys[*base].clone(),
                        BaseRef::Imported(key) => Some(key.clone()),
                    };
                    if let Some(key) = key {
                        terms.push((key, exponent.clone()));
                    }
                }
                terms.sort();
            }
            let node_owner = owners
                .get(&form.name.name)
                .unwrap_or(owner)
                .node_owner(package_name);
            let preimage = DimensionPreimage::new(node_owner, vec![form.name.name.clone()], terms)
                .map_err(|refusal| fault(refusal.into()))?;
            let key = preimage
                .key()
                .map_err(|refusal| fault(super::nominal_key_refusal(refusal)))?;
            dimension_keys[index] = Some(key.clone());
            dimension_nodes.push((preimage, key));
        }
    }
    let mut unit_keys: Vec<Option<NodeKey>> = vec![None; units.len()];
    let mut composed: Vec<Option<ComposedUnit>> = vec![None; units.len()];
    let mut unit_nodes = Vec::with_capacity(units.len());
    for component in strongly_connected(&unit_edges) {
        for index in component {
            let dimension = unit_dimension[index]
                .and_then(|dimension| dimension_keys[dimension].clone())
                .ok_or_else(|| {
                    fault(
                        InvalidSemanticGraph {
                            cause: SemanticGraphCause::UnknownDimension,
                        }
                        .into(),
                    )
                })?;
            let target = unit_edges[index]
                .first()
                .and_then(|target| unit_keys[*target].clone());
            let node_owner = owners
                .get(&units[index].name.name)
                .unwrap_or(owner)
                .node_owner(package_name);
            let preimage = UnitPreimage::new(
                node_owner,
                vec![units[index].name.name.clone()],
                dimension.clone(),
                target,
                &scales[index],
                &offsets[index],
            )
            .map_err(|refusal| fault(refusal.into()))?;
            let key = preimage
                .key()
                .map_err(|refusal| fault(super::nominal_key_refusal(refusal)))?;
            unit_keys[index] = Some(key.clone());
            // `root = s_t × (s × v + o) + o_t` along the target's edge.
            composed[index] = Some(match unit_edges[index].first() {
                Some(target) => {
                    let to = composed[*target].clone().ok_or_else(|| {
                        fault(
                            InvalidSemanticGraph {
                                cause: SemanticGraphCause::UnknownDimension,
                            }
                            .into(),
                        )
                    })?;
                    ComposedUnit {
                        dimension: dimension.clone(),
                        scale: to.scale.mul(&scales[index]),
                        offset: to.scale.mul(&offsets[index]).add(&to.offset),
                    }
                }
                None => ComposedUnit {
                    dimension: dimension.clone(),
                    scale: scales[index].clone(),
                    offset: offsets[index].clone(),
                },
            });
            unit_nodes.push((preimage, key));
        }
    }
    // The imported base dimensions a derived dimension names join the graph
    // as admitted nodes.
    let named_bases: BTreeSet<NodeKey> = maps
        .iter()
        .flat_map(BTreeMap::keys)
        .filter_map(|base| match base {
            BaseRef::Imported(key) => Some(key.clone()),
            BaseRef::Source(_) => None,
        })
        .collect();
    for key in named_bases {
        if let Some(preimage) = imported.bases.get(&key) {
            dimension_nodes.push((preimage.clone(), key));
        }
    }
    let owners = OwnerSelection::new(
        node_owners
            .into_iter()
            .chain(imported.owners.iter().cloned()),
    );
    let graph = admit_unit_graph(dimension_nodes, unit_nodes, &owners).map_err(fault)?;
    let mut spans = BTreeMap::new();
    for (key, span) in dimension_keys.iter().zip(&dimension_spans) {
        if let Some(key) = key {
            spans.insert(key.clone(), *span);
        }
    }
    for (key, span) in unit_keys.iter().zip(&unit_spans) {
        if let Some(key) = key {
            spans.insert(key.clone(), *span);
        }
    }
    let mut named_dimensions = BTreeMap::new();
    for (index, key) in dimension_keys.iter().enumerate() {
        let Some(key) = key else { continue };
        let map = maps[index]
            .iter()
            .filter_map(|(base, exponent)| {
                let key = match base {
                    BaseRef::Source(base) => dimension_keys[*base].clone()?,
                    BaseRef::Imported(key) => key.clone(),
                };
                Some((key, exponent.clone()))
            })
            .collect();
        named_dimensions
            .entry(dimension_names[index].to_owned())
            .or_insert((key.clone(), map));
    }
    let mut named_units = BTreeMap::new();
    for (index, unit) in composed.into_iter().enumerate() {
        if let Some(unit) = unit {
            named_units
                .entry(unit_names[index].to_owned())
                .or_insert(unit);
        }
    }
    Ok(Assembled {
        graph,
        spans,
        dimensions: named_dimensions,
        units: named_units,
    })
}
