---
id: SR-20261110042820830006
title: "QSL-1819 native RequiresBound gap-analysis"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language; QSL-1819; PR #2652; qsl-conformance/tests/it/prove_row.rs and the RequiresBound provider/projection contract"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-564
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-281
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-286
    type: references
  - target: ix://agent-ix/quire-spec-language/TC-1393
    type: references
---

## Summary

Independent scoped manual gap-analysis of the single-file test-only PR #2652. No findings.

## Verdict

PASS for this exact scoped test delta. Broader counterexample and routing acceptance remains open; this is not repository qualification.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Coverage

Reviewed frozen head `2aeb495153d9d26b251933324cc27aff992abf7a`; the only net changed file is `qsl-conformance/tests/it/prove_row.rs` (36 additions, one deletion). No production, API, case, dependency, fixture, spec or gate change. The earlier Analyze modification was reverted; Analyze remains FR-281-AC-3, and Prove remains the real EN-1 RequiresBound case mandated by FR-564. The new assertions inspect the actual record produced through public `run_case` / `provider::run`, not a substitute engine or reconstructed outcome.

The exact single record, request index, typed unsupported disposition and catalog code are asserted. Its actual public disposition payload must contain precisely temporal-satisfaction, en-1 and both advertised modes. The returned Report item must equal the typed projection of that same record, while independent literal wire assertions check the actual diagnostic detail, omission of cause and exit 21. Debug formatting is used only for a failing assertion message. `backend()` is correctly not used to infer an engine identity for a record without replay evidence. The native provider path and unbounded Integer-root fixture were inspected; no test-only API or invocation hook is needed.

The added assertions can fail on wrong index, wrong terminal classification, missing/wrong catalog code, missing/extra/different payload fields or modes, a mismatched returned item, present cause or wrong exit. The same-projection equality is a linkage control, not the sole semantic oracle. No duplicate producer, JSON overlay, vendor copy, new public surface or speculative behavior is introduced. No applicable AssuranceProfile was found. There are no concurrency, unsafe, Tokio or new resource-lifecycle paths in this test delta.

Supplied focused results were inspected: fmt passed; qsl-conformance test-inclusive all-target Clippy passed; matrix integration passed 16 tests with no failure. No Cargo or proof jobs were run by this reviewer; the lead owns the final affected gate. No full suite or repository qualification is claimed. Plan completion: not assessed.

This is the explicitly requested single-test scoped manual trace/assertion/producer review, not a repository-wide quoin matrix audit. No computed matrix counts or repository-wide PASS are claimed, and no handwritten TestMatrix was made. The existing real `#[trace("TC-1393")]` binding is retained; no full FR-564-AC-6 or compound FR-281-AC-11 tag is added. The refuted-item counterexample/wire obligations remain open and outside this test delta. This change does not close QSL-1819, QSL-1820 or the QSL-12 gate. PendingEngine is already absent from current source and normative specs; historical review statements are not edited.

## Examined Scope

- `FR-564-AC-6` — role: examined; `spec/functional/FR-564-produce-each-outcome-kind-of-the-lifecycle-conformance-matrix-through-the-library.md`; excerpt: Each of the applicable `prove` (QSL half) cells yields one FR-286 item of its kind: through `qsl_analyze::provider::run`, `proved` exit 0, `refuted` exit 10 with its counterexample, `unsupported` with `unsupported_projection`/`unbounded-extent` exit 21, `incomplete` naming `max_states` exit 22; the binding refusal exit 20; `incomplete` `timed-out` and `incomplete` cancelled, exit 22 (FR-291-AC-2's records); `failed` with `tool-failure{crash}`, exit 30.

- `FR-281-AC-11` — role: examined; `spec/functional/FR-281-analyze-claims-with-in-process-engines.md`; excerpt: Over a model one of whose operations `modifies` a field of type `Integer` (an unbounded FR-120 root), an EN-1 item over `Engines::qsl()` settled `Supported` on EN-1 settles `unsupported` with `unsupported_projection`/`unbounded-extent` whose payload names `temporal-satisfaction`, EN-1's `BackendId` and its advertised modes `bounded` and `unbounded`, and the item's exit is 21.

- `FR-564` — role: context_only; `spec/functional/FR-564-produce-each-outcome-kind-of-the-lifecycle-conformance-matrix-through-the-library.md`; excerpt: `prove` unsupported cell is the `RequiresBound` refusal of FR-281-AC-11, never a
state budget; the `analyze` unsupported cell is FR-281-AC-3's item.

- `TC-1393` — role: examined; `spec/test-cases/TC-1393-the-qsl-half-of-prove-yields-each-outcome-kind.md`; excerpt: Run the `prove` (QSL half) row of FR-564's matrix: QSL engines served as
providers, and FR-291's plugin settlement records.

- `FR-286` — role: examined; `spec/functional/FR-286-serialize-every-outcome-as-one-json-outcome-document.md`; excerpt: record carries exactly one of `cause` and `diagnostic`, so a disposition's
  record carries the diagnostic and no `cause`.
