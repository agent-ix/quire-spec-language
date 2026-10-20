---
id: SR-13530
title: "Spec review of quire-spec-language PR #1096: directional affects for POR independence (QSL-564 spec follow-up)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@dc9cfab4f3eddb2a459df326c19f3dec1d2f1e9f; PR #1096, git diff refs/remotes/origin/main...dc9cfab4f: spec/decisions/ADR-021-state-space-reduction.md, spec/functional/FR-155-derive-and-enforce-read-and-write-footprints.md, spec/functional/FR-156-expand-ample-sets-with-the-breadth-first-proviso.md, spec/functional/FR-157-make-fairness-enabling-writes-visible.md; context: spec/test-cases/TC-576, TC-577, TC-589, TC-617, ADR-027 FT-5"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/ADR-021
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-155
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-156
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-157
    type: reviews
---
# Spec review of quire-spec-language PR #1096

## Summary

Ticket: QSL-564. This is the spec side of SR-13470 FND-002, FND-004, FND-005,
FND-006 and FND-007, and it pairs with QSpec PR #299 (SR-13531). The diff is
4 files, +27/-22.

Checked against the oracle and probes from SR-13470 and against the engine
on PR #1094's branch (`qsl-564-por`, 6fc896165). I added scratch tests to a
copy of `qsl-analyze/tests/it/partial_order.rs` (`sr_counts`, in
`sr-1094/mut`).

**The directional rule.** The rule PR #1094 implements and the oracle
confirms is:

- a written `Membership{P,k}` affects the membership cell and every field
  cell of `k`;
- a read `Membership{P,k}` covers only the membership cell;
- a field write affects no membership read;
- write/write pairs keep the plain symmetric meet.

ADR-021 POR-4, FR-155 "Meeting" and "Independence" and QSpec FR-383 all state
this rule with the same meaning. FR-156 "Visibility" and "Candidates" and
FR-157's Description and "Fairness visibility" now say "affects". ADR-021
POR-5, POR-7 and POR-10 still say "meets" (FND-001).

**The other rewrites, checked against the engine:**

- **FR-156-AC-1:** 7 states in the reduced model-level search and 27
  unreduced; `check_model` reports 6 product states per instance, 18 in all.
  These match the #1094 test.
- **FR-156-AC-2:** the engine's deadlock prefix bumps
  `a, a, b, b, c, c`. That is correct.
- **FR-157 order:** the engine lists the Gate's identities as
  `arm2, arm, go, toggle`. The order is correct, but the reason given for it
  is wrong (FND-003).
- **FR-156-AC-4 rewrite:** reproduced in the oracle (`oracle/ac4.py`). The
  candidate seeded by `touch` is `{touch, arm2, arm, go}` with enabled part
  `{touch, arm}`. With the disabled-member closure removed, the enabled part
  is `{touch}`, so the AC now tells the two apart.
- **Counts:** model-level reduced and unreduced searches measured on the
  engine.
  - Matching the PR: FR-156-AC-5 6/6; FR-156-AC-6 deleting the receiver
    4/4, blocked delete 5/5, navigation 6/6; FR-155-AC-6 6/6.
  - Not matching: FR-157-AC-5 has 8 model states, not 6, and the type-wide
    delete fixture has 14 (FND-002).

**Gates:**

- `make check-index-completeness`: pass.
- `make check-spec-validation`: pass. The only warning on a changed file,
  FR-155 line 113, is on the unchanged "State-free" bullet and predates this
  PR.

## Verdict

Changes requested: one high finding, four medium and three low. Not
mergeable until FND-001 is fixed, because the ADR still contradicts
FR-157-AC-1 through POR-10.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | ADR-021 POR-5 (visibility), POR-7 (the disabled-member closure) and POR-10 (fairness visibility) still say a transition's "write footprint meets" a read or enabling footprint. ADR-027 FT-5 says the same. POR-4 now defines "meet" as "either affects the other", and a written membership affects a field. So a field write meets a read membership, and POR-10 makes `toggle` (writing `Field{g, a}`) fairness-visible through `go(g)`'s `Membership{gates, g}`. That contradicts §7.5 and FR-157-AC-1 once more, and POR-7 brings the same false dependence into the closure. Write "affects" in each place, as FR-156, FR-157 and QSpec FR-383 now do. | spec/decisions/ADR-021-state-space-reduction.md:175; spec/decisions/ADR-021-state-space-reduction.md:177; spec/decisions/ADR-021-state-space-reduction.md:180; spec/decisions/ADR-027-protocol-transition-system-for-parallel.md:297 |
| FND-002 | medium | Two of the new state counts are wrong for the engine. FR-157-AC-5 says "(6 model states in the model-level reduced search)", but the engine's model-level reduced search stores 8 model states, with 11 edges, and the unreduced search also stores 8. The 6 is `check_model`'s product-state count. FR-156-AC-6 gives the type-wide delete "(4 model states)", but the fixture that can be written today, with `cancel` unconstrained, has 14. The ADR's constrained vector cannot be written yet, so that 4 cannot be checked. The qualifier also varies: AC-5 says "in the model-level reduced search" and AC-6 says only "model states". The other counts I measured are correct. | spec/functional/FR-157-make-fairness-enabling-writes-visible.md:102; spec/functional/FR-156-expand-ample-sets-with-the-breadth-first-proviso.md:103 |
| FND-003 | medium | The order the FR-157 preamble states is correct, but its explanation is false: "the operation text inside the escaped identity makes `arm` end in `"`, which sorts after `2`". `"` is 0x22, which sorts before `2` (0x32); if that were the next byte, `arm` would come first. The operation text is a JSON document embedded as a string, so after `arm` comes the escape `\"`, and its first byte `\` (0x5C) sorts after `2`. The preamble also writes "FR-181" with no prefix. In a QSL document that names QSL FR-181 (check a single existential claim); the order is QSpec FR-181's, as ADR-021 §7.5 now writes it. | spec/functional/FR-157-make-fairness-enabling-writes-visible.md:88-89 |
| FND-004 | medium | The test cases were not updated with the ACs they verify. TC-577 step 2 still expects the old AC-4: "the candidate holds `arm`, `arm2` and `go`, with enabled part `{arm}`", seeded by `arm`, not `touch`. TC-576 step 2 still expects "the six-step prefix of the table", which AC-2 now says is not the path. TC-617 ("over 6 states") and TC-589 ("6 states each", "4 states each", "5 states") use counts with no definition of model state. | spec/test-cases/TC-577-proviso-prevents-ignoring-and-closure-is-deterministic.md:29; spec/test-cases/TC-576-ample-sets-reduce-independent-bumps.md:29; spec/test-cases/TC-617-complete-enabling-footprints-keep-fair-violations.md:36; spec/test-cases/TC-589-complete-enabling-footprints-and-membership-keep-safety-violations.md:49-58 |
| FND-005 | medium | POR-4's new rationale is partly false, and part of it records the review rather than the system. "The symmetric one ... so no ample set smaller than `E(s)` exists for the subjects of §7.2 and §7.5" is false for §7.2. `bump(a)` and `bump(b)` touch different objects, so they are independent under either relation, and the §7.2 tests stayed green with `affects` replaced by `meets` (SR-13470 mutation M3). Only §7.5's same-object operations need the directional rule. "A generated-model oracle of 22,000 models found no disagreement" describes reviewer evidence kept in scratch, not the system; nobody can reproduce it from the repository. Keep the reason (a field write does not change membership) and drop the evidence count. | spec/decisions/ADR-021-state-space-reduction.md:174 |
| FND-006 | low | The terms disagree between POR-4 and POR-11. POR-4 says a written location "affects more than it covers" and keeps "covers" for read locations. POR-11 says "a written (population, membership of `k`) covers the membership cell and every field cell of `k`". POR-4's "a written field location affects the field locations it meets" is circular, because "meet" is then defined from "affects". Use "affects" for writes throughout, and define the field-write case by shared field cells, as QSpec FR-383 does. | spec/decisions/ADR-021-state-space-reduction.md:181; spec/decisions/ADR-021-state-space-reduction.md:174 |
| FND-007 | low | The rewritten FR-156-AC-4 adds "an enabled `touch()` framed `modifies self.d`" but does not say which unit it joins. The other symbols (`arm`, `arm2`, `c`, `d`) are FR-157's `Gate`, and adding `touch` to that Gate would change its other vectors. Name the subject, for example "FR-157's `Gate` with an added operation `touch()`". | spec/functional/FR-156-expand-ample-sets-with-the-breadth-first-proviso.md:101 |
| FND-008 | low | FR-155-AC-5 and AC-6 still say "so `set(a)` meets it" for a write against an enabling (read) footprint. That case is field against field, where meeting and affecting coincide, so the meaning holds; for the new terms, the word should be "affects". | spec/functional/FR-155-derive-and-enforce-read-and-write-footprints.md:163-164 |

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-009 | low | Two test-case steps still use "meets" for a write against a read location. TC-589 step 1: `AnyField{v}`, "which `set(a)` meets". TC-617 step 2: `AnyMembership{gates}`, "which meets `Membership{gates, g}`". Both pairs are ones where the two relations agree (field against field, and a written membership against a read membership), so the meaning holds. But FR-155, FR-156, FR-157 and ADR-021 now say "affects" for every write/read pair. | spec/test-cases/TC-589-complete-enabling-footprints-and-membership-keep-safety-violations.md:44; spec/test-cases/TC-617-complete-enabling-footprints-keep-fair-violations.md:37 |

## Dispositions

Round 1, reviewed at `886ff3b40ccad6e2f445f24a285963e133b22035`.

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | 886ff3b40: POR-5, POR-7, POR-10 and ADR-027 FT-5 now say "affects". |
| FND-002 | fixed | 886ff3b40: FR-157-AC-5, TC-617 and §7.6's second Gate vector now say 8 model states, which matches my measurement. The unverifiable type-wide-delete count is gone from FR-156-AC-6, TC-589 and §7.6. Every count now carries the "model-level reduced search" qualifier. |
| FND-003 | fixed | 886ff3b40: the preamble now cites QSpec FR-181 and the escape `\` (0x5C) against `2` (0x32), and states the order `arm2`, `arm`. |
| FND-004 | fixed | 886ff3b40: TC-577 step 2 is seeded by `touch` with enabled part `{touch, arm}`; TC-576 step 2 gives the `a, a, b, b, c, c` path; TC-589 and TC-617 counts are defined. |
| FND-005 | fixed | 886ff3b40: the POR-4 rationale names only §7.5's same-object operations, and the oracle count is gone. |
| FND-006 | fixed | 886ff3b40: POR-11 says a written membership "affects"; POR-4 defines field writes by shared field cells, and write/write meeting by the cells the other covers. |
| FND-007 | fixed | 886ff3b40: FR-156-AC-4 says "over FR-157's `Gate` with an added operation `touch()`". |
| FND-008 | fixed | 886ff3b40: FR-155-AC-5 and AC-6 say "`set(a)` affects it". |

Counts that the author did not re-measure, checked against my measurements
on PR #1094's engine (6fc896165, model-level reduced search):

| Vector | Spec | Measured | Agree |
| --- | --- | --- | --- |
| §7.5 Gate, guard in the precondition | 8 | 8 | yes |
| §7.6 guard in the postcondition (FR-156-AC-5) | 6 | 6 | yes |
| §7.6 deleting the receiver | 4 | 4 | yes |
| §7.6 blocked delete | 5 | 5 | yes |
| §7.6 navigation | 6 | 6 | yes |
| §7.6 result read (FR-155-AC-6) | 6 | 6 | yes |

All six agree. §7.6's "proved over 3 model states" sentences describe runs
with the footprint left incomplete. No such engine exists, so they cannot be
measured; they are narrative, not ACs.

I grepped the PR's files for "meets". Every remaining use is a write/write
meet, the symmetry refinement `B meets u_c(E(c))`, or a reference to "FR-383's
locations, meets and derivation" (POR-1, ADR-021 §9, FR-155 lines 39 and 177).
The two exceptions are FND-009.

Gates at 886ff3b40:

- `make check-index-completeness`: pass.
- `make check-spec-validation`: pass. The only warning on these files is
  FR-155 line 113, which was already there.
- The committed copy of this file passes `quire validate`.

Round 2, reviewed at `fe846e0e830261066044780e7aa8ec8a3472bc19`.

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-009 | fixed | fe846e0e8: TC-589 step 1 reads "`AnyField{v}`, which `set(a)` affects", and TC-617 step 2 reads "`AnyMembership{gates}`, which affects `Membership{gates, g}`". A grep of ADR-021, ADR-027, FR-155 to FR-157 and TC-576 to TC-617 finds no remaining "meets" for a write/read pair. What remains is write/write meeting, the AQ-6 and refinement uses, and references to "FR-383's locations, meets". |

Gates at fe846e0e8: `make check-index-completeness` passes, and
`make check-spec-validation` passes with no warning on TC-589 or TC-617.
