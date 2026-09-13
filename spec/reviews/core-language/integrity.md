---
id: SR-392
title: "Integrity review of the reconciled core language model"
type: SpecReview
analysis: integrity
scope: "Issues #36/#37/#39/#40/#66: FR-046–050, NFR-009, TC-126–138, IT-009, docs/compiled-protocol-v2.md, spec/spec.md and TM-003; D Producer interface 1.2.0 at 6259d3a5b99088740df9bcc8e8d60f3720aaa603; merged L5 baseline 72507f856457ba0922719bd5d9f5cadcce4058cd; immutable native-v1 baseline 4d6230eb8aa9766ff3017360962f2d6368d74cb3"
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

**PASS AFTER RECHECK.** The expanded packet is complete, consistent, atomic at
its observable interfaces and testable as a specification. FR-049/NFR-009 close
the runtime-input, outcome and accounting model; FR-046/047 now select exact
query precedence and deterministic finite-graph evaluation; FR-048 separates a
static compiled template from downstream runtime occurrences; and FR-050 gives
the latest #40 temporal request its own strict `/2` wire, producer, reader and
v2-specific L5 interface while preserving `/1` compatibility.

All four original findings are preserved below and resolved with exact evidence.
A post-tasking recheck found seven additional ambiguities in the evaluator and
temporal interfaces; commit `72eb30f` resolves each and the dispositions are
recorded below. No integrity ambiguity remains in the reviewed requirement
content. This is a specification-integrity verdict only: Planned matrix rows,
external producer/campaign gates and unimplemented interfaces remain delivery
work.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | high | The new query and graph runtime promise zero/exact/one-short results but do not select an accounting version, define each charged operation and hard/effective limit, or link/extend the existing runtime NFR. The published NFR-006 counters do not name query materialization and FR-046 explicitly requires retained-output work, so TC-128/131 cannot independently compute their exact oracle. | FR-046 inputs and behavior lines 92–100; FR-046-AC-7; FR-047 behavior lines 86–91; FR-047-AC-7; TC-128; TC-131; NFR-006; baseline NFR-010 | missing-requirement |
| FND-002 | high | FR-048 alternates between a static protocol template and concrete O1/O2 workflow instances: future instances are excluded from compiler input, yet AC-1 requires those instances to survive emission/reading and TC-132 says to compile them. Its static-subject invariance also says “resource limits” without excluding compiler admission/emission limits, whose exhaustion necessarily changes artifact availability. Define static declaration/role requirements separately from downstream runtime occurrence identities and restrict the invariance to assessment-time bindings/limits. | FR-048 inputs lines 39–47; behavior lines 60–62 and 122–124; FR-048-AC-1; TC-132 | wrong-requirement |
| FND-003 | medium | FR-047 requires retry to return the same “path evidence” while declaring shortest-path and canonical-witness guarantees out of scope. Ordered expansion makes Boolean truth deterministic but does not state whether support is absent, all support, or the first discovered path, so a diamond graph has multiple valid reports. | FR-047 outputs lines 42–47; behavior lines 77–81 and 97–100; FR-047-AC-7; TC-130 diamond case; TC-131 expected results; baseline FR-043 | missing-requirement |
| FND-004 | high | FR-046 requires compiler, reference-evaluation and “lowered-execution” evidence to agree, but neither the requirement nor TC-127 selects the lowering target, executable representation, entry point, result envelope, supported fragment or failure boundary. This is not testable as one oracle and lets an implementer invent a backend contract outside #36. | FR-046-AC-3/8 and dependencies; TC-127 description/procedure; issue #36 delivery boundary | missing-requirement |
| FND-005 | high | FR-049 made `Available` recursively exclude every unavailable child, contradicting FR-046's required propagation and discard behavior for a reached unavailable aggregate slot. Record, option and sequence inputs therefore lacked a legal state for one required runtime outcome. | FR-049 Behavior and AC-3/4 before `72eb30f`; FR-046 Behavior and AC-4; TC-136 | wrong-requirement |
| FND-006 | high | NFR-009 specified recursive record/option/sequence comparison and comparison depth outside FR-040's admitted equality domain. Besides widening semantics in an NFR, that made TC-137's fourteenth counter unimplementable against the selected profile. | NFR-009 metrics/counter definitions/verification before `72eb30f`; TC-137 procedure; FR-040 | wrong-requirement |
| FND-007 | medium | Allocation refusal was neither a member of a closed exhaustion-cause vocabulary nor assigned to an affected charged dimension, so two implementations could report it incompatibly while satisfying the prose. | NFR-009 counter definitions before `72eb30f`; FR-049 output envelope; TC-137 | missing-requirement |
| FND-008 | medium | `AdmissionV2` and its enclosing `Report` both owned limits and usage, leaving two valid placements and no consistency rule. | FR-050 Interface and wire model before `72eb30f`; docs/compiled-protocol-v2.md Rust boundary; FR-042 report pattern | wrong-requirement |
| FND-009 | high | Returning an external `ArtifactRef` from compiler admission implicitly authorized its authority, identity and revision even though those selectors belong to an independent caller. The output contract therefore crossed its stated trust boundary. | FR-050 Outputs before `72eb30f`; docs/compiled-protocol-v2.md Version selection; FR-042 | wrong-requirement |
| FND-010 | high | `mapping_support_v2` was specified only with ellipses, then included in a rule requiring trace profile/revision/parameter comparison despite taking no defined trace input. Its behavior admitted no single testable interpretation. | FR-050 Interface and Behavior before `72eb30f`; docs/compiled-protocol-v2.md Rust boundary; TC-138 | missing-requirement |
| FND-011 | high | The v2 progress ledger lacked an identity tying retained progress to the admitted package digest and exact clock configuration, permitting unauthenticated progress lending across versions or artifacts. | FR-050 Behavior before `72eb30f`; docs/compiled-protocol-v2.md Rust boundary; TC-138 | missing-requirement |

## Resolution recheck

- **FND-001 — resolved.** FR-049 defines the public `AdmittedPackage` +
  declaration-local `EvaluationRequest` + borrowed `StateView` boundary and the
  closed completed/incomplete/refused/exhausted report. NFR-009 selects
  `quire.state.evaluation-work/1`, hard/effective ceilings, charge-before-work,
  fresh retries and thirteen independent counters. Input, expression, predicate
  and graph depth each define their root, increment, decrement and
  maximum-usage rules. TC-137 gives each dimension an independent exact,
  one-short, zero/no-work, above-hard and missing-charge oracle.
- **FND-002 — resolved.** FR-048 emits authored static role/relationship records
  and a runtime occurrence-key schema, never O1/O2 instances. TC-132 compiles one
  reusable template and binds O1/O2 only at the downstream fixture boundary.
  Invariance is limited to valid assessment-time values, relationship instances,
  observations and limits; insufficient compiler limits explicitly emit no
  subject.
- **FND-003 — resolved.** FR-047 returns only identity or Boolean results and
  expressly returns no path witness. It selects depth-first traversal in authored
  edge-occurrence order, endpoint testing before repeat-expansion suppression,
  complete-storage-key expansion at most once, and duplicate-edge charging.
  NFR-009 fixes the corresponding charge order; TC-130/131 independently count
  diamond, cycle, sibling-order and exact-limit cases.
- **FND-004 — resolved.** FR-046 no longer claims an unspecified lowered backend.
  AC-3 compares compilation with actual evaluation through FR-049's public
  admitted-artifact state evaluator; AC-8 distinguishes static compiler and
  evaluator evidence. TC-127 names the independent expected-value oracle and
  public evaluator rather than a fabricated execution target.
- **FND-005 — resolved in `72eb30f`.** FR-049 models every recursively typed
  value position as an explicit `Available` payload or typed `Unavailable`
  cause. Aggregate slots remain in authored order, enabling FR-046 to propagate
  only a reached unavailable child and discard partial materialization.
- **FND-006 — resolved in `72eb30f`.** NFR-009 confines equality to FR-040's
  supported scalar, enum, reference and object-identity types. Recursive
  container comparison and comparison depth are removed; TC-137 now verifies
  thirteen independent dimensions with supported comparison fixtures.
- **FND-007 — resolved in `72eb30f`.** NFR-009 closes exhaustion over `Limit`,
  `CounterOverflow` and `Allocation`, always naming the affected dimension. A
  bounded allocation follows its retained-entry charge, while allocation
  refusal remains distinct from numeric-limit exhaustion.
- **FND-008 — resolved in `72eb30f`.** `AdmissionV2` owns only canonical bytes,
  their digest and the admitted view; `Report<AdmissionV2>` exclusively exposes
  effective limits and usage in the existing FR-042 pattern.
- **FND-009 — resolved in `72eb30f`.** FR-050 returns raw canonical bytes and
  their digest, not a self-authorizing reference. Only the independent caller
  supplies external artifact authority, identity and revision.
- **FND-010 — resolved in `72eb30f`.** `mapping_support_v2` now has the exact
  `(package, declaration, closure)` signature. It authenticates the admitted
  definition/profile before existing formula-closure classification and makes
  no trace parameter-map claim; TC-138 checks that distinction.
- **FND-011 — resolved in `72eb30f`.** `evaluate_with_progress_v2` keys progress
  by package digest, declaration, definition identity/revision and exact clock
  configuration. TC-138 explicitly rejects reuse from `/1`, another artifact or
  a changed clock selection.

## Expanded-packet integrity recheck

- FR-046 validates the complete request, artifact and supplied value structure
  before runtime evaluation. It then preserves first-encountered runtime outcomes
  in source order, permits Boolean short-circuit only over admitted unavailable or
  unentered work, and discards partial map/filter/aggregate results.
- FR-049 pins D's Producer interface 1.2.0 revision and the immutable F-owned
  observation baseline, while keeping model/configuration authority, concrete
  observations and compiler evaluation distinct. Its typed unavailable value is
  not a semantic null, false, zero or empty collection.
- FR-048 pins D at `6259d3a5b99088740df9bcc8e8d60f3720aaa603`
  and merged L5 at `72507f856457ba0922719bd5d9f5cadcce4058cd`.
  It assigns static D exports, F runtime facts, E settlement and B conformance to
  their owners. Concurrent trigger ordering, redelivery provenance and
  non-reactivation are specified independently.
- FR-050 and `docs/compiled-protocol-v2.md` define a closed delta over `/1`: one
  declaration-indexed binding selects the inherited exact definition identity,
  revision and dependency artifact/digest plus exactly one profile-selected
  clock alternative. Missing/surplus/duplicate/order/profile/numeric/name/byte
  failures, strict cross-version refusal, original-byte digest recomputation and
  inherited accounting are externally testable in TC-138.
- The v2-specific L5 APIs accept only the v2 admitted type and compare the exact
  profile, revision, binding name and closed parameter map before positions. The
  existing `/1` signatures and unauthenticated-premise behavior remain explicit;
  clock progress and settlement meaning do not move into compiler admission.
- IT-009 is a narrow real D producer/linking enablement case. TC-135 terminates
  A's acceptance claim at byte-identical B intake and declares the D-owned,
  B/F-backed composed campaign a separate Planned gate. The master Purpose,
  Scope, System, Execution and Verification sections reflect this L2–L6 model.

## Completeness and traceability

| User need | Compiler requirement | Stakeholder path | Verification |
| --- | --- | --- | --- |
| Exact model, binding and artifact identity | FR-046, FR-047, FR-048, FR-049, FR-050 | US-002/US-004 → StR-001 | TC-126/127, TC-129/130, TC-132/134, TC-136/138; IT-009 static producer boundary |
| Bounded evaluation with honest outcomes | FR-046, FR-047, FR-049; NFR-009 | US-003 → StR-001 | TC-128, TC-131, TC-136/137 |
| Exact compiler contribution to assessment | FR-048, FR-050 | US-002/US-003/US-004 → StR-001 | TC-135 local A→B intake; TC-138 strict temporal artifact; separate external campaign |

Every FR has a linked user need, explicit inputs/outputs, observable behavior and
at least one planned case per acceptance criterion. NFR-009 is explicitly scoped,
constrains FR-046/047/049, has measurable thresholds and is verified by TC-137.
External CLI, pagination, authenticated network API, scaffolding and concurrent
remote-call probes do not apply. External producer and consumer dependencies are
versioned or explicitly left as separately owned gates rather than hidden
fallbacks.

## Verdict

**PASS for specification integrity.** The reviewed model is ready to drive
implementation and assertion-level evidence reconciliation. This verdict does
not mark any Planned row passed, resolve TM-003's separate status-column defect,
or claim the external composed campaign has run.
