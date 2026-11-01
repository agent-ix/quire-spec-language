---
id: SR-1957021
title: QSL-1337 PR 1930 spec-review-base
type: SpecReview
analysis: base
review_set: subset
scope: agent-ix/quire-spec-language; origin/main...d43ffa89a; QSL-1337
relationships:
- target: ix://agent-ix/quire-spec-language/FR-396
  type: references
---

## Summary

Ticket: QSL-1337. Frozen candidate d43ffa89a61e1aea151318a7f7518ac9c8302d47, QSpec 4ce0347572fd498b8606211189a08617c1b2b385.

Reviewed the sole edited criterion FR-396-AC-10 against the API and behavioral tests. It explicitly replaces by-wire helper calls with AdmittedContext::wire_graph().key followed by existing NodeKey lookups. The consumer receives a key through the resolver and need not construct or decode one. Lookup refusals remain distinct from graph conversion WireIdError; the graph API is available from the replay facade. Foreign declared ids contain string paths and resolve without rebasing dependency operands. Malformed-id and out-of-range tests exercise the new refusal boundary.

The edit changes an acceptance criterion only, with no edited shall statement, new entity, relationship, or boundary allocation. Base is the justified spec lens; EARS, integrity, object and dependency sub-analyses were not applicable to the one-row edit. No applicable AssuranceProfile found. Targeted quire validation: one document grammar-clean; zero grammar findings. No enforced review profile applicable.

## Verdict

PASS. Scoped mergeability requires the trace finding fixed and the lead-owned final gate. This review does not close QSL-1337.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - | correct-requirement-no-evidence |

## Coverage

```yaml
scope:
- id: FR-396-AC-10
  path: spec/functional/FR-396-expose-the-admitted-record-and-tuple-keys-through-the-replay-facade.md
  role: examined
  excerpt: 'A consumer holding a structural wire id (package, path, ordinal) resolves
    it through `AdmittedContext::wire_graph` (`WireGraph::key`) to the node''s key,
    so it never constructs or decodes a `NodeKey`, and the NodeKey lookups answer
    for that node with their own refusals: `Foreign` naming the dependency''s `package_id`,
    `Unknown` for an id no package admitted (naming the case for a missing enum case),
    and, for `composite_declaration`, `WrongKind` for another node of this package
    (the enum lookups refuse `Unknown` for it). No `AdmittedContext` lookup takes
    a checked-graph type (FR-270); an id that '
```

## Validation evidence

Frozen-head static review; coder confined focused evidence pending. No reviewer Cargo build, aggregate gate, source edit, push or merge performed.
