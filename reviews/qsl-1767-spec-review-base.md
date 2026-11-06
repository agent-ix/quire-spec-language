---
id: SR-20261105101075
title: spec-review/base review of QSL-1767 bare union construction
type: SpecReview
analysis: base
scope: agent-ix/quire-spec-language; QSL-1767; spec/functional/FR-517-write-a-checked-package-handoff-directory-from-source-in-process.md;
  spec/functional/FR-531-evaluate-from-a-checked-package-read-from-its-bytes-alone.md;
  spec/functional/FR-533-evaluate-a-protocol-slot-from-the-checked-package-s-own-nodes.md;
  spec/test-cases/TC-1331-evaluate-from-a-checked-package-read-from-its-bytes-alone.md;
  spec/test-cases/TC-1333-evaluate-a-protocol-slot-from-the-checked-package-s-own-nodes.md
review_set: subset
---

## Summary

Reviewed frozen source 0c5755042de18724267213c657c846ed4d16c2c8 against main 695eca34. Ticket: QSL-1767.

## Verdict

**PASS** — No defects found in this method’s changed scope; runtime verification remains pending.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Coverage

Plan completion: not assessed

All five changed spec artifacts and the changed production/test surface were inspected. Checked changed IDs, FR/TC cross-references, five-member identities/3 contract, explicit /2 refusal, native join limits, failure codes, and value/charge oracles. No added hash, digest, source-derived identity or tracking scheme.

No applicable AssuranceProfile found under spec; no enforced review workflow selected. No dependency lens (no relationship changes), object lens (no domain-object changes), or Jev criterion-strength lens selected.

## Examined units

```yaml
scope:
- id: FR-517-AC-23
  path: spec/functional/FR-517-write-a-checked-package-handoff-directory-from-source-in-process.md
  role: examined
  excerpt: 'The writer emits the required `identities/3` source-record and source-union
    lists from the same final emission as the package: original opaque keys, final
    structural/type references, original member identities and order round-trip through
    native joins. The reader requires both lists and refuses the old format or an
    omitted list; no optional compatibility path or source fallback supplies them.'
- id: FR-531-AC-8
  path: spec/functional/FR-531-evaluate-from-a-checked-package-read-from-its-bytes-alone.md
  role: examined
  excerpt: The handoff directory of **R** holds required `identities.json` with `format`
    `quire.checked-handoff.identities/3`; `ReadIdentityRows::read` over it and the
    package gives a slot call that evaluates; the file equals, byte for byte, the
    encoding above of the in-process unit's rows, and two writes of equal requests
    give equal files; omitting the file from a `BareSupply` makes the slot call refuse
    `MissingIdentityWitness` while the temporal call is unaffected.
- id: FR-531-AC-26
  path: spec/functional/FR-531-evaluate-from-a-checked-package-read-from-its-bytes-alone.md
  role: examined
  excerpt: An authentic final emission's source-union rows round-trip original type
    handles, settled keys, sealed member identities, final structural and payload
    node IDs and ordered Exact payload types; the read rows equal the emitter rows,
    native `source_union_rows` retains them and rewriting is byte-identical.
- id: FR-531-AC-27
  path: spec/functional/FR-531-evaluate-from-a-checked-package-read-from-its-bytes-alone.md
  role: examined
  excerpt: '`/2`, absent `source_unions`, malformed member identity, stale member
    name, foreign source owner and conflicting union rows return typed format, malformed
    or native-join refusals without a partial read. An empty union list never fabricates
    a producer witness from the graph.'
- id: FR-533-AC-20
  path: spec/functional/FR-533-evaluate-a-protocol-slot-from-the-checked-package-s-own-nodes.md
  role: examined
  excerpt: 'A strictly admitted variant of **R** constructs `Shape::Circle(sum<M::Total>(receipt
    in recoveredFull.receipts: receipt))` and matches `Circle(r)` in `Full`''s recovery
    predicate. Matching, mismatching, empty and activated supplies produce the source
    route''s Boolean values and ordered charges. The bare environment preserves the
    source union''s original type handle, sealed declaration/member identities, member
    order and payload types; changing the payload changes the predicate, while adding
    an unrelated union leaves it unchanged. Missing required type projections refuse
    `MissingTypeRepresentation'
```

## Validation evidence

`quire validate` accepted this explicit scratch artifact. Targeted specification validation accepted all five changed documents: 5/5 grammar-clean; zero grammar findings. Execution evidence remains pending; no tests or build gates were run by the reviewer.
