---
id: SR-977
title: "Code and Rust review of QSL-602 CheckMode and operation keys"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language; QSL-602; branch qsl-602-checkmode @ 3c43483e2 vs origin/main 52d2c36c1; qsl-semantics/src/check/{mod.rs,facts.rs,family.rs,lowering.rs,check/typing.rs}; qsl-semantics/tests/it/check_mode.rs; 228 mechanical call-site files"
review_set: subset
evaluated_revision: "3c43483e221ada7c794bc3fcff83540cbb8dc2ab"
---

## Summary

Ticket: QSL-602. Two commits: 13a74be09 adds a `CheckMode` parameter to
`PackageDeclarations::check` and its four variants (FR-495 Inputs,
FR-562-AC-3/AC-6), and 3c43483e2 adds `CheckedGraph::operation_key` (FR-093)
and the lowering that records it. Methods: code-review with the rust-review
lane (rust-style baseline), one pass, `git diff origin/main...HEAD` only.
Tools: quire 0.38.3 (engine 0.52.1), quoin 0.29.4. Ran
`cargo test -p qsl-semantics --test it check_mode`: 3 passed. No full gate run.

Mechanical rewrite: a normalizer that strips whitespace, every
`<path>::CheckMode::Linked` argument, `CheckMode` imports and trailing commas
showed that 228 of the 237 changed files differ from origin/main only by an
inserted `CheckMode::Linked`. No added line outside the 9 non-mechanical files
names any other mode. `Definedness::kernel` defaults to `false`, so Linked is
the old behaviour and the rewrite preserves behaviour. The 9 files that are
not purely mechanical are check/mod.rs, facts.rs, family.rs, lowering.rs,
check/typing.rs, the two expression_depth test helpers (a closure rewrapped by
rustfmt), tests/it/main.rs (a `mod` line) and the new tests/it/check_mode.rs.

## Verdict

Mergeable after the medium findings are fixed. The Kernel semantics are
correct. `Definedness::discharge` swallows only
`CheckCause::Unproved(_)`, and only when `kernel` is set. Call and dispatch
edges are pushed by the `Call` and `Dispatch` arms of `discharge_node`,
which never return `Unproved`, and the walk discharges each node separately,
so a dropped obligation never skips an operand or a call site. Termination
therefore sees the same call graph. `UnprovedDecrease` and every typing,
dispatch and limit refusal still refuse.

Kernel scope matches FR-495. `mode` reaches only `ValueDeclarations`, and
from there only `check_declaration_body`'s two `Definedness` walks (body and
measure). State-clause, protocol, temporal, hyper and probabilistic
`Definedness::new` calls never call `with_check_mode`, so they stay Linked,
as FR-104 "Definedness facts" requires. `check_clause_expression` keeps its
own existing mode gate.

The evaluator meets each dropped obligation at run time: `Coerce` gives
`IntegerOutOfDomain`, `value(none)` gives `Undefined::NoneValue` and an empty
reduce gives `Undefined::EmptyReduction`. A Kernel program therefore cannot
reach a checked-invariant fault through a dropped obligation.

Confirmed, and not a finding of this diff: facts.rs has no Definedness arm
for `NodeKind::IntegerDivision`. Such a node falls through to `_ => Ok(())` in
`discharge_node`, and the interval walk gives it its declared type, so
FR-495-AC-1 and AC-3 (integer `div`/`rem`/`mod` range and nonzero) remain
unimplemented under Linked. That is pre-existing. The commit subject cites
FR-562-AC-3, which this diff cannot satisfy for that reason.

`operation_key` is recorded in `Lowering::application` only, so helper nodes
built through `typed_application` (such as the deref under an attribute read)
do not overwrite the key of the outer operation. Draft keys resolve in
`settle` through the same `resolve` as binders. No compat shims or pins were
found, and no strings are matched.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The FR-093 accessor test does not implement FR-093-AC-62's vector: it does not use `(a + b) * (a + b)`, and it has no `a * b` function whose key differs from all three. No test in check_mode.rs has a `#[trace(..)]` attribute (the repo convention, for example `#[trace("FR-093-AC-59", "TC-1335")]`), so the matrix still shows FR-093-AC-62 and every FR-495/FR-562 criterion as untested. Fix: add the AC-62 vector tagged `#[trace("FR-093-AC-62", "TC-415")]`, and tag the Kernel tests only with the criteria they fully assert. FR-562-AC-6 has a JIT half that this test cannot reach, so leave it out of the tag or note it as partial. | qsl-semantics/tests/it/check_mode.rs:53-120 |
| FND-002 | medium | `kernel_mode_still_refuses_a_nonterminating_recursion` asserts only `is_err()` in both modes. It never checks that the refusal is the termination refusal, and the module doc says "Every assertion compares typed causes and locations". It also does not test the property it names, because `f(x)` returning `Int[0, 9]` carries no obligation that Kernel drops. Fix: assert the termination cause (missing-measure) and its location. Add a body such as `f(x) + 1` with result `Int[0, 9]`: Linked refuses `unproved-range` first, while Kernel must still reach the termination refusal. | qsl-semantics/tests/it/check_mode.rs:69-76 |
| FND-003 | medium | Kernel is tested only for `Obligation::Range` on `+`. No test shows Kernel dropping a nonzero obligation (FR-495-AC-3's decimal `d2`/`d4` or quantity `d3` divisors, which facts.rs does refuse under Linked), a presence obligation or a nonempty-reduction obligation. No test shows that a state-clause or hyper body under `CheckMode::Kernel` still refuses, so the scope boundary is untested. Fix: add Linked-refuses / Kernel-checks pairs for `d4` and `value(x)` over an `Option`, and a Kernel control that an invariant body still refuses `unproved-nonzero`. | qsl-semantics/src/check/facts.rs:1197-1206 |
| FND-004 | medium | The pending-draft branch of `record_operation_key` and its resolution in `settle` have no test, and neither does the key of a `call` application. FR-560/TC-1381 name `Operation(k)` with `k` the key of a `call`, so the backend depends on exactly that key. A wrong or missing key on a recursive or forward-referenced callee would pass every current test. Fix: assert `operation_key` for a call to a later-declared or self-recursive function (the case that lowers through a draft) and check that it equals the call's application node key. | qsl-semantics/src/check/lowering.rs:3862-3865, 4179-4186 |
| FND-005 | low | The set of obligations that Kernel drops is stated three different ways. FR-495 says "no range or nonzero obligation". The doc comments on `PackageDeclarations::check` and `discharge` say "range, nonzero or presence". The code drops every `Obligation`, including `NonemptyReduction` and `RationalRange`. The code matches quire-semantic-value's `CheckMode::Kernel` doc ("typing only, so an empty reduce or value(none) is a located undefined outcome"). Fix: make both doc comments name every dropped obligation, and add presence and nonempty reduction to FR-495's Kernel bullet, or narrow the match to the obligations the spec names. | qsl-semantics/src/check/mod.rs:1786-1789 |
| FND-006 | low | `kernel: bool` is set by `mode == CheckMode::Kernel`, which is not an exhaustive match: a third `CheckMode` variant would silently get Linked behaviour. Fix: store the `CheckMode` and `match` on it in `discharge`, or set the flag with an exhaustive `match mode { Linked => false, Kernel => true }`. | qsl-semantics/src/check/facts.rs:535-537, 558-565 |
| FND-007 | low | The diff reformats lines this change does not touch: the `integer_widens` arm and three spots in `checking_tests` (a `let-else` pattern and an `assert!`). This is rustfmt churn outside the ticket that inflates an already large mechanical diff. Fix: drop those hunks or land them separately. | qsl-semantics/src/check/check/typing.rs:541-543; qsl-semantics/src/check/family.rs:2782, 2813-2816 |

## Coverage

- FR-495 (examined): the Inputs CheckMode bullet and the "Bounded result domain, two observations" and "Zero divisor, two observations" Kernel bullets
- FR-495-AC-1 (context_only): IntegerDivision range, confirmed pre-existing gap
- FR-495-AC-3 (context_only): IntegerDivision nonzero, confirmed pre-existing gap
- FR-562-AC-3 (examined): d0 checked under Kernel; not reachable until the IntegerDivision gap closes
- FR-562-AC-6 (examined): checker half covered by an_unproved_result_bound_checks_only_under_kernel
- FR-093 (examined): the accessor paragraph at FR-093:131-137
- FR-093-AC-62 (examined)
- FR-104 (context_only): "Definedness facts" keeps state clauses Linked
- Mechanical call-site rewrite across 228 files (examined, clean)
