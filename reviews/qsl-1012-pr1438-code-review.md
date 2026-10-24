---
id: SR-17007
title: "Code review of PR #1438 ValueTypeRef::Unit arm in the domain checker (QSL-1012 follow-up)"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language PR #1438 at 5f17eb2fa; src/checking/types.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-042
    type: reviews
---

## Summary

Ticket: QSL-1012 (follow-up to #1431, e8ea5ae63). Code review with the rust-review lane folded in. origin/main (5a2c330db) fails E0004 because #1431 added `ValueTypeRef::Unit(String)`; this PR adds `ValueTypeRef::Unit(_) => return None` in `domain_leaf_type` plus a unit test. `cargo check --locked --workspace --all-targets` passes at head (no other non-exhaustive match). The new test passes and kills the mutation `Unit(_) => NativeType::String`.

Semantics: `None` is not a silent skip. Both callers turn it into an explicit refusal, `CauseKind::UnsupportedPrerequisite(Prerequisite::DomainRepresentation)` (field access via `DomainField::Unrepresented` in solver/expressions.rs:430; operation parameter/result via solver.rs:517), and protocol export maps `Unrepresented` to `Unsupported::Export`. This is the behaviour FR-042-AC-14 prescribes for "a parameter or result whose type has no native type here", and matches how Integer/Rational/Decimal/Timestamp leaves are already treated. #1431 scoped unit typeRefs to intake, assembler resolution (FR-056-AC-42) and probabilistic-model encoding (FR-120-AC-16, FR-189-AC-15); it did not claim domain-checker support. Mapping to a native quantity type is not possible here without the compiling unit's resolved unit, which `domain_leaf_type` does not receive, so it would be new feature work, not a fix.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | New test `unit_value_type_is_not_a_domain_leaf` carries no `#[trace]` tag, unlike its sibling test in the same module; the refusal behaviour it pins belongs to FR-042-AC-14. | src/checking/types.rs:818-831 |
| FND-002 | low | Doc comments enumerating the `None`/`Unrepresented` cases (`DomainField::Unrepresented`, `domain_value_type`) do not mention a unit typeRef. | src/checking/types.rs:32-35, 106-111 |

## Verdict

PASS. The fix is correct, sound (refuses rather than skips) and minimal; it unblocks main. Both findings are low and may be deferred to a follow-up without blocking merge.
