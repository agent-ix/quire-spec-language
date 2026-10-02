---
id: SR-391
title: "Failure-domain review of the reconciled core language and compiler handoff"
type: SpecReview
analysis: failure-domain
scope: "FR-046–050; NFR-009; TC-126–138; IT-009; spec/spec.md; TM-003; docs/compiled-protocol-v2.md; compiler issues #36/#37/#39/#40/#66; immutable native-v1 baseline 782c1ce39a197cd52b8b35b50adf2e5e3ecedd0f; D Producer interface 1.2.0 revision 6259d3a5b99088740df9bcc8e8d60f3720aaa603; L5 revision 72507f856457ba0922719bd5d9f5cadcce4058cd"
review_set: subset
review_date: "2026-09-11"
relationships:
  - { target: ix://agent-ix/quire-spec-language/FR-046, type: reviews }
  - { target: ix://agent-ix/quire-spec-language/FR-047, type: reviews }
  - { target: ix://agent-ix/quire-spec-language/FR-048, type: reviews }
  - { target: ix://agent-ix/quire-spec-language/FR-049, type: reviews }
  - { target: ix://agent-ix/quire-spec-language/FR-050, type: reviews }
  - { target: ix://agent-ix/quire-spec-language/NFR-009, type: reviews }
  - { target: ix://agent-ix/quire-spec-language/IT-009, type: reviews }
---

## Summary

PASS AFTER RECHECK. The original FAIL findings are preserved below as the
review record. FR-046–050, NFR-009 and TC-126–138 now define the previously
missing outcome precedence, complete graph identity and traversal accounting,
trigger idempotence, producer authority and authenticated temporal `/2` seam.
The recheck found four additional cross-document risks; all were resolved in
the reviewed packet before this verdict.

This is a specification verdict, not implementation evidence. In particular,
TC-135 correctly leaves the composed campaign run Planned until D records an
accepted B #6/#11/#12 plus F revision set. No branch snapshot or local A fixture
may substitute for that gate.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | Query execution does not define propagation and precedence when a binder body, equality check, projection or retained-output operation becomes refused, incomplete or exhausted. In particular, the contract must decide whether a decisive short-circuit before unavailable later data completes, whether an unavailable occurrence before the decisive value prevents completion, and whether partially materialized map/filter output is discarded. Add one explicit stage/outcome precedence rule and corresponding before/after-decisive mutations. | FR-046 Outputs and query table; FR-046-AC-4/6/7; TC-127/128; native-v1 state contract, “Ordered queries and exact aggregation” |
| FND-002 | high | Graph evaluation promises deterministic work usage and “actual path support,” while TC-130/131 require independently counted exact and one-short limits, but edge order plus “expand once” does not select depth-first versus breadth-first frontier handling, the definition of depth, target-test/visited-set charge order, or which path is retained. Cyclic diamonds can therefore produce different support and budgets in conforming implementations. Define the traversal/charge algorithm and documented hard-ceiling source, or remove exact support/counter promises that exceed Boolean reachability. | FR-047 Outputs and Behavior paragraphs 9–14; FR-047-AC-5/7; TC-130/131; native-v1 state contract, “Finite graph extension” |
| FND-003 | high | “Duplicate object identities” is not an explicit uniqueness key. FR-047 both rejects duplicates and permits the same model/universe/type/object identity to exist at distinct pre/post observations. Define arena uniqueness over the complete storage key, including snapshot/observation, while separately defining the observation-independent identity projection used for permitted equality. Add same-object/same-snapshot duplicate and same-object/pre-versus-post controls. | FR-047 Inputs; Behavior paragraphs 2, 5 and 6; FR-047-AC-1–3; TC-129; native-v1 state contract, “Operation anchors” and “Finite graph extension” |
| FND-004 | high | The positive choreography and ecosystem paths cannot currently exercise their own failure boundary: issue #40 records that `Correspondence` still yields `Unsupported::ProducerCorrespondence` and that authoritative relationship/component/endpoint exports are absent where the producer contract is unavailable. Before TC-132/134/135 can have a positive oracle, pin the producer selections, admitted correspondence path, and exact unsupported-versus-invalid behavior for each absent authority; do not make a mock or locally fabricated record the success case. | FR-048 Inputs, Behavior paragraphs 14–16, AC-1/2/6/8–10; IT-009 Preconditions and SC-01/02/06; TC-132/134/135; issue #40 comments after merged PR #69 |
| FND-005 | medium | Compensation activation omits two baseline failure cases: concurrent eligible triggers without sufficient ordering must not be selected by ingestion/timestamp order, and repeat delivery of one trigger must not reactivate a registration. State their disposition and preserve the first-trigger identity; add independent concurrent-trigger, duplicate-delivery and later-distinct-trigger mutations. | FR-048 compensation Behavior; FR-048-AC-6–8; TC-134; native-v1 choreography surface, “Deadlines, registration and recovery” steps 1–4 |

## Resolution and recheck

| Finding | Status | Recheck evidence |
| --- | --- | --- |
| FND-001 | resolved | FR-046 requires complete request, artifact and supplied-value validation before evaluation, makes every refusal win regardless of occurrence order, permits short-circuit only over admitted `Unavailable` values or unentered work, and discards partial traversal output on incomplete or exhausted outcomes. FR-049 defines the admitted unavailable representation; TC-127/128/136 exercise the precedence boundary. |
| FND-002 | resolved | FR-047 and NFR-009 select deterministic depth-first traversal in authored edge-occurrence order, define start and recursive graph depth, charge-before-work ordering, duplicate-edge charging, exact/one-short limits and fresh retries. Boolean reachability is the promised result; no path-evidence promise remains. TC-130/131/137 cover these rules. |
| FND-003 | resolved | FR-047 defines the arena storage key as anchor, model, universe, object type and object identifier, while separately defining its anchor-independent logical-object projection. TC-129/136 distinguish a same-key duplicate from the same logical object at distinct pre/post anchors. |
| FND-004 | resolved for specification | FR-048/049 pin D Producer interface 1.2.0 at `6259d3a5b99088740df9bcc8e8d60f3720aaa603`, split D static authority from F concrete assessment facts and require typed unsupported/incomplete/refused outcomes. FR-050/TC-138 own the `/2` producer, strict reader and authenticated L5 entry points. TC-135 names the D-owned external gate and explicitly prevents unavailable B/F revisions from being replaced by local fixtures. The remaining campaign run is delivery work, not an unstated failure contract. |
| FND-005 | resolved | FR-048 preserves the first exact trigger, refuses selection of unordered concurrent eligible triggers, makes repeat delivery and later distinct triggers non-reactivating, and separates provenance from activation identity. TC-134 supplies the independent mutations. |

Additional risks discovered during the complete-packet recheck were also
resolved:

| ID | Severity | Status | Recheck evidence |
| --- | --- | --- | --- |
| RCK-001 | high | resolved | An intermediate FR-046/049 reading allowed an invalid occurrence after a decisive value to be hidden by short-circuiting. FR-046 and TC-127 now require complete validation first and limit short-circuiting to admitted unavailable/unentered work. |
| RCK-002 | high | resolved | The #40 `/2` obligation initially appeared only in choreography prose. FR-050, TC-138 and `docs/compiled-protocol-v2.md` now own the closed wire schema, canonical form, native admission and strict reader. |
| RCK-003 | high | resolved | A `/2` admitted package initially had no type-correct path into L5's strict `/1` APIs. FR-050 now defines `evaluate_v2`, `evaluate_with_progress_v2` and `mapping_support_v2` over `v2::AdmittedPackage`, retains the existing `/1` signatures, and authenticates the exact trace clock parameter map before position evaluation. |
| RCK-004 | medium | resolved | The F observation authority was briefly pinned to an earlier baseline. FR-049 and the packet now select the immutable native-v1 revision `782c1ce39a197cd52b8b35b50adf2e5e3ecedd0f`. |

## Final checklist disposition

- **Extension/trust boundaries:** strict typed failure is stated for compiler,
  producer and consumer admission. `/1` and `/2` are mutually strict, and the
  v2 L5 boundary authenticates the admitted clock selection before evaluation.
- **Entity identity:** predicate and choreography identities are explicit;
  graph storage identity and logical identity are explicitly separated.
- **Evaluation purity:** immutable captures, no ambient receiver/store, no I/O
  and no business-action execution are explicit. No new purity requirement is
  proposed by this lens.
- **Topological robustness:** finite independent depth dimensions, deterministic
  DFS charging, cycle termination and fresh retry are explicit and independently
  testable.
- **External readiness:** the absence of an accepted B/F campaign revision set
  is explicit and correctly prevents a composed-acceptance claim; it does not
  reopen a compiler failure-domain ambiguity.
