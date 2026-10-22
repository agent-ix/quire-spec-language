---
id: SR-15526
title: "spec review of STD-297 checked-term boundary (QSL PR #1225)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@09f27280c4441e702eadfdae0d08691602dc974b; spec/functional/FR-400-separate-checked-terms-from-the-checker.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-400
    type: reviews
---
# SR-15526: spec review of STD-297 checked-term boundary (QSL PR #1225)

## Summary

Ticket: STD-297. This review checks whether FR-400-AC-1 to AC-9 can be tested, and judges the author's ten lead
defaults. The PR body labels them "lead defaults, not owner answers". This is a spec-only diff, so `code-review` and
`gap-analysis` have no code to review and were skipped. The checks against the code are in SR-15524, and the
cross-document checks are in SR-15525.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | minor: AC-8 compares every `Location`, `NodeKey`, `DeclarationKey`, correspondence entry and `SourceRegion` with "the values the checker produced before the move". Once the move lands, no baseline exists. Require a golden that is recorded in the QSL tree from main before the first move step, and say where it is kept. | spec/functional/FR-400-separate-checked-terms-from-the-checker.md:249 |
| FND-002 | medium | minor: AC-4 says "a call to a `construct` function when the `construct` feature is off fails to compile". In QSL, feature unification turns `construct` on for every crate built alongside `qsl-semantics`, as AD-001 Risks says. So this clause can only run as a doctest in quire-eval's own `quire-checked` with default features. Say which repository's gate runs it. | spec/functional/FR-400-separate-checked-terms-from-the-checker.md:245 |
| FND-003 | low | minor: AC-7 lists QSpec's `checked_package_v2` test. That test does not build QSL code, so this change cannot make it fail. Drop it, or keep only the QSL conformance run (`cargo test -p qsl-package --lib checked_v2` with `QSPEC_DIR`). AC-6's T12-B to T12-D clause passes trivially for C, because `arch-lint` scans only QSL's tree. Its clippy `disallowed-methods` clause is the real check. | spec/functional/FR-400-separate-checked-terms-from-the-checker.md:247-248 |

## Verdict

AC-1, AC-2, AC-3, AC-5 and AC-9 are testable as written. AC-2 is testable but cannot pass within S2 (SR-15524
FND-004). AC-4, AC-7 and AC-8 need the fixes above.

Judgement of the ten lead defaults (lead defaults, not owner answers):

1. A new repository and the name `quire-checked`: sound. A crate hosted in QSL would close a repository cycle once
   the evaluation core depends on it. The repository is an owner action that blocks S2, and AD-001 must list it as a
   blocker (SR-15525 FND-001).
2. Joining FB-05's class: sound. The row requires an amendment, and the PR makes one.
3. no_std with alloc and a default-on `std`: sound, and it matches SV.
4. Feature plus T12-G plus the FR-059 feature check: sound, and the feature-unification gap is stated honestly.
5. Spans stay with the checker: sound in intent, but applied only to `CheckedGraph`. Nested clause records still
   carry spans and forms values (SR-15524 FND-002).
6. The result vocabulary and `WrongSnapshotCause` do not move: this conflicts with AC-2 (SR-15524 FND-004).
7. No re-exports: matches ADR-011 §7.2 and X-11. Sound.
8. Runtime model admission stays: matches the code, where qsl-eval calls `model` and `value` directly. Sound.
9. `DeclarationKey` as a value, with `identity()` moving to `model`: sound. `validate_derivation` must follow
   (SR-15524 FND-005).
10. `OperationTable` moves with the `Scope` view: sound. The view itself is incomplete (SR-15524 FND-001).

Mergeable after these minor fixes and the majors in SR-15524 and SR-15525.
