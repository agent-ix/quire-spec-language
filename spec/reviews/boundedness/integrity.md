---
id: SR-625
title: "Integrity review of ADR-014 temporal, trace and boundedness architecture"
type: SpecReview
analysis: integrity
scope: "spec/decisions/ADR-014-temporal-trace-and-boundedness-architecture.md, spec/decisions/ADR-012-semantic-family-extension-contracts.md, spec/decisions/ADR-013-canonical-type-package-conversion-ownership.md, spec/functional/FR-057-admit-shared-capability-kinds.md, spec/functional/FR-082-resolve-conformance-subsetting-and-redefinition.md, spec/spec.md"
review_set: all
relationships:
  - target: ix://agent-ix/quire-spec-language/ADR-014
    type: reviews
---
# SR-625: Integrity review of ADR-014 temporal, trace and boundedness architecture

## Summary

Reviewed ADR-014 on `spec/17-boundedness-adr` at commit `bbe92fec`, and its
amendments to ADR-012 §1.1, ADR-013 (Q222 table, O-20, O-21, S-6), FR-057,
FR-082 and `spec/spec.md`. The review checked four things:

- completeness against Linear QSL-17 (#222), read as data;
- consistency with ADR-011, ADR-012 and ADR-013;
- consistency with QSpec `main` at `eb4234f`;
- the QSL code claims, each opened at its file and line.

What holds:

- All eight required decisions have a section: §3 (trace concepts), §1
  (bound kinds), §2 (absent bounds), §5 (facet), §6 (negotiation), §7
  (outcomes), §8 (bounded runtime) and §9 (versions).
- All five change scenarios are in §10. The M-6c exit criterion is cited.
- §11 names the interfaces that QSL-140, QSL-42 and QSL-43 build.
- The QSpec facts in Context check out on `eb4234f`. These are FR-144-AC-9,
  AC-12 and AC-13, FR-153-AC-9, FR-228-AC-5, FR-090-AC-7, FR-161-AC-2 and
  AC-7, the FR-341 rows, the FR-290 advertised-mode table, the value and
  model root definitions at `1-draft.2`, and the diagnostics catalog at
  `1-draft.7` with its causes.
- The code locations are correct. `CardinalityBound` is at
  `quire-exact/src/collection.rs:75`, `ValueType::Population(u64)` at
  `value.rs:205`, the grammar bound at `qsl-cst/src/grammar.rs:419-429`,
  `MAX_SEQUENCE_ITEMS` at `src/native_model/admission.rs:18`, and
  `proof_bounds: ScalarLimits` at `qsl-replay/src/witness.rs:346`.
  `DeclaredDomain.domain: String`, the opaque `TracePosition`,
  `explore::Outcome`, `IncompleteCause`, `UnavailabilityCause::SolverAbsent`,
  `routing::Disposition`, `qsl_route::Mode` and `model::Extent{Closed, Open}`
  are as described.
- SR-625 is unused elsewhere in the repo.

What does not hold:

- Two findings are high. The infinite-trace interval contradicts QSpec. The
  `requires-bound` re-request never becomes bounded under the §4 extent rule.
- The rest are gaps in the bound classification, in the `route` guard and
  in the trace provenance, plus ambiguous citations.
- The acceptance item "passes `/spec-review all`" is out of scope for this
  one analysis.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | TR-3, A-1, Context and scenario 5 admit a `[a,*]` interval, `TemporalInterval{upper: IntervalUpper::Unbounded}`, under `quire.temporal.infinite-trace/v1`, and scenario 5 compares the failing operator's interval key at replay. QSpec says the opposite. FR-250 (profile row and FR-250-AC-6) says the infinite-trace profile "admits no interval bound". FR-255 Absence says "an operator admitted under the infinite-trace profile carries no interval object at all". TC-238 says `[a,*]` "keeps the admission it already had under each profile", not that it is admitted only under infinite-trace. Fix: under infinite-trace an operator carries no interval, so `Option<TemporalInterval>` is always `None` there. Drop `IntervalUpper::Unbounded`, or cite a filed QSpec change before keeping it. Make `TemporalCounterexample.interval` optional, and make the scenario 5 replay check compare the profile selection and the absence of an interval. | ADR-014 §3 TR-3, §5 A-1, §10.5, Context; QSpec FR-250-AC-6, FR-255 (Absence), TC-238 |
| FND-002 | high | The `requires-bound` loop is never closed. §4 computes extent only from authored types (`CollectionBound::Unbounded`, an unbounded integer, and so on). Derivation 2 says a `ProofBound` "makes a new bounded request", but no rule makes that request's extent `Bounded`. Under §4 the re-request still has extent `Unbounded`, still carries a `ProofBound`, and settles `requires-bound` again. §4 also does not say how a request that carries a `ProofBound` for an unbounded item differs from the derived bounded request. Fix: state in §4 that the derived request replaces each unbounded domain with its `ProofBound`, and that its extent is therefore `Bounded` (FR-290: "A finite domain classifies as bounded"). Name the field or form that tells the two requests apart. | ADR-014 §1 derivation 2, §4, §6.5, §10.3; QSpec FR-290 (extent classification) |
| FND-003 | medium | FR-082 now contradicts itself. The unamended paragraph "Ancestor and conformance walks are bounded" still says every walk under `ModelNormalizationLimitsV1` refuses with the resource-exhaustion cause (`ancestor-steps`). The new paragraph right after it says a model-normalization walk is a stage limit reported `stage_limit_exceeded`. Fix: rewrite the earlier paragraph so it states only the S6a population-admission case, or make it defer to the new stage-based rule. | FR-082 "Ancestor and conformance walks are bounded", new ADR-014 paragraph |
| FND-004 | medium | The S6a `ancestor_steps` ceiling is misclassified. ADR-014 §1 calls it B-2, and the B-2 row says a reached B-2 bound yields `Outcome::Incomplete` (O-16 `incomplete`). FR-082 says this ceiling "is read, never charged", and that reaching it is "a `Refused` outcome, never the `Incomplete` outcome". The §1 rule says "a budget that S6a evaluation charges is B-2", which does not cover a ceiling that S6a reads without charging. Fix: extend the classification rule to read-only S6a ceilings, and state their outcome (a refusal, `resource_exhausted`/`ancestor-steps`) in the B-2 row or in a separate row. | ADR-014 §1 B-2 row and classification rule; FR-082 |
| FND-005 | medium | The classification rule leaves NFR-012's model-normalization ceilings unplaced. `ModelNormalizationLimitsV1` is charged during S3 `model` normalization, so the rule ("a ceiling that a compiler stage reads is B-3") makes it a stage limit. NFR-012 still requires an incomplete result and `resource_exhausted` for it (NFR-012 lines 20-21 and 57). ADR-013 O-21 calls `model::accounting` a separate layer-3 meter. ADR-014 amends FR-082 for model normalization but not NFR-012. Fix: place `ModelNormalizationLimitsV1` explicitly in §1. Either amend NFR-012 in §12 to `stage_limit_exceeded`, or state why it stays B-2. | ADR-014 §1, §12; NFR-012; ADR-013 O-21; FR-096 |
| FND-006 | medium | The §6 step 4 `route` guard is not specified against the current interface or requirements. `routing::route(&[Disposition]) -> Vec<Option<&Candidate>>` (`qsl-route/src/routing.rs:58`) receives no descriptor or extent and has no refusal return. FR-075 (lines 92-95) and the `Mode` doc (`qsl-route/src/lib.rs:108-111`) say the registry and `route` never compare a mode against an extent. The guard reuses `invalid_capability`/`inconsistent-candidates`, which FR-290 and the catalog define as a candidate-set disagreement settled by `negotiate_*` as `invalid-request`. §12 amends no FR for this. Fix: name the new `route` inputs and refusal type, and amend FR-075 (or the routing requirement) in §12. Either get a catalog cause for a malformed `supported` settlement, or record why `inconsistent-candidates` covers it. | ADR-014 §6.4, §10.3, §11 (QSL-42); FR-075; QSpec FR-290 candidate table; native-diagnostics `invalid_capability` |
| FND-007 | medium | The trace provenance claims do not match the code. TR-1 and TR-6 identify a sampled trace by (seed, sampler `DefinitionRef`, trace index) "carried by" `SampleProvenance`. That struct (`qsl-eval/src/simulation/trace.rs:29-36`) holds `seed`, `sampler_version: String` and `stopped`, with no `DefinitionRef` and no index. Scenario 4 says the exploration run's seed is in `SampleProvenance`, but `explore.rs` has no seed, and `SampleProvenance` is built only by `sample.rs:126`. Fix: name the ticket that adds the sampler `DefinitionRef` and trace index to `SampleProvenance` (QSpec FR-181 names the `DefinitionRef`). Drop the seed claim from scenario 4: exhaustive exploration is deterministic given `Limits` and the model. | ADR-014 §3 TR-1, TR-6, §10.4; QSpec FR-181 |
| FND-008 | medium | Scenario 1 says a bounded-only temporal backend settles `unbounded-extent` because "a temporal formula has no `ProofBound` (§4)". Neither §4 nor §1 states this. §4 lists an infinite-trace formula as an unbounded domain, and its predicate would accept a `ProofBound` for that domain. Fix: state in §4 that no `ProofBound` exists for an infinite-trace domain, because finite prefixes never prove infinite satisfaction (QSpec FR-161). The predicate is then always false for it. | ADR-014 §4, §10.1; QSpec FR-161 |
| FND-009 | medium | Bare FR ids are ambiguous between repos. The ADR cites QSpec FR-090, FR-091 and FR-092 bare, and cites QSL FR-057, FR-075, FR-082 and FR-096 bare. QSL has its own FR-090 to FR-096 with different meanings. For example, §2's "(FR-091, FR-092)" resolves in QSL to value forms and node keys, not to temporal interval refusal. Fix: prefix every QSpec id ("QSpec FR-091") or link it, as FR-057 does. | ADR-014 Context, §2, §5, §7, §10 |
| FND-010 | low | Ownership of the predicate is stated two ways. The amended O-20 owner row still says "the `requires-bound` predicate is IR's (AD-016)", and its Validation row says "IR `requires-bound` is the single predicate". ADR-014 §4 now defines the available finite bound predicate, which FR-290 assigns to #222. Fix: in O-20, say that IR's predicate classifies an unbounded construct, that the availability predicate is ADR-014 §4's, and that CG evaluates it on the FR-331 `domains`. | ADR-013 O-20; ADR-014 §4; QSpec FR-290, AD-016 |
| FND-011 | low | The §7 intro says "Every case already has a type". The cancellation row lists S6a, but it names only `IncompleteCause::Cancelled` (a backend FR-331 cause) and `explore::Outcome::Cancelled`. The kernel `Outcome` and `Incomplete` in `quire-exact` have no cancellation variant. Fix: remove S6a from the row, or name the type to be added and the ticket that adds it. | ADR-014 §7; `quire-exact/src/outcome.rs` |
| FND-012 | low | Derivation 1 ("B-1 → B-4") is not a conversion into a B-4 value: it says a bounded item needs no `ProofBound`. It also cites ADR-013 C-25, which covers the disposition of an unbounded domain. Fix: restate it as "a bounded authored domain needs no B-4 value", and cite the AD-016 harness-domain row instead of C-25. | ADR-014 §1 derivations; ADR-013 C-25 |
| FND-013 | low | The front matter omits QSpec artifacts that the decisions depend on: FR-250, FR-255, FR-341, FR-331, AD-016 and the `quire.native.diagnostics/v1` catalog. Fix: add `depends_on` edges for them. | ADR-014 front matter |
