---
id: SR-11404
title: "Gap analysis — QSL-760/QSL-788 PR #874 native scalar replay"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@cd94cfe81f8ef3cde74e35141fdb2e4fc6cc40df; QSL-760; QSL-788; FR-056-AC-35; TC-1028 E6-E14; qsl-replay/src/execute/tests/native_scalar.rs; qsl-package/src/emit/tests/admission_corpus.rs"
review_set: subset
relationships:
  - { target: ix://agent-ix/quire-spec-language/FR-056, type: references }
  - { target: ix://agent-ix/quire-spec-language/TC-1028, type: references }
---

## Summary

Ticket: QSL-760, QSL-788. Plan completion: not assessed. Each TC-1028 vector the PR credits was checked against the spec's table, as were the ticket criteria.

- E6 and E7 match the spec exactly: `...eeff` against itself gives true, and against `...eefe` gives false.
- E9 to E13 match: `0`/`0`, `0`/`1`, `-1`/`0`, MIN/MIN and MAX/MIN, with MIN and MAX as the exact signed-128 endpoints.
- E14 is asserted for every UUID and Timestamp row through the `!=` functions.
- E8 asserts the uppercase second operand is refused `invalid_runtime_input`/`invalid-value`. The refusal is at parameter 1 with kernel class `UuidNoncanonical`, and no comparison runs because `replay` returns `Err`.

QSL-788's remaining criterion was the gap SR-11258 FND-001 recorded: canonical UUID and Timestamp inputs reachable by the QSL-760 equality vectors through replay. This PR closes it. The PR body names E1 to E5 (String through `replay()`, QSL-798) and native literals (QSL-786) as not credited, and no test claims them. The F1 to F7 field vectors are not claimed either.

## Verdict

PASS with one low trace finding, recorded.

## Examined Scope

```yaml
scope:
  - id: FR-056-AC-35
    path: spec/functional/FR-056-admit-domain-package-model-declarations.md
    role: examined
    excerpt: "UUID: 00112233-4455-6677-8899-aabbccddeeff equals itself and differs from 00112233-4455-6677-8899-aabbccddeefe; the uppercase spelling of the first is refused at a unit input by FR-056's runtime-input adapter as invalid_runtime_input/invalid-value, over the kernel's noncanonical-payload class, so no value of it reaches =. Timestamp: 0 equals 0, differs from 1 and from -1, and both signed-128 endpoints equal themselves and differ from each other. != gives the negation of each result."
  - id: TC-1028
    path: spec/test-cases/TC-1028-unit-equality-on-native-string-uuid-and-timestamp.md
    role: examined
    excerpt: "E6-E14: equ/eqt vectors over UUID ...eeff/...eefe/uppercase and Timestamp 0, 1, -1, i128 MIN and MAX; E14 the negation for each != counterpart."
  - id: FR-056-AC-17
    path: spec/functional/FR-056-admit-domain-package-model-declarations.md
    role: context_only
    excerpt: "Admit Timestamp 0, 1, -1 and both signed-128 endpoints. Refuse -0, +1, 01, -01 ... as noncanonical-payload. Refuse each endpoint's adjacent exterior integer as payload-out-of-domain."
```

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | `tc_1028_noncanonical_and_out_of_domain_timestamps_are_refused_through_replay` is tagged `FR-056-AC-35`, but AC-35 states no Timestamp refusal; only the uppercase UUID one. The sibling `composite.rs` Timestamp refusal test tags `TC-1028` only. | qsl-replay/src/execute/tests/native_scalar.rs:139 |
| FND-002 | low | The module doc and the PR body say the spine includes "IR's checked-package/v2 read". Neither `compose` nor `replay()` reads the unit's own package through IR. IR admission of these forms is asserted only by the admission-corpus rows. | qsl-replay/src/execute/tests/native_scalar.rs:2-4 |

## Bindings

```yaml
bindings:
  - {test_id: tc_1028_uuid_equality_runs_through_replay, ac_id: TC-1028, trace: correct}
  - {test_id: tc_1028_timestamp_equality_runs_through_replay, ac_id: TC-1028, trace: correct}
  - {test_id: tc_1028_noncanonical_uuid_is_refused_through_replay, ac_id: FR-056-AC-35, trace: correct}
  - {test_id: tc_1028_noncanonical_uuid_is_refused_through_replay, ac_id: TC-1028, trace: correct}
  - {test_id: tc_1028_noncanonical_and_out_of_domain_timestamps_are_refused_through_replay, ac_id: FR-056-AC-35, trace: wrong}
  - {test_id: tc_1028_noncanonical_and_out_of_domain_timestamps_are_refused_through_replay, ac_id: TC-1028, trace: correct}
```
