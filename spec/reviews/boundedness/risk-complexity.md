---
id: SR-628
title: "Risk and complexity review of ADR-014 temporal, trace and boundedness architecture"
type: SpecReview
analysis: risk-complexity
scope: "spec/decisions/ADR-014-temporal-trace-and-boundedness-architecture.md (new); amendments to ADR-012 §1.1, ADR-013 (O-20, O-21, S-6, Q222 table), FR-057, FR-082 and spec/spec.md at bbe92fec against origin/main"
review_set: all
relationships:
  - target: ix://agent-ix/quire-spec-language/ADR-014
    type: reviews
---
# SR-628: Risk and complexity review of ADR-014

## Summary

Reviewed commit `bbe92fec` (branch `spec/17-boundedness-adr`) against
`origin/main`, with QSpec read at `eb4234f`. The QSL code claims in ADR-014
Context and §1 hold on inspection. These are `CardinalityBound` (mandatory min
and max), `ValueType::Population(u64)`, the mandatory `[uint, uint]` grammar
bound, `WitnessEnvelope.proof_bounds: ScalarLimits` at `witness.rs:346`,
`DeclaredDomain.domain: String`, `explore::Outcome{Exhaustive, Bounded,
Cancelled}`, `SampleProvenance`, `IncompleteCause::{TimedOut, Cancelled}`,
`UnavailabilityCause::SolverAbsent`, `Disposition::RequiresBound`,
`MAX_SEQUENCE_ITEMS` at `admission.rs:18`, and `model::Extent`.

The bound taxonomy (§1), the absent-bound table (§2) and the outcome table (§7)
are sound and add little complexity. They reuse existing types.

The highest risk is the §4 "available finite bound" predicate. As written,
`requires-bound` settles only when the caller has already supplied the bound.
A caller that has not supplied one gets `unsupported` and is never told that a
bound would help. The re-request that §4 prescribes has no rule that makes its
extent `Bounded`. That leaves a `requires-bound` loop that ADR-012 §1.1
("runs only after the caller supplies a bound") does not resolve.

The second risk is the FR-082 reclassification. It contradicts NFR-012, and
the classification rule reaches further than the amendment states. The test
matrix still shows TC-220 as passing against the old code.

The remaining risks are underspecification that QSL-140 and QSL-43 would
otherwise have to invent: the shape of `ProofBound`, the safety decision
procedure, the lasso cost, and which boundedness predicate is authoritative.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The §4 predicate makes `requires-bound` settle only when the request already carries a `ProofBound` for every unbounded domain. With no bound supplied, the item settles `unsupported`, so the caller never gets a "supply a bound" answer. When the caller does resubmit, the §4 extent rule is computed from authored types alone, so the resubmitted item is `Unbounded` again. It carries a `ProofBound`, the predicate is true, and it settles `requires-bound` again. That is a loop. ADR-012 §1.1 ("runs only after the caller supplies a bound") presumes the opposite order. Fix: pick one rule and state it with a scenario. Either (a) a request that carries a `ProofBound` for every unbounded domain is the derived bounded request (derivation 2 is applied when the request is built, the extent is `Bounded` under the `ProofBound`-qualified identity, and it never settles `requires-bound`), and `requires-bound` means "no `ProofBound` supplied, and every unbounded domain is of a boundable kind (collection, population, integer, loop)", with `unsupported`/`unbounded-extent` kept for domains that cannot be bounded (an infinite trace); or (b) keep the current predicate but state the extent rule for a request that carries `ProofBound`s. | ADR-014 §4, §6 step 5, §10 scenario 3; ADR-012 §1.1; QSpec FR-290 advertised-mode table |
| FND-002 | high | FR-082 now says a check-stage walk "in model normalization" reads a stage limit (B-3) and returns `stage_limit_exceeded`. NFR-012 (not amended) says normalization's `ancestor_steps` and `family_steps` refuse `resource_exhausted`, and that every normalization ceiling yields an incomplete result. The §1 classification rule ("a ceiling that a compiler stage reads is B-3") also covers every `ModelNormalizationLimits` ceiling (declaration records, facts, hashed bytes), not only `ancestor_steps`/`work_units`. Fix: amend NFR-012 in this change. Also state in §1/§12 whether all normalization ceilings become B-3, or narrow the rule. Otherwise QSL-140 inherits two contradictory SHALLs. | ADR-014 §1 classification rule, §12; FR-082 amendment; `spec/non-functional/NFR-012-bound-model-normalization-and-population-admission.md` lines 18-57 |
| FND-003 | medium | FR-082-AC-3, AC-6 and AC-7 now require `LimitExceeded` at check time. `spec/tests.md` still marks TC-220 "✅ Passed locally", backed by tests that assert `Code::ResourceExhausted`/`"resource_exhausted"` (`qsl-semantics/tests/it/model_conformance.rs` around line 2063, `type_environment_model.rs:388`). The matrix now claims coverage that the code contradicts. Fix: mark TC-220 as not implemented for AC-6/AC-7 (and the check-time half of AC-3), pending QSL-140, in `spec/tests.md`. | FR-082-AC-3/6/7; spec/tests.md TC-220 row |
| FND-004 | medium | FR-082-AC-6 requires check-time admission to use "the same `ancestor_steps` ceiling" as the S6a population walk. ADR-014 now makes those two readings two kinds (B-3 and B-2) whose values "never convert". The record does not say how one caller-supplied number becomes both a `LimitExceeded` stage limit and a `Meter` limit. Fix: state in §1 that the one configured value is supplied to both limit types by the caller (not derived one from the other), or give check time its own B-3 ceiling and restate AC-6's "never less strict" guarantee as a relation between the two values. | ADR-014 §1; FR-082-AC-6; NFR-012 |
| FND-005 | medium | `ProofBound` has no defined shape. §1 and §4 say "one per unbounded domain", but the domains are of different kinds: collection cardinality, population count, integer range, loop unwinding, and the infinite trace (which §10 scenario 1 says cannot take one). Its wire spelling in FR-331 `domains` and in `DeclaredDomain.domain` (a `String` today) is also unstated. QSL-140 would invent it. Fix: list the `ProofBound` variants per domain kind, which kinds admit none, and the canonical wire spelling that `DeclaredDomain.domain` parses. | ADR-014 §1 B-4, §4, §11 QSL-140; `qsl-replay/src/identity.rs:246-249` |
| FND-006 | medium | The extent rule makes any claim over an `Int` argument with no range `Unbounded`, because the kernel `Integer` is unbounded (§2). Under Kani's bounded-only manifest, every such claim then needs a `ProofBound` or settles `unsupported`. The native lowering today proves over `i64` (`src/lowering/inputs.rs:113`). This is the largest behaviour change in the record, yet Consequences and §8 ("every existing bounded corpus keeps its results") do not mention it. Fix: state it in Consequences, name `bounded_domain` as the authored way to make a claim `Bounded`, and add it to §10 scenario 3. | ADR-014 §2, §4, §6 step 5, §8, Consequences |
| FND-007 | medium | There are two boundedness predicates. ADR-012 §1.1 and the amended ADR-013 O-20 owner row still say "the `requires-bound` predicate is IR's (AD-016)", with QSL's extent as an input. ADR-014 §4 defines the extent rule and the availability predicate in QSL, and §6 has CG apply the table to QSL's extent. If IR classifies differently (for example, treating integers as machine-width), CG receives two answers. Fix: state which one is authoritative, and state the other as a consistency check that refuses on disagreement. Update the O-20 owner row so it no longer names both. | ADR-014 §4, §6; ADR-013 O-20 owner row; ADR-012 §1.1 |
| FND-008 | medium | A-4 requires S6a to return violation over a finite trace "only for a safety formula whose bad prefix is complete". Deciding whether an LTL formula is a safety property, and whether a prefix is bad, is PSPACE-complete in general. The record names no procedure, so QSL-43 has either a hard algorithm or an unsound shortcut. Fix: fix a sound, incomplete procedure, for example three-valued formula progression that reports violation only when progression reaches `false` and otherwise reports pending. Add the soundness condition (every infinite extension violates) as an acceptance criterion with a TC. | ADR-014 §5 A-4; QSpec FR-161, FR-161-AC-2 |
| FND-009 | low | A-4 states that lasso evaluation costs (prefix + loop) × formula size. That holds for unmetric LTL, but not for interval operators `[a,b]` or `[a,*]` whose `a` or `b` exceed the lasso length. Those need unrolling up to the interval endpoint. Fix: drop the closed-form bound and keep only "charged per (node, position) visit under TR-5", or state the bound including the interval endpoints. | ADR-014 §5 A-4, §3 TR-3, TR-5 |
| FND-010 | low | §6 step 4's `route` guard reuses `invalid_capability`/`inconsistent-candidates`. FR-057 defines that cause for a candidate set that is inconsistent with the manifest, or one that contains a candidate not advertising the kind. A `supported` settlement on a mode-mismatched candidate is a different condition, and FR-057's table does not list it. Fix: add the row to FR-057's case table in this change, so the cause's coverage is stated in one place. | ADR-014 §6 step 4; FR-057 candidate-set table (line 271) |
| FND-011 | medium | §13's open dependency is that QSpec has no v2 temporal operation identities, and that blocks S4 emission of every temporal formula and IR-7. No tracking ticket is named, and ADR-013 has no QC row for it, so nothing gates or unblocks it. Fix: name the QSpec ticket (file one if none exists) and add a QC row to ADR-013, as other QSpec asks are tracked there. | ADR-014 §13; ADR-013 QC table |
