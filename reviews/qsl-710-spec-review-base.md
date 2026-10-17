---
id: SR-7812
title: QSL-710 occurrence conversion context spec-review/base
type: SpecReview
analysis: base
scope: agent-ix/quire-spec-language@58344c4101f8053830abbd8a989a1539611c0cb3; spec/functional/FR-062-implement-checked-family-contract.md,
  spec/functional/FR-075-compute-candidates-from-registered-backends.md, spec/test-cases/TC-449-request-builder-writes-one-item-per-requirement-record.md,
  spec/tests.md
review_set: base
---

## Summary

Ticket: QSL-710. Frozen spec-only diff reviewed against RR-16 and current claims/request source. Mandatory base and language-independent code review only; optional selection pending. No implementation or runtime qualification is claimed.

## Verdict

CONDITIONAL — Two acceptance-vector gaps remain.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The driver-to-IR mapping has no discriminating acceptance vector. AC-15 and TC-449 stop at builder outputs, so a driver that drops context or swaps RR-16 contexts still satisfies every added expected result. Add the single-function example and RR-16 driver-call oracle, matching each occurrence/request index to its own root and context, with no extra routed claim. | spec/functional/FR-075-compute-candidates-from-registered-backends.md:162-174 |
| FND-002 | medium | The newly required missing/ambiguous direct-narrow refusal and direct-versus-outer selection lack concrete test vectors. TC-449 covers normal RR-16 and None; TC-160 step 11 exercises missing application occurrences, not missing or multiple narrow occurrences. Add zero/one/multiple narrow-candidate cases plus nested direct-versus-outer selection with independently stated expected identity and typed refusal. | spec/functional/FR-062-implement-checked-family-contract.md:285-295 |

## Examined scope

```yaml
scope:
- id: FR-062
  path: spec/functional/FR-062-implement-checked-family-contract.md
  role: examined
  excerpt: "- the application's **result bound**: the target range of the\n  `quire.op.numeric.narrow`\
    \ that wraps it, when a narrow's operand is this\n  application, and otherwise\
    \ the application's own result type;\n- its **direct conversion context**: the\
    \ canonical `OccurrenceKey` of the\n  `expression` occurrence for the narrow that\
    \ directly wraps it (the\n  innermost narrow), when the result bound comes from\
    \ that narrow; otherwise\n  no conversion context. The identity is retained separately\
    \ from the\n  result-bound type node. A narrow\n  is never a claim of its own.\n\
    - the application's **path condition**: the"
- id: FR-062
  path: spec/functional/FR-062-implement-checked-family-contract.md
  role: examined
  excerpt: "- **Direct conversion context.** When a narrow directly wraps the scalar\n\
    \  application at this site, the record SHALL carry that direct (innermost)\n\
    \  narrow's canonical `OccurrenceKey` (its `expression` occurrence) as its\n \
    \ conversion context.\n  Otherwise it SHALL carry no conversion context. Only\
    \ the narrow directly\n  wrapping the application supplies this context; an outer\
    \ conversion is\n  not substituted. The context is a member of this occurrence's\
    \ record, not\n  a value indexed only by the scalar application's `WireNodeId`.\
    \ If keying\n  finds zero or multiple matching direct narrow occurrenc"
- id: FR-062-AC-13
  path: spec/functional/FR-062-implement-checked-family-contract.md
  role: examined
  excerpt: '| FR-062-AC-12 | A family `check` that reaches one of its two stage-entry
    limits returns `StageFailure::Limit` naming the limit kind, the configured bound
    and the actual counter: the measured preimage byte length for input bytes and
    the cumulative spend the denied charge would reach for work budget. Configured
    one below that counter, or at 0 for a declaration whose counter exceeds 1, `check`
    returns that same counter; configured at it, that limit does not stop `check`.
    A family `check` compares no measured node count before typing: its node limit
    is `CheckingLimits::nodes`, charged at the node'
- id: FR-075
  path: spec/functional/FR-075-compute-candidates-from-registered-backends.md
  role: examined
  excerpt: "- the record's occurrence key, and that key's node (`WireNodeId`);\n-\
    \ the record's capability kind;\n- its extent classification, as `RequestWriter::item`\
    \ computes it from the\n  record's `ClaimExtent` (FR-097);\n- its unbounded domains:\
    \ the `DomainKey` and kind of each domain of an\n  `Unbounded` extent, in key\
    \ order, and none for a `Bounded` one;\n- the record's result bound, as the `WireNodeId`\
    \ of its FR-092 type node;\n- its direct conversion context, as `Option<OccurrenceKey>`:\
    \ the canonical\n  `OccurrenceKey` of the `expression` occurrence for the narrow\
    \ that\n  directly wraps this record's scala"
- id: FR-075
  path: spec/functional/FR-075-compute-candidates-from-registered-backends.md
  role: examined
  excerpt: '`RequirementItem::conversion_context()` SHALL expose the record''s direct

    conversion context as `Option<&OccurrenceKey>`. This value is carried once

    per requirement occurrence, independently of the `WireNodeId` shared by the

    scalar application. For each item, the driver SHALL associate the context

    with the item by its occurrence key/request index; it SHALL NOT select a

    conversion by expression node alone. A record whose result bound comes from

    the application''s own result type yields `None`. A record whose result bound

    comes from a narrow carries exactly the direct (innermost) narrow''s

    occurren'
- id: FR-075-AC-15
  path: spec/functional/FR-075-compute-candidates-from-registered-backends.md
  role: examined
  excerpt: '| FR-075-AC-15 | For every request item, `conversion_context()` returns
    the record''s direct conversion context as `Option<&OccurrenceKey>`, separate
    from `result_bound()`. In FR-062 RR-16, the two requirement records share the
    `+` node but have distinct occurrence keys and directly wrapping narrow occurrence
    keys; the builder returns two items in occurrence-key order, and each item''s
    `conversion_context()` equals its own record''s narrow occurrence key while its
    `result_bound()` is respectively `Int[0, 10]` or `Int[0, 20]`. A record with no
    directly wrapping narrow returns `None`; when the same'
- id: TC-449
  path: spec/test-cases/TC-449-request-builder-writes-one-item-per-requirement-record.md
  role: examined
  excerpt: "   named backend `missing` that the registry does not hold.\n5. Check\
    \ a unit whose only function body is `b and c` over Boolean\n   parameters and\
    \ run the builder over its records.\n6. Check FR-062's RR-16 unit and run the\
    \ builder over its records with the\n   step 1 registry. Inspect each source record\
    \ and its corresponding item,\n   including `conversion_context()` and `result_bound()`.\n"
- id: TC-449
  path: spec/test-cases/TC-449-request-builder-writes-one-item-per-requirement-record.md
  role: examined
  excerpt: "- Step 6: two items at the one `+` node, with distinct occurrence keys,\n\
    \  carrying result bounds `Int[0, 10]` and `Int[0, 20]` in key order. The\n  records\
    \ and items each carry the narrow occurrence key directly wrapping\n  that site's\
    \ `+` application; the two conversion contexts remain distinct\n  even though\
    \ the application node key is shared. Neither narrow is a second\n  item. For\
    \ the unbounded `n + 1` record from step 2, the builder exposes no\n  conversion\
    \ context because its result bound is its own result type."
- id: spec/tests.md
  path: spec/tests.md
  role: examined
  excerpt: "| TC-449 | The request builder writes one item per requirement record\
    \ | Unit | P1 | FR-075-AC-8, FR-075-AC-15 | \u2705 |"
- id: claims.rs
  path: qsl-semantics/src/check/claims.rs
  role: context_only
  excerpt: "            let narrow = self\n                .frames\n             \
    \   .last()\n                .and_then(|parent| match parent.node.kind() {\n \
    \                   NodeKind::Coerce(operand, interval)\n                    \
    \    if coerce_builds_narrow(\n                            parent.node.at(*operand).value_type(),\n\
    \                            interval,\n                        ) =>\n       \
    \             {\n                        Some(interval)\n                    }\n\
    \                    _ => None,\n                });\n            let walk = Classify\
    \ {\n                guards: &self.guards,\n               "
- id: request.rs
  path: qsl-route/src/request.rs
  role: context_only
  excerpt: "/// record's path condition.\n#[derive(Clone, Debug, Eq, PartialEq)]\n\
    pub struct RequirementItem {\n    index: RequestIndex,\n    occurrence: OccurrenceKey,\n\
    \    kind: Capability,\n    extent: ExtentClassification,\n    unbounded: Vec<(DomainKey,\
    \ DomainKind)>,\n    result_bound: WireNodeId,\n    path_condition: Vec<PathGuard>,\n\
    \    candidates: CandidateOutcome,\n}\n\nimpl RequirementItem {\n    /// The item's\
    \ request index: its position in record key order.\n    pub fn index(&self) ->\
    \ RequestIndex {\n        self.index\n    }"
```

## Evidence and limits

The four changed documents validate with no document errors; installed-module discovery emits duplicate archetype/inverse-edge and inline-schema advisories. Quire 0.36.2 / engine 0.50.2; Quoin 0.28.3. No applicable AssuranceProfile was found. Identity is the existing semantic occurrence tuple (node identity, role, ordinal); this change adds no file-tracking hash or duplicate representation. Optional analyses are pending selection. No Rust production or test files changed; no builds were run. Existing code lacks the new field by design in this spec-first slice and is not being claimed implemented. The direct-parent Coerce rule at claims.rs:597-615 supports innermost selection; current keying finds a narrow among same-site occurrences and discards its key. RequirementItem and its builder currently preserve path guards but not conversion identity.

## Tool completion

The computed matrix completed with exit 0. FR-075-AC-15 is untagged, consistent with this spec-first slice; FR-062-AC-13 has existing bindings, which do not prove the newly added obligations. Selected optional lenses subsequently ran as SR-7814 (EARS) and SR-7815 (integrity), both PASS with no additional findings.

## Disposition pass 1

Reviewed 88c8497ab9265df470afc53054082366c65dc29e against the original frozen head. **PASS** for this spec-only slice. Original findings remain unchanged above. No new findings. All five affected documents validate with 5/5 grammar-clean and zero grammar findings. No Rust, build or runtime claims.

## Dispositions

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | 88c8497ab9265df470afc53054082366c65dc29e — FR-075-AC-16 and TC-449 step 7 require matched per-occurrence driver calls for the single-function example and RR-16. |
| FND-002 | fixed | 88c8497ab9265df470afc53054082366c65dc29e — TC-160 step 12 fixes expected identities independently and covers zero/one/two matches and direct-versus-outer selection. |

```yaml
dispositions:
- fnd: FND-001
  outcome: fixed
  fix_sha: 88c8497ab9265df470afc53054082366c65dc29e
  after_excerpt: 'For `function f using v(a: Int[0, 9]): Int[0, 10] pure { a + 1 }`,
    the driver''s IR call includes roots `[item.node(), conversion_context.node()]`
    (or the equivalent contextual entry point), with context paired to the item by
    request index and occurrence key. For RR-16, the driver makes this mapping independently
    for both items, which share the `+` node but have different request indices and
    conversion contexts: each item passes its own narrow context root, and neither
    context is swapped or selected by node key. In both vectors, the narrow is context
    only; it creates no additional req'
- fnd: FND-002
  outcome: fixed
  fix_sha: 88c8497ab9265df470afc53054082366c65dc29e
  after_excerpt: "- Step 12: (a) the `SiteBound::Own` record carries no conversion\
    \ context;\n  (b) the narrowed record carries exactly the independently specified\
    \ key of\n  the sole narrow that directly references its scalar application; (c)\
    \ zero\n  direct matches and (d) two direct matches each return the typed\n  `KeyFault::UnkeyableRequirements`\
    \ and produce no checked package; (e) the\n  context is the independently specified\
    \ inner-narrow key, not the outer\n  narrow key. The outer narrow is excluded\
    \ because it references the inner\n  node rather than the scalar application.\
    \ The expected key is fixed befo"
```
