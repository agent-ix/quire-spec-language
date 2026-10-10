// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-415 step 10 (FR-093-AC-14) over the model forms: a
//! `lookup` chain, an attribute read and a dispatched call over one,
//! `allInstances` under a nested sum, nested dispatch arguments, and a
//! nested precondition a redefinition inherits through the dispatch bridge,
//! each checked on a 512 KiB thread nested 2 and 1,000 levels deep at the
//! default limits: no limit names a depth.

use qsl_foundation::absence::AbsenceMode;

use super::*;

const STACK: usize = 512 * 1024;

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
    /// `acme/orders`' dispatch package for `Order.size` with `f`, over
    /// `p: Population<M::Order>[3]` and `r: Reference<M::Order>`, holding this
    /// form nested `levels` times.
    fn declarations(self, levels: usize) -> PackageDeclarations {
        let acme = admitted("1.0.0");
        let parameters = [("p", population("M::Order", 3)), ("r", named("M::Order"))];
        let mut chain = name("r");
        for _ in 0..levels {
            chain = Expression::lookup(named("M::Order"), name("p"), chain, AbsenceMode::Undefined);
        }
        let f = match self {
            Self::Lookup => function("f", &parameters, named("M::Order"), chain),
            Self::Attribute => function(
                "f",
                &parameters,
                builtin(BuiltinType::Int).with_bounds(vec!["0".to_owned(), "9".to_owned()]),
                Expression::field(Expression::deref(chain), "total".to_owned()),
            ),
            Self::Dispatch => FunctionDeclaration::clause(
                "f",
                parameters
                    .iter()
                    .map(|(name, form)| ((*name).to_owned(), form.clone()))
                    .collect(),
                builtin(BuiltinType::Boolean),
                None,
                Expression::binary(
                    BinaryOperator::GreaterOrEqual,
                    Expression::dispatch(chain, "size".to_owned(), Vec::new()),
                    Expression::integer(0_i64),
                ),
                DeclaredClauseKind::Precondition,
            ),
            Self::AllInstances => {
                let mut body = Expression::integer(0_i64);
                for _ in 0..levels {
                    body = Expression::binary(
                        BinaryOperator::Add,
                        Expression::size(Expression::all_instances(named("M::Order"), name("p"))),
                        body,
                    );
                }
                function("f", &parameters, builtin(BuiltinType::Integer), body)
            }
            Self::DispatchArguments => return dispatch_arguments(&acme, levels),
            Self::InheritedPrecondition => return inherited_precondition(&acme, levels),
        };
        let mut declarations = dispatch(&acme, "Order/size");
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
            .expect("the check completes on a 512 KiB stack")
    }
}

/// `acme/orders`' dispatch package for `Order.scaled` with the clause `f`,
/// over `r: Reference<M::Order>`: `r.scaled(r.scaled(… 0)) >= 0`, nested
/// `levels` times.
fn dispatch_arguments(acme: &Acme, levels: usize) -> PackageDeclarations {
    let mut argument = Expression::integer(0_i64);
    for _ in 0..levels {
        argument = Expression::dispatch(name("r"), "scaled".to_owned(), vec![argument]);
    }
    let mut declarations = dispatch(acme, "Order/scaled");
    declarations.functions.push(FunctionDeclaration::clause(
        "f",
        vec![("r".to_owned(), named("M::Order"))],
        builtin(BuiltinType::Boolean),
        None,
        Expression::binary(
            BinaryOperator::GreaterOrEqual,
            argument,
            Expression::integer(0_i64),
        ),
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
        sum = Expression::binary(BinaryOperator::Add, name("n"), sum);
    }
    let mut clauses = clauses(acme);
    clauses.own_precondition.insert(
        key("Order/scaled"),
        Expression::binary(
            BinaryOperator::GreaterOrEqual,
            sum,
            Expression::integer(0_i64),
        ),
    );
    dispatch_with(acme, "Order/scaled", &clauses)
}

/// Each model form checks nested 2 and 1,000 levels deep at the default
/// limits.
#[trace("FR-093-AC-14", "TC-415")]
#[test]
fn every_nested_model_form_checks_1000_levels_deep_on_a_small_stack() {
    for form in MODEL_FORMS {
        for levels in [2, 1_000] {
            if let Err(refusals) = form.check(levels, CheckingLimits::default()) {
                panic!("{form:?} at {levels} levels checks: {refusals:?}");
            }
        }
    }
}
