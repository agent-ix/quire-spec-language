---
id: SR-1357
title: "Gap analysis of quire-spec-language PR #645: composite WitnessValue replay and composite bounded_shadow settlement (QSL-640)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@6ec5705d9187c030367108c3078c3297490b1a96; PR #645 diff against origin/main; FR-070-AC-8..AC-12, FR-098-AC-8..AC-10, FR-263-AC-1, FR-357-AC-16, FR-358-AC-1..AC-8; trace tags TC-905 and TC-736 in qsl-replay/src/witness/value_text/tests.rs, TC-906 and TC-736 in qsl-replay/src/execute/tests/composite.rs, TC-907 in qsl-replay/src/execute/tests/composite_parity.rs, TC-904 in qsl-replay/src/execute/tests.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-358
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-098
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-070
    type: reviews
---
# Gap analysis of quire-spec-language PR #645

## Summary

Ticket: QSL-640. Planless. Plan completion: not assessed.

`quoin matrix --json` (quoin 0.28.1) at this head lists FR-070-AC-8 to
AC-12, FR-263-AC-1, FR-357-AC-16 and FR-358-AC-1 to AC-8 as `tagged`, each
bound to the TC-905, TC-736, TC-904 or TC-907 tests named in scope. It does
not list FR-098-AC-8 to AC-10 at all (FND-001).

Focused tests pass at this head (see SR-1356). A mutation that forces the
identity tie to accept any digest is killed by two TC-907 tests.

Questions from the brief:

- (3) The ADR-013 O-09 worked digest `4c36290f…` is not reproduced through
  the composite entry, because a real package cannot have node id `11…11`.
  `tc_907_the_composite_arm_builds_the_adr_worked_argument` instead builds
  the composite argument for `f`'s left operand and compares its digest
  with canonical text written out by hand with the real node id, in O-09's
  form and key order (`[0]`, `[1,0]`, `[1]`). With
  `parity_identity`'s own test reproducing `4c36290f…` through the same
  encoder (FR-357-AC-19), that is an adequate pin of the composite
  argument. It pins only `arguments[0]`: the node, occurrence key and
  obligation kind members of the composite preimage, and the right
  operand, are pinned only through the shared encoder (FND-004).
- (5) The unbounded `K<T>` cases are covered by a derive/declare/cover unit
  test on a type built in Rust, and AC-5's text leaf by a rational leaf.
  A probe confirmed the text substitution is forced: with
  `label: Text[0, 4; nfc]` the unit fails to compile with
  `MissingSelection { role: TextProfile }` at `t`'s `a = b`, so text
  equality needs a text-profile selection the spine test helper does not
  pass. The rational leaf exercises the same `Whole` branch. Both
  substitutions are acceptable evidence for the logic, but they are
  open deferrals with no ticket (FND-003).

## Verdict

Request changes, for FND-001. Every FR-358 criterion has tests that would
fail on a real defect; the remaining items are low.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-098-AC-8, AC-9 and AC-10 are invisible to the computed matrix. The PR appends them after a blank line that ends the criteria table, so they form a second, headerless table that quire does not parse. `quoin matrix --json` lists FR-098-AC-1 to AC-7 only, though TC-906's tests carry `#[trace("TC-906", "FR-098-AC-8")]` and the others. Fix: delete the blank line after the AC-7 row. | spec/functional/FR-098-execute-a-replay-request.md:226-230 |
| FND-002 | low | FR-358-AC-3's identity case, an `obligation_identity` computed over `[0, 9]` on the first `x` while the supplied harness bound is `[0, 8]`, is not tested as written. `tc_907_a_tampered_obligation_digest_refuses` flips one byte of the digest instead, so no test shows end to end that a different harness bound gives a different identity and refuses naming both digests. Fix: mint the identity over the covering bounds, send the claim with `[0, 8]`, and assert `Obligation { claimed, recomputed }` with both digests. | qsl-replay/src/execute/tests/composite_parity.rs:970-1000 |
| FND-003 | low | Two FR-358 criteria cases are deferred in the Status with no ticket: the `K<T>` (`k`) cases of AC-4 and AC-5, which need a source form for an unbounded collection, and AC-5's text leaf, which needs a text-profile selection in the test harness (probe: `MissingSelection { role: TextProfile }`). TC-907's row says "Passed locally except" them. Fix: file the tickets and cite them in FR-358's Status and TC-907, or rewrite the `k` cases to a form the grammar takes (for example a request `DeclaredDomain` over a recursive `Depth`). | spec/functional/FR-358-settle-a-composite-bounded-shadow-item.md:392-399, qsl-replay/src/execute/tests/composite_parity.rs:655-660 |
| FND-004 | low | The TC-907 fixture computes every request's `obligation_identity` with `parity_preimage` and `parity_obligation`, the code under test. Only `arguments[0]` of the composite preimage is checked against independent text. The occurrence key QSL picks, the right operand (a parameter or a composite literal's empty `bounds`) and the whole-preimage digest are not pinned independently, so a wrong but self-consistent choice there (see SR-1356 FND-001) passes every test. Fix: extend the worked-argument test to the whole preimage text for `f`, and to `g`'s composite literal operand. | qsl-replay/src/execute/tests/composite_parity.rs:147-178, qsl-replay/src/execute/tests/composite_parity.rs:1055-1084 |

## Dispositions

Round 1, reviewed at dfc72b6d120b4bc914933300007032a7c6c67537 (fix commits 6ec5705d9..dfc72b6d1).

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | 70693d605 |
| FND-002 | fixed | 74d7fca97 |
| FND-003 | deferred | The Text leaf of FR-358-AC-5 is QSL-646 (sub-ticket of QSL-640), cited in FR-358's Status and TC-907. The source-level `k` cases cannot be written in the grammar (it takes only `K<T>[min, max]`); they are tested on the derivation and coverage seam over all four collection kinds, and FR-358's Status says so |
| FND-004 | fixed | 74d7fca97 |
