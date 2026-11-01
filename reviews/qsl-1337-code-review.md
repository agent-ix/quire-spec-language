---
id: SR-1957019
title: QSL-1337 PR 1930 code-review
type: SpecReview
analysis: code-review
review_set: subset
scope: agent-ix/quire-spec-language; origin/main...d43ffa89a; QSL-1337
relationships:
- target: ix://agent-ix/quire-spec-language/FR-396
  type: references
---

## Summary

Ticket: QSL-1337. Frozen candidate d43ffa89a61e1aea151318a7f7518ac9c8302d47, QSpec 4ce0347572fd498b8606211189a08617c1b2b385.

The Rust lane traced structural-id interning, graph-local resolution, forward references, index range/cycle refusal, index emission across graph/body/source-map sites, admission adapters, replay binding and lookup. Conversion duplication is removed in favor of wire_id. No concrete behavior defect, vendoring, unsafe code, new cap, weakened gate, or compatibility layer was found. Reviewed Rust baseline and API, implementation, safety, wire, testing references. No Tokio changes.

The shared structural DAG keeps identity separate from wire position. The emitter collects ids before fixing graph order and rewrites operands afterward. Package admission validates malformed wire before WireGraph rebuilds admitted keys. Panic sites in memoized traversals rely on completed operands, not unchecked input indexes. WireGraph is a resolver, not an independent strict package admission API. FR-396 API changes are intentional prerelease changes and covered by the revised criterion.

One trace defect is recorded solely in the gap-analysis artifact to avoid duplicate findings. No applicable AssuranceProfile found.

## Verdict

PASS. Scoped mergeability requires the trace finding fixed and the lead-owned final gate. This review does not close QSL-1337.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - | correct-requirement-no-evidence |

## Coverage

```yaml
scope:
- id: QSpec-FR-322-AC-278
  path: quire-specification/spec/objects/interfaces/FR-322-checked-package-artifact.md
  role: examined
  excerpt: 'No published V2 package, fixture or vector holds a `node_id` object inside
    a `node_id.path`: every operand is an index segment `{"node": i}`. Over `fixtures/positive-dependency-call.json`,
    the V2 schema refuses as `malformed_wire`, at the patched segment, a path segment
    that is a nested `node_id` object (`NI-01`), `{"node": -1}` (`NI-02`), `{"node":
    "0"}` (`NI-03`), `{"node": 1.5}` (`NI-04`), `{"node": 0, "ordinal": 0}` (`NI-05`)
    and `{}` (`NI-06`), and a `dependency_reference` `node` whose path holds an index
    segment (`NI-07`).'
- id: QSpec-FR-322-AC-279
  path: quire-specification/spec/objects/interfaces/FR-322-checked-package-artifact.md
  role: examined
  excerpt: 'A schema-valid index segment whose `node` is not below the length of `semantic_graph.nodes`
    refuses `malformed_wire`, with no cause, at that segment''s `node` member, before
    every `semantic_graph` check: in a node''s own `node_id` (`NI-08`, index `4` of
    four nodes, at `/semantic_graph/nodes/2/node_id/path/3/node`), in a literal''s
    `type` (`NI-09`, index 9007199254740991) and in a `source_map` entry''s `node_id`
    (`NI-10`, at `/source_map/2/node_id/path/3/node`).'
- id: QSpec-FR-322-AC-280
  path: quire-specification/spec/objects/interfaces/FR-322-checked-package-artifact.md
  role: examined
  excerpt: 'A cycle through own-path index segments refuses `malformed_wire`, with
    no cause, at the first segment by holding node''s graph index and then path position
    whose named node reaches the holding node: a node that names itself (`NI-11`,
    at `/semantic_graph/nodes/2/node_id/path/3/node`) and two nodes that name each
    other (`NI-12`, at node `0`''s segment `/semantic_graph/nodes/0/node_id/path/2/node`).
    An index segment that names a later node without a cycle is admitted (the reversed
    fixtures of FR-322-AC-95).'
- id: QSpec-FR-322-AC-281
  path: quire-specification/spec/objects/interfaces/FR-322-checked-package-artifact.md
  role: examined
  excerpt: A graph holding the Integer node and a left-nested chain of 100,000 derived
    `expression`/`application` nodes, each naming the previous one through one index
    segment (`NI-13`), writes its node ids in at most 200 bytes per node, at the same
    maximum JSON nesting depth as a chain of 100, and its text is parsed by a JSON
    reader with a nesting limit of 128. The index checks admit it, and its rebuilt
    structural keys are 100,001 distinct keys, the key of each chain node holding
    its predecessor's key as its last path element.
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

## New findings (disposition pass 2)

The first review missed this integration defect. Original clean findings and verdict above are historical and remain unchanged. The lead's aggregate gate on 1130dc99 passed the dependency-selection case in 0.76s, then xtask correctly refused its missing required summary and the aggregate exited 2. This was an in-scope reporting contract defect, with no baseline exception.

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-002 | medium | The dependency-selection conformance test emits no required completion summary, so xtask rejects its successful assertions and make ci exits 2. Emit the asserted entry/order vector counts after both loops. | qsl-package/src/checked_v2/tests.rs:2901; xtask/src/conformance.rs:67 | implementation-bug-despite-evidence |

Before (1130dc99), the function ended immediately after its final assertions:

```rust
        assert_eq!(loci, expected_loci, "{id}");
        assert_eq!(Some(&pointer.as_str()), expected_loci.first(), "{id}");
    }
}
```

After (805840689b4d67af33a4733108e281278af2a87c), both original arrays are bound once and both assertion loops are preserved, followed by:

```rust
    println!(
        "conformance: {} dependency-selection entry mutations and {} order vectors",
        entry_mutations.len(),
        order_vectors.len()
    );
```

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-002 | fixed | Pass 2: frozen fix 80584068 prints the actual lengths of the two asserted arrays after all assertions. Exact test reports 3 entry mutations and 3 order vectors; all eight make conformance selectors pass. No assertion, loop, or gate was weakened. |

## Disposition pass 2 evidence

Inspected the entire diff 1130dc99..80584068: one test function, 9 insertions and 2 deletions, with no production changes or additional findings. Before/after excerpts above are verbatim. The original placeholder FND-001 remains only a schema placeholder and needs no disposition.

Read actual logs under /home/peter/dev/.agents/tmp/pr1930-checks: dependency-summary-fix2.log exact test passes (3 entry mutations, 3 order vectors); conformance.log runs all eight selectors successfully, exit 0 (1m49.79s); checked-input.log exit 0 (11.41s). results.log records all 13 phases exit 0. Other observed focused results: wire-id 5 (100k conversion included), checked-v2 53, tc745/checked-input tests 4, admitted wire 9, bounded flat wire 1, provenance 6, package 447 (2 existing ignored, 10 filtered), replay bind 15, replay clause 324, root union 2. These selections overlap and are not summed as unique tests. Full100k package/Debug acceptance is not claimed. Workspace all-targets/all-features Clippy passed on prior production candidate; the subsequent code changes are trace attributes and this reporting fix.

Latest scoped verdict: no open review findings; mergeability is conditional only on the lead's final aggregate gate after artifact custody. Full100k/Debug acceptance and QSL-1337 remain open as recorded in the gap artifact. Reviewer ran no Cargo or aggregate gate and made no branch edits.
