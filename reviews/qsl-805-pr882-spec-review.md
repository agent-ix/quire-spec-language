---
id: SR-11624
title: "Spec review of quire-spec-language PR #882: the object environment a replay supplies"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@46f6039e0c84cf06bd35c7fec6b5ba06387938b4; spec/functional/FR-098-execute-a-replay-request.md, spec/functional/FR-071-implement-typed-replay-request.md, spec/functional/FR-070-implement-typed-counterexample-witness-envelope.md, spec/test-cases/TC-1042-replay-dereferences-a-reference-over-an-admitted-object-environment.md"
review_set: subset
---
# Spec review of quire-spec-language PR #882

## Summary

Ticket: QSL-805. One pass of spec-review with the contradiction, ambiguity,
completeness, testability and spec-integrity sub-analyses over the PR diff
(`git diff origin/main...HEAD`). Spec-only PR, so no code review. Only major
issues block. Mediums and lows are recorded in the Verdict.

Oracles: FR-046 (forall), FR-084, FR-100 (call boundary order, AC-16),
FR-106 checks 1 to 8, FR-109 (`Function` selection), FR-364 (`resolve`,
manifest, bundle), TC-906 and TC-905 as merged; QSpec FR-181 typed canonical
form on main (cited by id only).

Examined:
- FR-098 Description paragraph, conversion rule (reference bullets), the new
  "The object environment" section steps 1 to 5, the refusal list, AC-10 and
  AC-14 to AC-19 (examined)
- FR-071 statement, Inputs bullet, the three `object_environment` rules and
  AC-12 (examined)
- FR-070 transcript-entry sentence, reference table row, decode refusal
  bullet, AC-16 and AC-17 (examined)
- TC-1042 fixture, variants S1 to S2 and all 33 rows (examined)
- FR-106 checks 1, 3, 4, 6.1 to 6.9, 7, 8 and the required-population rule;
  FR-364 `resolve` rules 1 to 5; FR-100 call-boundary order; FR-046 forall;
  FR-109 `Function` admission; QSpec FR-181 reference form (context_only)

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The reference rules skip FR-100's universe check, which comes first. FR-098 now cites "the reference admission of FR-100" for type conformance. It does not take FR-100's order, which is: value shape, universe correspondence, conformance, then object-world resolution. FR-100 says the `foreign_reference`/`foreign-universe` cause wins over conformance, and that neither case "becomes a generic type-equality or dangling refusal" (FR-100-AC-16). Two TC-1042 rows contradict this. Row 30 gives `ref(a, TN)` with universe `V`, while `Node`'s universe is `U`. It expects `dangling_reference` from step 4. FR-100 says this is `foreign-universe` and never a dangling refusal. Row 17 gives `ref(z, TL)`. `Lone` has universe `V` and does not conform to `Node`. The row expects `WrongValueKind` at conversion, but FR-100 says `foreign-universe` wins first. AC-19 ("one whose universe differs ... refuses `dangling_reference`") states the same wrong result. Fix: decide universe correspondence at conversion, before conformance, with FR-100's cause. Then change rows 17 and 30 and AC-16 and AC-19 to match. Or state why replay departs from FR-100 and amend FR-100 in the same PR. | spec/functional/FR-098-execute-a-replay-request.md:176-181, spec/functional/FR-098-execute-a-replay-request.md:433, spec/test-cases/TC-1042-replay-dereferences-a-reference-over-an-admitted-object-environment.md:117, spec/test-cases/TC-1042-replay-dereferences-a-reference-over-an-admitted-object-environment.md:151 |
| FND-002 | high | FR-098-AC-14 states a result that cannot happen. It says `fe` over `[ref(a), ref(b)]` "completes `false` through two reference reads". FR-046 says `forall` stops at the first false body result. The element `a` holds `items` `[3, 7]`, so `7 < 5` is false, and both the inner and the outer `forall` stop at `a`. `deref(b)` is never evaluated, so there is one reference read, not two. A test that counts the reads or their charges fails against a correct implementation. Fix: order the sequence `[ref(b), ref(a)]`, so `b` (`[1, 2]`, all true) is read first and `a` second, and change TC-1042 row 13 to match. Or drop "through two reference reads". | spec/functional/FR-098-execute-a-replay-request.md:428, spec/test-cases/TC-1042-replay-dereferences-a-reference-over-an-admitted-object-environment.md:104 |

## Verdict

FAIL: 2 blocking findings (FND-001, FND-002).

Clean on the points the brief asked to trace:
- Rows 25 and 27, re-traced first. Row 25: S1-dup's second `a` fails check
  6.3 in walk order, after 6.1, 6.2, 6.4 and 6.5 pass for `a`, `b`, `h` and
  `z`. `n9` belongs to step 4, which never runs, so `conflicting-identity` is
  right. Row 27: `a.next` names `gone`. Check 6.5 skips the type fit of an
  unlisted target (FR-106-AC-4 precedent). `G` is required because it holds
  `a`, and it is complete, so check 7 passes and check 8 refuses
  `dangling_reference` naming `gone` and `G`. Right.
- The other 31 rows trace through steps 1 to 5 and FR-106/FR-364 to the
  results they state, except rows 13, 17 and 30 (FND-001, FND-002). Row 29 is
  rule 5 and row 31 is rule 4. Row 31's admission succeeds because no held
  object makes `G` required.
- The witness rows (6, 7, 13, 16, `ref`) write members in JCS order. The
  value texts hold no `%`, `;`, `<` or `>`, so no escape is needed. The FR-070
  reference row matches QSpec FR-181's triple.
- Choice 8 (an unheld read is an internal fault) is sound. A reference field
  of a required population's object makes its population required (FR-106).
  Check 7 then requires that population to be complete, and check 8 requires
  the named key to be listed. Argument references are closed by step 4. So
  every reference reachable by `deref` through fields, nested records
  included, is held. Navigation over relationship ends is out of scope
  (choice 9).
- No test parses FR-070, FR-071, FR-098, TC-905 or TC-1042. Only FR-092,
  FR-093, FR-094 and FR-255 are `include_str!`ed.
- No id collides with another open PR: #878, #879, #880 and #881 touch only
  FR-267 or no spec. No index row is needed. The new spec text holds no ticket
  ids. The `sha256-jcs` digests address documents by content, which is not
  ceremony. No second store or grammar is added, and nothing contradicts
  #880's spec.

Recorded (not blocking):
- medium: FR-071 leaves the inner form of `object_environment.identity`
  unstated (members, blank labels), and nothing compares it with the
  document's own identity.
- medium: an FR-106 `Incomplete` record wrapped as a `ReplayRefusal` (rows
  22, 28 and 31) differs from FR-098's rule that an S6a `incomplete` settles
  `inconclusive`/`NoValue`. The result is stated, but the split is not
  explained.
- medium: replay refuses an unresolved argument with `dangling_reference`,
  while FR-109's `Function` selection uses `wrong-role-mapping`. This matches
  FR-100, but FR-098 cites FR-109's admission.
- low: TC-906's "since a replayed function has no object environment" is now
  stale wording.
- low: the FR-070 table row is still printed in reading order, and the new
  prose sentence explains it.
- low: FR-098-AC-10's "another type's" could now be read to include a proper
  subtype, which admits.
