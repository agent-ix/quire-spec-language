---
id: SR-1935003
title: "spec review of PR #1935 (QSL-1306): FR-315 Normal form, FR-315-AC-53/56, TC-900 step 28"
type: SpecReview
analysis: integrity
date: 2026-10-31
scope: "agent-ix/quire-spec-language; PR #1935 git diff origin/main...13402c26a (spec/functional/FR-315-encode-the-smt-lib-transition-relation.md, spec/test-cases/TC-900-the-smt-lib-encoding-is-canonical.md)"
review_set: subset
relationships:
- target: ix://agent-ix/quire-spec-language/FR-315
  type: reviews
- target: ix://agent-ix/quire-spec-language/TC-900
  type: reviews
- target: ix://agent-ix/quire-spec-language/FR-314
  type: references
---
# SR-1935003: spec review of PR #1935 (QSL-1306)

## Summary

Ticket: QSL-1306. Spec-review, integrity and consistency lane, over the FR-315 and TC-900 edits checked against the code in `qsl-eval/src/smt/normal.rs` and `layout.rs`. `quire validate` on both files exits 0. The seven rules match the code rule for rule, including `=` in rule 5 and `(- t 0)`/`(- t t)` in rule 7. The Canonical printing exception and FR-314's "byte comparison is over the normalised script" agree with the checker, which rebuilds through the same encoder.

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| FR-315 Normal form | spec/functional/FR-315-encode-the-smt-lib-transition-relation.md:927-967 | examined |
| FR-315 Canonical printing exception | spec/functional/FR-315-encode-the-smt-lib-transition-relation.md:701-706 | examined |
| FR-315-AC-53 | spec/functional/FR-315-encode-the-smt-lib-transition-relation.md:1412 | examined |
| FR-315-AC-56 | spec/functional/FR-315-encode-the-smt-lib-transition-relation.md:1415 | examined |
| TC-900 scope and step 28 | spec/test-cases/TC-900-the-smt-lib-encoding-is-canonical.md:16,90,200 | examined |
| FR-315 Applications / Initial | spec/functional/FR-315-encode-the-smt-lib-transition-relation.md:872-925 | context_only |
| FR-314 query rule | spec/functional/FR-314-check-an-smt-proof-certificate.md:199 | context_only |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Normal form claims the script then holds "no subterm a solver's rewriter still simplifies". That is false. `(mod k n)` over a literal, `(- 0 t)`, `(+ t (- u))`, `(* n (- t))`, and arithmetic atoms in source orientation all survive, and the PR body itself says cvc5 may still rewrite them. The same claim appears in normal.rs:3-4 and refinement.rs:13-14 | spec/functional/FR-315-encode-the-smt-lib-transition-relation.md:927-931 |
| FND-002 | low | "the initial-state predicate prints an absent or `false` Boolean field as `(not s0.<object>.<field>)`" is wrong for an absent field. That prints `(not s0.<object>.<field>.present)`, as it did before normalisation (layout.rs:298) | spec/functional/FR-315-encode-the-smt-lib-transition-relation.md:964-965 |

## Verdict

**Two wording defects, no contradiction with the code's behaviour.** FND-001 overstates what the normal form guarantees. Narrow the purpose clause to the listed shapes. FND-002 should read "a `false` Boolean field as `(not s0.<object>.<field>)`".
