---
id: SR-13750
title: "base review of QSL-949 quantity literals outside V1 scope (PR #1179)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@adf8a78b0c0756b5ca69c4df772c621476932cf9; spec/functional/FR-093-lower-checked-value-expressions-to-fr-322-terms.md, spec/functional/FR-094-key-model-owned-reference-population-and-quantity-nodes.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-093
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-094
    type: reviews
---
# SR-13750: base review of QSL-949 (PR #1179)

## Summary

Ticket: QSL-949. Base checklist over
`git diff origin/main...adf8a78b0` (2 files, +35/-0). The diff adds the FR-093
section "Quantity literals are outside V1 scope", FR-093-AC-33, and one
paragraph in FR-094. Spec-only diff, so code-review and gap-analysis do not
apply. The EARS check of the one new SHALL statement is folded in here.
No `docs/` file and no test-parsed vector table is touched, so `make ci` is
not needed.

Checks made by hand:

- Refusal probe (scratch test in a detached worktree, reverted). With
  `dimension Time`, `unit s`, `unit ms`, `check` refuses each of
  `f(3 ms)` (f taking `Quantity<ms>`), a body `3 ms` returning
  `Quantity<ms>`, `d = 3 ms` and `d + 3 ms` with exactly one refusal,
  `Probabilistic(QuantityLiteral)`, catalog code
  `unsupported_construct/not-yet-implemented`. The PR's code, cause and
  "no literal node" claim match the checker. The existing test
  `a_quantity_argument_of_another_unit_is_a_type_mismatch`
  (FR-091-AC-44, TC-483) already asserts the `f(3 ms)` case.
- Shared contract (read only). QSpec `quire-specification` main 014f0ce
  defines a `quantity` literal kind: FR-322 "A quantity literal `n u`
  lowers to a `literal` term of `value_kind` `quantity`" (FR-322 line 999),
  the literal-fit list `quantity`→`quantity`, the canonical magnitude
  spelling, FR-322-AC-79, and both checked-package-v2 schemas enumerate
  `quantity`. Added by c9a3ef6 (STD-113, #296). The Contract IR reader QSL
  pins (Cargo.lock d41e521) has no `LiteralKind::Quantity` and its fit
  rule has no quantity arm. FND-001.
- Rational-literal rule. "A `rational` literal typed at a unit violates the
  literal/type-family contract" agrees with FR-322's fit rule and the IR
  reader. Clean.
- FR-091 `Quantity<u>` (FR-091-AC-44), FR-093 quantity operations and
  FR-094 unit type nodes. The new text keeps quantity parameters and
  operations in scope and adds no node or second type. Consistent.
- Sweep for stale "supported/planned" statements. QSL FR-185 (Dependencies:
  "the quantity literal form are QSpec's (FR-142, FR-322)") and TC-620
  step 8 (FR-322-AC-79) already say QSpec owns the form, which the new
  FR-093 text contradicts (FND-001). One test comment still records the
  limit as remaining work (FND-005).
- Gates. `make check-spec-validation` exits 0; one new warning on a touched
  line (FR-093 line 873, `ac:vague-response`, `support`). All other
  warnings are pre-existing. `tools/check-index-completeness.sh` passes.
  `git diff --check` is clean. FR-093-AC-33 is unique and follows AC-32.
  Tools: quire 0.36.2 (engine 0.50.2), quoin 0.28.3.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The new FR-093 section says "The shared QSpec FR-322 literal contract and Contract IR reader have no quantity literal kind", and the reopen path (FR-093-AC-33, FR-094) starts with "the QSpec owner must first define a shared FR-322/schema representation". That is false for QSpec: FR-322 on main defines `value_kind` `quantity` typed at the `scalar_type`/`unit` node with a canonical rational magnitude (FR-322 line 999, fit rule, "Scalar literal values", FR-322-AC-79), and both v2 schemas enumerate it (STD-113, #296). It also contradicts QSL's own FR-185 Dependencies and TC-620 step 8. Only the IR reader half is true (pinned d41e521 has no `LiteralKind::Quantity`). State the limit as: QSpec FR-322 defines the quantity literal; the Contract IR reader does not admit it; QSL refuses it in V1; reopening needs IR admission of FR-322-AC-79, then QSL lowering and CG. The V1-scope decision was reached on the premise that QSpec lacked the kind, so the lead should confirm it still stands. | spec/functional/FR-093-lower-checked-value-expressions-to-fr-322-terms.md:153-157; spec/functional/FR-093-lower-checked-value-expressions-to-fr-322-terms.md:164-168; spec/functional/FR-093-lower-checked-value-expressions-to-fr-322-terms.md:873; spec/functional/FR-094-key-model-owned-reference-population-and-quantity-nodes.md:316-318 |
| FND-002 | medium | FR-093-AC-33 is verified by `Analysis` with no TC, although its refusal is directly testable and already tested: `a_quantity_argument_of_another_unit_is_a_type_mismatch` asserts the single `ProbabilisticCause::QuantityLiteral` refusal for `f(3 ms)`, tagged FR-091-AC-44/TC-483. Make it `Test (TC-483)` (or TC-415) and tag that assertion with FR-093-AC-33. | spec/functional/FR-093-lower-checked-value-expressions-to-fr-322-terms.md:873 |
| FND-003 | low | FR-093-AC-33 is compound: a testable refusal, a scope statement ("remain in scope") and a process rule for reopening ("requires ... ; a QSL-local wire extension or reader bypass is forbidden"), which no test can fail. Its "remain in scope" wording also raises the new quire warning `ac:vague-response` (`support`). Keep the refusal and the no-literal-node outcome in the AC; leave the reopen path in the section prose. | spec/functional/FR-093-lower-checked-value-expressions-to-fr-322-terms.md:873 |
| FND-004 | low | Two non-behaviour sentences: "No such wire extension or delivery commitment is introduced here." describes this PR, not the system, and "the IR-263 MVP source row can use ..." cites a planning ticket's source row in an FR. Delete the first; reword the second as behaviour ("a function over quantity parameters using equality, addition and exact conversion lowers without a quantity literal") or drop it. | spec/functional/FR-093-lower-checked-value-expressions-to-fr-322-terms.md:159-162; spec/functional/FR-093-lower-checked-value-expressions-to-fr-322-terms.md:168-169 |
| FND-005 | low | Outside the diff: the TC-483 test comment still records the limit as pending, "the checked package has no literal form to emit a quantity as ... (Remaining work: QSL-949)", with `expect_err("a quantity literal has no emitted form yet")`. Once QSL-949 closes as a documented V1 limit, cite FR-093-AC-33 instead and drop "yet". | qsl-semantics/src/check/assemble/tests.rs:2321-2330 |

## Verdict

Not mergeable as is: one high, one medium, three low. The stated behaviour
is true: the checker refuses every probed quantity literal in a body with
exactly `unsupported_construct`/`not-yet-implemented`
(`ProbabilisticCause::QuantityLiteral`) and emits no literal node, and the
rational-literal rule and the FR-091/FR-094 type rules are stated
consistently. The defect is the dependency story: QSpec FR-322 already
defines the quantity literal kind, so the "QSpec first" reopen path and
the "no shared kind" rationale are wrong, and they contradict FR-185 and
TC-620. Only the Contract IR reader lacks the kind. Gates pass; no docs or
vector tables touched.

Examined and clean: FR-093 statement lines 147-151 (refusal), the
rational-literal sentence, the quantity parameters/operations scope
paragraph, FR-094's "no literal node or second quantity type" sentence,
FR-091-AC-44 consistency.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-006 | low | FR-093-AC-33 now names `Test (TC-483)`, and the computed matrix binds it to the retagged test, but TC-483 itself is unchanged. Its `relationships` verify only FR-091, its Scope line lists FR-091 criteria only, and step 5 does not state the `f(3 ms)` check or its expected `ProbabilisticCause::QuantityLiteral` refusal. TC-415 shows the convention: a TC verifies each FR whose criteria it covers. Add a `verifies` FR-093 relationship, add FR-093-AC-33 to Scope, and name the `f(3 ms)` call with its expected refusal in step 5. | spec/test-cases/TC-483-assembler-admits-dimensions-and-units.md:4-7; spec/test-cases/TC-483-assembler-admits-dimensions-and-units.md:23; spec/test-cases/TC-483-assembler-admits-dimensions-and-units.md:41 |
| FND-007 | low | The rewrite adds tracker and decision prose to FR-093 and FR-094: "The IR/CG owners retain quantity literals outside V1 scope", "IR-955 tracks that admission work", "coordinated with the quantity evaluation leaf (IR-853)", and STD-113 cited beside FR-322-AC-79 as an authority. The first states who decided rather than what V1 does. The ticket ids are planning references; the repo convention for unfinished work is a `Remaining work: IR-955` line. The planner asked for IR-955 and IR-853 to be named, so this is wording only. State the limit as behaviour, put the tickets in one `Remaining work:` line, and cite FR-322-AC-79 alone. | spec/functional/FR-093-lower-checked-value-expressions-to-fr-322-terms.md:153-168; spec/functional/FR-094-key-model-owned-reference-population-and-quantity-nodes.md:316-320 |

## Dispositions

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | 690c5bd5c: FR-093 now says QSpec defines `value_kind: quantity` in FR-322-AC-79 with both V2 schemas, and that the Contract IR reader does not yet admit it. The reopen path is an IR reader change to admit FR-322-AC-79, then QSL lowering and CG. FR-094 says the same. A sweep of `spec/` outside `reviews/` finds no remaining claim that QSpec lacks the kind. |
| FND-002 | fixed | 690c5bd5c: FR-093-AC-33 is `Test (TC-483)`. The test `a_quantity_argument_of_another_unit_is_a_type_mismatch` carries `#[trace("FR-091-AC-44", "FR-093-AC-33", "TC-483")]` and passes. `quire matrix` shows FR-093-AC-33 as tagged at `qsl-semantics/src/check/assemble/tests.rs:2296`. The TC-483 document gap is FND-006. |
| FND-003 | fixed | 690c5bd5c: AC-33 now states one outcome, the `f(3 ms)` refusal and no literal node. The scope and reopen sentences are gone from the AC. `make check-spec-validation` exits 0 and reports no warning on FR-093, FR-094 or TC-483. |
| FND-004 | fixed | 690c5bd5c: "No such wire extension or delivery commitment is introduced here." and the IR-263 MVP source-row sentence are removed. The new tracker prose is FND-007. |
| FND-005 | fixed | 690c5bd5c: the test comment reads "FR-093-AC-33: `3 ms` has the right type but retains the V1 quantity-literal refusal until the IR reader admits QSpec's kind.", and `expect_err` reads "quantity literals are outside V1 scope". No `QSL-949` reference remains outside `reviews/`. |
