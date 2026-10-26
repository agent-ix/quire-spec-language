// SPDX-License-Identifier: AGPL-3.0-or-later
//! The assembler's dimension equivalences (FR-091 "Unit-graph topology",
//! QSpec FR-142 "Dimension equivalence"): each `dimension A ~ B by x U =
//! y V;` is decided by FR-142's steps 1 to 7, the first match winning, base
//! pairs (steps 1 to 5, then step 7) before every other pair (steps 1 to
//! 6), and the admitted ones are keyed over their
//! `quire.dimension-equivalence-node/v1` preimages. [`EquivalenceClasses`]
//! derives the classes and each member's root ratio to its class root, which
//! the checker converts time quantities through (FR-230).

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use qsl_forms::{DimensionEquivalenceForm, NameForm, RatioNumberForm, RatioNumberKind};
use qsl_foundation::diagnostic::LimitExceeded;
use qsl_foundation::{Setting, Span};
use quire_exact::{Integer, NodeKey, Rational};

use super::units::{Assembled, ComposedUnit};
use super::{AssemblyCause, AssemblyError, AssemblyLimits};
use crate::check::lowering::AdmittedModel;
use crate::check::node_key::{nominal_key, SourceOwner};
use crate::check::time_units::package_unit_scale;
use crate::value::semantic_node::{ModelSubject, NodeOwner};
use crate::value::unit::{DimensionEquivalencePreimage, DimensionPreimage};

/// Why a dimension equivalence was refused past name resolution (QSpec
/// FR-142 "Dimension equivalence" steps 3 to 7).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EquivalenceFault {
    /// Step 3, a pair another declaration already names, or step 7, a
    /// cycle of base pairs whose ratios do not multiply to one:
    /// `invalid_package`/`conflicting-definition`.
    Conflicting,
    /// Step 4, a unit not of the dimension it is named for, or step 6, two
    /// derived dimensions whose maps or root ratio disagree:
    /// `invalid_package`/`incompatible-definition`.
    Incompatible,
    /// Step 5, an affine unit or a ratio number that is not positive:
    /// `invalid_package`/`invalid-value`.
    InvalidValue,
}

/// One admitted dimension equivalence.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DimensionEquivalence {
    /// The node key over its preimage.
    pub key: NodeKey,
    /// The preimage's RFC 8785 bytes.
    pub preimage: Vec<u8>,
    /// The two related dimension nodes, in ascending digest order.
    pub dimensions: [NodeKey; 2],
    /// One canonical root of `dimensions[0]` in canonical roots of
    /// `dimensions[1]`.
    pub root_scale: Rational,
    /// The declaration's span.
    pub span: Span,
}

/// The equivalence classes of the admitted pairs (QSpec FR-142 "Effect"):
/// each member's class root (the least dimension key of its class) and the
/// size of one of its canonical roots in canonical roots of the class root.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct EquivalenceClasses {
    members: BTreeMap<NodeKey, (NodeKey, Rational)>,
}

impl EquivalenceClasses {
    /// The classes `equivalences` derive.
    pub fn new(equivalences: &[DimensionEquivalence]) -> Self {
        let pairs: Vec<_> = equivalences
            .iter()
            .map(|equivalence| {
                let [first, second] = equivalence.dimensions;
                (first, second, equivalence.root_scale.clone())
            })
            .collect();
        Self {
            members: sizes(&pairs),
        }
    }

    /// `dimension`'s class root and the size of its canonical root in
    /// canonical roots of the class root; a dimension in no pair is its own
    /// class with size one.
    pub fn class(&self, dimension: NodeKey) -> (NodeKey, Rational) {
        self.members
            .get(&dimension)
            .cloned()
            .unwrap_or_else(|| (dimension, Rational::from_integer(Integer::one())))
    }
}

/// Each node of `pairs` (`root(a) = r × root(b)`) with its component's least
/// key and its root's size in roots of that key.
fn sizes(pairs: &[(NodeKey, NodeKey, Rational)]) -> BTreeMap<NodeKey, (NodeKey, Rational)> {
    let mut adjacent: BTreeMap<NodeKey, Vec<(NodeKey, Rational)>> = BTreeMap::new();
    for (first, second, ratio) in pairs {
        adjacent
            .entry(*first)
            .or_default()
            .push((*second, ratio.clone()));
        if let Some(inverse) = Rational::from_integer(Integer::one()).div(ratio) {
            adjacent.entry(*second).or_default().push((*first, inverse));
        }
    }
    let mut members = BTreeMap::new();
    // Ascending keys: the first unvisited node is its component's least key.
    for root in adjacent.keys() {
        if members.contains_key(root) {
            continue;
        }
        members.insert(*root, (*root, Rational::from_integer(Integer::one())));
        let mut pending = VecDeque::from([*root]);
        while let Some(node) = pending.pop_front() {
            let Some((_, size)) = members.get(&node).cloned() else {
                continue;
            };
            for (next, ratio) in adjacent.get(&node).into_iter().flatten() {
                if members.contains_key(next) {
                    continue;
                }
                // root(node) = ratio × root(next), so root(next) is
                // size / ratio class roots.
                if let Some(next_size) = size.div(ratio) {
                    members.insert(*next, (*root, next_size));
                    pending.push_back(*next);
                }
            }
        }
    }
    members
}

/// A resolved dimension: its key, its label as written, and its normalized
/// base map (a base dimension's is itself with exponent one).
struct Dimension {
    key: NodeKey,
    label: String,
    map: BTreeMap<NodeKey, Integer>,
}

impl Dimension {
    fn is_base(&self) -> bool {
        self.map.len() == 1 && self.map.get(&self.key) == Some(&Integer::one())
    }
}

/// What a dimension name resolves to.
enum Named {
    Dimension(Dimension),
    /// A derived dimension of a domain package, whose terms intake does not
    /// retain, so no key or base map can be formed for it.
    DerivedPackage {
        alias: String,
        node: String,
    },
}

/// The names one unit of compilation sees: its own dimensions and units
/// and those of each selected package by its model alias.
struct Scope<'a> {
    source: Option<&'a Assembled>,
    models: &'a [AdmittedModel],
    identity: qsl_foundation::IdentityLimits,
}

impl Scope<'_> {
    fn model(&self, alias: &str) -> Option<&AdmittedModel> {
        self.models
            .iter()
            .find(|model| model.alias() == Some(alias))
    }

    fn package_dimension_key(
        &self,
        model: &AdmittedModel,
        dimension: &crate::model::domain_package::PackageDimensionRecord,
    ) -> Option<NodeKey> {
        let owner = NodeOwner::Model(ModelSubject {
            identity: model.selection().identity.clone(),
            node: dimension.key.node.clone(),
        });
        let preimage = DimensionPreimage::new(owner, vec![dimension.name.clone()], vec![]).ok()?;
        nominal_key(&preimage, self.identity).ok()
    }

    #[ix_trace_rs::string_edge]
    fn dimension(&self, name: &NameForm) -> Option<Named> {
        match name.name.split_once("::") {
            None => {
                let (key, map) = self.source?.dimensions.get(&name.name)?;
                Some(Named::Dimension(Dimension {
                    key: *key,
                    label: name.name.clone(),
                    map: map.clone(),
                }))
            }
            Some((alias, rest)) => {
                let model = self.model(alias)?;
                let dimension = model.dimensions.iter().find(|d| d.name == rest)?;
                if !dimension.base {
                    return Some(Named::DerivedPackage {
                        alias: alias.to_owned(),
                        node: dimension.key.node.clone(),
                    });
                }
                let key = self.package_dimension_key(model, dimension)?;
                Some(Named::Dimension(Dimension {
                    key,
                    label: name.name.clone(),
                    map: BTreeMap::from([(key, Integer::one())]),
                }))
            }
        }
    }

    /// A unit by name: `Some(None)` for a package unit whose dimension is a
    /// derived package dimension or whose target chain fixes no scale.
    #[ix_trace_rs::string_edge]
    fn unit(&self, name: &str) -> Option<Option<ComposedUnit>> {
        match name.split_once("::") {
            None => Some(Some(self.source?.units.get(name)?.clone())),
            Some((alias, rest)) => {
                let model = self.model(alias)?;
                let unit = model.units.iter().find(|unit| unit.name == rest)?;
                let dimension = model
                    .dimensions
                    .iter()
                    .find(|dimension| dimension.key == unit.dimension)
                    .filter(|dimension| dimension.base)
                    .and_then(|dimension| self.package_dimension_key(model, dimension));
                Some(dimension.zip(package_unit_scale(unit, &model.units)).map(
                    |(dimension, (scale, offset))| ComposedUnit {
                        dimension,
                        scale,
                        offset,
                    },
                ))
            }
        }
    }
}

/// A declaration past steps 1 and 2.
struct Resolved<'f> {
    form: &'f DimensionEquivalenceForm,
    dimensions: [Dimension; 2],
    units: [Option<ComposedUnit>; 2],
}

impl Resolved<'_> {
    fn label(&self) -> String {
        format!(
            "{} ~ {}",
            self.dimensions[0].label, self.dimensions[1].label
        )
    }

    fn pair(&self) -> BTreeSet<NodeKey> {
        BTreeSet::from([self.dimensions[0].key, self.dimensions[1].key])
    }
}

fn error(cause: AssemblyCause, span: Span) -> AssemblyError {
    AssemblyError { cause, span }
}

fn refused(fault: EquivalenceFault, retained: Vec<String>, span: Span) -> AssemblyError {
    error(
        AssemblyCause::DimensionEquivalence { fault, retained },
        span,
    )
}

/// A ratio number's exact value, or the error it is.
fn ratio_number(
    number: &RatioNumberForm,
    limits: AssemblyLimits,
) -> Result<Rational, AssemblyError> {
    let denominator = match number.kind {
        RatioNumberKind::Integer | RatioNumberKind::Rational => number.second.clone(),
        RatioNumberKind::Decimal => {
            match number
                .second
                .to_u64()
                .filter(|scale| *scale <= limits.decimal_scale)
            {
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

fn positive(value: &Rational) -> bool {
    !value.is_zero() && !value.numerator().is_negative()
}

/// The unit's admitted dimension equivalences, or every error found.
pub(super) fn assemble(
    forms: &[DimensionEquivalenceForm],
    source: Option<&Assembled>,
    models: &[AdmittedModel],
    owner: &SourceOwner,
    limits: AssemblyLimits,
) -> Result<Vec<DimensionEquivalence>, Vec<AssemblyError>> {
    let scope = Scope {
        source,
        models,
        identity: limits.identity,
    };
    let mut errors = Vec::new();
    // Steps 1 and 2, in source order.
    let mut base = Vec::new();
    let mut derived = Vec::new();
    'forms: for form in forms {
        let mut dimensions = Vec::with_capacity(2);
        for name in &form.dimensions {
            match scope.dimension(name) {
                Some(Named::Dimension(dimension)) => dimensions.push(dimension),
                Some(Named::DerivedPackage { alias, node }) => {
                    errors.push(error(
                        AssemblyCause::UnsupportedModelMember { alias, node },
                        name.span,
                    ));
                    continue 'forms;
                }
                None => {
                    errors.push(error(
                        AssemblyCause::UnresolvedQuantityName {
                            name: name.name.clone(),
                        },
                        name.span,
                    ));
                    continue 'forms;
                }
            }
        }
        let mut units = Vec::with_capacity(2);
        for term in &form.ratio {
            match scope.unit(&term.unit.name) {
                Some(unit) => units.push(unit),
                None => {
                    errors.push(error(
                        AssemblyCause::UnresolvedQuantityName {
                            name: term.unit.name.clone(),
                        },
                        term.unit.span,
                    ));
                    continue 'forms;
                }
            }
        }
        let (Ok(dimensions), Ok(units)) = (
            <[Dimension; 2]>::try_from(dimensions),
            <[Option<ComposedUnit>; 2]>::try_from(units),
        ) else {
            continue;
        };
        if dimensions[0].key == dimensions[1].key {
            let label = dimensions[0].label.clone();
            errors.push(error(
                AssemblyCause::QuantityCycle {
                    edges: vec![(label.clone(), label)],
                },
                form.span,
            ));
            continue;
        }
        let resolved = Resolved {
            form,
            dimensions,
            units,
        };
        if resolved.dimensions.iter().all(Dimension::is_base) {
            base.push(resolved);
        } else {
            derived.push(resolved);
        }
    }

    // Phase one: base pairs, steps 3 to 5 in source order, then step 7.
    let mut admitted: Vec<(&Resolved<'_>, Rational)> = Vec::new();
    let mut named: Vec<&Resolved<'_>> = Vec::new();
    for resolved in &base {
        if let Some(ratio) = steps_three_to_five(resolved, &named, limits, &mut errors) {
            admitted.push((resolved, ratio));
        }
        named.push(resolved);
    }
    let mut consistent: Vec<(&Resolved<'_>, Rational)> = Vec::new();
    for (resolved, ratio) in admitted {
        let [first, second] = [resolved.dimensions[0].key, resolved.dimensions[1].key];
        match path(&consistent, first, second) {
            Some((implied, cycle)) if implied != ratio => {
                let mut retained: Vec<_> = cycle
                    .into_iter()
                    .map(|index| consistent[index].0)
                    .chain([resolved])
                    .collect();
                retained.sort_by_key(|declaration| declaration.form.span.start);
                errors.push(refused(
                    EquivalenceFault::Conflicting,
                    retained
                        .iter()
                        .map(|declaration| declaration.label())
                        .collect(),
                    resolved.form.span,
                ));
            }
            Some(_) | None => consistent.push((resolved, ratio)),
        }
    }

    // The base classes: each base dimension's representative and root size.
    let classes = sizes(
        &consistent
            .iter()
            .map(|(resolved, ratio)| {
                (
                    resolved.dimensions[0].key,
                    resolved.dimensions[1].key,
                    ratio.clone(),
                )
            })
            .collect::<Vec<_>>(),
    );
    let class_of = |base: NodeKey| {
        classes
            .get(&base)
            .cloned()
            .unwrap_or_else(|| (base, Rational::from_integer(Integer::one())))
    };
    // A dimension's map over the class representatives, and the size its
    // root has against the product of those representatives' roots.
    let over_classes = |dimension: &Dimension| {
        let mut map: BTreeMap<NodeKey, Integer> = BTreeMap::new();
        let mut size = Rational::from_integer(Integer::one());
        for (base, exponent) in &dimension.map {
            let (representative, ratio) = class_of(*base);
            let entry = map.entry(representative).or_insert_with(Integer::zero);
            *entry = entry.add(exponent);
            if let Some(power) = ratio.pow(exponent) {
                size = size.mul(&power);
            }
        }
        map.retain(|_, exponent| !exponent.is_zero());
        (map, size)
    };

    // Phase two: every other pair, steps 3 to 6 in source order.
    let mut kept: Vec<(&Resolved<'_>, Rational)> = consistent;
    for resolved in &derived {
        let Some(ratio) = steps_three_to_five(resolved, &named, limits, &mut errors) else {
            named.push(resolved);
            continue;
        };
        named.push(resolved);
        let (first_map, first_size) = over_classes(&resolved.dimensions[0]);
        let (second_map, second_size) = over_classes(&resolved.dimensions[1]);
        let fixed = first_size.div(&second_size);
        if first_map != second_map || fixed.as_ref() != Some(&ratio) {
            errors.push(refused(
                EquivalenceFault::Incompatible,
                vec![
                    resolved.dimensions[0].label.clone(),
                    resolved.dimensions[1].label.clone(),
                ],
                resolved.form.span,
            ));
            continue;
        }
        kept.push((resolved, ratio));
    }
    if !errors.is_empty() {
        return Err(errors);
    }

    // Keys, in source order.
    kept.sort_by_key(|(resolved, _)| resolved.form.span.start);
    let node_owner = owner.node_owner();
    let mut equivalences = Vec::with_capacity(kept.len());
    for (resolved, ratio) in kept {
        let fault = |cause| vec![error(cause, resolved.form.span)];
        let preimage = DimensionEquivalencePreimage::new(
            node_owner.clone(),
            resolved.dimensions[0].key,
            resolved.dimensions[1].key,
            &ratio,
        )
        .ok_or_else(|| {
            fault(AssemblyCause::nominal(
                quire_semantic_value::semantic_node::InvalidSemanticGraph {
                    cause:
                        quire_semantic_value::semantic_node::SemanticGraphCause::NonCanonicalPreimage,
                }
                .into(),
            ))
        })?;
        let key = nominal_key(&preimage, limits.identity)
            .map_err(|refusal| fault(AssemblyCause::nominal(refusal)))?;
        let bytes = preimage
            .preimage_bytes(limits.identity)
            .map_err(|refusal| fault(AssemblyCause::nominal(refusal)))?;
        equivalences.push(DimensionEquivalence {
            key,
            preimage: bytes,
            dimensions: preimage.dimensions(),
            root_scale: preimage.root_scale().clone(),
            span: resolved.form.span,
        });
    }
    Ok(equivalences)
}

/// Steps 3 to 5 of one declaration: its root ratio, or `None` with the
/// refusal pushed (or, for a unit whose chain fixes no scale, nothing to
/// decide past step 4).
fn steps_three_to_five(
    resolved: &Resolved<'_>,
    named: &[&Resolved<'_>],
    limits: AssemblyLimits,
    errors: &mut Vec<AssemblyError>,
) -> Option<Rational> {
    let span = resolved.form.span;
    if let Some(earlier) = named
        .iter()
        .filter(|earlier| earlier.form.span.start < span.start)
        .find(|earlier| earlier.pair() == resolved.pair())
    {
        errors.push(refused(
            EquivalenceFault::Conflicting,
            vec![earlier.label(), resolved.label()],
            span,
        ));
        return None;
    }
    let mut scales = Vec::with_capacity(2);
    for ((term, unit), dimension) in resolved
        .form
        .ratio
        .iter()
        .zip(&resolved.units)
        .zip(&resolved.dimensions)
    {
        let unit = unit.as_ref()?;
        if unit.dimension != dimension.key {
            errors.push(refused(
                EquivalenceFault::Incompatible,
                vec![term.unit.name.clone(), dimension.label.clone()],
                span,
            ));
            return None;
        }
        scales.push(unit);
    }
    for (term, unit) in resolved.form.ratio.iter().zip(&scales) {
        if !unit.offset.is_zero() {
            errors.push(refused(
                EquivalenceFault::InvalidValue,
                vec![term.unit.name.clone(), unit.offset.to_string()],
                span,
            ));
            return None;
        }
    }
    let mut numbers = Vec::with_capacity(2);
    for term in &resolved.form.ratio {
        match ratio_number(&term.number, limits) {
            Ok(number) if positive(&number) => numbers.push(number),
            Ok(_) => {
                errors.push(refused(
                    EquivalenceFault::InvalidValue,
                    vec![resolved.label()],
                    span,
                ));
                return None;
            }
            Err(refusal) => {
                errors.push(refusal);
                return None;
            }
        }
    }
    // `x U = y V`, `U` being `a` roots of the first and `V` `b` roots of the
    // second: one root of the first is `(y × b) / (x × a)` of the second.
    let ([x, y], [a, b]) = (numbers.as_slice(), scales.as_slice()) else {
        return None;
    };
    y.mul(&b.scale).div(&x.mul(&a.scale))
}

/// The root ratio `pairs` imply between `from` and `to` (`root(from) = r ×
/// root(to)`) and the indexes of the pairs on that path, when they are
/// connected.
fn path(
    pairs: &[(&Resolved<'_>, Rational)],
    from: NodeKey,
    to: NodeKey,
) -> Option<(Rational, Vec<usize>)> {
    let one = Rational::from_integer(Integer::one());
    let mut reached: BTreeMap<NodeKey, (Rational, Vec<usize>)> =
        BTreeMap::from([(from, (one, Vec::new()))]);
    let mut pending = VecDeque::from([from]);
    while let Some(node) = pending.pop_front() {
        let Some((size, via)) = reached.get(&node).cloned() else {
            continue;
        };
        for (index, (resolved, ratio)) in pairs.iter().enumerate() {
            let [first, second] = [resolved.dimensions[0].key, resolved.dimensions[1].key];
            // root(from) = size × root(node); root(first) = ratio × root(second).
            let step = if first == node {
                Some((second, size.mul(ratio)))
            } else if second == node {
                size.div(ratio).map(|next| (first, next))
            } else {
                None
            };
            let Some((next, next_size)) = step else {
                continue;
            };
            if reached.contains_key(&next) {
                continue;
            }
            let mut next_via = via.clone();
            next_via.push(index);
            reached.insert(next, (next_size, next_via));
            pending.push_back(next);
        }
    }
    reached.remove(&to)
}
