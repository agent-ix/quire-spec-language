---
id: SR-1957020
title: QSL-1337 PR 1930 gap-analysis
type: SpecReview
analysis: gap-analysis
review_set: subset
scope: agent-ix/quire-spec-language; origin/main...d43ffa89a; QSL-1337
relationships:
- target: ix://agent-ix/quire-spec-language/FR-396
  type: references
---

## Summary

Ticket: QSL-1337. Frozen candidate d43ffa89a61e1aea151318a7f7518ac9c8302d47, QSpec 4ce0347572fd498b8606211189a08617c1b2b385.

Plan completion: not assessed

The changed production behavior has an owning external requirement: QSpec FR-322 structural identity and Operand index segments, with QSL FR-396-AC-10 for the admitted facade. Reverse-gap and stub inspection of this diff found no unowned new behavior or placeholders. The reconstruction tests assert exact structural keys, distinct counts, forward-reference order and typed refusals; NI-01..12 invokes the whole I2 reader and checks code/cause/locus. These are behavioral oracles. The bounded emitter test asserts real output byte size and nesting depth.

Full FR-322-AC-281 acceptance remains unmet in this slice: the 100k reconstruction test does not serialize and parse the 100k wire with the 200-byte/depth assertions; emitted byte/depth coverage uses 1k/4k inputs. QSL-1337 stays open for QSL-1378 -> IR-1191 and final 100k acceptance, with QSL Debug in #1947 and upstream Debug #740 not yet consumable. This is retained acceptance work, not a new defect assigned to this scoped PR. QSL-381 no-cap obligation remains intact.

Optional whole-repository semantic fan-out was not run; review is the requested frozen diff. Two computed-matrix invocations produced no report after 3m32s and 4m04s at about 94% CPU and were terminated. The matrix result is unavailable; no whole-repository matrix PASS is claimed. Manual index comparison confirms the trace defect independently. No applicable AssuranceProfile found.

Affected new tests: a_100000_node_chain_round_trips_through_index_segments (AC281), an_index_segment_naming_a_later_node_resolves (AC280), out_of_range_and_cyclic_index_segments_are_refused (AC279), a_declared_id_refuses_an_operand (AC278), conformance_node_index_vectors_refuse_malformed_wire (AC278/279/280), emitted_node_ids_stay_bounded_and_flat_as_a_sum_deepens (AC281). All six also claim bare TC-233. QSpec TC-233 explicitly verifies AC278..281 and owns the published package/index vectors, so QSpec-TC-233 is appropriate. Existing unrelated historical tags are outside this finding.

## Verdict

CONDITIONAL. Scoped mergeability requires the trace finding fixed and the lead-owned final gate. This review does not close QSL-1337.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | medium | New indexed-wire tests bind QSpec FR-322 and TC-233 using unprefixed QSL ids; local FR-322 has only AC-1..4 and local TC-233 tests systems classification, so the new tests are unbound to the intended external criteria. Prefix these new tags with QSpec-. | qsl-semantics/src/check/wire_id.rs:601,624,645,696; qsl-package/src/checked_v2/tests.rs:3277; qsl-package/src/emit/tests/deep_bodies.rs:393 | wrong-requirement |

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

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | Pass 1: inspected 29d8fb4e52da9f9e26e4d9ccb0804e0635369859 against d43ffa89a. Precisely the six new test attributes now use QSpec-TC-233 and QSpec-FR-322-AC-278..281. QSpec TC-233 explicitly owns those criteria; unrelated historical tags are preserved. |

## Disposition pass 1 evidence

Frozen head 29d8fb4e52da9f9e26e4d9ccb0804e0635369859. Compared the complete fix diff: six trace attributes and verbatim review custody only. No introduced behavior or test-oracle changes, and no new findings. Original Findings table remains unchanged. Static finding is fixed; scoped mergeability is conditional on completion of coder-owned focused tests and the lead-owned final aggregate gate. Workspace all-targets/all-features Clippy passed on d43ffa89a (9m27s); only tags and SR artifacts changed after it. Format and diff checks passed according to coder evidence. Focused corrected-source tests remain running at this disposition; lead must reconcile their results before merge. No duplicate Cargo or aggregate gate run by reviewer. Full100k/Debug acceptance and QSL-1337 remain open as recorded above.
