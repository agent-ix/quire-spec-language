// SPDX-License-Identifier: AGPL-3.0-or-later
//! The assembler's dimension and unit declarations (FR-091 "Dimension and
//! unit declarations"): every source error is reported, the unit-graph
//! topology checks run only when no other check found an error, and each
//! dimension and unit is then keyed with `nominal_key` and admitted through
//! `UnitGraph::admit`. The assembler stops at the admitted graph, which
//! keeps each node's preimage bytes; lowering builds the unit and dimension
//! nodes from them (FR-094).

use std::collections::BTreeMap;

use qsl_forms::{DimensionForm, ExactNumberForm, ExactNumberKind, TermOperator, UnitForm};
use qsl_foundation::diagnostic::LimitExceeded;
use qsl_foundation::{Setting, Span};
use quire_exact::{Integer, NodeKey, Rational};

use super::{AssemblyCause, AssemblyError, AssemblyLimits, TopologyFault};
use crate::check::lowering::strongly_connected;
use crate::check::node_key::{nominal_key, SourceOwner};
use crate::value::semantic_node::{NominalRefusal, OwnerSelection};
use crate::value::unit::{admit_unit_graph, DimensionPreimage, UnitPreimage};
use quire_semantic_value::semantic_node::{InvalidSemanticGraph, SemanticGraphCause};
use quire_semantic_value::unit::UnitGraph;

/// The admitted graph and the span of each declared name by its key.
pub(super) struct Assembled {
    pub(super) graph: UnitGraph,
    pub(super) spans: BTreeMap<NodeKey, Span>,
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

/// The edges of one graph as the components that reach themselves, each as
/// its dependency edges by name.
fn cycles(names: &[&str], spans: &[Span], edges: &[Vec<usize>], errors: &mut Vec<AssemblyError>) {
    for component in strongly_connected(edges) {
        let cyclic = component.len() > 1
            || component
                .first()
                .is_some_and(|member| edges[*member].contains(member));
        if !cyclic {
            continue;
        }
        let mut cycle = Vec::new();
        for &member in &component {
            for &target in &edges[member] {
                if component.contains(&target) {
                    cycle.push((names[member].to_owned(), names[target].to_owned()));
                }
            }
        }
        cycle.sort();
        cycle.dedup();
        let first = component.iter().min().copied().unwrap_or_default();
        errors.push(error(
            AssemblyCause::QuantityCycle { edges: cycle },
            spans[first],
        ));
    }
}

/// The base-dimension exponents of every dimension, indexed as `dimensions`,
/// once no cycle remains: a base dimension is itself with exponent one, a
/// derived dimension is its terms' maps, multiplied by their exponents
/// (negated after `/`) and added, with zero exponents dropped.
fn normalized(
    dimensions: &[DimensionForm],
    by_name: &BTreeMap<&str, usize>,
    edges: &[Vec<usize>],
) -> Vec<BTreeMap<usize, Integer>> {
    let mut maps: Vec<BTreeMap<usize, Integer>> = vec![BTreeMap::new(); dimensions.len()];
    for component in strongly_connected(edges) {
        for index in component {
            let form = &dimensions[index];
            if form.terms.is_empty() {
                maps[index].insert(index, Integer::one());
                continue;
            }
            let mut sum: BTreeMap<usize, Integer> = BTreeMap::new();
            for term in &form.terms {
                let Some(&named) = by_name.get(term.name.name.as_str()) else {
                    continue;
                };
                let exponent = term
                    .exponent
                    .as_ref()
                    .map_or_else(Integer::one, |(exponent, _)| exponent.clone());
                let exponent = match term.operator {
                    Some(TermOperator::Divide) => exponent.neg(),
                    Some(TermOperator::Multiply) | None => exponent,
                };
                for (base, power) in &maps[named] {
                    let entry = sum.entry(*base).or_insert_with(Integer::zero);
                    *entry = entry.add(&power.mul(&exponent));
                }
            }
            sum.retain(|_, exponent| !exponent.is_zero());
            maps[index] = sum;
        }
    }
    maps
}

/// The dimensions and units of one unit, admitted (FR-091 "Dimension and
/// unit declarations"), or every error found.
pub(super) fn assemble(
    dimensions: &[DimensionForm],
    units: &[UnitForm],
    owner: &SourceOwner,
    owners: &OwnerSelection,
    limits: AssemblyLimits,
) -> Result<Assembled, Vec<AssemblyError>> {
    if dimensions.is_empty() && units.is_empty() {
        return Ok(Assembled {
            graph: UnitGraph::default(),
            spans: BTreeMap::new(),
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

    // Names resolve in their own namespaces: dimension terms and a unit's
    // `:` name against dimensions, a unit's `*` target against units.
    let unresolved = |name: &str, span: Span| {
        error(
            AssemblyCause::UnresolvedQuantityName {
                name: name.to_owned(),
            },
            span,
        )
    };
    let mut dimension_edges = vec![Vec::new(); dimensions.len()];
    for (index, form) in dimensions.iter().enumerate() {
        for term in &form.terms {
            match dimension_index.get(term.name.name.as_str()) {
                Some(&named) => dimension_edges[index].push(named),
                None => errors.push(unresolved(&term.name.name, term.name.span)),
            }
        }
    }
    let mut unit_dimension = Vec::with_capacity(units.len());
    let mut unit_edges = vec![Vec::new(); units.len()];
    for (index, form) in units.iter().enumerate() {
        unit_dimension.push(dimension_index.get(form.dimension.name.as_str()).copied());
        if unit_dimension[index].is_none() {
            errors.push(unresolved(&form.dimension.name, form.dimension.span));
        }
        if let Some(target) = &form.target {
            match unit_index.get(target.name.as_str()) {
                Some(&named) => unit_edges[index].push(named),
                None => errors.push(unresolved(&target.name, target.span)),
            }
        }
    }
    cycles(
        &dimension_names,
        &dimension_spans,
        &dimension_edges,
        &mut errors,
    );
    cycles(&unit_names, &unit_spans, &unit_edges, &mut errors);

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
    if !errors.is_empty() {
        return Err(errors);
    }
    let scales: Vec<Rational> = scales.into_iter().flatten().collect();
    let offsets: Vec<Rational> = offsets.into_iter().flatten().collect();

    // Unit-graph topology (FR-091-AC-35), only now that no other check
    // found an error.
    let maps = normalized(dimensions, &dimension_index, &dimension_edges);
    let topology = |fault, declarations: Vec<&str>, span| {
        error(
            AssemblyCause::UnitGraphTopology {
                fault,
                declarations: declarations.into_iter().map(str::to_owned).collect(),
            },
            span,
        )
    };
    for (index, form) in dimensions.iter().enumerate() {
        if !form.terms.is_empty() && maps[index].is_empty() {
            errors.push(topology(
                TopologyFault::EmptyDerivedDimension,
                vec![dimension_names[index]],
                dimension_spans[index],
            ));
        }
    }
    let one = Rational::from_integer(Integer::one());
    let mut roots: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
    for (index, form) in units.iter().enumerate() {
        let dimension = unit_dimension[index].unwrap_or_default();
        if scales[index].is_zero() {
            errors.push(topology(
                TopologyFault::ZeroScale,
                vec![unit_names[index]],
                unit_spans[index],
            ));
        }
        match unit_edges[index].first() {
            None => {
                roots.entry(dimension).or_default().push(index);
                if scales[index] != one || !offsets[index].is_zero() {
                    errors.push(topology(
                        TopologyFault::NonIdentityRoot,
                        vec![unit_names[index]],
                        unit_spans[index],
                    ));
                }
            }
            Some(&target) => {
                if unit_dimension[target] != unit_dimension[index] {
                    errors.push(topology(
                        TopologyFault::CrossDimensionTarget,
                        vec![unit_names[index], unit_names[target]],
                        unit_spans[index],
                    ));
                }
            }
        }
        debug_assert!(form.target.is_some() == !unit_edges[index].is_empty());
    }
    for (dimension, roots) in roots {
        if roots.len() > 1 {
            let mut names = vec![dimension_names[dimension]];
            names.extend(roots.iter().map(|root| unit_names[*root]));
            errors.push(topology(
                TopologyFault::TwoRoots,
                names,
                unit_spans[roots[1]],
            ));
        }
    }
    if !errors.is_empty() {
        return Err(errors);
    }

    // Keys: each base dimension, then each derived one, then each unit
    // after its target.
    let first_span = dimension_spans
        .iter()
        .chain(&unit_spans)
        .min_by_key(|span| span.start)
        .copied()
        .unwrap_or(Span { start: 0, end: 0 });
    let fault = |refusal: NominalRefusal| vec![error(AssemblyCause::nominal(refusal), first_span)];
    let node_owner = owner.node_owner();
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
                    if let Some(key) = dimension_keys[*base] {
                        terms.push((key, exponent.clone()));
                    }
                }
                terms.sort();
            }
            let preimage =
                DimensionPreimage::new(node_owner.clone(), vec![form.name.name.clone()], terms)
                    .map_err(|refusal| fault(refusal.into()))?;
            let key = nominal_key(&preimage, limits.identity).map_err(fault)?;
            dimension_keys[index] = Some(key);
            dimension_nodes.push((preimage, key));
        }
    }
    let mut unit_keys: Vec<Option<NodeKey>> = vec![None; units.len()];
    let mut unit_nodes = Vec::with_capacity(units.len());
    for component in strongly_connected(&unit_edges) {
        for index in component {
            let dimension = unit_dimension[index]
                .and_then(|dimension| dimension_keys[dimension])
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
                .and_then(|target| unit_keys[*target]);
            let preimage = UnitPreimage::new(
                node_owner.clone(),
                vec![units[index].name.name.clone()],
                dimension,
                target,
                &scales[index],
                &offsets[index],
            )
            .map_err(|refusal| fault(refusal.into()))?;
            let key = nominal_key(&preimage, limits.identity).map_err(fault)?;
            unit_keys[index] = Some(key);
            unit_nodes.push((preimage, key));
        }
    }
    let graph =
        admit_unit_graph(dimension_nodes, unit_nodes, owners, limits.identity).map_err(fault)?;
    let mut spans = BTreeMap::new();
    for (key, span) in dimension_keys.iter().zip(&dimension_spans) {
        if let Some(key) = key {
            spans.insert(*key, *span);
        }
    }
    for (key, span) in unit_keys.iter().zip(&unit_spans) {
        if let Some(key) = key {
            spans.insert(*key, *span);
        }
    }
    Ok(Assembled { graph, spans })
}
