---
id: SR-12127
title: "QSL-849 PR #956 code review: channels, joins, terminal and scheduling members (FR-218, FR-308)"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@a5115665a91042bc33b64b0571ca9a21df0664b8; qsl-cst/src/cst.rs, qsl-cst/src/grammar.rs, qsl-cst/tests/it/complete_grammar.rs, qsl-forms/src/lib.rs, qsl-forms/src/protocol_clause.rs, qsl-forms/src/protocol_control.rs, qsl-forms/src/syntax.rs, qsl-forms/src/value.rs, qsl-forms/tests/it/main.rs, qsl-forms/tests/it/protocol_control_forms.rs, qsl-replay/src/spine.rs, qsl-semantics/src/check/mod.rs, qsl-semantics/src/check/protocol_clause/coverage.rs, qsl-semantics/src/check/protocol_clause/mod.rs, qsl-semantics/src/check/protocol_clause/tests/coverage.rs, qsl-semantics/src/check/protocol_clause/types.rs, qsl-semantics/src/check/protocol_controls.rs, qsl-semantics/src/check/refusal.rs"
review_set: subset
---

## Summary

Ticket: QSL-849. PR quire-spec-language#956. Code review with the rust-review lane (agent-skills `skills/rust-review/SKILL.md`), scoped to `git diff origin/main...HEAD` (18 files, +2459/-44, 3 commits). Read only. No build was run. The PR description reports the gates (`cargo clippy --workspace --all-targets --all-features -D warnings`, `make ci`) as the lead's receipt, and this review did not re-run them.

The grammar changes (loss selector, optional repeat maximum and loop proof, `terminal`/`scheduling` members, `bound()` counts, an empty join list) were compared with the shared grammar productions `repetition`, `protocol-clause`, `protocol-terminal` and `protocol-scheduling`. They match. The empty join list is a deliberate superset, because FR-218-AC-23 needs `join any []` to reach S3. The S2 builders read only direct tokens and nested nodes. They do not panic, and every malformed shape returns `unexpected`. The S3 checks in `protocol_controls.rs` keep the refusal code, cause, span and retained members as typed fields.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Verdict

PASS, with no major bug. The change is correct on the paths it claims, and the coverage gaps are recorded in SR-12128.

Checked and clean:

- `check_channel` refuses each undeclared end independently (`from` first, by span sort), refuses an unresolved `carries` type through `resolve_form`, and refuses an object or population payload once at the type. It records the four premises exactly as written, with no derivation.
- `check_join` refuses unknown labels at each label, and only then checks the policy. `join all` omitting a branch, `quorum` below one or above the distinct label count (with a `usize::try_from` overflow guard), and an empty `any`/`predicate` list each refuse at the list or the count.
- `refuse_repeated_members` refuses the second `terminal` and the second `scheduling` at its own span, naming the first (FR-211, FR-212).
- `coverage.rs` stops refusing `parallel`/`branch` and plain channels as unimplemented. It still refuses a `fifo` key, a `predicate` join and a `terminal when` predicate through new construct markers, so none of them compiles unchecked. `protocol_reference::operator` already names `parallel` as a root operator, so a protocol whose root is `parallel` that now checks reaches S4's existing stub path.
- The new refusal variants map to the FR-218 catalog codes in `refusal.rs`. The `qsl-replay` message arms cover every new variant, with no catch-all.

Recorded lows, not findings:

- `protocol_controls.rs:150-155` sets `ChannelCarriesObject.name` to `String::new()` when the head is not a name. That is a sentinel where an `Option` or the type's text would do.
- The role-model rule is split across `coverage.rs:111-112` (no type at all: `MissingRoleType`) and `protocol_controls.rs:89` (a type that is not an object type: `RoleModel`), with complementary conditions. That puts one fact in two places.
- `protocol_controls.rs:61` duplicates `protocol_clause::refusal`.
- `protocol_controls.rs:703` checks the one-case `choice` and one-branch `parallel` parser failures with `!is_admissible()` alone. It never asserts the syntax diagnostic code.
