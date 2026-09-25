---
id: SR-624
title: "failure-domain review of ADR-014 temporal, trace and boundedness architecture"
type: SpecReview
analysis: failure-domain
scope: "spec/decisions/ADR-014-temporal-trace-and-boundedness-architecture.md, spec/decisions/ADR-012-semantic-family-extension-contracts.md, spec/decisions/ADR-013-canonical-type-package-conversion-ownership.md, spec/functional/FR-057-admit-shared-capability-kinds.md, spec/functional/FR-082-resolve-conformance-subsetting-and-redefinition.md, spec/spec.md"
review_set: all
relationships:
  - target: ix://agent-ix/quire-spec-language/ADR-014
    type: reviews
---
# SR-624: Failure-domain review of ADR-014

## Summary

Reviewed ADR-014 and its amendments at commit bbe92fec on
`spec/17-boundedness-adr`, against QSpec `main` at eb4234f (FR-090, FR-144,
FR-153, FR-161, FR-228, FR-255, FR-290, FR-341, FR-346, FR-347, FR-348, AD-016,
native-diagnostics `1-draft.7`) and QSL code. The checklist covered failure
behaviour at trust boundaries (negotiation, `route`, replay), identity keys
(proof bound, interval, trace, sample), purity and decidability of S6a
evaluation, and completeness of the extent classification.

The QSL code citations check out: `CardinalityBound` at
`quire-exact/src/collection.rs:75`, `ValueType::Population(u64)` at
`quire-exact/src/value.rs:205`, the mandatory `[uint, uint]` bound at
`qsl-cst/src/grammar.rs:419-429`, `proof_bounds: ScalarLimits` at
`qsl-replay/src/witness.rs:346`, `MAX_SEQUENCE_ITEMS` at
`src/native_model/admission.rs:18`, and the `IncompleteCause`,
`UnavailabilityCause`, `explore::Outcome` and `Mode` shapes.

The bound taxonomy, the no-conversion rule and the Kani non-narrowing argument
hold up. Five high findings remain. The `[a,*]` interval contradicts FR-090 and
FR-255. Fairness is absent from the infinite-trace design. §8 contradicts
FR-346 for quantifiers over unbounded collections. The extent rule omits
recursive value types. The FR-082 reclassification leaves NFR-012 and FR-082's
own earlier paragraph stating the old code. The medium and low findings cover
proof-bound keying, the `route` guard's inputs, safety classification in S6a,
sample identity, predicate ownership and some unnamed refusal codes.

Verdict: REVISE. The high findings change decided rules, so they should be
fixed before QSL-140, QSL-42 and QSL-43 build against the record.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | TR-3, §2 and the Context admit `[a,*]` under `quire.temporal.infinite-trace/v1` as a `TemporalInterval` with `IntervalUpper::Unbounded`, keyed by FR-255. FR-090 says "The infinite-trace profile shall admit no interval bound", and FR-255 says "An operator admitted under the infinite-trace profile carries no interval object at all". Only shared-grammar line 498 permits `*`. TR-3 also lets a finite `[a,b]` through under the infinite-trace profile, which FR-090 forbids. The FR-255 key includes a clock binding, which the infinite-trace profile does not have. Fix: record this QSpec conflict as a second open dependency in §13 and do not cite FR-255 as support. Until QSpec rules on it, either refuse `[a,*]` and `[a,b]` under infinite-trace with `unsupported_construct` (TR-3 then has only `Option<TemporalInterval>` with `None`), or name the clock-binding component of the key. | ADR-014 §2, TR-3, A-2; QSpec FR-090 l.52, FR-255 Absence, shared-grammar l.438/498 |
| FND-002 | high | Fairness appears nowhere in ADR-014. FR-161 says "Fairness restricts the admitted infinite traces before formula evaluation", and FR-161-AC-5 includes fairness in every proof or counterexample replay identity. FR-341 `unsupported` covers "a missing fairness/model closure premise". A-4 evaluates a lasso "exactly", and scenario 5's `TemporalCounterexample{prefix, loop, interval}` and replay check carry no fairness. An unfair lasso would therefore replay as a real violation. Fix: add the fairness assumptions to A-4's inputs, to `TemporalCounterexample`, and to the scenario 5 recompile comparison. Make A-4 check the lasso against fairness before it evaluates the formula, and settle a missing fairness premise as FR-341 `unsupported`. | ADR-014 A-4, A-5, TR-1, §10 sc.5; QSpec FR-161 Behavior, AC-5; FR-341 |
| FND-003 | high | §8 and scenario 2 say S6a evaluation of a concrete value of an unbounded type is exact and stops only on B-2. FR-346 says that for `forall`/`exists` over a collection with no FR-144 bound, "the reference evaluator does not visit occurrences to decide the quantifier" and must negotiate instead. ADR-014 does not say what S6a does when it reaches such a quantifier, so it contradicts FR-346 by omission. Fix: add an §8 rule for this case. S6a must not decide `forall`/`exists` over an unbounded-typed collection by visiting occurrences. Either it yields a typed non-verdict or it defers to negotiation (FR-346-AC-2). Traversal and construction stay exact (FR-144-AC-12). | ADR-014 §8, §10 sc.2; QSpec FR-346 Description, Behavior, AC-2 |
| FND-004 | high | The §4 extent rule lists five unbounded domains: collections, populations, `Integer`, FR-228-AC-5 loops and infinite-trace formulas. It omits FR-143 recursive value types and FR-347's "another domain with an authored well-founded order". It also does not say whether a domain nested inside a composite, option or element type counts. So a claim over a recursive type classifies `Bounded`, reaches Kani `supported`, and is narrowed silently to the unwind depth. That is the failure the ADR exists to prevent. Fix: define the extent over the transitive closure of the argument types, and add recursive value types (FR-143) and authored well-founded domains (FR-347) to the unbounded list. | ADR-014 §4; QSpec FR-347 Description, Inputs; FR-290 Advertised mode |
| FND-005 | high | The FR-082 amendment makes check-stage walks, "in model normalization", return `stage_limit_exceeded`. But FR-082's own unchanged paragraph just above it still says the model checker "SHALL refuse with a resource-exhaustion cause (`ancestor-steps`)" for every walk. NFR-012 (implemented, TC-434) says `ancestor_steps` and `family_steps` in normalization refuse `resource_exhausted`. So the two SHALLs conflict. `family_steps` is not classified at all. The §1 classification rule covers "a ceiling a stage reads" and "a budget S6a charges", but not the normalization `Meter` charged at check stage (`effective_declarations`, `derivation_facts`), which B-2 names as its type. Fix: scope FR-082's earlier paragraph to S6a population admission, and amend NFR-012 and TC-434 in this change. Classify `family_steps`, and add a classification clause for a meter that a compiler stage charges. | FR-082 §"Ancestor and conformance walks are bounded"; NFR-012 l.55-57, l.122-124; ADR-014 §1 |
| FND-006 | medium | The identity of a `ProofBound` is not defined. §4 says "one per unbounded domain", but the existing `DeclaredDomain` is keyed by `parameter: WireNodeId` (`qsl-replay/src/identity.rs:246`). One parameter can hold several unbounded domains, for example `Set<Seq<Integer>>`. Loops and populations are not parameters. Only scenario 1 states that a temporal formula can never have a `ProofBound`, and §4 says nothing about loops. There is also no failure path for a malformed request: a `ProofBound` for a domain that is already bounded or does not exist, two bounds for one domain, or an empty or inverted range. Fix: in §4, define the `ProofBound` key (the checked node id of the domain's type position), state which domain kinds can take one (not infinite-trace formulas, and say whether loops can), and refuse each malformed case as `invalid-request` with a named `invalid_capability` cause. | ADR-014 §4, §11; `qsl-replay/src/identity.rs:246-249` |
| FND-007 | medium | The §6 step 4 `route` guard cannot be built from `route`'s current inputs. `route(dispositions: &[Disposition]) -> Vec<Option<&Candidate>>` (`qsl-route/src/routing.rs:58`) sees neither the item's `ClaimExtent` nor the descriptor's `advertises`, and it cannot return a refusal. The guard also does not check a `RequiresBound` that arrives for an item whose predicate is false. It reuses `inconsistent-candidates`, which FR-290 defines as a candidate-set mismatch and not a mode mismatch. Fix: name the new `route` signature in §11, taking per-item extent and the registry snapshot and returning a per-item refusal. Guard `RequiresBound` too, and use a distinct cause (for example a mode-mismatch cause in QSL's catalog claim), or cite the FR-290 text that makes `inconsistent-candidates` cover this case. | ADR-014 §6.4, §11; `qsl-route/src/routing.rs:58`; QSpec FR-290 candidate table |
| FND-008 | medium | A-4 lets S6a return a violation over a finite trace "only for a safety formula whose bad prefix is complete", and pending otherwise. The ADR does not define which formulas count as safety, or how to decide that a bad prefix is complete. The result therefore depends on the implementation. Fix: name the decision rule in A-4, for example a syntactic safety fragment, or three-valued evaluation where violation means every extension is false. State that any formula outside the rule yields pending. | ADR-014 A-4; QSpec FR-161 Behavior |
| FND-009 | medium | TR-1 says a sampled trace's identity is (seed, sampler `DefinitionRef`, trace index), "carried by" `SampleProvenance`. TR-6 says the same seed and `DefinitionRef` reproduce the same traces. But `SampleProvenance` has `seed`, `sampler_version: String` and `stopped` (`qsl-eval/src/simulation/trace.rs:29-36`). It has no `DefinitionRef` and no trace index. Fix: either list the change to `SampleProvenance` in §11 as QSL-140 work, or restate TR-1 and TR-6 in terms of the fields it actually has. | ADR-014 TR-1, TR-6; `qsl-eval/src/simulation/trace.rs:29` |
| FND-010 | medium | The owner of the predicate is unclear. ADR-014 §4 defines the "available finite bound" predicate, and the amended ADR-013 O-20 row says "Decided in ADR-014 §4". The same row still says "the `requires-bound` predicate is IR's (AD-016)", and AD-016 (accepted) calls IR `requires-bound` "the single predicate". Fix: in §4 and the O-20 row, state the split. ADR-014 defines the input, which is the caller's `ProofBound` per domain. IR's single predicate evaluates that input, and CG reports the result. Or file the QSpec question if AD-016 must change. | ADR-013 O-20 owner row; ADR-014 §4; QSpec AD-016 l.180, l.450 |
| FND-011 | low | The lasso has no validity rule. A-4 and TR-2 assume a finite prefix plus a loop, but nothing refuses an empty loop, and the ADR does not say at which position the loop re-enters. TR-2's "decimal ASCII with no leading zero" does not say that `0` itself is valid. Fix: require a non-empty loop, and state that it re-enters at the first loop position. Refuse a malformed lasso at reconstruction with a named cause, and state that `0` encodes position zero. | ADR-014 TR-2, A-4, §10 sc.5 |
| FND-012 | low | TR-4 says an overflowing horizon sum "refuses at check", but does not name the code or cause. Fix: name it. Either it is `stage_limit_exceeded` with a named limit kind, or it is an `ill_typed`-style refusal, and TR-4 should say which. | ADR-014 TR-4 |
| FND-013 | low | The §7 outcome table has no row for a tool that changes after a passing probe. FR-290 records that case as FR-331 result `failed` with `unsupported_projection`/`tool-unavailable`, distinct from absence found at the probe. Fix: add the row. | ADR-014 §7; QSpec FR-290 Tool absence, AC-8 |
| FND-014 | low | FR-082 maps `ancestor_steps` to the limit kind nesting depth. FR-082 defines it as "the types one walk expands". Under multiple supertypes that is a count of expanded types, not a depth. Fix: either justify nesting depth (the walk is chain-shaped, as FR-082 says with "n generalization steps"), or map it to node count. | FR-082 amendment; FR-096 `LimitKind` |
