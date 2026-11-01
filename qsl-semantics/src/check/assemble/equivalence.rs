// SPDX-License-Identifier: AGPL-3.0-or-later
//! The assembler's dimension equivalences (FR-091 "Unit-graph topology",
//! QSpec FR-142 "Dimension equivalence"): each `dimension A ~ B by x U =
//! y V;` is decided by FR-142's steps 1 to 7, the first match winning, base
//! pairs (steps 1 to 5, then step 7) before every other pair (steps 1 to
//! 6), and the admitted ones have structural node keys. [`EquivalenceClasses`]
//! derives the classes and each member's root ratio to its class root, which
//! the checker converts time quantities through (FR-230).

use qsl_foundation::source::node_table::NodeKeyDebug;
use std::collections::{BTreeMap, BTreeSet, VecDeque};

use qsl_forms::{DimensionEquivalenceForm, NameForm, RatioNumberForm, RatioNumberKind};
use qsl_foundation::diagnostic::LimitExceeded;
use qsl_foundation::{Setting, Span};
use quire_exact::{Integer, NodeKey, PathSegment, Rational};

use super::units::{Assembled, Provisional};
use super::{AssemblyCause, AssemblyError, AssemblyLimits};
use crate::check::lowering::AdmittedModel;
use crate::check::node_key::{NodeTag, SourceOwner};
use crate::check::time_units::{package_unit_scale, scalar_type_key};
use crate::value::unit::DimensionEquivalencePreimage;
use quire_checked::key::DeclarationKey;

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

/// What a dimension-equivalence refusal retains (QSpec FR-142 "Dimension
/// equivalence"): the declarations, dimensions, units and values its step
/// names.
#[derive(Clone, Eq, PartialEq)]
pub enum EquivalenceRetained {
    /// An equivalence declaration: its two dimension names as written and
    /// its span.
    Declaration {
        /// The two dimension names, in written order.
        dimensions: [String; 2],
        /// The declaration's span.
        span: Span,
    },
    /// A dimension: its name as written and its node key, when fixed.
    Dimension {
        /// The name as written.
        name: String,
        /// The dimension node's key.
        key: Option<NodeKey>,
    },
    /// A unit, by its name as written.
    Unit {
        /// The name as written.
        name: String,
    },
    /// A value: a unit's composed offset or a ratio number.
    Value(Rational),
}

impl std::fmt::Debug for EquivalenceRetained {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Declaration { dimensions, span } => f
                .debug_struct("Declaration")
                .field("dimensions", &dimensions)
                .field("span", &span)
                .finish(),
            Self::Dimension { name, key } => f
                .debug_struct("Dimension")
                .field("name", &name)
                .field("key", &key.as_ref().map(NodeKeyDebug))
                .finish(),
            Self::Unit { name } => f.debug_struct("Unit").field("name", &name).finish(),
            Self::Value(value0) => f.debug_tuple("Value").field(&value0).finish(),
        }
    }
}

impl std::fmt::Display for EquivalenceRetained {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Declaration {
                dimensions: [first, second],
                ..
            } => write!(f, "{first} ~ {second}"),
            Self::Dimension { name, .. } | Self::Unit { name } => f.write_str(name),
            Self::Value(value) => write!(f, "{value}"),
        }
    }
}

/// One admitted dimension equivalence.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DimensionEquivalence {
    /// The structural node key.
    pub key: NodeKey,
    /// The two related dimension nodes, in ascending structural key order.
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
#[derive(Clone, Default, Eq, PartialEq)]
pub struct EquivalenceClasses {
    members: BTreeMap<NodeKey, (NodeKey, Rational)>,
}

impl std::fmt::Debug for EquivalenceClasses {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Self { members } = self;
        f.debug_struct("EquivalenceClasses")
            .field(
                "members",
                &members
                    .iter()
                    .map(|(key, value)| (NodeKeyDebug(key), (NodeKeyDebug(&value.0), &value.1)))
                    .collect::<Vec<_>>(),
            )
            .finish()
    }
}

impl EquivalenceClasses {
    /// The classes `equivalences` derive.
    pub fn new(equivalences: &[DimensionEquivalence]) -> Self {
        let pairs: Vec<_> = equivalences
            .iter()
            .map(|equivalence| {
                let [first, second] = equivalence.dimensions.clone();
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
            .entry(first.clone())
            .or_default()
            .push((second.clone(), ratio.clone()));
        if let Some(inverse) = Rational::from_integer(Integer::one()).div(ratio) {
            adjacent
                .entry(second.clone())
                .or_default()
                .push((first.clone(), inverse));
        }
    }
    let mut members = BTreeMap::new();
    // Ascending keys: the first unvisited node is its component's least key.
    for root in adjacent.keys() {
        if members.contains_key(root) {
            continue;
        }
        members.insert(
            root.clone(),
            (root.clone(), Rational::from_integer(Integer::one())),
        );
        let mut pending = VecDeque::from([root.clone()]);
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
                    members.insert(next.clone(), (root.clone(), next_size));
                    pending.push_back(next.clone());
                }
            }
        }
    }
    members
}

/// What a dimension name names: a dimension form of the unit, by its name,
/// or a dimension of an admitted domain package, by its FCD identity.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
enum DimensionRef {
    Source(String),
    Package(DeclarationKey),
}

/// A resolved dimension: what it names, its label as written, its key and
/// normalized base map when they are fixed (a base dimension's map is
/// itself with exponent one), and whether it is a base dimension.
#[derive(Clone)]
struct Dimension {
    id: DimensionRef,
    label: String,
    key: Option<NodeKey>,
    map: Option<BTreeMap<NodeKey, Integer>>,
    base: bool,
}

impl Dimension {
    /// The key the equivalence relates this dimension by: a model's base
    /// dimension is the `scalar_type` node its time units name, keyed by its
    /// declaration; any other dimension is keyed as resolved.
    fn node_key(&self) -> Option<NodeKey> {
        match &self.id {
            DimensionRef::Package(declaration) if self.base => {
                scalar_type_key(declaration, "dimension")
            }
            _ => self.key.clone(),
        }
    }

    fn retained(&self) -> EquivalenceRetained {
        EquivalenceRetained::Dimension {
            name: self.label.clone(),
            key: self.key.clone(),
        }
    }
}

/// A resolved unit: the dimension it belongs to and, when its target chain
/// fixes one, its composed scale and offset to that dimension's root.
#[derive(Clone)]
struct Unit {
    name: String,
    dimension: DimensionRef,
    composed: Option<(Rational, Rational)>,
    /// Whether the unit is a package's: a package unit's chain was admitted
    /// at intake, so one that fixes no scale is refused here rather than at
    /// a dimension's turn.
    package: bool,
}

/// What the unit's own dimension and unit declarations fix: provisionally,
/// before the topology is decided, or as the admitted graph holds them.
pub(super) enum SourceView<'a> {
    /// Before the first dimension turn: the base dimensions' keys and each
    /// unit's chain as far as it is sound.
    Provisional(&'a Provisional),
    /// After the last turn: the admitted graph's keys, maps and units.
    Admitted(&'a Assembled),
}

impl SourceView<'_> {
    fn dimension(&self, name: &str) -> Option<Dimension> {
        let id = DimensionRef::Source(name.to_owned());
        match self {
            Self::Provisional(fixed) => {
                if let Some(key) = fixed.bases.get(name) {
                    return Some(Dimension {
                        id,
                        label: name.to_owned(),
                        key: Some(key.clone()),
                        map: Some(BTreeMap::from([(key.clone(), Integer::one())])),
                        base: true,
                    });
                }
                fixed.derived.contains(name).then(|| Dimension {
                    id,
                    label: name.to_owned(),
                    key: None,
                    map: None,
                    base: false,
                })
            }
            Self::Admitted(assembled) => {
                let (key, map) = assembled.dimensions.get(name)?;
                let base = map.len() == 1 && map.get(key) == Some(&Integer::one());
                Some(Dimension {
                    id,
                    label: name.to_owned(),
                    key: Some(key.clone()),
                    map: Some(map.clone()),
                    base,
                })
            }
        }
    }

    fn unit(&self, name: &str) -> Option<Unit> {
        match self {
            Self::Provisional(fixed) => {
                let (dimension, composed) = fixed.units.get(name)?;
                Some(Unit {
                    name: name.to_owned(),
                    dimension: DimensionRef::Source(dimension.clone()),
                    composed: composed.clone(),
                    package: false,
                })
            }
            Self::Admitted(assembled) => {
                let unit = assembled.units.get(name)?;
                let dimension = assembled
                    .dimensions
                    .iter()
                    .find(|(_, (key, _))| *key == unit.dimension)
                    .map(|(name, _)| name.clone())?;
                Some(Unit {
                    name: name.to_owned(),
                    dimension: DimensionRef::Source(dimension),
                    composed: Some((unit.scale.clone(), unit.offset.clone())),
                    package: false,
                })
            }
        }
    }
}

/// The names one unit of compilation sees: its own dimensions and units
/// and those of each selected package by its model alias.
struct Scope<'a> {
    source: SourceView<'a>,
    models: &'a [AdmittedModel],
}

impl Scope<'_> {
    fn model(&self, alias: &str) -> Option<&AdmittedModel> {
        self.models
            .iter()
            .find(|model| model.alias() == Some(alias))
    }

    #[ix_trace_rs::string_edge]
    fn dimension(&self, name: &NameForm) -> Option<Dimension> {
        let Some((alias, rest)) = name.name.split_once("::") else {
            return self.source.dimension(&name.name);
        };
        let model = self.model(alias)?;
        let dimension = model.dimensions.iter().find(|d| d.name == rest)?;
        let (key, map, _) = super::units::package_dimension(model, dimension, &mut Vec::new())?;
        Some(Dimension {
            id: DimensionRef::Package(dimension.key.clone()),
            label: name.name.clone(),
            key: Some(key),
            map: Some(map),
            base: dimension.base,
        })
    }

    #[ix_trace_rs::string_edge]
    fn unit(&self, name: &str) -> Option<Unit> {
        let Some((alias, rest)) = name.split_once("::") else {
            return self.source.unit(name);
        };
        let model = self.model(alias)?;
        let unit = model.units.iter().find(|unit| unit.name == rest)?;
        Some(Unit {
            name: name.to_owned(),
            dimension: DimensionRef::Package(unit.dimension.clone()),
            composed: package_unit_scale(unit, &model.units),
            package: true,
        })
    }
}

/// A declaration past steps 1 and 2.
struct Resolved<'f> {
    form: &'f DimensionEquivalenceForm,
    dimensions: [Dimension; 2],
    units: [Unit; 2],
}

impl Resolved<'_> {
    fn retained(&self) -> EquivalenceRetained {
        EquivalenceRetained::Declaration {
            dimensions: [
                self.dimensions[0].label.clone(),
                self.dimensions[1].label.clone(),
            ],
            span: self.form.span,
        }
    }

    fn pair(&self) -> BTreeSet<DimensionRef> {
        BTreeSet::from([self.dimensions[0].id.clone(), self.dimensions[1].id.clone()])
    }

    /// The two dimensions' keys, both fixed for a base pair and after the
    /// last turn.
    fn keys(&self) -> Option<[NodeKey; 2]> {
        Some([
            self.dimensions[0].key.clone()?,
            self.dimensions[1].key.clone()?,
        ])
    }

    /// The two dimensions' keys as the emitted node relates them.
    fn emitted_keys(&self) -> Option<[NodeKey; 2]> {
        let [first, second] = &self.dimensions;
        Some([first.node_key()?, second.node_key()?])
    }
}

fn error(cause: AssemblyCause, span: Span) -> AssemblyError {
    AssemblyError { cause, span }
}

fn refused(
    fault: EquivalenceFault,
    retained: Vec<EquivalenceRetained>,
    span: Span,
) -> AssemblyError {
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

/// Which declarations FR-091's assembler decides in one call.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum Phase {
    /// Before the first dimension turn: every declaration's steps 1 and 2,
    /// and the base pairs' steps 3 to 5 and 7. Admits nothing.
    Base,
    /// After the last turn: every declaration, admitted with its key.
    All,
}

/// The unit's dimension equivalences decided in `phase`: in
/// [`Phase::Base`] no equivalence and every error of that phase, in
/// [`Phase::All`] the admitted equivalences or every error found.
pub(super) fn assemble(
    forms: &[DimensionEquivalenceForm],
    source: SourceView<'_>,
    models: &[AdmittedModel],
    owner: &SourceOwner,
    package_name: &str,
    limits: AssemblyLimits,
    phase: Phase,
) -> Result<Vec<DimensionEquivalence>, Vec<AssemblyError>> {
    let scope = Scope { source, models };
    let mut errors = Vec::new();
    // Steps 1 and 2, in source order.
    let mut base = Vec::new();
    let mut derived = Vec::new();
    'forms: for form in forms {
        let mut dimensions = Vec::with_capacity(2);
        for name in &form.dimensions {
            match scope.dimension(name) {
                Some(dimension) => dimensions.push(dimension),
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
            <[Unit; 2]>::try_from(units),
        ) else {
            continue;
        };
        if dimensions[0].id == dimensions[1].id {
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
        if resolved.dimensions.iter().all(|dimension| dimension.base) {
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
        let Some([first, second]) = resolved.keys() else {
            continue;
        };
        match path(&consistent, first, second) {
            Some((implied, cycle)) if implied != ratio => {
                let mut retained: Vec<_> = cycle
                    .into_iter()
                    .filter_map(|index| consistent.get(index).map(|(declaration, _)| *declaration))
                    .chain([resolved])
                    .collect();
                retained.sort_by_key(|declaration| declaration.form.span.start);
                errors.push(refused(
                    EquivalenceFault::Conflicting,
                    retained
                        .iter()
                        .map(|declaration| declaration.retained())
                        .collect(),
                    resolved.form.span,
                ));
            }
            Some(_) | None => consistent.push((resolved, ratio)),
        }
    }
    if phase == Phase::Base {
        return if errors.is_empty() {
            Ok(Vec::new())
        } else {
            Err(errors)
        };
    }

    // The base classes: each base dimension's representative and root size.
    let classes = sizes(
        &consistent
            .iter()
            .filter_map(|(resolved, ratio)| {
                let [first, second] = resolved.keys()?;
                Some((first, second, ratio.clone()))
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
    // root has against the product of those representatives' roots; `None`
    // when a power has no exact value.
    let over_classes = |dimension: &Dimension| {
        let mut map: BTreeMap<NodeKey, Integer> = BTreeMap::new();
        let mut size = Rational::from_integer(Integer::one());
        for (base, exponent) in dimension.map.as_ref()? {
            let (representative, ratio) = class_of(base.clone());
            let entry = map.entry(representative).or_insert_with(Integer::zero);
            *entry = entry.add(exponent);
            size = size.mul(&ratio.pow(exponent)?);
        }
        map.retain(|_, exponent| !exponent.is_zero());
        Some((map, size))
    };

    // Phase two: every other pair, steps 3 to 6 in source order.
    let mut kept: Vec<(&Resolved<'_>, Rational)> = consistent;
    for resolved in &derived {
        let Some(ratio) = steps_three_to_five(resolved, &named, limits, &mut errors) else {
            named.push(resolved);
            continue;
        };
        named.push(resolved);
        let incompatible = |errors: &mut Vec<AssemblyError>| {
            errors.push(refused(
                EquivalenceFault::Incompatible,
                vec![
                    resolved.dimensions[0].retained(),
                    resolved.dimensions[1].retained(),
                ],
                resolved.form.span,
            ));
        };
        let (Some((first_map, first_size)), Some((second_map, second_size))) = (
            over_classes(&resolved.dimensions[0]),
            over_classes(&resolved.dimensions[1]),
        ) else {
            incompatible(&mut errors);
            continue;
        };
        let fixed = first_size.div(&second_size);
        if first_map != second_map || fixed.as_ref() != Some(&ratio) {
            incompatible(&mut errors);
            continue;
        }
        kept.push((resolved, ratio));
    }
    if !errors.is_empty() {
        return Err(errors);
    }

    // Keys, in source order.
    kept.sort_by_key(|(resolved, _)| resolved.form.span.start);
    let node_owner = owner.node_owner(package_name);
    let mut equivalences = Vec::with_capacity(kept.len());
    for (resolved, ratio) in kept {
        let fault = |cause| vec![error(cause, resolved.form.span)];
        let non_canonical = || {
            fault(AssemblyCause::nominal(
                quire_semantic_value::semantic_node::InvalidSemanticGraph {
                    cause:
                        quire_semantic_value::semantic_node::SemanticGraphCause::NonCanonicalPreimage,
                }
                .into(),
            ))
        };
        let [first, second] = resolved.emitted_keys().ok_or_else(non_canonical)?;
        let preimage = DimensionEquivalencePreimage::new(node_owner.clone(), first, second, &ratio)
            .ok_or_else(non_canonical)?;
        let [first, second] = preimage.dimensions();
        let key = crate::check::derived_node_key(
            package_name,
            NodeTag::ScalarType,
            "dimension_equivalence",
            vec![PathSegment::Node(first), PathSegment::Node(second)],
            0,
        )
        .map_err(|refusal| fault(AssemblyCause::nominal(super::nominal_key_refusal(refusal))))?;
        equivalences.push(DimensionEquivalence {
            key,
            dimensions: preimage.dimensions(),
            root_scale: preimage.root_scale().clone(),
            span: resolved.form.span,
        });
    }
    Ok(equivalences)
}

/// Steps 3 to 5 of one declaration: its root ratio, or `None` with the
/// refusal pushed. A source unit whose target chain fixes no scale is
/// decided up to step 4: its dimension's turn reports the defect.
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
            vec![earlier.retained(), resolved.retained()],
            span,
        ));
        return None;
    }
    // Step 4: each unit belongs to the dimension it is named for.
    for (unit, dimension) in resolved.units.iter().zip(&resolved.dimensions) {
        if unit.dimension != dimension.id {
            errors.push(refused(
                EquivalenceFault::Incompatible,
                vec![
                    EquivalenceRetained::Unit {
                        name: unit.name.clone(),
                    },
                    dimension.retained(),
                ],
                span,
            ));
            return None;
        }
    }
    let mut scales = Vec::with_capacity(2);
    for (unit, dimension) in resolved.units.iter().zip(&resolved.dimensions) {
        match &unit.composed {
            Some(composed) => scales.push(composed),
            // A package unit's chain was admitted, so one that fixes no
            // scale is not a unit of the dimension's graph.
            None if unit.package => {
                errors.push(refused(
                    EquivalenceFault::Incompatible,
                    vec![
                        EquivalenceRetained::Unit {
                            name: unit.name.clone(),
                        },
                        dimension.retained(),
                    ],
                    span,
                ));
                return None;
            }
            None => return None,
        }
    }
    // Step 5: an affine unit, or a ratio number that is not positive.
    for (unit, (_, offset)) in resolved.units.iter().zip(&scales) {
        if !offset.is_zero() {
            errors.push(refused(
                EquivalenceFault::InvalidValue,
                vec![
                    EquivalenceRetained::Unit {
                        name: unit.name.clone(),
                    },
                    EquivalenceRetained::Value(offset.clone()),
                ],
                span,
            ));
            return None;
        }
    }
    let mut numbers = Vec::with_capacity(2);
    for term in &resolved.form.ratio {
        match ratio_number(&term.number, limits) {
            Ok(number) if positive(&number) => numbers.push(number),
            Ok(number) => {
                errors.push(refused(
                    EquivalenceFault::InvalidValue,
                    vec![resolved.retained(), EquivalenceRetained::Value(number)],
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
    let ([x, y], [(a, _), (b, _)]) = (numbers.as_slice(), scales.as_slice()) else {
        return None;
    };
    y.mul(b).div(&x.mul(a))
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
        BTreeMap::from([(from.clone(), (one, Vec::new()))]);
    let mut pending = VecDeque::from([from]);
    while let Some(node) = pending.pop_front() {
        let Some((size, via)) = reached.get(&node).cloned() else {
            continue;
        };
        for (index, (resolved, ratio)) in pairs.iter().enumerate() {
            let Some([first, second]) = resolved.keys() else {
                continue;
            };
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
            reached.insert(next.clone(), (next_size, next_via));
            pending.push_back(next);
        }
    }
    reached.remove(&to)
}
