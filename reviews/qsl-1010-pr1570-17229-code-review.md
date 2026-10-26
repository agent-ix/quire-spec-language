---
id: SR-17229
title: "Code review (rust-review lane) of QSL-1010 PR B(a), hyper product identity and refusals (FR-315)"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@34dab5e0880b9b27319c8903ae209b579d1935b2; PR #1570 (branch qsl-1010-hyper-identity), QSL-1010 PR B(a): qsl-eval/src/smt/refusal.rs, qsl-replay/src/model_check/hyper_smt_query.rs, qsl-replay/src/lib.rs, qsl-replay/src/model_check.rs, qsl-replay/tests/it/hyper_identity.rs, qsl-replay/tests/it/vault_world.rs"
review_set: subset
date: '2026-10-25'
---

## Summary

Ticket: QSL-1010. Rust review of the six new `Refusal` variants (rows 18-23) and of `HyperQueryRequest`, `HyperQuery` and `HyperProductIdentity::of`. The check order hyper-form, subject-binding, instance-binding, product-identity matches FR-315 "Refusals and their order"; `of` evaluates nothing and touches no meter; an unassembled clause (`assembled_occurrence()` `None`) gives product-identity; `HyperProductIdentity` has private members and derives Clone, Debug, Eq, Hash, PartialEq with no serde. Each test tag matches its assertion and none is a stub or tautology. Gates run: `cargo test -p qsl-replay` (all binaries and doc-tests pass), `cargo test -p qsl-eval --test it -- smt_query` (36 pass), `cargo clippy -p qsl-eval -p qsl-replay --all-targets -- -D warnings` clean, `cargo fmt --check` clean, `xtask string-edge` reports exactly the 11 baseline findings (execute.rs, process_request.rs), none new; arch-lint api-surface-qsl and qualified-core pass. `arch-lint-canonical-encoder` fails only on qsl-semantics and root-crate files this PR does not touch.

The instance-binding fallback is not a hole in `of`: it only widens the keys accepted for a subject that cannot list instances, and the identity it returns is still the exact identity of the request. The admitted residual (a non-admitting subject whose package has an emission passes `of`) is acceptable to defer to PR B(b) provided B(b) gives `encode_hyper_query` a defined outcome for it (FND-002).

## Verdict

**CONDITIONAL** — no blocker for B(a); FND-001 and FND-002 need a fix here or an explicit B(b) ticket.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | `HyperProductIdentity::of` has no `Fault` arm although FR-315 Outcome handoff and AC-30 require `of` to return `SmtEncodingRefusal::Fault` for a broken checked-clause invariant. The one branch that is such an invariant (a parameter whose alias is not in `clause.aliases()`, so `slot` is `None`) is reported as the user-facing refusal `instance-binding`. | qsl-replay/src/model_check/hyper_smt_query.rs:192-198 |
| FND-002 | medium | The instance-binding fallback applies not only when the subject does not admit but also whenever `ModelSystem::instances` errors (RequiresBound, Extent), and then accepts a key of any universe of any population. A non-admitting subject with an emission therefore gets an identity from `of`, and no hyper refusal code (the hyper list has no subject-admission, requires-bound or no-instance) names that case for `encode_hyper_query`; nothing in the PR tests it or records where PR B(b) will handle it. | qsl-replay/src/model_check/hyper_smt_query.rs:199-222 |
| FND-003 | low | AC-13's last sentence (a `match` conjunct whose evaluation exhausts the meter: `of` returns the identity and the consumed counters stay zero) is not asserted by one test: the small-budget case in the AC-12 test does not check the meter, and the charges-nothing test uses an unlimited meter and reads `admission_count()` only, not `consumed`. | qsl-replay/tests/it/hyper_identity.rs:100-120, 270-285 |
| FND-004 | low | The new `Refusal` variants `HyperBody` and `MatchConjunct` and the rows 18-23 returned by `row()` have no test in this PR (the qsl-replay tests reach only four of the six codes through `code()`); the `row()` doc says "FR-315's refusal table" but that table lists rows 1 to 16 only. | qsl-eval/src/smt/refusal.rs:240-306 |

## Coverage

Examined: FR-315 Refusals and their order, Members, In-process only, QSL mints it; FR-315-AC-12, AC-13, AC-49; `check_subjects` (same package by pointer or emitted package id, excess subject names no alias), `check_instance`, `form_name` against FR-173's HP-1 to HP-6, privacy and derives of `HyperProductIdentity`. Test-to-AC bindings: hyper_identity.rs tests carry TC-1190 with AC-12 (two), AC-13 (five) and AC-49 (one); each asserts what its tag says.

## Dispositions

(none yet)
