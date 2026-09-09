// SPDX-License-Identifier: AGPL-3.0-only
//! FR-009/FR-033: explicit native targets consumed by the existing executable binder.

mod wire;

use std::collections::BTreeMap;
use std::io::{self, Write};

use quire_contract_ir as ir;

use crate::checking::{CheckedClause, ClauseBinding, NativeType, Observation};
use crate::linking::{DeclarationKey, DeclarationLocation, ResolutionTarget};
use crate::package::NativePackage;
use crate::syntax::{BinaryOp, ClauseKind, ExprId, ExprKind, UnaryOp};
use crate::{ByteDigest, Span};

/// The existing backend's Boolean expression domain; not a proof attestation.
pub const PROFILE: &str = "boolean-oracle/v1";

/// Explicit lowering domain; IR binding and backend acceptance are separate.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProjectionTarget {
    /// The existing generated Boolean backend's qualified domain.
    BooleanOracleV1,
    /// Bounded primitive integer expressions for the strict IR binder.
    IntegerIrV1,
}

impl ProjectionTarget {
    /// Stable target name used in command selection and failure reports.
    pub const fn name(self) -> &'static str {
        match self {
            Self::BooleanOracleV1 => PROFILE,
            Self::IntegerIrV1 => "integer-ir/v1",
        }
    }

    fn admits(self, ty: &ir::ValueType) -> bool {
        matches!(ty, ir::ValueType::Boolean)
            || (self == Self::IntegerIrV1 && matches!(ty, ir::ValueType::Integer { .. }))
    }
}

/// Caller-lowered ceilings for one fresh atomic projection.
#[derive(Clone, Copy, Debug)]
pub struct LoweringLimits {
    /// Visited native nodes, including parentheses; at most 10,000.
    pub nodes: usize,
    /// Native nesting depth, with each clause root at one; at most 64.
    pub depth: usize,
    /// Serialized IR wire bytes; at most 16 MiB.
    pub bytes: usize,
}

impl Default for LoweringLimits {
    fn default() -> Self {
        Self {
            nodes: 10_000,
            depth: 64,
            bytes: 16_777_216,
        }
    }
}

impl LoweringLimits {
    fn bounded(self) -> Self {
        let hard = Self::default();
        Self {
            nodes: self.nodes.min(hard.nodes),
            depth: self.depth.min(hard.depth),
            bytes: self.bytes.min(hard.bytes),
        }
    }
}

/// Completed lowering work; a new request starts at zero.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct LoweringUsage {
    /// Native nodes visited across the complete clause population.
    pub nodes: usize,
    /// Greatest visited native depth.
    pub max_depth: usize,
    /// Exact serialized projection length.
    pub bytes: usize,
}

/// Refusal category, separate from native runtime truth.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LoweringCode {
    /// An expression or owner population is outside this target.
    Unsupported,
    /// A caller or implementation work ceiling was reached.
    ResourceExhausted,
    /// The IR rejected the derived expression or complete wire projection.
    Binding,
    /// An invariant of the already checked native input was not satisfied.
    InvalidCorrespondence,
}

/// No partial projection is returned with a refusal.
#[derive(Debug)]
pub struct LoweringError {
    /// Stable stage-independent refusal classification.
    pub code: LoweringCode,
    /// Authored obligation when a particular clause caused the failure.
    pub clause: Option<ir::ClauseRef>,
    /// Exact native source locus, when available.
    pub source: Option<Span>,
    /// Human-readable explanation.
    pub message: String,
    /// Original IR diagnostics; the compiler does not reinterpret their codes.
    pub upstream: Vec<ir::Diagnostic>,
}

impl std::fmt::Display for LoweringError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{:?}: {}", self.code, self.message)
    }
}

impl std::error::Error for LoweringError {}

type Result<T> = std::result::Result<T, Box<LoweringError>>;

fn failure(
    code: LoweringCode,
    clause: Option<&ClauseBinding>,
    source: Option<Span>,
    message: &str,
) -> Box<LoweringError> {
    Box::new(LoweringError {
        code,
        clause: clause.map(identity),
        source,
        message: message.into(),
        upstream: Vec::new(),
    })
}

fn upstream(
    clause: Option<&ClauseBinding>,
    source: Option<Span>,
    diagnostics: Vec<ir::Diagnostic>,
) -> Box<LoweringError> {
    let mut error = failure(
        LoweringCode::Binding,
        clause,
        source,
        "existing IR rejected the native derivation",
    );
    error.upstream = diagnostics;
    error
}

fn identity(clause: &ClauseBinding) -> ir::ClauseRef {
    ir::ClauseRef::new(clause.requirement.clone(), clause.clause.clone())
}

/// One derived primitive parameter's exact native declaration correspondence.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectedRead {
    /// Complete authored clause identity in the derived package.
    pub clause: ir::ClauseRef,
    /// Name used in this clause's IR declaration environment.
    pub name: ir::SymbolName,
    /// Linked original declaration identity and source; never inferred from text.
    pub declaration: DeclarationLocation,
    /// Exact model artifact containing the declaration.
    pub model_digest: ByteDigest,
    /// Checked native read observation, including captured pre-inputs.
    pub native_observation: ir::StateObservation,
    /// IR input reads use current; state reads preserve their observation.
    pub ir_observation: ir::StateObservation,
}

/// Derived executable wire and its strict binding, attached to original authority.
#[derive(Debug)]
pub struct NativeProjection<'p, 'model> {
    native: &'p NativePackage<'model>,
    target: ProjectionTarget,
    bytes: Vec<u8>,
    bound: ir::BoundPackage,
    reads: Vec<ProjectedRead>,
    usage: LoweringUsage,
}

impl<'p, 'model> NativeProjection<'p, 'model> {
    /// Original exact native artifact and separately retained static identity.
    pub fn native(&self) -> &'p NativePackage<'model> {
        self.native
    }
    /// Named lowering domain. Backend generation may impose further limits.
    pub fn profile(&self) -> &'static str {
        self.target.name()
    }
    /// Public IR executable-projection/v1 wire consumed by the strict binder.
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
    /// Constructor-validated complete executable binding.
    pub fn bound(&self) -> &ir::BoundPackage {
        &self.bound
    }
    /// Read correspondence in source-clause order, then declaration-name order.
    pub fn reads(&self) -> &[ProjectedRead] {
        &self.reads
    }
    /// Work performed by this successful request.
    pub fn usage(&self) -> &LoweringUsage {
        &self.usage
    }
}

struct Prepared<'a> {
    binding: &'a ClauseBinding,
    environment: ir::DeclarationEnvironment,
    expression: ir::Expression,
}

/// Lower the complete native package in the named Boolean target.
///
/// All native models and runtime validation obligations remain on `native`.
/// This operation neither evaluates runtime data nor authenticates backend evidence.
pub fn lower<'p, 'model>(
    native: &'p NativePackage<'model>,
    limits: LoweringLimits,
) -> Result<NativeProjection<'p, 'model>> {
    lower_for(native, ProjectionTarget::BooleanOracleV1, limits)
}

/// Lower the complete package for an explicitly selected target (FR-033).
/// Integer IR output still requires a backend that admits its expression domain.
pub fn lower_for<'p, 'model>(
    native: &'p NativePackage<'model>,
    target: ProjectionTarget,
    limits: LoweringLimits,
) -> Result<NativeProjection<'p, 'model>> {
    let limits = limits.bounded();
    let checked = native.checked();
    let linked = checked.linked();
    let unit = linked.unit();
    let Some(first) = checked.clauses().first() else {
        return Err(failure(
            LoweringCode::Unsupported,
            None,
            None,
            "empty native packages have no executable projection owner",
        ));
    };
    let package_id = first.binding().requirement.package();
    let mut owners = BTreeMap::new();
    for clause in checked.clauses() {
        let binding = clause.binding();
        if binding.requirement.package() != package_id
            || owners
                .insert(
                    binding.requirement.requirement(),
                    binding.requirement.revision(),
                )
                .is_some_and(|revision| revision != binding.requirement.revision())
        {
            let span = unit
                .clauses()
                .iter()
                .find(|clause| clause.name.value == binding.name)
                .map(|clause| clause.span);
            return Err(failure(
                LoweringCode::Unsupported,
                Some(binding),
                span,
                "one projection requires one package and one revision per requirement",
            ));
        }
    }
    let mut usage = LoweringUsage::default();
    let mut prepared = Vec::new();
    let mut reads = Vec::new();
    let mut requirements: BTreeMap<ir::RequirementRef, Vec<ir::Clause<ir::ReferenceBody>>> =
        BTreeMap::new();
    // The checker constructs these populations together; verify before paired traversal.
    if checked.clauses().len() != unit.clauses().len()
        || linked.clauses().len() != unit.clauses().len()
    {
        return Err(failure(
            LoweringCode::InvalidCorrespondence,
            None,
            None,
            "native clause populations disagree",
        ));
    }
    for ((clause, syntax), linked_clause) in checked
        .clauses()
        .iter()
        .zip(unit.clauses())
        .zip(linked.clauses())
    {
        let binding = clause.binding();
        let model = &linked.models()[linked_clause.model()];
        let occurrences = linked_clause
            .occurrences()
            .iter()
            .filter_map(
                |occurrence| match (&occurrence.expression, &occurrence.target) {
                    (Some(id), ResolutionTarget::Formal(location)) => Some((id.0, location)),
                    _ => None,
                },
            )
            .collect();
        let mut lowering = ClauseLowering {
            native,
            target,
            clause,
            model,
            occurrences,
            // Reuse the checker's immutable model index across all clauses.
            declarations: &checked.catalogs()[linked_clause.model()].values,
            used: BTreeMap::new(),
            limits,
            usage: &mut usage,
        };
        let expression = lowering.expression(syntax.expression, 1)?;
        let (values, clause_reads): (Vec<_>, Vec<_>) = lowering.used.into_values().unzip();
        let environment = ir::DeclarationEnvironment::new(
            binding.requirement.clone(),
            Vec::new(),
            values,
            Vec::new(),
        )
        .map_err(|errors| upstream(Some(binding), Some(syntax.span), errors))?;
        let typed = environment
            .check_expression(
                &expression,
                &ir::ValueType::Boolean,
                &binding.execution_point,
                true,
            )
            .map_err(|errors| upstream(Some(binding), Some(syntax.span), errors))?;
        let body = ir::ReferenceBody::Composite {
            children: typed
                .dependencies()
                .iter()
                .cloned()
                .map(|identity| ir::ReferenceBody::Reference { identity })
                .collect(),
        };
        let source = checked
            .bindings()
            .source
            .to_ir(unit.source(), syntax.span)
            .map_err(|_| {
                failure(
                    LoweringCode::InvalidCorrespondence,
                    Some(binding),
                    Some(syntax.span),
                    "clause source differs from checked correspondence",
                )
            })?;
        let kind = match syntax.kind {
            ClauseKind::Invariant => ir::ClauseKind::Invariant,
            ClauseKind::Precondition => ir::ClauseKind::Precondition,
            ClauseKind::Postcondition => ir::ClauseKind::Postcondition,
        };
        let derived = ir::Clause::new(
            binding.clause.clone(),
            kind,
            Some(binding.execution_point.clone()),
            source,
            body,
        )
        .map_err(|error| upstream(Some(binding), Some(syntax.span), vec![error]))?;
        requirements
            .entry(binding.requirement.clone())
            .or_default()
            .push(derived);
        reads.extend(clause_reads);
        prepared.push(Prepared {
            binding,
            environment,
            expression,
        });
    }
    let whole_source = checked
        .bindings()
        .source
        .to_ir(
            unit.source(),
            Span {
                start: 0,
                end: unit.source().text().len(),
            },
        )
        .map_err(|_| {
            failure(
                LoweringCode::InvalidCorrespondence,
                None,
                None,
                "package source differs from checked correspondence",
            )
        })?;
    let requirements = requirements
        .into_iter()
        .map(|(owner, clauses)| {
            ir::Requirement::new(
                package_id,
                owner.requirement().clone(),
                owner.revision(),
                whole_source.clone(),
                clauses,
            )
            .map_err(|error| upstream(None, None, vec![error]))
        })
        .collect::<Result<Vec<_>>>()?;
    let schema = ir::SchemaVersion::new(1, 1).map_err(|error| upstream(None, None, vec![error]))?;
    let package = ir::ContractPackage::new(
        package_id.clone(),
        schema,
        checked.bindings().source.identity().clone(),
        requirements,
    )
    .map_err(|errors| upstream(None, None, errors))?;
    let mut output = Output {
        bytes: Vec::new(),
        maximum: limits.bytes,
    };
    serde_json::to_writer(&mut output, &wire::Projection::new(&package, &prepared)).map_err(
        |error| {
            failure(
                if error.is_io() {
                    LoweringCode::ResourceExhausted
                } else {
                    LoweringCode::InvalidCorrespondence
                },
                None,
                None,
                &error.to_string(),
            )
        },
    )?;
    usage.bytes = output.bytes.len();
    let bound = ir::BoundPackage::from_json_bytes(&output.bytes)
        .map_err(|errors| upstream(None, None, errors))?;
    Ok(NativeProjection {
        native,
        target,
        bytes: output.bytes,
        bound,
        reads,
        usage,
    })
}

struct ClauseLowering<'a, 'model> {
    native: &'a NativePackage<'model>,
    target: ProjectionTarget,
    clause: &'a CheckedClause<'model>,
    model: &'a crate::linking::LinkedModel<'model>,
    occurrences: BTreeMap<usize, &'a DeclarationLocation>,
    declarations: &'a BTreeMap<&'model ir::SymbolName, &'model ir::ValueDeclaration>,
    used: BTreeMap<ir::SymbolName, (ir::ValueDeclaration, ProjectedRead)>,
    limits: LoweringLimits,
    usage: &'a mut LoweringUsage,
}

impl ClauseLowering<'_, '_> {
    fn error(&self, code: LoweringCode, span: Span, message: &str) -> Box<LoweringError> {
        failure(code, Some(self.clause.binding()), Some(span), message)
    }

    fn expression(&mut self, id: ExprId, depth: usize) -> Result<ir::Expression> {
        let checked = self.native.checked();
        let unit = checked.linked().unit();
        let node = &unit.expressions()[id.0];
        if self.usage.nodes >= self.limits.nodes || depth > self.limits.depth {
            return Err(self.error(
                LoweringCode::ResourceExhausted,
                node.span,
                "native lowering node/depth limit",
            ));
        }
        self.usage.nodes += 1;
        self.usage.max_depth = self.usage.max_depth.max(depth);
        let primitive = match self.clause.expression_type(id) {
            Some(NativeType::Boolean) => Some(&ir::ValueType::Boolean),
            Some(NativeType::Scalar { representation, .. }) => Some(*representation),
            _ => None,
        };
        if !primitive.is_some_and(|ty| self.target.admits(ty)) {
            return Err(self.error(
                LoweringCode::Unsupported,
                node.span,
                "expression type is outside the selected projection target",
            ));
        }
        let kind = match &node.kind {
            ExprKind::Group { inner } => return self.expression(*inner, depth + 1),
            ExprKind::Boolean(value) => ir::ExpressionKind::BooleanLiteral { value: *value },
            ExprKind::Integer(text) if self.target == ProjectionTarget::IntegerIrV1 => {
                let Some(ir::ValueType::Integer { value: value_type }) = primitive else {
                    return Err(self.error(
                        LoweringCode::InvalidCorrespondence,
                        node.span,
                        "integer literal has no checked scalar representation",
                    ));
                };
                let value = text.parse().map_err(|_| {
                    self.error(
                        LoweringCode::InvalidCorrespondence,
                        node.span,
                        "checked integer literal cannot be represented",
                    )
                })?;
                ir::ExpressionKind::IntegerLiteral {
                    value,
                    value_type: value_type.clone(),
                }
            }
            ExprKind::Name(_) => self.read(id, node.span)?,
            ExprKind::Unary {
                op: UnaryOp::Not,
                argument,
            } => ir::ExpressionKind::BooleanNot {
                operand: Box::new(self.expression(*argument, depth + 1)?),
            },
            ExprKind::Unary {
                op: UnaryOp::Negate,
                argument,
            } if self.target == ProjectionTarget::IntegerIrV1 => {
                ir::ExpressionKind::NumericNegate {
                    operand: Box::new(self.expression(*argument, depth + 1)?),
                }
            }
            ExprKind::Binary { op, left, right } => {
                self.binary(*op, *left, *right, depth, node.span)?
            }
            _ => {
                return Err(self.error(
                    LoweringCode::Unsupported,
                    node.span,
                    "expression form is outside the selected projection target",
                ))
            }
        };
        let source = checked
            .bindings()
            .source
            .to_ir(unit.source(), node.span)
            .map_err(|_| {
                self.error(
                    LoweringCode::InvalidCorrespondence,
                    node.span,
                    "expression source differs from checked correspondence",
                )
            })?;
        Ok(ir::Expression::new(kind, source))
    }

    fn binary(
        &mut self,
        op: BinaryOp,
        left: ExprId,
        right: ExprId,
        depth: usize,
        span: Span,
    ) -> Result<ir::ExpressionKind> {
        if self.target == ProjectionTarget::BooleanOracleV1
            && !matches!(op, BinaryOp::And | BinaryOp::Or | BinaryOp::Implies)
        {
            return Err(self.error(
                LoweringCode::Unsupported,
                span,
                "operator is outside the Boolean backend domain",
            ));
        }
        let left = Box::new(self.expression(left, depth + 1)?);
        let right = Box::new(self.expression(right, depth + 1)?);
        Ok(match op {
            BinaryOp::And | BinaryOp::Or | BinaryOp::Implies => ir::ExpressionKind::Boolean {
                operator: match op {
                    BinaryOp::And => ir::BooleanOperator::ShortCircuitAnd,
                    BinaryOp::Or => ir::BooleanOperator::ShortCircuitOr,
                    _ => ir::BooleanOperator::Implication,
                },
                left,
                right,
            },
            BinaryOp::Add
            | BinaryOp::Subtract
            | BinaryOp::Multiply
            | BinaryOp::Divide
            | BinaryOp::Remainder => ir::ExpressionKind::Numeric {
                operator: match op {
                    BinaryOp::Add => ir::NumericOperator::Add,
                    BinaryOp::Subtract => ir::NumericOperator::Subtract,
                    BinaryOp::Multiply => ir::NumericOperator::Multiply,
                    BinaryOp::Divide => ir::NumericOperator::Divide,
                    _ => ir::NumericOperator::Remainder,
                },
                left,
                right,
            },
            BinaryOp::Equal
            | BinaryOp::NotEqual
            | BinaryOp::Less
            | BinaryOp::LessEqual
            | BinaryOp::Greater
            | BinaryOp::GreaterEqual => ir::ExpressionKind::Compare {
                operator: match op {
                    BinaryOp::Equal => ir::ComparisonOperator::Equal,
                    BinaryOp::NotEqual => ir::ComparisonOperator::NotEqual,
                    BinaryOp::Less => ir::ComparisonOperator::Less,
                    BinaryOp::LessEqual => ir::ComparisonOperator::LessEqual,
                    BinaryOp::Greater => ir::ComparisonOperator::Greater,
                    _ => ir::ComparisonOperator::GreaterEqual,
                },
                left,
                right,
            },
        })
    }

    fn read(&mut self, id: ExprId, span: Span) -> Result<ir::ExpressionKind> {
        let location = self.occurrences.get(&id.0).copied().ok_or_else(|| {
            self.error(
                LoweringCode::InvalidCorrespondence,
                span,
                "read has no linked declaration",
            )
        })?;
        let DeclarationKey::Value(name) = &location.identity.key else {
            return Err(self.error(
                LoweringCode::Unsupported,
                span,
                "read is not a direct model value",
            ));
        };
        if &location.identity.owner != self.model.environment().owner() {
            return Err(self.error(
                LoweringCode::Unsupported,
                span,
                "read does not belong to the clause context model",
            ));
        }
        let declaration = self.declarations.get(name).copied().ok_or_else(|| {
            self.error(
                LoweringCode::InvalidCorrespondence,
                span,
                "linked value declaration is unavailable",
            )
        })?;
        if !self.target.admits(declaration.value_type()) {
            return Err(self.error(
                LoweringCode::Unsupported,
                span,
                "read declaration is outside the selected projection target",
            ));
        }
        let Some(Observation::Snapshot(native_observation)) = self.clause.observation(id) else {
            return Err(self.error(
                LoweringCode::InvalidCorrespondence,
                span,
                "read has no concrete checked observation",
            ));
        };
        let ir_observation = match declaration.kind() {
            ir::ValueDeclarationKind::Input => ir::StateObservation::Current,
            ir::ValueDeclarationKind::State => native_observation,
        };
        self.used.entry(name.clone()).or_insert_with(|| {
            (
                declaration.clone(),
                ProjectedRead {
                    clause: identity(self.clause.binding()),
                    name: name.clone(),
                    declaration: location.clone(),
                    model_digest: self.model.digest(),
                    native_observation,
                    ir_observation,
                },
            )
        });
        Ok(ir::ExpressionKind::ValueReference {
            name: name.clone(),
            observation: ir_observation,
        })
    }
}

// Serde owns all JSON grammar. This sink only enforces the output byte ceiling.
struct Output {
    bytes: Vec<u8>,
    maximum: usize,
}

impl Write for Output {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if bytes.len() > self.maximum.saturating_sub(self.bytes.len()) {
            return Err(io::Error::other("executable projection byte limit"));
        }
        self.bytes.extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
