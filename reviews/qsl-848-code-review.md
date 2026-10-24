---
id: SR-12111
title: "Code review of quire-spec-language PR 948: S2 forms for protocol captures, relationships, requirements, related-by clauses and compensations"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@27217c8fcee844f1816899585ec6c2a46e0dd49a; PR #948: qsl-cst/src/grammar.rs, qsl-forms/src/lib.rs, qsl-forms/src/protocol_clause.rs, qsl-forms/src/syntax.rs, qsl-forms/src/temporal_clause.rs, qsl-forms/tests/it/main.rs, qsl-forms/tests/it/protocol_member_forms.rs, spec/functional/FR-308-build-the-complete-protocol-forms-at-s2.md, PR body"
review_set: subset
---

## Summary

Ticket: QSL-848 (C1a under QSL-839). This is one code-review pass with the
rust-review lane folded in. It covers `git diff origin/main...HEAD` (8 files,
+813/-27) at head 27217c8fc. The review was read-only: nothing was built or
run. The leader's gate receipt (check, clippy -D warnings, fmt, the crate
tests, xtask, string-edge, VAL, IDX all EXIT=0) is taken as given.

What the diff builds:

- `CompensationForm` and its members (`CompensationAttempts`,
  `CompensationRetry`, `CompensationCommit`, `CompensationRecover`).
- `RelationshipForm` and `RequirementForm`.
- `RelatedForm` on `ProtocolNodeDeclaration.related`.
- `ProtocolDeclarationForm.captures`, `relationships`, `requirements` and
  `compensations`.
- The S1 grammar change for the `attempts` count, from `uint()` to the
  `u64`-checked `bound()` terminal.
- FR-308 Outputs, rewritten to the built type names.

Each form maps its S1 production in grammar order:

- Relationship is `ident = ModelName ;`.
- Requirement is `requires temporal ident ;`.
- Related is `related by ident ( Expression , Expression )`.
- Compensation has five `Parameter` nodes (forward, trigger, earlier, later,
  recover) and three `Block` nodes (guard, relation, predicate). Its captures
  are split by the guard block's start into registration and activation
  stages. The first `NodeReference` is `for` and the second is `commit`, with
  an exclusive `never`.

A malformed CST returns `unexpected(node)` and never panics. A count that does
not parse after S1 is an invariant break, mapped to a refusal. Leading zeros
are refused by the lexer (`LexError::LeadingZero`). A count past `u64` fails
the `Bound` terminal with `invalid_syntax` at the count token. The diff adds
no `#[allow]`, no `unwrap` or `expect` outside tests, no ticket ids in code
comments, and no vendoring or compatibility layer.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The PR body's claims do not match the diff. (a) It says the PR builds "the control constructs ... and the channel overflow policy". It builds neither: no control-construct form changes, and `P::OverflowPolicy` stays `c[reject, block, loss]` with no selector, so FR-308-AC-13 is unbuilt. (b) It never names what the PR does build: the capture, relationship, `requires temporal`, `related by` and `CompensationForm` members (FR-308-AC-7..12, TC-1055). (c) It sends ChannelForm, replicated RoleForm and activation to QSL-850, but QSL-849 is "control constructs and channels" and QSL-850 is the S3 roles/captures/compensations/recovery checks. (d) "the grammar they need" is a one-terminal change. The body becomes the squash-commit message, so it should be corrected before merge. | PR #948 body; qsl-cst/src/grammar.rs:1217-1220 |

## Verdict

PASS, with a correction needed to the PR body (FND-001). The code adds no
high-severity defect. Every new S2 form follows its S1 production's order and
optionality. Spans are the CST node or token spans. A malformed CST refuses
rather than panics. The `attempts` count is `u64`-checked at S1. The idioms
are clean: `let [..] = ..[..] else` destructuring, an exhaustive `match` on
the commit/never pair, no `#[allow]`, and `pub(crate)` widening of four
helpers in `temporal_clause.rs` instead of copying them.

Notes (not findings): FR-308-AC-1..4 and TC-887 are unchanged from main.
They are present, and no test is traced to them, because their forms
(ParallelForm, RepeatForm, the replicated role, the channel, activation,
terminal and scheduling) are not built in this PR. FR-308-AC-13's grammar
(the `loss head|tail|new` selector) is also absent on main. Today
`overflow loss` with no selector parses and `overflow loss head` does not,
which is the reverse of what AC-13 states. That is outside this diff, and it
belongs to the channels slice.
