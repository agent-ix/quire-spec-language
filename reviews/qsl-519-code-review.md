---
id: SR-1205
title: "Code review of quire-spec-language PR #593: state-clause separating witness (E19a)"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language PR #593 diff against origin/main: qsl-eval/src/value/expression/{evaluate.rs,mod.rs,s6a/mod.rs,s6a/protocol_clause.rs,s6a/separation.rs}, qsl-eval/src/value/mod.rs, qsl-foundation/src/{lib.rs,witness.rs}, qsl-replay/Cargo.toml, qsl-replay/src/{execute.rs,execute/frame.rs,execute/state_clause.rs,execute/tests.rs,lib.rs,proof_result.rs,result.rs,result/wire.rs,spine.rs,spine/clause.rs,witness.rs,witness/derivation.rs,witness/state_clause.rs}, qsl-replay/src/spine/clause/tests/*, Cargo.lock"
review_set: subset
---
# Code review of quire-spec-language PR #593

## Summary

Ticket: QSL-519. The PR adds a `Trail` to the S6a evaluator. On the claim's
own level, the trail records each connective and `if` decision, each
quantifier stop report and each collection's provenance. FR-265's
derivation (`derive_separating_witness`) walks the decision path over the
trail. `ClauseRunReport` gains `basis` and `witness`. FR-122's replay
re-derives the record, compares it with the payload's record and runs
FR-268's separation check (`CheckedPackageEvaluation::check_separation`).
`DisagreementCause::Witness` and a serde JSON codec are added. The
placeholder record becomes the QSpec FR-351 record, and the FR-207 path
types move to `qsl-foundation::witness`.

Rust lane (rust-review): there are no panics, no `unsafe` code and no
`unwrap` in shipped code. Counters use saturating arithmetic. The
`iterations` counter is decremented on both the early-stop path and the
`finish` path, never on both. The trail keeps its recorded `Arc`s alive, so
`Arc::ptr_eq` lookups cannot alias a freed allocation. `compare_keys` is
reflexive, so the hand-written `Eq` on the record is lawful. The strict
reader is sound: it denies unknown fields, refuses `null`, and checks the
canonical integer and lowercase-hex digests.

Gates run at this head with `<worktree>/target`:
- `cargo test --locked -p qsl-replay -p qsl-eval -p qsl-foundation` passed, including all 16 `tc_74x` tests.
- `cargo clippy --locked` on the same crates with `--all-targets -D warnings` was clean.

These items were judged on the merits and found **not** to be defects:
- **New serde and serde_json dependencies in qsl-replay.** They are the
  repo's norm: qsl-foundation, qsl-eval, qsl-package and the root crate use
  the same versions inline. FR-269 needs a codec. The ADR-013 single-encoder
  lint concerns hashing, and `wire.rs` hashes nothing.
- **"New arch-lint re-export rule."** The diff changes nothing under
  `tools/`. Commit 4fb4e69 moves the FR-207 types to qsl-foundation, so the
  existing FR-100-AC-8 spine-surface check keeps passing. One surface leak
  that check cannot see is FND-005.
- **`FamilyPayload::measured_bytes`.** It is a real feature: the FR-268
  reader bound needs it. The default keeps the old `size_of_val` behaviour,
  and the state-clause override adds the record. The defect is in how the
  record is measured (FND-002), not in the trait method.
- **The `SeparationRefusal` code held as a `String`.** This is forced:
  `CatalogCode` is a `&'static str` pair, and no catalog index exists to
  read one back. The remaining reader-strictness gap is FND-006.

The team-leader rulings are not raised: the population fixture, the
unreachable undefined variants, and the result-level round trip deferred to
E19b.

## Verdict

Not mergeable as is: four medium findings. The decision-path walk, the stop
reporting, the filter/map source binding, basis assignment and the replay
settlement logic are correct and match ADR-031 SW-2/SW-3/SW-7/SW-12/SW-13.
The defects are at the edges:
- the codec cannot encode most value kinds;
- the reader bound under-counts variable-length members;
- the provenance fallback mislabels stored collections as `built`;
- a precondition's built paths name the wrong observation.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The `Witness` cause codec encodes only Boolean, Integer and Reference deciding elements. A record whose element is Text, Enum, Rational, Decimal, Float, Quantity, Option, Composite or Collection makes `DisagreementCause::to_json` fail with `UnsupportedElement`. FR-269 requires the cause to serialize with `given`/`derived` in full, and QSpec FR-351's deciding element is "the exact value". A clause such as `forall(t in names: ...)` over an enum or text collection produces such a record. | qsl-replay/src/result/wire.rs:312-332 |
| FND-002 | medium | `SeparatingWitnessRecord::measured_bytes` gives variable-length members a fixed width. `ValuePathSubject::Object` and `Value::Reference` count 96 bytes, though `ObjectId` is an unbounded string. Text, Decimal, Rational, Quantity, Enum and Population count 32. An oversized record (a long object identity or text element) therefore passes the FR-070-AC-7 / ADR-031 SW-14 reader bound that FR-268 requires it to fail. | qsl-replay/src/result.rs:294-365 |
| FND-003 | medium | When a domain collection has no recorded provenance, `provenance_of` labels it a `built` subject at the domain expression's occurrence. That includes a collection read through `Field` from a record stored on an object, a `ConvertCollection` of a stored collection, and an unwrapped optional stored collection. QSpec FR-207 reserves `built` for "an element that has no stored location", and the computed-collection path must name the stored source. The producer and the separation check share the fallback, so step 3's path comparison agrees with itself and cannot detect the wrong path. | qsl-eval/src/value/expression/evaluate.rs:743-750 |
| FND-004 | medium | A precondition evaluated over an invocation gets `Trail.observation` from `ClauseSetup`, which picks `observations.post.or(pre)`. Built-domain value paths therefore name the post snapshot, which the clause never reads. The field doc says "`pre` for a precondition". The same precondition over the same pre snapshot gives different value paths in a pre-call run and in an invocation run. | qsl-eval/src/value/expression/mod.rs:434-435 |
| FND-005 | low | `SeparatingWitnessRecord::from_stop` is `pub` on a type that qsl_replay re-exports, and its signature names `qsl_eval::value::StopReport`, which qsl_replay does not re-export. This puts qsl-eval on the facade surface that FR-100-AC-8 keeps free of it. The arch-lint check scans re-export items, not inherent methods of re-exported types, so it passes. The only caller is in-crate (`witness/derivation.rs`), so `pub(crate)` suffices. | qsl-replay/src/result.rs:281 |
| FND-006 | low | The strict reader accepts any `code`, `cause` and field-name spelling in a `Refused` reason, including empty strings. The same reader refuses an empty `object_identity`. A document naming a code the catalog does not define reads back as a typed refusal. | qsl-replay/src/result.rs:181 |
