---
id: SR-13605
title: "Code review of quire-spec-language PR #1123: QSL-555 E14c3, copy-swap canonicaliser for hyper checks (FR-174)"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@2f4479baf; PR #1123 (git diff origin/main...2f4479baf): qsl-eval/src/hyper/{copy_swap,product,mod}.rs, qsl-eval/src/automaton/{formula,translate,mod}.rs, qsl-analyze/src/model_check/hyper.rs, qsl-analyze/src/model_check/hyper/{lift,reductions}.rs, qsl-replay/src/model_check/{hyper_outcome,hyper_replay}.rs, qsl-replay/src/{lib,proof_result}.rs, qsl-semantics/src/check/{hyper_clause,mod}.rs, qsl-analyze/tests/it/{hyper_swap,hyper,hyper_world,main}.rs, qsl-semantics/tests/it/hyper_clauses.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-174
    type: reviews
---
# SR-13605: Code review of quire-spec-language PR #1123: QSL-555 E14c3, copy-swap canonicaliser for hyper checks (FR-174)

## Summary

Ticket: QSL-555 (E14c3). This is the code review, with the rust-review lane folded in.

Gates run at 2f4479baf in a fresh clone with its own `CARGO_TARGET_DIR`. I did not run `make ci`. Every gate passed, with 0 tests failed (4,868 passed, 7 ignored in all):

- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test -p qsl-eval -p qsl-replay -p qsl-analyze -p qsl-semantics`
- root `cargo test --test it` (975 passed, 6 ignored)
- `make string-edge`
- `make arch-lint-api-surface-qsl`
- `make arch-lint-canonical-encoder`
- `make arch-lint-qualified-core`

The branch merges cleanly onto main at 10a7af550, one commit ahead of its base.

I compared the quotient check with the unreduced one in a separate scratch copy that was never pushed (`qsl-analyze/tests/it/hyper_swap_probe.rs` at /private/tmp/claude-501/-Users-peter-dev/be6a486a-dbff-4734-b40c-5d7654187d9c/scratchpad/rv1123-probe). There, `NO_SWAP=1` withholds the copy group from `HyperProduct`, so the same `check_hyper` runs unreduced.

- **The models.** Random vault contracts: 7 `step` contracts (set, leaky, flip, invert, enabled at `l = 0` only, enabled at `l = 1` only, constant) and, half the time, 4 `reset` contracts with and without a precondition. Random starts, 1 to 4 of the `(h, l)` pairs.
- **The clauses.** 2 or 3 copies. Random fairness: none, or weak or strong on `step` or `reset`, the same on every copy, or missing on the first copy (asymmetric). Random `match`: none, op equality, or op and input equality. 19 two-copy bodies and 6 three-copy bodies: safety, liveness, persistence, `eventually` conjunctions whose swap permutes the automaton, `always` conjunctions whose swap exchanges acceptance sets, asymmetric bodies, past operators (`once`, `since`) and interval operators (`eventually[0,2]`, `eventually[0,3]`).
- **What it checks.** The verdict class (`Holds`/`HoldsReduced` against `Holds`, `Violated` with the same counterexample kind, the same `Undecided` cause), reduced states <= unreduced states, and that every reduced counterexample replays `Reproduced` by FR-183.
- **Result.** 1,250 random cases over 7 seeds (7, 11, 23, 41, 53, 67, 89); 19 were refused by the fixture seams (an atom with `!=`) and dropped. Of the 1,231 that ran, 774 ran reduced. 1,229 agree, and every reduced counterexample among them replays `Reproduced`. 24 of the agreeing refutations have a different canonical lasso from the unreduced run (`trace_position` 1 against 0, 5 against 2, and so on); gap FND-009.
- **2 cases disagree, and both have weak fairness on `reset` on the swapped copies.** One is a false proof and one a spurious refutation whose counterexample replay refuses as unfair. A targeted probe gives two more false proofs. This is FND-001.

So, without fairness, the quotient is sound on these models: same verdicts, lifted counterexamples that replay, loops that close on permuted states, three copies, and bodies that permute automaton states. Swaps that exchange acceptance sets fall back to the whole product, and the oracle agrees on all of those.

Mutation testing of the PR's own tests: 36 hand-made mutants of lift.rs, hyper.rs, reductions.rs, copy_swap.rs, translate.rs, formula.rs and hyper_clause.rs. Each was run against `qsl-analyze --test it hyper`, plus `qsl-semantics --test it hyper` for hyper_clause.rs. 27 were killed and 9 survived. A 37th mutant (L3) was equivalent as I wrote it; I replaced it with L3b and do not count it.

- **27 killed:**
  - lifting: L1, the concrete state not reordered; L3b, the edge permutation ignored in the lift; L4, the permutation inverted (killed only by the three-copy test); L5, the identity permutation recorded;
  - loop repeat: R1, no repeat; R2, the order always 1; R3, one extra repeat; R4, the loop's permutation ignored;
  - canonical order: C1, the greatest key instead of the least; C2, the first member; C3, an orbit of one generator step;
  - atom exchange: A1, the generator's atom map the identity; A2, an undefined atom not exchanged; A3, the stored atom named in the counterexample; A5, a generator that swaps a variable with itself;
  - automaton rename: N1, `must` not renamed; N2, `exchange` returns its input; N3, the fold unsorted; N4, atoms not exchanged in the arena;
  - acceptance set: S1, the acceptance-set check removed;
  - reduced flag: H1, `HoldsReduced` never named; H2, the quotient applied when the automaton offers no swap;
  - preservation rows: F1, F2, F3 and F5;
  - concrete findings: U1, the contract finding's component left in stored order.
- **9 survived:**
  - L2, the lifted joint step not reordered after a permuted state (gap FND-005);
  - U2, an undefined `match` at a swapped state reported in stored order (gap FND-005);
  - N5 and N6, past-operator memory and interval counters not renamed; S3, the check that the temporal operators stay in the arena removed (gap FND-006);
  - F4, the `whole`-fairness condition of the key-symmetry row ignored; F6, key symmetry for HP-3 refused as not preserving (gap FND-007);
  - A4 (the involution check: an exchange of two variables always matches atoms in pairs) and S2 (the initial-state check: a swap that fixes the body fixes the automaton's initial state) are equivalent on every clause S3 accepts.

The author's judgement calls:

1. **AC-2 not delivered as a proof.** Accepted as out of scope: FR-163's checker and FR-182's settlement are not on main. But AC-2 is still bound by five tests (gap FND-002).
2. **The leaky count, 7 against 10.** Correct for this engine: the shared automaton is deterministic, as `tests/it/hyper.rs` on main explains. AC-2's 9 against 14, and TC-599 step 2, need a spec fix (gap FND-002).
3. **No reduction for a swap that exchanges acceptance sets (CS-3 silent).** Sound, and detected correctly. A generator must fix the initial state and every eventuality core, or the automaton offers no exchanges and the product is stored whole and named nothing. Mutant S1 is killed by `a_swap_that_exchanges_acceptance_sets_is_not_applied`, and H2 by the same test. The oracle agrees on all such bodies. The spec should say so (gap FND-010).
4. **A 'yes' row settles `Unsupported` (V-8).** It is a stopgap against FR-174's 'SHALL admit', and no ticket tracks it (gap FND-003). The test that asserts it is over-tagged (gap FND-008).
5. **`and`/`or` operands sorted by node, not by checked encoding.** Accepted. The arena's normal form needs one fixed order on operand sets, so that a swap maps the arena onto itself. The NodeId order gives that, and S3's CS-1 equality test is unchanged. Only state numbering differs from a checked-encoding order.
6. **The TC-601 counts (10 to 7, 8 to 6, 22/96 to 18/80).** Justified. Each changed test now asserts the unreduced count through `World::explore` beside the quotient count. The capped two-instance run's 18/80 is then re-run as exact limits and gives the same outcome, so it is measured, not typed in.
7. **`translate_negation` gains `Option<&AtomSymmetry>`, `HyperCheckRequest` gains `reductions`, and `HyperInputs` gains `copy_swap`.** All callers are updated. No shim or alias remains.

Other checks:

- **Canonicalisation.** Members of one orbit get one key: the canonical form is the least key over the whole orbit, and once a state is numbered its index does not change. Distinct orbits get distinct keys, because the canonical state is a member of its own orbit. C1 to C3 are killed.
- **Refusal paths.** `reductions::refusal` runs before any `ModelSystem` is built or any state expanded, and the AC-4 tests assert `RunStats::default()`.
- **Tags.** Every new test carries `#[trace("TC-599", "FR-174-AC-n")]` with a merged AC id, and the semantics test carries TC-599 alone. But every hyper_swap.rs doc comment opens with the AC id (`/// FR-174-AC-2: ...`). Five of those AC-2 bindings are not honest (gap FND-002, FND-004, FND-011 to FND-013), and the yes-row test is over-tagged (gap FND-008).
- **Dead code and compat layers.** None. `CopySwap::active` filters by a group that is always the clause's whole copy group (FND-003).
- **Public API.** `concrete_atom`, `copy_population`, `copy_symmetry`, `CopySwapInputs`, `AtomSymmetry`, `HyperReductionCause` and `PropertyAutomaton::exchange`/`is_symmetric` all have callers outside their module.
- **Merge risk with QSL-554 (#1121).** None found. Both PRs touch hyper.rs, but this branch merges cleanly onto current main. HP-6 projected products will need their own `copy_swap` input when #1121 lands (gap FND-014).

Spec follow-ups (route to /specify):

- ADR-023 CS-3 and PH-5, and FR-174's detection rule: the claim that the quotient keeps fairness for any granularity is false for a per-slot fairness check (gap FND-001).
- FR-174-AC-2 and TC-599 step 2: 9 and 14 should be 7 and 10 (gap FND-002).
- FR-176 / FR-174: which canonical lasso a refutation under copy-swap reports (gap FND-009).
- ADR-023 CS-3: the swap that exchanges acceptance sets (gap FND-010).

Declared out of scope, and confirmed not built: the `Proved{Reduced, Certified}` settlement (FR-163, FR-182), diagonal key symmetry and state constraints for hyper items (V-8 stopgap), and copy-swap for HP-6.

## Verdict

**Not mergeable: FND-001 (high) must be fixed first.** With fairness on the swapped copies, the quotient check reports `HoldsReduced` for items that are refuted, with a counterexample that replays `Reproduced`. It also reports refutations whose counterexamples replay refuses as unfair. Withholding copy-swap from classes with a non-empty fairness set is a small, sound fix, and it needs the reproductions as tests.

- **FND-002 and FND-003 (low).** A silent fallback in the contract-finding remap, and the generator filter `active`, which only `canonicalise` applies.

Without fairness, the quotient check is sound on the oracle's models. The canonical form, the automaton rename, the acceptance-set fallback, the lift and the loop repeat all hold up, and every reduced counterexample replays. The coverage and trace findings, the AC-2 count fix and the spec soundness fix are in SR-13606, the gap analysis.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The quotient is unsound when the swapped copies carry fairness: it gives false proofs and spurious refutations. `run_instance` reduces whenever the automaton offers the swap (`reduced = product.offers_symmetry() && !self.group.is_trivial()`), whatever the fairness sets. But `fair_graph` scopes each fairness constraint to a stored slot: component index `k` of the stored, canonical state. Under the quotient a concrete copy moves between slots each time an edge's permutation swaps them, so a slot's enabled and taken steps are not one behaviour's. A fair concrete lasso can be unfair in every slot, and an unfair one fair in each. The second phase (CX-5's loop) and the HM-7 `VacuousMatch` check (`fair_cycle`) both read this graph. The PR's tests have no fairness on swapped copies. Reproduction A (false proof): the step contract `(pre(self.l) = 0 and self.l = 1) or (pre(self.l) = 1 and self.l = 0)` (every `step` flips `l`), `pre R { self.l = 1 }` / `post Reset { self.l = 1 }` on `reset`, starts `(0, 0)` and `(0, 1)`, and `forall trace a of V fair weak V::Vault::reset; forall trace b of V fair weak V::Vault::reset; match { a.step.op = b.step.op } { always holds(v.l @ a = v.l @ b) }`. The engine at this head returns `HoldsReduced{[CopySwap{[[a, b]]}]}` over 3 states. The same run with the copy group withheld (an env toggle in my scratch clone) returns `Violated` over 4 states, with `a` from `(0, 0)` and `b` from `(0, 1)` flipping together, and that counterexample replays `Reproduced` by FR-183, fairness check included. The body `always eventually holds(v.l @ a = v.l @ b)` gives the same split. Why: in the concrete loop `a` and `b` alternate `l`, so each copy's `reset` is enabled only every other step and weak fairness holds. The quotient stores the pair once, with the copy at `l = 1` always in slot 1, so slot 1 has `reset` enabled at every state of the loop and never takes it, and the per-slot constraint rejects the cycle. Reproduction B (false proof, from the random oracle, seed 89 case 103): the leaky step contract, `pre R { self.l = 0 }` / `post Reset { self.l = 1 }`, start `(1, 1)`, both copies `fair weak V::Vault::reset;`, `match { a.step.op = b.step.op } { always eventually holds(v.l @ a = v.l @ b) }`. Reduced: holds over 7 states. Unreduced: `Violated` over 10. Reproduction C (spurious refutation, seed 41 case 195): `post Step { self.l = i }`, `pre R { self.l = 0 }` / `post Reset { self.l = 1 }`, start `(1, 0)`, three copies with `fair weak V::Vault::reset;`, `{ eventually holds(v.l @ a = 1) and eventually holds(v.l @ b = 1) and eventually holds(v.l @ c = 1) }`. Reduced: `Violated` over 7 states, and replay refuses the counterexample `HyperUnfair { variable: "c" }`. Unreduced: holds over 20. Fix: do not reduce when any variable of a swapped class has a non-empty fairness set. Clear `copy_swap` (or drop the class from the run's group), store the product whole and name no reduction, as for the acceptance-set case. Add reproductions A to C as tests. An annotated quotient (Emerson and Sistla, fairness tracked per concrete copy through the permutation) is the alternative, but it needs a spec first. ADR-023 CS-3 and PH-5 claim this case is sound, so the spec needs fixing too (gap SR-13606 FND-001). | qsl-analyze/src/model_check/hyper.rs:1196, 1812-1850 |
| FND-002 | low | The contract-finding remap falls back silently to the stored index (`unwrap_or(component)`) if `positions` does not contain it. `positions` is always a permutation of the components, so the fallback is unreachable today. But if it is ever reached, the counterexample names the wrong variable without a fault, where every neighbouring invariant (`a-copy-permutation-is-an-element-of-the-copy-group`, `a-contract-finding-names-a-component`) raises one. Use `.ok_or_else(\|\| fault("a-lifted-path-holds-every-component"))?`. | qsl-analyze/src/model_check/hyper.rs:1671-1676 |
| FND-003 | low | One fact, two places. `canonicalise` acts only by the generators that `active(group)` keeps: those whose two variables the run's `SymmetryGroup` puts in one class. `concrete_atom` composes all of `swap.generators`. The only group ever passed is `copy_symmetry(clause.copy_group())`, which keeps every generator, so the filter is dead generality today. If a sub-group is ever passed (FR-174's 'product of the copy group and any admitted diagonal key symmetry'), the counterexample's atom would be composed in a larger group than the one the quotient was built under. Drop `active` and use the copy group's generators in both places, or pass the group to `concrete_atom`. | qsl-eval/src/hyper/copy_swap.rs:86-103, 234-262 |

## Dispositions

Round 1, at PR #1123 head 4da6d7719. Gates at 4da6d7719 (main 10a7af550 merged): fmt, clippy -D warnings, the four crates' tests, root `it` (975 passed), string-edge and the three arch-lints all pass; 4,904 passed, 0 failed. Mutation, re-run at 4da6d7719: 39 mutants, 37 killed, 2 survived. The round-0 set is re-run, minus the equivalent L3 and the S3 mutant whose code is deleted, plus FA1 to FA4 for the new `unfair_classes`: FA1 keeps the fair classes, FA2 keeps their generators, FA3 names the clause's classes, FA4 treats fairness on any variable as fairness on every class. The round-0 survivors L2, U2, N5, N6, F4 and F6 are now killed, and so are FA1 to FA4. A4 (the involution check) and S2 (the initial-state check) still survive. Both are equivalent on every clause S3 accepts: a body that S3 proves symmetric fixes the negation's initial state. With S3 deleted, S2 is the only closure gate, but it never fires on such a clause.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | `unfair_classes` drops every copy-group class with a fair variable, and its generators, before the run's group, the product's generators and the `HoldsReduced` classes are taken from it. Reproductions A, B and C are tests that assert the unreduced verdicts and counts, and a fourth asserts that only `[[c, d]]` is named. Mutants FA1 to FA4 are killed. The oracle was re-run at 4da6d7719, with the copy-group toggle reapplied in the scratch clone and fairness made more frequent: weak or strong fairness on `step` or `reset` in 5 of 6 draws, on every copy, on every copy but the first, or on the first copy alone (so a three-copy clause keeps an unfair class `[b, c]` beside a fair `a`). 1,400 cases over seeds 7, 11, 23, 41, 53, 67 and 89: 1,400 agree, 0 disagree, and every reduced counterexample replays `Reproduced`. 851 cases carry fairness; 36 of those still run reduced, all by a class without fairness, and no `HoldsReduced` names a class with a fair variable. Reproduction A and its liveness variant now agree with the unreduced run (refuted over 4 and 6 states, each counterexample `Reproduced`); with `strong` fairness both hold, as unreduced. |
| FND-002 | fixed | The fallback is now the fault `a-lifted-path-holds-every-component`; mutant U1 is still killed. |
| FND-003 | fixed | `active` is removed. `canonicalise` and `concrete_atom` both read `swap.generators`, the filtered copy group's. The S3 deletion (the check that the exchange keeps temporal operators in the arena) is accepted. The arena is hash-consed (`intern` keys on the node), and `exchanged` builds each image from its operands' images with the same constructors. So the image of a node is the node structurally equal to its `σ`-image, and its operands' images are its image's operands. The automaton states name only descendants of the initial state's obligations (operands, cores and the negated state operands that `reachable` adds). `translate_formula` keeps the exchanges only if each fixes the initial state, so every image lies among the descendants of the initial state's own image, which is in `order`. The assumption is that no state names a node outside the initial obligations' closure. I found none in `reachable`. The oracle's past and interval bodies, and the PR's two new tests, agree. The unused `_group` argument of `canonical` is accepted too. It is the `TransitionSystem` hook's signature, shared with key symmetry, and the product's generators are now fixed at construction from the same filtered `CopyGroup` that the run's `SymmetryGroup` is built from (`check_hyper`), so there is one source for the group. |

Round 2, at PR #1123 head 596b3ef4a: the merge of origin/main (QSL-554 #1121, HP-6 projection). Only the merge delta was reviewed: `git show --remerge-diff 596b3ef4a`, plus the added PH-4 row and the `a_projected_clause_is_stored_unreduced` test. The conflict resolutions keep both features. `Explored` has both `reduced` and `bounded`. The Projected arm of retention passes the run's group, which is trivial because `offers_symmetry` is false for Projected, and `retain` was already `retain_reduced` under the trivial group, so the HP-6 horizon is unchanged. `traces` reads the lifted path and keeps HP-6's step-less components and marks; `lift` is the identity on a projected graph. `undefined_counterexample` takes `position_of` as its stem and the lifted state and positions for its keys. Projected is not symmetric and its atoms are not concretised. `copy_swap.rs` raises a fault on the projected-only properties `Rejected` and `MatchUndefined`, which a self-composition state cannot hold. `reductions::refusal` gains `Row::Projected` per ADR-023 PH-4: partial-order reads 'no', key symmetry over one subject and a state constraint read 'yes' and settle V-8. That also closes a merge-only hole: without the row, a reduction selected for an HP-6 item would have been ignored. Gates at 596b3ef4a all pass: fmt, clippy -D warnings, the four crates' tests, root `it` (975 passed), string-edge and the three arch-lints; 4,951 passed and 0 failed in all. The branch merges cleanly onto the current main (4180046f3). The copy-swap oracle (1,400 cases, 851 with fairness) agrees on 1,400 of 1,400, with every reduced counterexample replaying and no fair class named. The targeted fairness repros agree with the unreduced run. #1121's HP-6 brute-force oracle (1,540 cases, seeds 7, 11, 23 and 41) has 0 mismatches, and the AC-5 skip-fixture replay check passes. Key mutants, 11 of 11 killed: L2, U2 and FA1 to FA4 for this PR; P09, P13, P14, P28 and P30 for #1121. No new findings, and no finding's latest outcome changes.
