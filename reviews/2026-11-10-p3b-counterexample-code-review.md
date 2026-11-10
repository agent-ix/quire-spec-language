---
id: SR-20261110041940
title: "Code and Rust review of l5/p3b-counterexample (FR-281/FR-286 counterexample wire)"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@e2c84f33e; diff origin/l5/p3-matrix...HEAD: qsl-replay/src/outcome{.rs,/counterexample.rs}, qsl-replay/src/contract_check.rs, qsl-analyze/src/{analyze.rs,analyze/*.rs,contract_check/mod.rs,provider.rs}, qsl-eval/src/simulation/{model.rs,model/encode.rs,model/letters.rs,protocol.rs,protocol/wire.rs}, qsl-conformance/src/run.rs, tests"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-281
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-286
    type: reviews
---

## Summary

Ticket: QSL-12. Branch `l5/p3b-counterexample` at `e2c84f33e`, stacked on `l5/p3-matrix`, no PR yet. Code review with the Rust lane (rust-style loaded). The wire spellings were checked against QSpec FR-364, FR-379, FR-413, FR-433, FR-438 and FR-485 and against `proposals/backend-provider-v1/schema.json` in `/home/peter/dev/quire-specification`. `cargo test -p qsl-analyze -p qsl-conformance -p qsl-replay` was run at head through the locked build lane; the result is in the Verdict.

Known approximations the lead listed were evaluated, not auto-filed. Contract argument ordinals and state keys in place of typed values are kept as one medium finding (FND-004), because the schema refuses them. The FR-379 `Undefined` row object is part of FND-003. An unspelled `ClaimPredicate` locus is accepted: only EN-4 builds it, and EN-4 never settles `refuted`. A timed undefined site that is not spelled is part of FND-001, because EN-6 does reach it. The missing Hybrid/Hyper/Graph variants are accepted: `analyze` routes no hybrid item to an engine.

## Verdict

**FAIL: not mergeable as is.** FND-001 and FND-002 are high. `cargo test -p qsl-analyze -p qsl-conformance -p qsl-replay` at `e2c84f33e`: every suite passed, 0 failed (qsl-analyze it 1469, qsl-replay lib 1214, among others). The tests pass because none of them exercises the failing paths.

What is right: `split_records` gives exactly one `ItemCounterexample` per reproduced `refuted` record and replaces the old `counterexamples()` take-loop. `OutcomeItem::from_analyze` enforces `MissingCounterexample`. `analyze_document` is the one item writer shared by `AnalyzeOutcome::document`, `ProviderResults::document` and the conformance `prove` row, so analyze and prove give equal bytes. The temporal, protocol, race, probabilistic, contract and refinement tests go through the real engines and check that the document carries the same spelled value. No ceremony, digests, pins, vendoring or compatibility layer were added.

What blocks: a spelling error on any one item fails the whole `analyze` document, and real EN-1/EN-6 paths hit that error (FND-001). The undefined-evaluation `where.locus` is not the FR-364/FR-331 `Locus` (FND-002).

## Scope

- FR-281 "exactly one `ItemCounterexample` for each `refuted` record": examined (FND-001, FND-006).
- FR-286 "Analyze items and their counterexamples" table, rows Temporal, Probabilistic, Assignments and Contract: examined (FND-002, FND-003, FND-004, FND-005, FND-007).
- FR-286 `from_analyze` / `MissingCounterexample`: examined, clean.
- FR-286 "One spelling serves the document `analyze` writes and the item `prove` writes": examined (FND-006).
- QSpec FR-364 Wire form, FR-379 refinement_failure, FR-413 WitnessScheduler, FR-433 Wire, FR-438 Race counterexample, FR-485 contract-counterexample: context_only (the schema is the authority read).
- `qsl-conformance/src/run.rs` analyze/prove rows: examined (FND-010).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | A spelling error on one item fails the whole analyze document. `analyze_item` propagates `ItemCounterexample.wire`'s `Err` with `?`, so `AnalyzeOutcome::document`, `ProviderResults::document` and the conformance rows return `Err`, and no document is written. Real refuted paths produce that `Err`. (a) EN-6 zone liveness sets `over_binding: Some` (zone_check/liveness.rs:1371), but only `ClauseSubject` fills `over_reference`, so `EngineReport::Zone`, and `EngineReport::Model` from the public `model_item`, give `Unspelled("over binding")`. (b) An EN-6 undefined site carries `time: Some` (zone_check/search.rs:2130), which gives `Unspelled("timed undefined site")`. Before this branch these runs wrote a document. Each other item should still settle, and the unspelled item should not kill its siblings. No test covers the `Err` path. | qsl-analyze/src/analyze.rs:583-592, qsl-replay/src/outcome/counterexample.rs:148-154, qsl-replay/src/outcome/counterexample.rs:332-337, qsl-analyze/src/analyze/clause_item.rs:221-232 |
| FND-002 | high | The undefined-evaluation `where.locus` is written in engine-internal shapes: `{formula_node}`, `{guard:"on-each"}`, `{capture}`, `{contract}`, `{terminal}`, `{constraint}`, `{protocol_node}` and `{mapping:{row,object}}`. FR-364 defines `locus` as "the source locus of the expression that has no value", and the schema's `Where.locus` is `Locus {node, occurrence}` with `additionalProperties: false`. FR-379's row belongs in `where.row`, a `Nonempty` string, not inside `locus`. Every undefined-evaluation counterexample this branch writes is therefore refused by an FR-364 replayer. | qsl-replay/src/outcome/counterexample.rs:117-146 |
| FND-003 | medium | `refinement_failure` departs from the FR-379 schema in two ways. `Undefined.row` is written as an object (`{kind, object_type, field, ...}`), but the schema requires a `Nonempty` string. `Divergence` treats `constraint` as optional and adds `protocol_class`, but the schema requires `constraint` and forbids extra members. The Divergence case was not on the list of accepted approximations. | qsl-replay/src/outcome/counterexample.rs:181-219, qsl-replay/src/outcome/counterexample.rs:249-261, qsl-replay/src/outcome/counterexample.rs:280-284 |
| FND-004 | medium | The contract-counterexample does not match the FR-485 schema. `arguments[].parameter` is written as an integer ordinal, but the schema requires a `NodeId`. `pre_state`/`post_state` are written as a State `{key}`, but the schema requires a `TypedValue`. These are known approximations, so this needs a ticket with an expiry rather than a block. As written, every refuted contract item is schema-invalid. | qsl-replay/src/contract_check.rs:724-756 |
| FND-005 | medium | The probabilistic `scheduler.entries` are keyed by `StateKey::from_canonical_bytes(state.model)`, the model state, and not by the product state key that FR-413 requires ("its product state key"). Under a bounded monitor, two product states that share a model state give two entries with the same `state` and different choices. The entries are also left in production order, while the internal scheduler is sorted. | qsl-analyze/src/analyze/exact.rs:1093-1099 |
| FND-006 | medium | There are two diverging item writers for one analyze record. The retained public `AnalyzeRecord::outcome_item` writes a `refuted` item with no `counterexample`, which breaks FR-286's "A `refuted` item carries `counterexample`". For an undefined refutation it writes cause `undefined-evaluation{at, cause}`. `analyze_item`/`from_analyze` writes the counterexample but no cause, because `from_terminal` gives `None` for `Refuted`, so the document loses FR-281-AC-7's `UndefinedEvaluation` cause. FR-286 calls for one spelling. | qsl-analyze/src/analyze.rs:1126-1160, qsl-replay/src/outcome.rs:1089-1099 |
| FND-007 | low | The protocol counterexample always writes `max_live_instances`, but FR-433 gives it only "for an `on each` subject". | qsl-replay/src/outcome/counterexample.rs:425 |
| FND-008 | low | Rust idiom: `AnalyzeRecord` gains parallel `Option` pairs (`contract_check`/`contract_wire`, `probabilistic_counterexample`/`probabilistic_wire`) whose pairing is not enforced by the type. `take_counterexample` covers a mismatch with made-up `Unspelled` fallbacks. The wire is also stored twice, inside `Counterexample::{Probabilistic,Contract}` and in `ItemCounterexample.wire`. `Counterexample::Protocol.subject` is an `Option` that is always `Some`. One `Option<(Box<T>, Wire)>` per engine would remove all of this. | qsl-analyze/src/analyze.rs:1008-1023, qsl-analyze/src/analyze.rs:1040-1088, qsl-analyze/src/analyze/items.rs:603-633 |
| FND-009 | low | The canonical-bytes to `serde_json` round-trip is written three times (`canonical`, `protocol::wire::json`, `encode::enc_json`). Zero is tested by `elapsed.numerator().to_string() != "0"`. `over_reference` clones the whole simulation and builds a `ModelSystem` for every protocol item before the check runs, even for items that prove. | qsl-replay/src/outcome/counterexample.rs:49-57, qsl-eval/src/simulation/protocol/wire.rs:19-22, qsl-eval/src/simulation/model/encode.rs:1263-1267, qsl-replay/src/outcome/counterexample.rs:525, qsl-eval/src/simulation/protocol/wire.rs:96-103 |
| FND-010 | low | `assignments_wire` zips parameters with values, so a length mismatch is silently truncated instead of being reported as an error. The conformance rows map `OutcomeItemError` to `CaseError::at(CaseStage::Document)` and drop the cause. `assert_same_outcome` compares counterexamples by their `Debug` strings. | qsl-replay/src/outcome/counterexample.rs:454-465, qsl-conformance/src/run.rs:714, qsl-conformance/src/run.rs:795-797, qsl-conformance/tests/it/support.rs:116-124 |
