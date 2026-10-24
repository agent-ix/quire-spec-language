---
id: SR-16298
title: "Spec review of quire-spec-language PR 1357: STD-396"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@73558f64; spec/functional/FR-056-admit-domain-package-model-declarations.md; spec/functional/FR-091-produce-value-forms-and-assemble-package-declarations.md; spec/functional/FR-098-execute-a-replay-request.md; spec/functional/FR-106-admit-snapshots-and-invocations.md; spec/functional/FR-323-key-union-values-in-collections.md; spec/decisions/ADR-012-semantic-family-extension-contracts.md; spec/test-cases/TC-1171-snapshots-carry-set-bag-and-ordered-set-values.md"
review_set: subset
date: 2026-10-23
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-056
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-091
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-098
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-106
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-323
    type: references
  - target: ix://agent-ix/quire-spec-language/TC-1171
    type: references
---

# SR-16298: Spec review of quire-spec-language PR 1357 (STD-396)

## Summary

Ticket: STD-396. Head 73558f64 (PR 1357, contains 1379 head 6270d77a). Own delta: commits bfd31361 and 73558f64 (FR-056, FR-091, FR-098, FR-106, FR-323, ADR-012, TC-1171, TC-405, TC-406, TC-465, TC-833). The new value forms and their vectors hold by hand: Rational normalization and bound checks on the normalized pair, JCS-byte ordering of Rational, float64 and Quantity elements, bit-identity duplicates for floats (`+0` and `-0` distinct, NaN payloads distinct), Quantity magnitude read to its exact rational whether Rational or Decimal, and String elements keeping composed and decomposed spellings apart. The float-equality design is consistent across PRs: structural `=` uses numericEqual per leaf (PR 1351) while set/bag membership and the frame check use bit identity, and FR-323-AC-3 states both. The retained `unsupported-feature` refusal is for a collection of a population, which is not a value, so it is outside this ticket. Stale text refusing float-bearing element types (FR-098 bullet, ADR-012 cells, FR-323 bullet) is gone at the tip.

Also checked:

- FR-106-AC-55..60 are unique across the other open PR heads. Merge-tree with main clean; conflicts with open PRs 1335, 1333, 1314, 706 (textual).
- Gate: exit 0, 20 warnings against 19 on main; the added warning is FND-001.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | medium | The new bullet holds two `shall` statements (checker admits `=`/`!=`, kernel compares by numericEqual). `make check-spec-validation` reports `ears:non-singular` at this line, the only gate delta against main (19 to 20 warnings). Fix: split into one requirement per `shall`. | spec/functional/FR-323-key-union-values-in-collections.md:71 | wrong-requirement |

## Verdict

**Mergeable after** FND-001 (medium) are fixed or dispositioned by the lead.
