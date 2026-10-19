---
id: SR-12147
title: "Code review of quire-spec-language PR #983: symmetric population annotation at S3"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@5e191733f2bd0688a14d30214131c4345c07f8b4; PR #983: qsl-semantics/src/check/check.rs, qsl-semantics/src/check/check/typing.rs, qsl-semantics/src/check/lowering/model.rs, qsl-semantics/src/check/mod.rs, qsl-semantics/src/check/refusal.rs, qsl-semantics/src/check/protocol_obligation.rs, qsl-semantics/src/model/intake.rs, qsl-semantics/src/model/domain_package.rs, qsl-replay/src/spine/refinement.rs, qsl-semantics/tests/it/symmetric_population.rs, fixture updates"
review_set: subset
---
# Code review of quire-spec-language PR #983

## Summary

Ticket: QSL-539. Includes the rust-review lane. QSL FR-150 AC-1..4 (TC-565, TC-566); QSpec FR-381 / TC-320 by id.

Reviewed the diff `origin/main...5e191733f` read-only. No cargo build or `make ci` was run by this review; claims below are traced through the source.

The three areas the brief named:

1. **Intake schema filter** (`population_symmetric_schema_diagnostic`, intake.rs:1436-1504). It is narrow: it drops a diagnostic only when the code is `SCHEMA_VIOLATION`, the message is exactly the semantic-IR `forbid_extra` text for `symmetric`, the pointer is `/ir/populations/<n>` (the object pointer `forbid_extra` emits), and that document population really carries `symmetric`. It cannot hide a different violation: other members and other pointers are untouched, and a non-Boolean `symmetric` is still refused by `read_population`. If FCD rewords the message the filter stops matching and intake refuses, so it fails closed. It is still a shim around the pinned FCD schema (rev `fd261ca`); FCD main's `POPULATION_MEMBERS` still lacks `symmetric`, while QSpec FR-381 says the semantic-IR population record carries it. See FND-002.
2. **`CheckCause::IdentityObservingForm` soundness.** For `fold`/`reduce` and the standalone `convert`, the new cause only relabels a refusal the checker already made (`operator-ineligible` for an uncatalogued unordered fold, `type-mismatch` for an inadmissible conversion), so neither can newly pass. Ordering over references stays ill-typed. Equality, `deref`, `reaches`, `contains`, `size`, `count` are untouched. One identity-observing form still checks silently: a collection-kind conversion from `Set`/`Bag` to `Sequence` (FND-001). The equality-operand path over-refuses an identity conversion (FND-003).
3. **Package identity.** The marker is in the domain package bytes, so the `DomainPackageRef` selection over those bytes changes with it; TC-565's test asserts that against bytes that differ only by the member (serde_json re-serialisation is byte-stable here: no `preserve_order`). Clean.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | A collection-kind conversion from an unordered `Set`/`Bag` to a `Sequence` of references into a `symmetric` population checks with no refusal. `convert`'s `(Collection, Collection)` arm checks only element equality, and the evaluator builds the `Sequence` in the identity-sorted element order of the set (FR-181). So `fold<Flag>(acc, p in convert<Sequence<Peer>[0, 5]>(self.peers): p = self, identity: false)` checks: the fold source is now ordered, so the catalogue rule never runs, and the result is decided by object identity. The same holds for comparing the converted sequence by equality. This is FR-150 Behavior item 1 ("a fold, reduce or other traversal whose result depends on element order, over a Set or Bag") silently passing. Refuse `identity-observing-form` (node `convert`, `OrderSensitiveTraversal`) in that arm when the source kind is unordered, the target kind is ordered, and `symmetric_population_in(element)` is `Some`; add a TC-566 test. | qsl-semantics/src/check/check.rs:2976-2990; qsl-eval/src/value/expression/evaluate.rs:1887-1915; qsl-semantics/src/check/check/typing.rs:2299 |
| FND-002 | medium | The intake filter that drops the semantic-IR "the member symmetric is not one this schema admits" diagnostic is a shim around the pinned FCD schema (`agent-ix-semantic-ir` rev `fd261ca`; FCD main's `POPULATION_MEMBERS` also lacks `symmetric`). It is narrow and fails closed (see Summary), so it hides no other violation, but it has no ticket or expiry and stays as dead code once FCD admits the member. QSpec FR-381 puts `symmetric` on the semantic-IR population record, so the owner of the fix is FCD's population schema. Add `symmetric` (optional Boolean, `expect_bool`) to FCD's population schema, bump the rev, and delete `population_symmetric_schema_diagnostic`. | qsl-semantics/src/model/intake.rs:1436-1439; qsl-semantics/src/model/intake.rs:1486-1504; qsl-semantics/Cargo.toml:44 |
| FND-003 | medium | The equality-operand `convert` path refuses `identity-observing-form` before any admissibility check, for any operand whose type contains a symmetric reference. An identity conversion (`convert<Peer>(self) = p`, with `Peer` an alias of `ConfigVersion`, or `convert<R>(r)` for a record `R` holding a reference) is admitted without the annotation (`admits_equality_conversion` returns true for `source == target`) and by the standalone path, but refuses here under the annotation. That breaks "passing as a value SHALL check as it does without the annotation" and makes the two `convert` paths disagree. Refuse only when the conversion changes the type (`source != target`), as `check.rs:3013` does. | qsl-semantics/src/check/check/typing.rs:1363-1372; qsl-semantics/src/check/check.rs:3011-3024 |
| FND-004 | medium | `symmetric_object_types` maps only object types that conform to a population member type. A `Reference<Super>` whose values can be objects of a symmetric subtype population is not flagged. Today that only mislabels the refusal (`operator-ineligible` / `type-mismatch` instead of `identity-observing-form`), but the FND-001 fix keyed on `symmetric_population_in` would silently pass `convert<Sequence<Super>>` over such a set. Also map a supertype `S` to the population when some member type of a symmetric population conforms to `S`, or state in FR-150 that a supertype reference does not read the population. | qsl-semantics/src/check/lowering/model.rs:573-599 |
| FND-005 | low | `IdentityObserving.node` is a public `&'static str` holding one of three fixed kinds (`fold`, `reduce`, `convert`). A small enum (or reusing `Accumulation` plus a convert variant) keeps the payload typed, as `IdentityForm` already is. | qsl-semantics/src/check/refusal.rs:1477-1485 |

## Verdict

Changes requested. One high finding blocks: FND-001 (a Set/Bag to Sequence conversion of symmetric references passes S3 silently). FND-002 to FND-004 are medium, FND-005 is low; none of them blocks on its own.

What is correct: the marker is read as an optional Boolean (absent is `false`, non-Boolean refuses `malformed-declaration`); the new cause has code `ill_typed` and cause `identity-observing-form`; protocol-obligation wrapping and the refinement limit match list the new variant; refusals are located at the refused node; all fixture `PopulationRecord` literals carry `symmetric: false`; unannotated units are unchanged on every touched path.
