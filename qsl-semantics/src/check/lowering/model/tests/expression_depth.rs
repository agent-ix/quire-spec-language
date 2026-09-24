// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-415 step 10 (FR-093-AC-14, QSL-228, QSL-231) over the model forms: a
//! `lookup` chain, an attribute read and a dispatched call over one,
//! `allInstances` under a nested sum, nested dispatch arguments, and a
//! nested precondition a redefinition inherits through the dispatch bridge,
//! each checked on a 2 MiB thread at the deepest nesting the default limits
//! admit and refused on the depth limit one level deeper and 1,000 levels
//! deep.

use qsl_foundation::absence::AbsenceMode;

use super::*;
use crate::check::refusal::{CheckingLimitKind, CheckingStage, StageLimitCause};
use crate::check::MAX_CHECKING_DEPTH;

const STACK: usize = 2 * 1024 * 1024;

#[derive(Clone, Copy, Debug)]
enum ModelForm {
    /// `lookup<M::Order>(p, lookup<M::Order>(p, … r) absent undefined)
    /// absent undefined`.
    Lookup,
    /// `deref(lookup<M::Order>(p, … r)).total`.
    Attribute,
    /// The clause `lookup<M::Order>(p, … r).size() >= 0`.
    Dispatch,
    /// `size(allInstances<M::Order>(p)) + (size(allInstances<M::Order>(p))
    /// + … 0)`.
    AllInstances,
    /// The clause `r.scaled(r.scaled(… 0)) >= 0`: each level a dispatch
    /// argument, typed against `Order.scaled`'s parameter `n: Integer`.
    DispatchArguments,
    /// `Order.scaled`'s precondition `n + (n + (… n)) >= 0`, which the
    /// dispatch bridge renames to `m` for `Sub.scaled`'s effective
    /// precondition `false or (m + (… m) >= 0)`.
    InheritedPrecondition,
}

const MODEL_FORMS: [ModelForm; 6] = [
    ModelForm::Lookup,
    ModelForm::Attribute,
    ModelForm::Dispatch,
    ModelForm::AllInstances,
    ModelForm::DispatchArguments,
    ModelForm::InheritedPrecondition,
];

impl ModelForm {
    /// The deepest nesting the default limits admit: the `lookup` chain
    /// reaches the limit itself; an attribute read or a dispatched call over
    /// it, and each `size(allInstances(p))` operand, add two levels.
    fn deepest(self) -> usize {
        match self {
            Self::Lookup => 127,
            Self::Attribute | Self::Dispatch | Self::AllInstances => 125,
            Self::DispatchArguments => DEEPEST_ARGUMENTS,
            Self::InheritedPrecondition => DEEPEST_INHERITED,
        }
    }

    /// `acme/orders`' dispatch package for `Order.size` with `f`, over
    /// `p: Population<M::Order>[3]` and `r: Reference<M::Order>`, holding this
    /// form nested `levels` times.
    fn declarations(self, levels: usize) -> PackageDeclarations {
        let acme = admitted("1.0.0");
        let parameters = [("p", population("M::Order", 3)), ("r", named("M::Order"))];
        let mut chain = name("r");
        for _ in 0..levels {
            chain = Expression::Lookup {
                target: named("M::Order"),
                population: Box::new(name("p")),
                reference: Box::new(chain),
                absence: AbsenceMode::Undefined,
            };
        }
        let f = match self {
            Self::Lookup => function("f", &parameters, named("M::Order"), chain),
            Self::Attribute => function(
                "f",
                &parameters,
                builtin(BuiltinType::Int).with_bounds(vec!["0".to_owned(), "9".to_owned()]),
                Expression::Field {
                    operand: Box::new(Expression::Deref(Box::new(chain))),
                    field: "total".to_owned(),
                },
            ),
            Self::Dispatch => FunctionDeclaration::clause(
                "f",
                parameters
                    .iter()
                    .map(|(name, form)| ((*name).to_owned(), form.clone()))
                    .collect(),
                builtin(BuiltinType::Boolean),
                None,
                Expression::Binary {
                    operator: BinaryOperator::GreaterOrEqual,
                    left: Box::new(Expression::Dispatch {
                        receiver: Box::new(chain),
                        member: "size".to_owned(),
                        arguments: Vec::new(),
                    }),
                    right: Box::new(Expression::Integer(Integer::from(0_i64))),
                },
                DeclaredClauseKind::Precondition,
            ),
            Self::AllInstances => {
                let mut body = Expression::Integer(Integer::from(0_i64));
                for _ in 0..levels {
                    body = Expression::Binary {
                        operator: BinaryOperator::Add,
                        left: Box::new(Expression::Size(Box::new(Expression::AllInstances {
                            target: named("M::Order"),
                            population: Box::new(name("p")),
                        }))),
                        right: Box::new(body),
                    };
                }
                function("f", &parameters, builtin(BuiltinType::Integer), body)
            }
            Self::DispatchArguments => return dispatch_arguments(&acme, levels),
            Self::InheritedPrecondition => return inherited_precondition(&acme, levels),
        };
        let mut declarations = dispatch(&acme, "Order/size");
        declarations.models = vec![acme.model];
        declarations.functions.push(f);
        declarations
    }

    /// Check this form nested `levels` times under `limits` on a
    /// [`STACK`]-byte thread.
    fn check(self, levels: usize, limits: CheckingLimits) -> Result<(), Vec<CheckRefusal>> {
        std::thread::Builder::new()
            .stack_size(STACK)
            .spawn(move || self.declarations(levels).check(limits).map(drop))
            .expect("the check thread spawns")
            .join()
            .expect("the check completes on a 2 MiB stack")
    }
}

/// The deepest dispatch-argument nesting the default limits admit: the
/// comparison, 126 dispatches and the innermost `0` are 128 levels.
const DEEPEST_ARGUMENTS: usize = 126;

/// The deepest inherited-precondition nesting the default limits admit:
/// `Sub.scaled`'s effective `false or (…)`, the comparison, 125 additions
/// and the innermost `m` are 128 levels.
const DEEPEST_INHERITED: usize = 125;

/// `acme/orders`' dispatch package for `Order.scaled` with the clause `f`,
/// over `r: Reference<M::Order>`: `r.scaled(r.scaled(… 0)) >= 0`, nested
/// `levels` times.
fn dispatch_arguments(acme: &Acme, levels: usize) -> PackageDeclarations {
    let mut argument = Expression::Integer(Integer::from(0_i64));
    for _ in 0..levels {
        argument = Expression::Dispatch {
            receiver: Box::new(name("r")),
            member: "scaled".to_owned(),
            arguments: vec![argument],
        };
    }
    let mut declarations = dispatch(acme, "Order/scaled");
    declarations.functions.push(FunctionDeclaration::clause(
        "f",
        vec![("r".to_owned(), named("M::Order"))],
        builtin(BuiltinType::Boolean),
        None,
        Expression::Binary {
            operator: BinaryOperator::GreaterOrEqual,
            left: Box::new(argument),
            right: Box::new(Expression::Integer(Integer::from(0_i64))),
        },
        DeclaredClauseKind::Precondition,
    ));
    declarations
}

/// `acme/orders`' dispatch package for `Order.scaled`, built by the
/// dispatch bridge with `Order.scaled`'s precondition `n + (n + (… n)) >=
/// 0`, nested `levels` times. `Sub.scaled(this, m)` redefines it, so the
/// bridge renames the precondition into `Sub.scaled`'s effective one.
fn inherited_precondition(acme: &Acme, levels: usize) -> PackageDeclarations {
    let mut sum = name("n");
    for _ in 0..levels {
        sum = Expression::Binary {
            operator: BinaryOperator::Add,
            left: Box::new(name("n")),
            right: Box::new(sum),
        };
    }
    let mut clauses = clauses(acme);
    clauses.own_precondition.insert(
        key("Order/scaled"),
        Expression::Binary {
            operator: BinaryOperator::GreaterOrEqual,
            left: Box::new(sum),
            right: Box::new(Expression::Integer(Integer::from(0_i64))),
        },
    );
    dispatch_with(acme, "Order/scaled", &clauses)
}

fn assert_depth_refusals(form: ModelForm, refusals: &[CheckRefusal], limit: u64) {
    assert!(!refusals.is_empty(), "{form:?}");
    for refusal in refusals {
        assert_eq!(
            refusal.cause,
            CheckCause::ResourceExhausted(Box::new(StageLimitCause {
                stage: CheckingStage::Typing,
                kind: CheckingLimitKind::Depth,
                limit,
                actual: u128::from(limit) + 1,
            })),
            "{form:?}: {refusal:?}"
        );
    }
}

/// Each model form checks at its deepest admitted nesting and refuses on
/// the depth limit one level deeper, and 1,000 levels deep at the default
/// limits, at the maximum depth with nodes, input bytes and work unlimited,
/// and at a caller depth of 16.
#[trace("FR-093-AC-14", "TC-415")]
#[test]
fn every_nested_model_form_refuses_on_the_depth_limit_on_a_small_stack() {
    let maximum = CheckingLimits::new(u64::MAX, MAX_CHECKING_DEPTH)
        .expect("the maximum depth is admitted")
        .with_work_budget(u64::MAX)
        .with_input_bytes(u64::MAX);
    let narrowed = CheckingLimits::new(u64::MAX, 16).expect("16 is admitted");
    for form in MODEL_FORMS {
        let deepest = form.deepest();
        if let Err(refusals) = form.check(deepest, CheckingLimits::default()) {
            panic!("{form:?} at {deepest} levels checks: {refusals:?}");
        }
        for (levels, limits, limit) in [
            (deepest + 1, CheckingLimits::default(), MAX_CHECKING_DEPTH),
            (1_000, CheckingLimits::default(), MAX_CHECKING_DEPTH),
            (1_000, maximum, MAX_CHECKING_DEPTH),
            (1_000, narrowed, 16),
        ] {
            let refusals = form
                .check(levels, limits)
                .expect_err("the nesting passes the depth limit");
            assert_depth_refusals(form, &refusals, limit);
        }
    }
}
