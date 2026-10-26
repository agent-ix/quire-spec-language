---
id: SR-17236
title: "Review of delay distribution declarations at S1-S3 (quire-spec-language PR 1578, QSL-1003)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@3477375c; spec/functional/FR-253-check-delay-distributions-and-define-the-race.md; QSpec proposals/quire-v1/shared-grammar.md delay-decl, delay-dist, weighted-delay"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-253
    type: references
---

# Review of QSL-1003 delay declarations (PR 1578)

## Summary

One pass over `git diff origin/main...3477375c` (rust-review, code-review, gap-analysis,
spec-review of the FR-253 wording). The grammar is faithful to QSpec `delay-decl`,
`delay-dist` and `weighted-delay`: bare `uniform`, `uniform[a, b]`, `exponential(x)`,
and `discrete { ... }` with at least one weighted delay and an optional trailing
comma. `delay`, `uniform`, `exponential` and `discrete` are matched as identifiers
(`literal`), and TC-708 parses them as enum members in the same source. Every
production table is updated (cst.rs, grammar.rs, value.rs both match arms, editor.rs
is_symbol, dispatch, the corpus). The S3 pass resolves the operation, requires a
`time dense` source, converts each quantity exactly to the model's unit (probed:
`1 s` is 1000 ms, `0.0003 s` is 3/10 ms, `rational(1, 3) s` is 1000/3 ms, a mixed
`1 ms, 1 s, 0 ms` list), and refuses with the FR-253 codes. Probes (not committed):
`kg` quantity refuses TimeQuantity/NotTimeDimension; `uniform[2 ms, 2 ms]`,
a single `0 ms` discrete delay, `exponential` of 1e-30 and of 1e36, and a
44-digit weight pair all check; weight 0, `exponential(0.0)`, a negative delay
(`rational(-1, 1) ms`), a repeated delay, a second declaration of one operation,
an unknown operation (unresolved-operation) and an untimed model all refuse with
the right code and a span on the offending value. Focused gates at 3477375c all pass:
qsl-cst, qsl-forms, qsl-semantics (test-support), qsl-replay, quire-spec-language lib,
clippy -D warnings on the four touched crates, xtask conformance, string-edge and
cargo test -p xtask.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | low | FR-253-AC-3 and the Behavior bullet name untimed models; the code also refuses a `time tick` model (it admits only `TimeSourceKind::Dense`), but no test covers a tick model. Only the untimed case is asserted. Add a tick-model case to timed_declarations.rs. | qsl-semantics/tests/it/timed_declarations.rs | missing-test |
| FND-002 | low | Refusal tests cover exponential(0), a repeated discrete delay, uniform[3 ms, 1 ms] and untimed. Not asserted: a non-positive discrete weight, a negative discrete or uniform bound, a second delay on one operation (RepeatedDeclaration), an unknown operation, a mixed-unit list, `uniform[a, a]`, a zero delay, and the `Exponential` and `Discrete` recorded values (the only recorded-value test is UniformRange in s to ms). All behave correctly under review probes; they are branches with no test. | qsl-semantics/tests/it/timed_declarations.rs | missing-test |
| FND-003 | low | Spec gap, not a code defect: QSpec FR-405 says an operation declares at most one `delay`, but names no code for a second declaration. The PR refuses it as `invalid_model_binding`/`malformed-declaration` (DelayFault::RepeatedDeclaration). Reasonable and recorded in a doc comment; FR-253 Behavior does not list it, so FR-253 (or QSpec) should name the code. Also the second declaration is refused even when the first was itself malformed, a cascade refusal. | qsl-semantics/src/check/probabilistic/model.rs; spec/functional/FR-253-check-delay-distributions-and-define-the-race.md | spec-gap |
| FND-004 | low | The claim fingerprint (`reward_bytes`, `weight_bytes` in claim.rs) hashes rewards but not delays. A delay changes the probability measure a claim is about, so two units that differ only in a delay share a claim identity. Harmless today since nothing consumes delays (the race is QSL-588), but QSL-588 should add delays to the claim normal-form bytes. | qsl-semantics/src/check/probabilistic/claim.rs | deferred-consumer |
| FND-005 | info | `exponential(rate)` is recorded unconverted, as per model time unit (matches the FR-253-AC-1 `1/1000 per ms`); the rate is never converted from a different written unit because it carries none. FR-253 Outputs says so only implicitly. | qsl-semantics/src/check/probabilistic/model.rs | none |
| FND-006 | info | Only the first fault of a malformed `discrete` list is reported, and a time-quantity conversion fault returns before later entries are checked. Acceptable; no correctness impact. | qsl-semantics/src/check/probabilistic/model.rs | none |

## Verdict

PASS. No blocking or medium findings; FND-001 and FND-002 (low, missing tests) should be fixed in this PR if the coder is already in the file, otherwise tracked with QSL-588. FND-003 and FND-004 are tracked for the spec owner and QSL-588.
