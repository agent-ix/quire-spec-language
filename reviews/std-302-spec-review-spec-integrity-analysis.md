---
id: SR-15621
title: "integrity review of STD-302 S9 switch against FR-402, FR-403, ADR-011, AD-001 and open PR #1236 (QSL PR #1237)"
type: SpecReview
analysis: integrity
scope: "agent-ix/quire-spec-language@4e8346155083e1344c75afbc50057ced6615f5fb; spec/functional/FR-405-retire-the-std-reference-evaluator-for-quire-eval.md, spec/test-cases/TC-1124-quire-eval-is-the-only-evaluator-after-the-switch.md, spec/assurance/AD-001-shared-evaluation-core-interface.md, spec/decisions/ADR-011-stage-dag-and-dependency-architecture.md, spec/functional/FR-402-differentially-test-the-no-std-evaluator-against-the-std-reference.md, spec/functional/FR-403-implement-the-no-std-evaluator-in-quire-eval.md, spec/spec.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-405
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-403
    type: reviews
  - target: ix://agent-ix/quire-spec-language/AD-001
    type: reviews
  - target: ix://agent-ix/quire-spec-language/ADR-011
    type: reviews
---
# SR-15621: integrity review of STD-302 S9 switch against FR-402, FR-403, ADR-011, AD-001 and open PR #1236 (QSL PR #1237)

## Summary

Ticket: STD-302. Diff `origin/main...4e8346155` (main 48108c9c1 is an ancestor of the head). This review checks consistency across artifacts: the merged FR-400 to FR-403, NFR-015, AD-001, ADR-011 X-14 to X-16, and open PR #1236 (STD-301, NFR-016, TC-1130 and TC-1131, head of `spec/std-301-unsafe-audit`).

These claims hold:

- The ids are unique. FR-405, TC-1124 and X-17 are not on main, and none of the 21 open PR heads checked uses them (#1236 uses NFR-016, TC-1130 and TC-1131).
- `git merge-tree` of this head with #1236's head is clean (exit 0). The adjacent AD-001 frontmatter inserts (NFR-016 after NFR-015, FR-405 after FR-403) do not overlap, and the S8 sentence at AD-001:200 is edited only by this PR.
- NFR-015 is consistent: `qsl-eval` has no NFR-015 row and FR-405 keeps it `std` with no `std` feature.
- The FR-405 relationships and the spec.md index entry are well formed.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | major: FR-405 adds a clause entry to `quire-eval`, but the merged FR-403 still says, without condition, that there is none. FR-403 line 105-109: "It has no entry that evaluates a state clause ... Those entries stay in `qsl-eval` and run on the std reference evaluator". Line 188-189: "no clause entry is part of `quire-eval`". The last sentence of FR-403-AC-9: "`quire-eval`'s source contains no ... clause, frame, separation or value-invariant entry". The PR bounds only the second sentence of AC-9. ADR-011 X-16 likewise says "clause evaluation stays in layer 5" and that E evaluates every node kind except the four, and X-17 does not say that it amends X-16. After the switch, FR-403 and FR-405 contradict each other. <br>Fix: bound these FR-403 sentences the same way (or scope them to the `Value`-family entry and point to FR-405), and write "amending X-16" in X-17. | spec/functional/FR-403-implement-the-no-std-evaluator-in-quire-eval.md:105-109, spec/functional/FR-403-implement-the-no-std-evaluator-in-quire-eval.md:186-189, spec/functional/FR-403-implement-the-no-std-evaluator-in-quire-eval.md:402, spec/decisions/ADR-011-stage-dag-and-dependency-architecture.md:1234-1235 |
| FND-002 | high | major: FR-402's acceptance criteria become false at the switch, and nothing amends them. FR-402-AC-9 says "`make ci` runs `evaluator-differential`", while FR-405-AC-8 says it "exists neither as an `xtask` command nor as a `make` target". FR-402-AC-6 and AC-7 test "the std implementation's compared record" and the pair `expected/std`. FR-402-AC-10 runs `make evaluator-differential`. FR-403-AC-3, AC-6, AC-7, AC-10, AC-12 and AC-13 compare against the reference. The pointer paragraph added to FR-402 does not supersede any of these, so after the switch the matrix keeps ACs that can never pass. <br>Fix: in FR-405, list which FR-402 and FR-403 ACs the switch retires or rewrites, and require the switch change to amend those specs. Or bound them the way FR-403-AC-9 is bounded. | spec/functional/FR-402-differentially-test-the-no-std-evaluator-against-the-std-reference.md:428-432, spec/functional/FR-405-retire-the-std-reference-evaluator-for-quire-eval.md:221 |
| FND-003 | high | major: Lead default 3 conflicts with AD-001's "RT and CG acceptance", which this PR leaves unchanged. Its fourth bullet requires RT to "Differentially compare the shared implementation with the QSL std reference on the common evaluation corpus". FR-405 deletes that reference at S9 and states that RT's migration "needs no QSL evaluator". If RT accepts after S9, that bullet cannot be met. <br>Fix (owner or planner decision): either order RT's differential acceptance before S9 (and make it a switch prerequisite), or amend the AD-001 bullet so that RT compares against the recorded corpus expectations. | spec/assurance/AD-001-shared-evaluation-core-interface.md:237-238, spec/functional/FR-405-retire-the-std-reference-evaluator-for-quire-eval.md:88-90 |
| FND-004 | medium | minor: The S8 audit scope does not match #1236's NFR-016. FR-405 criterion 4 and AC-3 require the audit to pass "for that revision of `quire-eval`, `quire-checked` and `quire-eval-corpus`", and call it an "unsafe and dependency audit". NFR-016 explicitly excludes `quire-eval-corpus` ("`quire-eval-corpus` and every other workspace member that `quire-eval` does not depend on" lie outside the closure) and audits unsafe code only. <br>Fix: cite NFR-016's `make audit-unsafe` gate in `agent-ix/quire-eval`, drop `quire-eval-corpus`, and drop "dependency". Whichever of #1236 and #1237 merges second makes the citation. | spec/functional/FR-405-retire-the-std-reference-evaluator-for-quire-eval.md:81-83, spec/functional/FR-405-retire-the-std-reference-evaluator-for-quire-eval.md:216 |
| FND-005 | low | minor: AD-001 keeps "default" wording that contradicts the single-evaluator end state. The Decisions bullet reads "only then may IR-593 switch defaults", and the S7-S9 paragraph says "S9 switches the default and retires the std path". FR-405 says that nothing selects an evaluator. <br>Fix: "only then may IR-593 make it the only evaluator" and "S9 makes the `no_std` implementation the only evaluator and retires the std path". | spec/assurance/AD-001-shared-evaluation-core-interface.md:286, spec/assurance/AD-001-shared-evaluation-core-interface.md:202 |
| FND-006 | low | minor: FR-405 cites ADR-031, FR-106, FR-138, FR-139, FR-391 and FR-366 in its body (ADR-031 also in Dependencies), but its `relationships` list none of them, so the link graph misses the clause dependencies. "The umbrella's completion" is also planning content keyed to tickets (IR-584), which SR-15570 FND-012 moved out of FRs into AD-001 for FR-402. <br>Fix: add at least ADR-031 as `depends_on`, and move the IR-584 completion paragraph to AD-001. | spec/functional/FR-405-retire-the-std-reference-evaluator-for-quire-eval.md:5-26, spec/functional/FR-405-retire-the-std-reference-evaluator-for-quire-eval.md:203-208 |

## Verdict

Not mergeable as is. Three major findings: FND-001 (FR-403 and X-16 still say that `quire-eval` has no clause entry), FND-002 (FR-402 and FR-403 ACs become false after the switch) and FND-003 (AD-001's RT acceptance needs the deleted reference). FND-004 to FND-006 are minor. No textual conflict with #1236. The ids are unique.
