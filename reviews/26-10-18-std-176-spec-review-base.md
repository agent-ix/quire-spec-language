---
id: SR-12892
title: "Base review of PR #986: FR-165 S1 parsing, AC-4, AC-6 and TC-590 against QSpec graph-predicate"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@2cb28aa4b; FR-165 Parsing (S1), Predicates (S3), AC-4, AC-6; ADR-022 SG-4 and the paragraph after the SG table; TC-590 fixtures, steps 4 and 5; agent-ix/quire-specification@origin/main 4f65ed2 shared-grammar state-graph section, FR-390, TC-330 SG-R09 to SG-R18"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-165
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-590
    type: reviews
  - target: ix://agent-ix/quire-spec-language/ADR-022
    type: reviews
---

## Summary

Ticket: STD-176. PR quire-spec-language#986. The PR rewrites FR-165's S1
Parsing bullets to read every predicate through QSpec's merged
`graph-predicate`. It splits the placements into those S1 recognises (S3
refuses them) and those that are `invalid_syntax` at S1. S3 now refuses at
the first temporal operator in source order. It widens AC-4, adds AC-6 for
the S1 syntax errors, and adds TC-590 step 5. This base review covers the
checklist, AC to TC coverage, and whether the AC-4 and AC-6 vectors can
fail.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | AC-4's new S3 oracles cannot be decided over TC-590's fixtures. The vectors `c.a until c.b`, `(c.a until c.b) until c.c`, `if c.a then eventually c.b else c.c` and `fair weak M::Cfg::step; c.done` read members `a`, `b`, `c` and `done` and a model `M::Cfg`. The ConfigVersion, game and job fixtures declare none of these. FR-165 does not say whether `missing-name` or `expression-form` wins when a predicate has both. An S3 that resolves names first refuses `missing-name` at `c.a` and fails AC-4. An S3 that refuses temporal operators first passes. Both satisfy FR-165 as written. | spec/functional/FR-165-check-state-graph-claims-at-s3.md:178; spec/test-cases/TC-590-s3-checks-state-graph-claims.md:23-24,35-38,55-59 |
| FND-002 | medium | AC-4 drops the interval and `to` vectors that the PR names as its reference. QSpec SG-R10 (`not eventually[0,5] c.n = 2`) and SG-R14 (`eventually[1,*] c.done`, with no profile refusal) have no AC-4 counterpart. Neither does SG-R11's `to (eventually c.done)`, or a `let` body. FR-165 line 94 says S1 SHALL NOT refuse a temporal operator "with or without an interval". An S1 that refuses `eventually[0,5]` inside a graph predicate, or that skips the `to` operand, still passes every AC. | spec/functional/FR-165-check-state-graph-claims-at-s3.md:94-100,178 |

## Verdict

Mergeable once the lead accepts or fixes the two mediums. There is no high
finding. Both mediums are oracle gaps in AC-4, and each can be fixed with a
single sentence or vector.

Checked and clean:

- **AC-6 discriminates.** An S1 that admits `exists(d in c.items: eventually
  d.n = 2)` produces a claim. S3 then refuses `expression-form`, but AC-6
  requires S1 `invalid_syntax` with no claim, so that S1 fails AC-6. The
  same holds for the call-argument, `if`-condition, `let`-binding,
  fairness-placement and flat-chain vectors. `exists(ident in expr: expr)` is
  a QSpec `primary` (shared-grammar.md lines 143-144), so the vector is well
  formed.
- **First-in-source-order rule.** It matches FR-390 (lines 118-120) and
  SG-R18. In `(c.a until c.b) until c.c` the refusal lands on the inner
  `until`.
- **Code name.** `invalid_syntax` is the S1 code that FR-002-AC-2, FR-362
  and TC-1060 use. It is also what `CompleteCode::InvalidSyntax` and
  `tests/it/compile_command.rs` emit.
- **Coverage.** TC-590 scope now names FR-165-AC-1 to AC-6. Step 5 covers
  AC-6 and step 4 covers AC-4.
- **Ids.** FR-165-AC-6 is new. No open PR (#968, #985, #987, #984, #983)
  defines FR-165-AC-6 or edits TC-590. #968 edits FR-165's Forms bullet, a
  separate hunk.
- **Checklist.** `quire validate` exits 0 on the three changed files. There
  are no hashes, digests, pins or tracking records.
- **FR-125 (STD-162).** FR-125 refers to FR-165 only for the checked claim
  and its `over` instantiation (FR-125 lines 60-61, 99, AC-7). Nothing
  there depends on S1 placement, so the change is consistent.
- **FR-165 AC-1, AC-2, AC-3 and AC-5.** They are unchanged and consistent
  with the new Parsing text. AC-2's `eventually c.versionNumber = 2` is a
  recognised placement.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-003 | low | The new name-first precedence rule is an S3 SHALL, but it sits under `### Parsing (S1)` rather than `### Predicates`, and its vector (`{ eventually c.versionNumber = newName }` refusing `missing-name`) sits in FR-165-AC-6, the S1 `invalid_syntax` criterion, rather than beside AC-5's `missing-name` case. The behaviour is correct; a reader looking for S3 refusal order under Predicates will miss it. | spec/functional/FR-165-check-state-graph-claims-at-s3.md:116-118,189 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | 0a710fa5e rewrites every AC-4 and AC-6 vector over `over (c: Config::ConfigVersion)` with `c.versionNumber` and `Config::ConfigVersion::attemptUpdate`, one fault per vector, and adds the S3 rule that names resolve first (an unresolved name plus a recognised operator refuses `missing-name`), with a vector in AC-6. The S3 oracle is now decidable over TC-590's fixture. |
| FND-002 | fixed | 0a710fa5e adds `not eventually[0,5] ...`, `eventually[1,*] ...` (with "no profile refusal"), `unique path` with `to (eventually c.versionNumber = 2)`, `always possible` with `from (always ...)`, a `let` body and an `if` branch to AC-4, and TC-590 step 4 lists them. |
