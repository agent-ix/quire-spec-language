---
id: SR-17210
title: "QSL-1072 PR #1532 code review: lower hyper holds atoms and match conjuncts to checked nodes"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@95a7da3c2; git diff origin/main...HEAD (PR #1532, branch qsl-1072-lowering-code): qsl-semantics/src/check/{assemble.rs,check.rs,check/typing.rs,facts.rs,hyper_clause.rs,hyper_clause/lowering.rs,hyper_typing.rs,mod.rs}, qsl-eval/src/hyper/{checked.rs,joint.rs,mod.rs,product.rs,seam.rs,step_value.rs}, qsl-eval/src/simulation/model/{hyper_letters.rs,letters.rs,encode.rs,model.rs}, qsl-eval/src/value/expression/{mod.rs,evaluate.rs}, qsl-analyze/src/model_check/{hyper.rs,hyper/single.rs}, qsl-replay/src/model_check/hyper_replay.rs, qsl-semantics/tests/it/hyper_clauses.rs, qsl-analyze/tests/it/{hyper.rs,hyper_lowering.rs,hyper_world.rs,hyper_witness.rs,hyper_single.rs,hyper_projected.rs,hyper_replay.rs,hyper_swap.rs,hyper_settle.rs}, qsl-eval/tests/it/hyper_tuple.rs; spec FR-487 AC-1..12, FR-172-AC-5/7/8, FR-315-AC-43, TC-1264"
review_set: subset
date: "2026-10-25"
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-487
    type: references
---

## Summary

Ticket: QSL-1072. PR: quire-spec-language#1532 at 95a7da3c2. Code review with the
rust-review lane folded in. The lowering and the two production seams are real
implementations: atoms get per-(trace, parameter) slots, a kept checked condition and a
per-read component map; conjuncts get step-label slots and a checked condition; the
product, analyze and replay read `Halt::Atom`, `ProductHalt::Atom` and
`MatchReading::Refused`. The reverted QSL-867/#1521 route (`type_body`, `HyperType`
markers) is not revived. Tests use `#[trace]` attributes, no `/// Trace:` lines.

Gates run (own target dir, under `flock /tmp/qsl-cargo.lock`): `cargo test -p
qsl-semantics --features test-support --test it hyper_clauses` 27 passed; `cargo test -p
qsl-analyze --test it hyper` 179 passed. The PR merged with current main (which now
contains #1527, QSL-1037) merges with no textual conflict, and on that merge `cargo check
--tests` for the four crates, `cargo clippy --all-targets -- -D warnings` (0 warnings),
`cargo fmt --all --check` and the semantics hyper tests all pass.

Mutation check of the new tests (one mutant at a time, reverted after each):

| Mutant | Result |
| --- | --- |
| `facts.rs`: `Step::Component` replaced by `Step::Attribute` (definedness ignores components) | SURVIVED |
| `evaluate_hyper_holds`: slot-length check removed | SURVIVED |
| `lowering.rs`: component index reversed | killed (4 analyze tests; no semantics test) |
| `CheckedStepMatch`: no short-circuit after a false conjunct | killed (AC-5 test only; the AC-4 test passes) |
| label constant lowered to a non-operation text | SURVIVED |
| label misuse check disabled | killed (semantics AC-2/AC-5 tests) |

## Verdict

**FAIL**: two `high` findings. FND-001 is a hand-written third copy of the
declaration-identity canonical text, and the duplication rule makes that `high`.
FND-002 is a decoder that refuses checked conjuncts at run time. The remaining
findings are wrong error channels, a missing source region, and tests that do not
detect real regressions. Merge conflict with #1527: none. #1527 has merged, and the
merged tree builds and passes.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | `lowering::operation_text` hand-formats the RFC 8785 declaration identity text (`{"digest_domain":"sha256-jcs","node":…,"package":…}`) with `format!` and `serde_json::to_string`. That is a third statement of a form already owned by `qsl-semantics/src/model/key.rs` (the `DeclarationKey` preimage) and by `qsl-eval` `encode::declaration_enc`/`declaration_text`, which `ModelSystem::operation_text_of` serves. `step_value::slot_value` compares `ModelTransition::operation()` against it and label constants lower to it, so if either side drifts, every argument read silently reads absent and every label constant compares false. | qsl-semantics/src/check/hyper_clause/lowering.rs:68, qsl-eval/src/hyper/step_value.rs:46, qsl-semantics/src/check/hyper_clause.rs:1821 |
| FND-002 | high | `step_value::decode` decodes only Boolean, Integer/Int, String, Reference and Option. `value_enc` also writes Rational, Quantity, Float, Text and collections as argument text, and S3 types `t.step.O.p` at any declared parameter type. A conjunct that reads such an argument checks but faults (`step-label-decodes-at-its-type`) at every `O` step, and `CheckedStepMatch` turns that fault into `MatchReading::Refused`, so the product reports `MatchUndetermined` for a clause S3 accepted. The decoder is also a second, hand-rolled statement of the FR-181 wire format (prefix strings and its own JSON `unquote`) beside `encode.rs`. | qsl-eval/src/hyper/step_value.rs:67-131, qsl-semantics/src/check/hyper_clause.rs:2033-2046 |
| FND-003 | medium | A value-typer or definedness refusal of a `match` conjunct (from `lower_conjunct`) is located at `hyper <c> match <k>`. `assemble.rs` registers site spans only for `hyper <c> holds <n>`, so `PackageDeclarations::region` returns `None` for it (the `TypeDeclaration` arm finds no site, and no type span exists at depth 0). The refusal reaches the user with no source region. FR-487 Outputs requires a refusal "with a span". `locate = \|_\| draft.span` only orders refusals, and is not carried on the `CheckRefusal`. | qsl-semantics/src/check/assemble.rs:3591-3609, qsl-semantics/src/check/hyper_clause.rs:1185, qsl-semantics/src/check/region.rs:129-132 |
| FND-004 | medium | `qsl-replay` `hyper_replay::halted` sends the new `Halt::Atom(_)` to the `Some(_) => Ok(no_value())` wildcard. `Halt::Atom(AtomStop::Fault(..))`, a broken evaluator invariant, therefore settles as "no value", while `Halt::Fault` on the line above is a replay fault. The variant was added without updating this exhaustive reading. | qsl-replay/src/model_check/hyper_replay.rs:739-740, qsl-eval/src/hyper/joint.rs:173-178 |
| FND-005 | medium | `CheckedStepMatch::read` maps every `Err` of `evaluate_step_predicate` (an out-of-range conjunct, a wrong label count, an undecodable label: each an `InternalFault` by FR-487) to `MatchReading::Refused { outcome: FamilyRefused(BROKEN) }` and drops the fault. The products then read a broken invariant as a refused conjunct (FR-176 `MatchUndetermined`) and not as a fault, and FND-002's decode failures are hidden this way. | qsl-eval/src/hyper/checked.rs:58-65 |
| FND-006 | medium | `hyper_typing::type_condition`, main's forward `Ty` pass, still runs over every atom before the lowering. It still refuses `Call`, `Size`, `Case`, `Lookup`, `Count` and others as unsupported, although the value typer that now lowers the atom types them. FR-487 "Dependencies" says the lowering replaces `type_condition`, and FR-172 "Body expressions" types atoms under the value typer's rules. The PR lifts only `present`/`value`/`deref`. | qsl-semantics/src/check/hyper_typing.rs:386-413, qsl-semantics/src/check/hyper_clause.rs:1091-1112 |
| FND-007 | low | `HyperDeclarations.nodes_used` is seeded from the package total, and the lowered conditions charge it, but the value is never written back. The later `with_node_limit(limits.nodes(), nodes_used)` runs on the pre-hyper total. This is latent today, because every checked hyper clause refuses `not_emitted`, but it becomes a node-ceiling leak once QSL-1071 emits. | qsl-semantics/src/check/mod.rs:2883, qsl-semantics/src/check/mod.rs:2990 |
| FND-008 | medium | Per-component definedness (`Step::Component` in `Definedness::stable_path`) is untested. A mutant that ignores components survives every PR test. The AC-10 test's two reads already have different slot roots (`v@a`, `v@b`), so it passes either way. It needs a case where one reference is read in two components, such as `present(deref(value(v.next @ a)).p @ a) and value(deref(value(v.next @ a)).p @ b) = 1`. | qsl-semantics/src/check/facts.rs:494-500, qsl-semantics/tests/it/hyper_clauses.rs:1438 |
| FND-009 | medium | The FR-487-AC-1 test checks only the set `{0, 1}` of components, the slot counts, the identity and the slot list. It does not check which read gets which component, the condition's shape (an equality of two `l` reads, no `@` node), or the equality of conditions and component maps on the second check. A reversed component map passes every semantics test; only four analyze tests catch it. | qsl-semantics/tests/it/hyper_clauses.rs:1315-1356 |
| FND-010 | medium | The FR-487-AC-3 wrong-length case passes `components = &[]` with the 1-element `slots`, so the component check faults first. With the slot-length check removed, the test still passes (mutant survived). The zero-budget case asserts only `AtomStop::Incomplete(_)`, not `resource_exhausted`/`insufficient-next-charge`. | qsl-analyze/tests/it/hyper_lowering.rs:102-117 |
| FND-011 | medium | The FR-487-AC-4 test does not detect a missing short-circuit: the no-short-circuit mutant passes it, and only the AC-5 test catches it. Its only charge assertion is `first_conjunct < both_conjuncts`, so the AC's "conjunct 0's charges only" claim for `(step(0), stutter)` is not checked. | qsl-analyze/tests/it/hyper_lowering.rs:171-214 |
| FND-012 | medium | The evaluated value of a label constant is untested. Lowering `V::Vault::step` to an arbitrary non-operation text survives every test. The AC-2 and FR-172-AC-5 tests check slots and refusals only. The FR-176-AC-2 fixture `a.step.op = V::Vault::step and b.step.op = stutter` gives `VacuousMatch` whether or not the constant ever equals a step. AC-2's "compares its one slot with the literal operation text of `step`" and FR-172-AC-5's "holds at exactly the joint steps at which …" are not asserted, and neither are AC-2's `Option<Int[0, 1]>` and `Option<Reference<Vault>>` slot types. | qsl-semantics/tests/it/hyper_clauses.rs:1362-1430, qsl-semantics/tests/it/hyper_clauses.rs:1462-1505, qsl-analyze/tests/it/hyper.rs:216 |
| FND-013 | medium | The FR-172-AC-8 test omits the criterion's `deref(value(v.next @ a)).l = 1` refusal (`ill_typed` at `.l`) and does not read `component_reads()`. Its `let` fixture compares `value(r) = value(r)` and not `deref(value(r)).l = 1`, so the new `Deref` branches of `check_atom` and the recording of a `deref` read's component run in no test. | qsl-semantics/tests/it/hyper_clauses.rs:1536-1553, qsl-semantics/src/check/hyper_clause.rs:1651-1657 |
