// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-1416 (FR-577, FR-121-AC-37): a postcondition's range obligations are
//! discharged from its operation's unit preconditions and from difference
//! bounds between stable paths, over the `test/bank` fixture.
//!
//! Every compile of the bank unit goes through `crate::call_site` with the
//! `Bank::Account::withdraw` operation selection; the two function units
//! (steps 3 and 12) go through it with a function selection.

use super::frame::identifier;
use super::frame_replay::unit_for;
use super::state_clause_replay;
use super::*;
use crate::spine::OperationName;
use crate::{call_site, CallSite, CallSiteRefusal, OperationSite, QualifiedName};

const BANK: &str = "test/bank";

/// How `Premium`, a subtype of `Account`, is declared.
#[derive(Clone, Copy, Eq, PartialEq)]
enum Premium {
    /// No `Premium` type.
    Absent,
    /// `Premium` inherits `withdraw`.
    Inherits,
    /// `Premium` redeclares `withdraw` with `redefines` `Account/withdraw`.
    Redefines,
}

/// The `test/bank` fixture variants.
#[derive(Clone, Copy)]
struct Bank {
    premium: Premium,
    deposit: bool,
    /// A kept `requires` clause `amount <= self.balance` on `withdraw`.
    requires: bool,
}

const PLAIN: Bank = Bank {
    premium: Premium::Absent,
    deposit: false,
    requires: false,
};

fn id(path: &str) -> String {
    format!("ix://{BANK}/{path}")
}

fn generated(identity: &str) -> serde_json::Value {
    json!({"generated": {
        "generatorIdentity": identity,
        "generatorVersion": "1.0.0",
        "inputIdentities": [identity],
    }})
}

fn field(owner: &str, name: &str) -> serde_json::Value {
    let identity = id(&format!("{owner}/{name}"));
    json!({
        "identity": identity,
        "name": name,
        "typeRef": id("Amount"),
        "presence": "required",
        "nullable": false,
        "defaultKind": "none",
        "multiplicity": {"lower": 1, "upper": 1, "ordered": false, "unique": true},
        "origin": generated(&identity),
    })
}

/// `name(amount: Int[0, 100])` on `owner`, modifying `Account/balance`.
fn operation_of(
    owner: &str,
    name: &str,
    redefines: Option<&str>,
    requires: Option<&str>,
) -> serde_json::Value {
    let identity = id(&format!("{owner}/{name}"));
    let parameter = format!("{identity}/amount");
    let origin = json!({"source": {
        "sourceIdentity": id("spec"),
        "path": "spec.qspec",
        "startLine": 1,
        "startColumn": 1,
    }});
    let mut operation = json!({
        "identity": identity,
        "name": name,
        "params": [{
            "identity": parameter,
            "name": "amount",
            "typeRef": id("Amount"),
            "presence": "required",
            "nullable": false,
            "defaultKind": "none",
            "multiplicity": {"lower": 1, "upper": 1, "ordered": false, "unique": true},
            "origin": generated(&parameter),
        }],
        "pre": [],
        "post": [],
        "origin": origin.clone(),
        "frame": {"modifies": [id("Account/balance")], "creates": [], "deletes": []},
    });
    if let Some(requires) = requires {
        operation["pre"] = json!([{
            "language": "quire",
            "text": requires,
            "origin": origin,
        }]);
    }
    if let Some(parent) = redefines {
        operation["redefines"] = json!(id(&format!("{parent}/{name}")));
    }
    operation
}

fn object_type(
    name: &str,
    supertypes: &[&str],
    fields: Vec<serde_json::Value>,
    operations: Vec<serde_json::Value>,
) -> serde_json::Value {
    let identity = id(name);
    json!({
        "identity": identity,
        "displayName": identity,
        "kind": {"module": BANK, "name": "object_type"},
        "roles": [],
        "origin": generated(&identity),
        "constraints": [],
        "extensions": [],
        "unknownPolicy": "reject",
        "supertypes": supertypes.iter().map(|name| id(name)).collect::<Vec<_>>(),
        "fields": fields,
        "operations": operations,
    })
}

impl Bank {
    /// The `test/bank` domain package: `Account` with `balance` and `limit`
    /// (`Int[0, 100]`), the `accounts` population and `withdraw`.
    fn document(self) -> Vec<u8> {
        let amount = id("Amount");
        let bound = |keyword: &str, value: &str| {
            json!({
                "identity": format!("{amount}/constraints/{keyword}"),
                "keyword": keyword,
                "operands": {"value": value},
                "appliesTo": "ix://quire/native/Integer",
                "diagnosticCode": format!("bound.{keyword}"),
                "origin": generated(&amount),
            })
        };
        let requires = self.requires.then_some("amount <= self.balance");
        let mut account_operations = vec![operation_of("Account", "withdraw", None, requires)];
        if self.deposit {
            account_operations.push(operation_of("Account", "deposit", None, None));
        }
        let mut types = vec![
            json!({
                "identity": amount,
                "displayName": amount,
                "kind": {"module": BANK, "name": "value_type"},
                "roles": [],
                "origin": generated(&amount),
                "extensions": [],
                "unknownPolicy": "reject",
                "scalar": "integer",
                "constraints": [bound("min", "0"), bound("max", "100")],
            }),
            object_type(
                "Account",
                &[],
                vec![field("Account", "balance"), field("Account", "limit")],
                account_operations,
            ),
        ];
        match self.premium {
            Premium::Absent => {}
            Premium::Inherits => types.push(object_type("Premium", &["Account"], vec![], vec![])),
            Premium::Redefines => types.push(object_type(
                "Premium",
                &["Account"],
                vec![],
                vec![operation_of("Premium", "withdraw", Some("Account"), None)],
            )),
        }
        let population = id("accounts");
        let kind = |name: &str| {
            json!({
                "kind": {"module": BANK, "name": name},
                "moduleVersion": "1.0.0",
                "construct": {
                    "identity": "none",
                    "shape": "record",
                    "members": {},
                    "meaning": format!("quire.meaning.model.{}/v1", name.replace('_', "-")),
                },
            })
        };
        json!({
            "contractVersion": "2.0.0",
            "canonicalization": {"algorithm": "rfc8785-v1"},
            "source": {"identity": id("spec"), "version": "1.0.0", "dialect": "spec-bundle"},
            "package": {
                "identity": BANK,
                "version": "1.0.0",
                "mappingVersions": [],
                "profileVersions": [],
            },
            "occurrences": [],
            "extensions": [],
            "constructs": [kind("object_type"), kind("population"), kind("value_type")],
            "types": types,
            "populations": [{
                "identity": population,
                "displayName": population,
                "kind": {"module": BANK, "name": "population"},
                "members": [id("Account")],
                "extent": "closed",
                "origin": generated(&population),
            }],
        })
        .to_string()
        .into_bytes()
    }
}

const HEADER: &str = "language \"ix:native\" edition \"1-draft\";\n\
                      profile v = \"quire.model.complete/v1\";\n";

const CAN_AFFORD: &str =
    "pre CanAfford using v on Bank::Account::withdraw { amount <= self.balance }\n";
const DEBITED: &str = "post Debited using v on Bank::Account::withdraw \
                       { self.balance = pre(self.balance) - amount }\n";
const NON_NEGATIVE: &str =
    "invariant NonNegative using v on Bank::Account at current { self.balance >= 0 }\n";

/// The clause `kind name using v on Bank::<context>::withdraw { body }`.
fn clause(kind: &str, name: &str, context: &str, body: &str) -> String {
    format!("{kind} {name} using v on Bank::{context}::withdraw {{ {body} }}\n")
}

/// The bank unit's text over the given clauses.
fn unit(clauses: &[&str]) -> String {
    format!("{HEADER}model Bank = {BANK:?};\n{}", clauses.concat())
}

fn withdraw() -> OperationName {
    OperationName {
        model: identifier("Bank"),
        object: identifier("Account"),
        operation: identifier("withdraw"),
    }
}

/// `call_site` over the bank unit with `Bank::Account::withdraw`.
fn bank_site(bank: Bank, unit: &str) -> Result<CallSite<OperationSite>, Box<CallSiteRefusal>> {
    call_site(
        source(),
        "fixture".to_owned(),
        "bank.native",
        unit.as_bytes(),
        [bank.document().as_slice()],
        &DependencyInput::default(),
        &withdraw(),
    )
}

/// The first refusal of `result`, a `Compile` refusal: its message, and the
/// source text its region spans.
fn refusal(
    result: Result<impl std::fmt::Debug, Box<CallSiteRefusal>>,
    unit: &str,
) -> (String, String) {
    let refused = result.expect_err("the unit is refused");
    let CallSiteRefusal::Compile {
        code,
        message,
        region,
    } = *refused
    else {
        panic!("expected a Compile refusal, got {refused:?}");
    };
    assert_eq!(code, qsl_foundation::Code::UndefinedExpression, "{message}");
    assert!(message.contains("unproved-range"), "{message}");
    let region = region.expect("an unproved range is located");
    let text = unit
        .get(region.start() as usize..region.end() as usize)
        .expect("the region lies in the unit");
    (message, text.to_owned())
}

/// Assert that `unit` over `bank` refuses at the arithmetic node `at`.
fn refuses_at(bank: Bank, unit: &str, at: &str) -> String {
    let (message, text) = refusal(bank_site(bank, unit), unit);
    assert_eq!(text, at, "{message}");
    message
}

/// Assert that `unit` over `bank` checks.
fn checks(bank: Bank, unit: &str) {
    if let Err(refused) = bank_site(bank, unit) {
        panic!("refused: {refused:?}\n{unit}");
    }
}

/// The bank unit with `CanAfford`'s body replaced.
fn with_can_afford(body: &str) -> String {
    unit(&[
        &clause("pre", "CanAfford", "Account", body),
        DEBITED,
        NON_NEGATIVE,
    ])
}

/// The bank unit with `CanAfford`'s body and `Debited1` as the postcondition.
fn with_debited1(body: &str) -> String {
    unit(&[
        &clause("pre", "CanAfford", "Account", body),
        &clause(
            "post",
            "Debited1",
            "Account",
            "self.balance = (pre(self.balance) - amount) - 1",
        ),
        NON_NEGATIVE,
    ])
}

fn bank_unit() -> String {
    unit(&[CAN_AFFORD, DEBITED, NON_NEGATIVE])
}

fn without_can_afford() -> String {
    unit(&[DEBITED, NON_NEGATIVE])
}

const DIFFERENCE: &str = "pre(self.balance) - amount";

#[trace("TC-1416", "FR-577-AC-1")]
#[test]
fn the_precondition_discharges_the_postcondition_range() {
    checks(PLAIN, &bank_unit());
}

#[trace("TC-1416", "FR-577-AC-2")]
#[test]
fn a_precondition_declared_after_the_postcondition_discharges_it_with_the_same_identities() {
    let first = bank_unit();
    let moved = unit(&[DEBITED, CAN_AFFORD, NON_NEGATIVE]);
    let clause_nodes = |unit: &str| {
        let compiled = compose(
            source(),
            "bank.native",
            unit.as_bytes(),
            &qsl_semantics::model::intake::package_input([PLAIN.document().as_slice()]).unwrap(),
            &DependencyInput::default(),
            SpineLimits::default(),
        )
        .unwrap_or_else(|refusal| panic!("compile refused: {refusal:?}\n{unit}"));
        let mut nodes: Vec<_> = state_clause_nodes(compiled.package.graph())
            .into_iter()
            .map(|(node, kind)| (kind, node.key()))
            .collect();
        nodes.sort();
        nodes
    };
    assert_eq!(clause_nodes(&first), clause_nodes(&moved));
}

#[trace("TC-1416", "FR-577-AC-3", "FR-577-AC-4", "FR-121-AC-37")]
#[test]
fn without_the_precondition_the_difference_is_refused_with_operand_intervals_and_region() {
    let unit = without_can_afford();
    let message = refuses_at(PLAIN, &unit, DIFFERENCE);
    let operand = message.find(DIFFERENCE).expect("names the operand");
    let required = message
        .find("[0, 100]")
        .expect("names the required interval");
    let proved = message
        .find("[-100, 100]")
        .expect("names the proved interval");
    assert!(operand < required && required < proved, "{message}");
    // FR-121-AC-37: the region spans exactly the bytes of the node.
    let refused = bank_site(PLAIN, &unit).expect_err("refused");
    let CallSiteRefusal::Compile { region, .. } = *refused else {
        panic!("expected a Compile refusal");
    };
    let region = region.expect("located");
    let start = unit.find(DIFFERENCE).expect("in the unit");
    assert_eq!(
        (region.start() as usize, region.end() as usize),
        (start, start + DIFFERENCE.len())
    );
}

fn function_site(
    unit: &str,
    name: &str,
) -> Result<CallSite<crate::FunctionSite>, Box<CallSiteRefusal>> {
    call_site(
        source(),
        "fixture".to_owned(),
        "function.native",
        unit.as_bytes(),
        std::iter::empty::<&[u8]>(),
        &DependencyInput::default(),
        &QualifiedName::new(vec![identifier(name)]).expect("one segment"),
    )
}

#[trace("TC-1416", "FR-577-AC-5")]
#[test]
fn an_unbounded_operand_is_refused_with_infinite_ends() {
    let unit = format!(
        "{HEADER}function g using v(n: Integer, r: Int[0, 100]): Boolean pure {{ r = n - 1 }}\n"
    );
    let (message, text) = refusal(function_site(&unit, "g"), &unit);
    assert_eq!(text, "n - 1");
    assert!(message.contains("[-inf, +inf]"), "{message}");
}

#[trace("TC-1416", "FR-577-AC-6")]
#[test]
fn a_strict_precondition_discharges_a_tighter_postcondition() {
    checks(PLAIN, &with_debited1("amount < self.balance"));
}

#[trace("TC-1416", "FR-577-AC-7")]
#[test]
fn a_flipped_precondition_discharges_the_postcondition() {
    checks(PLAIN, &with_can_afford("self.balance >= amount"));
}

#[trace("TC-1416", "FR-577-AC-8")]
#[test]
fn a_negated_precondition_discharges_the_postcondition() {
    checks(PLAIN, &with_can_afford("not (amount > self.balance)"));
}

#[trace("TC-1416", "FR-577-AC-9")]
#[test]
fn the_smaller_bound_of_a_repeated_pair_in_one_clause_is_kept() {
    checks(
        PLAIN,
        &with_debited1("amount <= self.balance and amount < self.balance"),
    );
}

#[trace("TC-1416", "FR-577-AC-10")]
#[test]
fn the_smaller_bound_of_a_repeated_pair_across_clauses_is_kept() {
    let unit = unit(&[
        CAN_AFFORD,
        &clause("pre", "Strict", "Account", "amount < self.balance"),
        &clause(
            "post",
            "Debited1",
            "Account",
            "self.balance = (pre(self.balance) - amount) - 1",
        ),
        NON_NEGATIVE,
    ]);
    checks(PLAIN, &unit);
}

#[trace("TC-1416", "FR-577-AC-11")]
#[test]
fn a_join_keeps_the_larger_bound() {
    refuses_at(
        PLAIN,
        &with_debited1("amount < self.balance or amount <= self.balance"),
        "(pre(self.balance) - amount) - 1",
    );
}

#[trace("TC-1416", "FR-577-AC-12")]
#[test]
fn a_bound_on_one_alternative_only_is_dropped_at_a_join() {
    refuses_at(
        PLAIN,
        &with_can_afford("amount <= self.balance or amount <= 5"),
        DIFFERENCE,
    );
}

#[trace("TC-1416", "FR-577-AC-13")]
#[test]
fn a_guard_reading_pre_discharges_inside_the_postcondition() {
    let unit = unit(&[&clause(
        "post",
        "Debited",
        "Account",
        "amount <= pre(self.balance) implies self.balance = pre(self.balance) - amount",
    )]);
    checks(PLAIN, &unit);
}

#[trace("TC-1416", "FR-577-AC-14")]
#[test]
fn a_guard_reading_post_does_not_discharge_a_pre_read() {
    let unit = unit(&[&clause(
        "post",
        "Debited",
        "Account",
        "amount <= self.balance implies self.balance = pre(self.balance) - amount",
    )]);
    refuses_at(PLAIN, &unit, DIFFERENCE);
}

#[trace("TC-1416", "FR-577-AC-15")]
#[test]
fn a_precondition_of_another_operation_transfers_nothing() {
    let unit = unit(&[
        &clause("pre", "CanAfford", "Account", "amount <= self.balance")
            .replace("::withdraw", "::deposit"),
        DEBITED,
        NON_NEGATIVE,
    ]);
    let bank = Bank {
        deposit: true,
        ..PLAIN
    };
    refuses_at(bank, &unit, DIFFERENCE);
}

#[trace("TC-1416", "FR-577-AC-16")]
#[test]
fn an_equality_between_two_paths_yields_no_bound() {
    refuses_at(PLAIN, &with_can_afford("amount = self.balance"), DIFFERENCE);
}

#[trace("TC-1416", "FR-577-AC-17")]
#[test]
fn an_arithmetic_operand_yields_no_bound() {
    refuses_at(
        PLAIN,
        &with_can_afford("amount + 0 <= self.balance"),
        DIFFERENCE,
    );
}

#[trace("TC-1416", "FR-577-AC-18")]
#[test]
fn a_bound_does_not_refine_a_sum() {
    let unit = unit(&[
        &clause(
            "pre",
            "CanAfford",
            "Account",
            "self.balance + amount <= 100",
        ),
        &clause(
            "post",
            "Credited",
            "Account",
            "self.balance = pre(self.balance) + amount",
        ),
        NON_NEGATIVE,
    ]);
    refuses_at(PLAIN, &unit, "pre(self.balance) + amount");
}

#[trace("TC-1416", "FR-577-AC-19")]
#[test]
fn bounds_do_not_chain() {
    refuses_at(
        PLAIN,
        &with_can_afford("amount <= self.limit and self.limit <= self.balance"),
        DIFFERENCE,
    );
}

#[trace("TC-1416", "FR-577-AC-20")]
#[test]
fn a_let_that_is_not_an_alias_transfers_nothing() {
    let unit = unit(&[
        &clause(
            "pre",
            "CanAfford",
            "Account",
            "let x = amount + 1 in x <= self.balance",
        ),
        &clause(
            "post",
            "Debited",
            "Account",
            "let x = amount + 1 in self.balance = pre(self.balance) - x",
        ),
        NON_NEGATIVE,
    ]);
    refuses_at(PLAIN, &unit, "pre(self.balance) - x");
}

#[trace("TC-1416", "FR-577-AC-21")]
#[test]
fn a_let_aliasing_a_path_keys_its_facts_on_the_path() {
    checks(
        PLAIN,
        &with_can_afford("let b = self.balance in amount <= b"),
    );
}

#[trace("TC-1416", "FR-577-AC-22")]
#[test]
fn an_unreachable_precondition_transfers_nothing() {
    refuses_at(
        PLAIN,
        &with_can_afford("amount < 0 and amount <= self.balance"),
        DIFFERENCE,
    );
}

#[trace("TC-1416", "FR-577-AC-23")]
#[test]
fn an_empty_intersection_across_preconditions_transfers_nothing() {
    let unit = unit(&[
        &clause(
            "pre",
            "CanAfford",
            "Account",
            "amount <= 3 and amount <= self.balance",
        ),
        &clause("pre", "Big", "Account", "amount >= 5"),
        DEBITED,
        NON_NEGATIVE,
    ]);
    refuses_at(PLAIN, &unit, DIFFERENCE);
}

#[trace("TC-1416", "FR-577-AC-24")]
#[test]
fn a_precondition_on_a_subtype_context_transfers_nothing() {
    let bank = Bank {
        premium: Premium::Inherits,
        ..PLAIN
    };
    let unit = unit(&[
        &clause("pre", "CanAfford", "Premium", "amount <= self.balance"),
        DEBITED,
        NON_NEGATIVE,
    ]);
    refuses_at(bank, &unit, DIFFERENCE);
}

#[trace("TC-1416", "FR-577-AC-25")]
#[test]
fn a_postcondition_on_a_subtype_context_receives_nothing() {
    let bank = Bank {
        premium: Premium::Inherits,
        ..PLAIN
    };
    let unit = unit(&[
        CAN_AFFORD,
        &clause(
            "post",
            "Debited",
            "Premium",
            "self.balance = pre(self.balance) - amount",
        ),
        NON_NEGATIVE,
    ]);
    refuses_at(bank, &unit, DIFFERENCE);
}

#[trace("TC-1416", "FR-577-AC-26")]
#[test]
fn a_redefined_operation_transfers_nothing() {
    let bank = Bank {
        premium: Premium::Redefines,
        ..PLAIN
    };
    refuses_at(bank, &bank_unit(), DIFFERENCE);
}

#[trace("TC-1416", "FR-577-AC-27")]
#[test]
fn a_redefining_operation_transfers_nothing() {
    let bank = Bank {
        premium: Premium::Redefines,
        ..PLAIN
    };
    let unit = unit(&[
        &clause("pre", "CanAfford", "Premium", "amount <= self.balance"),
        &clause(
            "post",
            "Debited",
            "Premium",
            "self.balance = pre(self.balance) - amount",
        ),
        NON_NEGATIVE,
    ]);
    refuses_at(bank, &unit, DIFFERENCE);
}

#[trace("TC-1416", "FR-577-AC-28")]
#[test]
fn a_precondition_receives_no_precondition_fact() {
    let unit = unit(&[
        CAN_AFFORD,
        &clause(
            "pre",
            "Fits",
            "Account",
            "self.limit = self.balance - amount",
        ),
        DEBITED,
        NON_NEGATIVE,
    ]);
    refuses_at(PLAIN, &unit, "self.balance - amount");
}

#[trace("TC-1416", "FR-577-AC-29")]
#[test]
fn an_invariant_receives_no_precondition_fact() {
    let unit = unit(&[
        &clause(
            "pre",
            "CanAfford",
            "Account",
            "amount <= self.balance and self.limit <= self.balance",
        ),
        DEBITED,
        NON_NEGATIVE,
        "invariant Room using v on Bank::Account at current \
         { self.limit = self.balance - self.limit }\n",
    ]);
    refuses_at(PLAIN, &unit, "self.balance - self.limit");
}

#[trace("TC-1416", "FR-577-AC-30")]
#[test]
fn a_kept_requires_clause_transfers_nothing() {
    let bank = Bank {
        requires: true,
        ..PLAIN
    };
    refuses_at(bank, &without_can_afford(), DIFFERENCE);
}

#[trace("TC-1416", "FR-577-AC-31")]
#[test]
fn a_function_body_guard_discharges_its_difference() {
    let unit = format!(
        "{HEADER}function f using v(a: Int[0, 100], b: Int[0, 100], r: Int[0, 100]): Boolean pure \
         {{ a <= b implies r = b - a }}\n"
    );
    if let Err(refused) = function_site(&unit, "f") {
        panic!("refused: {refused:?}");
    }
}

/// A `withdraw` snapshot of the one account `a` holding `balance` and a zero
/// `limit`, at `observation`.
fn bank_snapshot(label: &DocumentRef, observation: &str, balance: i64) -> Vec<u8> {
    json!({
        "format": "quire.state.snapshot/v1",
        "identity": document_identity_json(label),
        "observation": observation,
        "model": {"identity": BANK, "version": "1.0.0"},
        "populations": [{
            "population": id("accounts"),
            "complete": true,
            "objects": [{
                "key": "a",
                "type": id("Account"),
                "fields": {
                    "balance": {"integer": balance.to_string()},
                    "limit": {"integer": "0"},
                },
            }],
        }],
    })
    .to_string()
    .into_bytes()
}

#[trace("TC-1416", "FR-577-AC-32")]
#[test]
fn a_replay_outside_the_precondition_is_inconclusive_with_no_value() {
    let unit = unit_for(bank_unit(), PLAIN.document());
    let (pre, pre_bytes) =
        document_ref_and_bytes("bank-pre", |label| bank_snapshot(label, "pre", 3));
    let (post, post_bytes) =
        document_ref_and_bytes("bank-post", |label| bank_snapshot(label, "post", 0));
    let (invocation, invocation_bytes) = document_ref_and_bytes("bank-invocation", |label| {
        json!({
            "format": "quire.state.invocation/v1",
            "identity": document_identity_json(label),
            "model": {"identity": BANK, "version": "1.0.0"},
            "context": id("Account"),
            "operation": "withdraw",
            "self": {"population": id("accounts"), "key": "a"},
            "pre": {"identity": document_identity_json(&pre)},
            "post": {"identity": document_identity_json(&post)},
            "parameters": {"amount": {"integer": "5"}},
            "result": serde_json::Value::Null,
            "created": [],
            "deleted": [],
        })
        .to_string()
        .into_bytes()
    });
    let documents = state_clause_replay::Documents {
        observation: ClauseSelectionInput::Invocation {
            invocation: invocation.clone(),
        },
        read: vec![invocation.clone(), pre.clone(), post.clone()],
        bytes: vec![
            (invocation, invocation_bytes),
            (pre, pre_bytes),
            (post, post_bytes),
        ],
    };
    let result = state_clause_replay::replay(state_clause_replay::case(unit, "Debited", documents))
        .expect("the replay settles");
    let arm = state_clause_replay::witness_arm(&result);
    assert_eq!(arm.settlement(), crate::WitnessSettlement::Inconclusive);
    assert!(
        matches!(
            arm.disagreement(),
            Some(crate::DisagreementCause::NoValue { .. })
        ),
        "{:?}",
        arm.disagreement()
    );
    assert_eq!(arm.value(), None);
}

/// The bank unit with the protocol `P`, whose one attempt of `withdraw` lists
/// `contracts [Debited]` only.
fn bank_protocol_unit() -> String {
    let unit = unit(&[CAN_AFFORD, DEBITED, NON_NEGATIVE]);
    let unit = unit.replacen(
        "profile v = ",
        "profile pp = \"quire.protocol.finite-global/v1\";\nprofile v = ",
        1,
    );
    format!(
        "{unit}protocol P using pp over (k: Bank::Account) on origin {{\n\
         role w on Bank::Account;\n\
         run sequence Main {{ attempt X by w on Bank::Account::withdraw \
         contracts [Debited] as (x: Boolean) {{ true }}; }}\n\
         finish Done as (d: Boolean) {{ d }};\n\
         }}\n"
    )
}

/// The canonical spelling of the argument `amount = 5`.
const FIVE: &str = r#"{"type":"integer","value":"5"}"#;

#[trace("TC-1416", "FR-577-AC-33")]
#[test]
fn an_attempt_outside_the_precondition_records_one_finding_per_candidate_post_state() {
    use qsl_eval::model_subject::{ModelInstance, ModelSubject, ProtocolSubject, RoleBinding};
    use qsl_eval::simulation::model::{
        ExpansionLimits, ModelFinding, ModelSimulationRequest, ModelSystem, PopulationUniverse,
    };
    use qsl_eval::simulation::protocol::{ProtocolSystem, ResolvedMemory, Sc};
    use qsl_eval::simulation::TransitionSystem;

    let compiled = unit_for(bank_protocol_unit(), PLAIN.document()).compiled;
    let package = compiled.package;
    let packages =
        qsl_semantics::model::intake::package_input([PLAIN.document().as_slice()]).unwrap();
    let accounts = quire_checked::key::DeclarationKey {
        package: BANK.to_owned(),
        node: id("accounts"),
    };
    let bytes = json!({
        "format": "quire.state.snapshot/v1",
        "identity": {"authority": "test", "identity": "start"},
        "observation": "current",
        "model": {"identity": BANK},
        "anchor": {"kind": "initialization", "name": "start"},
        "populations": [{
            "population": id("accounts"),
            "complete": true,
            "objects": [{
                "key": "a",
                "type": id("Account"),
                "fields": {"balance": {"integer": "3"}, "limit": {"integer": "0"}},
            }],
        }],
    })
    .to_string()
    .into_bytes();
    let snapshots = BTreeMap::from([(
        qsl_semantics::model::observation::document_reference(&bytes).expect("names itself"),
        bytes,
    )]);
    let simulation = ModelSimulationRequest {
        emit: qsl_package::EmitLimits::fixture(),
        package: &package,
        packages: &packages,
        normalization: qsl_semantics::model::accounting::ModelNormalizationLimits::default(),
        snapshots: &snapshots,
        initial: vec![DocumentRef {
            authority: "test".to_owned(),
            identity: "start".to_owned(),
        }],
        universes: vec![PopulationUniverse {
            population: accounts.clone(),
            keys: vec!["a".to_owned()],
        }],
        observation: qsl_semantics::model::observation::ObservationLimits::default(),
        expansion: ExpansionLimits {
            max_candidates: 1_000_000,
        },
        meter: quire_exact::ScalarLimits {
            integer_bits: 1 << 20,
            decimal_digits: 1 << 20,
            scale_expansion: 1 << 20,
            text_input_bytes: 1 << 20,
            text_scalars: 1 << 20,
            normalized_scalars: 1 << 20,
            unit_edges: 1 << 20,
            value_occurrences: 1 << 20,
            work_units: 1 << 30,
            result_units: 1 << 30,
        },
    };
    let subject = ProtocolSubject {
        model: ModelSubject {
            simulation: simulation.clone(),
            proof_bounds: Vec::new(),
        },
        protocol: "P".to_owned(),
        over: ModelInstance {
            population: accounts,
            key: "a".to_owned(),
        },
        roles: vec![RoleBinding {
            role: "w".to_owned(),
            model: BANK.to_owned(),
        }],
        memory: package
            .graph()
            .protocol("P")
            .map(|checked| ResolvedMemory::resolve(checked, &[]).expect("no selection resolves"))
            .unwrap_or_default(),
    };
    let model = ModelSystem::new(simulation).expect("the subject admits");
    let system = ProtocolSystem::new(model, &subject, Sc).expect("the protocol system builds");
    let expanded = system
        .successors(&system.initial()[0])
        .expect("the state expands");
    // The argument vector `amount = 5`: the postcondition is refused at every
    // candidate post-state, and the application gives no successor.
    let undetermined: Vec<_> = expanded
        .findings
        .iter()
        .filter_map(|finding| match finding {
            ModelFinding::ContractUndetermined {
                transition,
                candidate,
                clause,
                ..
            } if clause == "Debited" && transition.arguments() == [FIVE] => {
                Some((transition.clone(), candidate.clone()))
            }
            _ => None,
        })
        .collect();
    assert_eq!(undetermined.len(), 101, "{:?}", expanded.findings.first());
    let candidates: BTreeSet<_> = undetermined
        .iter()
        .map(|(_, candidate)| candidate.clone())
        .collect();
    assert_eq!(candidates.len(), 101);
    assert!(expanded.successors.iter().all(|(transition, _)| {
        transition
            .application()
            .is_some_and(|application| application.arguments() != [FIVE])
    }));
}
