---
id: SR-629
title: "Scope-boundary review of ADR-014 temporal, trace and boundedness architecture"
type: SpecReview
analysis: scope-boundary
scope: "spec/decisions/ADR-014-temporal-trace-and-boundedness-architecture.md; amendments to ADR-012, ADR-013, FR-057, FR-082 and spec/spec.md"
review_set: all
relationships:
  - target: ix://agent-ix/quire-spec-language/ADR-014
    type: reviews
---
# SR-629: Scope-boundary review of ADR-014

## Summary

Round 1. Reviewed commit `bbe92fec` (branch `spec/17-boundedness-adr`), the new
ADR-014 and `git diff origin/main -- spec/`. QSpec authority: `main` at
`eb4234f`, read-only. CG code was read at `origin/main` `e2a5671` to check the
negotiation boundary.

The question: does ADR-014 decide only what #222 owns (Q222-1 to Q222-3, the
ADR-013 O-20 owner row, the ADR-012 §1.1 "available finite bound" predicate
and the ticket's eight decisions), without redeciding ADR-011/012/013, QSpec
(FR-290's vocabulary), CG `negotiate_*` or IR? And do its owner and layer
placements follow the ADR-011 §6.1 allow-list?

What holds:

- The ADR adds no capability kind, mode or flag. It keeps FR-290's
  vocabulary and advertised-mode table as QSpec's (§6, Alternatives).
- ADR-012 §2 left extent and authored bound in `Requirements` to #222.
  ADR-014 §4 fills exactly that slot and changes no ADR-012 mechanics.
- `ClaimExtent` and `ProofBound` sit in `qsl-semantics::family`, which is the
  layer-3 `check` core (ADR-011 §6.2). Their consumers, `qsl-route` (R),
  `qsl-package` (4) and `qsl-replay` (6), may all import layer 3.
  `TemporalInterval` sits in the TemporalTrace family `check` module, and its
  consumers are that family's own lower-layer evaluator and replay.
- The native-v1 ceilings (B-6) are left to SEAM-1, as ADR-011 §10 scenario 3
  requires. N-4 adds no compatibility layer.

What does not hold: two named interfaces break §6.1 layer rules (FND-003,
FND-004). One adds kernel types that AD-016's closed kernel row does not
list (FND-001). The component that evaluates the §4 predicate is not named,
though CG reads a `finite_bound_available` flag and computes none (FND-002).
The route guard reassigns part of CG's FR-290 check to QSL without amending
FR-057 (FND-005). The FR-082 amendment goes further than ADR-014 §1 and
conflicts with QSpec FR-150 and with FR-082's own unchanged text (FND-006),
and its code change is given to the wrong ticket (FND-007).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | B-1 and §11 put the new types `CollectionBound{Bounded, Unbounded}` and `PopulationBound{AtMost, Unbounded}` in `quire-exact`, and change the payload of `ValueType::Population`. AD-016's Shared-type row says the kernel holds "exactly the types listed in this row plus `Undefined`". Neither new type is listed, and ADR-011 §6.1 ("`ValueType::Population` carries its count only") fixes the payload. The ADR files no AD-016 amendment, which is a QSpec decision, so QSL-140 would build kernel types the authority does not admit. Fix: add a QC-NN row to ADR-013 §8, an AD-016 kernel-row amendment adding both types and the `PopulationBound` payload. Make QSL-140 (S-6) depend on it in §11 and in the ADR-013 S-6 row. Update the ADR-011 §6.1 K-row wording in the same change. | ADR-014 §1 B-1, §11; QSpec AD-016 Shared-type row; ADR-011 §6.1 "K is a leaf"; ADR-013 §8 |
| FND-002 | high | §4 defines the "available finite bound" predicate but does not say which component evaluates it or where the result travels. CG reads `ExtentClassification.finite_bound_available` from the request and "computes none" (CG `src/capability.rs:152-162`). FR-331's request carries the #222 extent classification. The amended ADR-013 O-20 owner row now says both "Decided in ADR-014 §4" and "the `requires-bound` predicate is IR's (AD-016)", and §6 step 3 says CG "applies the table to the extent". That makes three claimants and no evaluator. Fix: state in §4 and §11 that QSL-140's O-20 request representation computes `finite_bound_available` (true exactly when a `ProofBound` is present for every unbounded domain) and writes it into the FR-331 request's extent classification, and that CG only reads it. Reword the O-20 owner row to separate IR's AD-016 `requires-bound` form predicate (which IR forms have an unbounded domain) from this availability flag. | ADR-014 §4, §6 step 3, §11; ADR-013 O-20; QSpec FR-290 L118-124, FR-331 `request`, AD-016 L450; CG `src/capability.rs` |
| FND-003 | high | §7 and §11 put the `explore::Outcome` → O-16 map in F `diagnostic`. F depends on K only (ADR-011 §6.1; `qsl-foundation/Cargo.toml` names only `quire-exact`), and `explore::Outcome` is a layer-5 `qsl-eval` type. The map cannot compile where it is placed. Fix: put the map in `qsl_eval::simulation::explore`, for example `Outcome::category(&self) -> qsl_foundation::diagnostic::Category`, because layer 5 may depend on F. | ADR-014 §7 closing paragraph, §11 QSL-140 bullet; ADR-011 §6.1 F row |
| FND-004 | medium | §10 scenario 5 and §11 (QSL-43) put `TemporalCounterexample: FamilyPayload` in the layer-5 TemporalTrace evaluator. `FamilyPayload` is defined in `qsl-replay` (`witness.rs:316`, layer 6). `qsl-eval` cannot depend on it, so the impl cannot live in layer 5. The existing family payload precedent (`StateForallPayload`) lives in `qsl-replay`, and ADR-011 §6.1 says each family's ticket widens `replay`. Fix: place `TemporalCounterexample` and its `FamilyPayload` impl in `qsl-replay`, built from the layer-3 `TemporalInterval` and the evaluator's `TemporalPosition`. Keep only the lasso evaluation (A-4) in layer 5. | ADR-014 §10.5, §11 QSL-43; `qsl-replay/src/witness.rs:316`; ADR-011 §6.1 layer 6 `replay` row |
| FND-005 | medium | §6 step 4 adds a QSL `route` guard that re-checks a `supported` settlement against `ClaimExtent` and the descriptor's advertised modes. Three things conflict with it. (1) The input contract of FR-057 "Stage ownership" and ADR-011 §6.1 R: after E7, route reads the FR-331 dispositions only, and `routing.rs` "reads nothing but its input... routing needs only the disposition". (2) FR-290-AC-6 makes "unbounded never settles `supported` on a bounded-only advertisement" a `negotiate_*` obligation. (3) It reuses FR-290's negotiation cause `inconsistent-candidates` as a QSL refusal. §11 gives the guard to QSL-42, but route is #185's, and FR-057 and FR-075 are not amended. Fix: drop the guard and rely on CG's FR-290-AC-6 (TC-271). If it is kept, amend FR-057 and ADR-011's R row with route's new inputs (the item's `Requirements` and the registry descriptor), give it a QSL-owned cause that is not borrowed from FR-290, and assign it to #185. | ADR-014 §6.4, §10.3, §11 QSL-42; FR-057 Stage ownership; `qsl-route/src/routing.rs:1-18`; QSpec FR-290 L177, L191, AC-6; ADR-011 §6.1 R row, "`route` is not S3" |
| FND-006 | high | ADR-014 §1 reclassifies only the ceilings that "the check-stage type environment reads" as B-3. The FR-082 amendment extends this to "a walk the check stage performs, in model normalization". That conflicts with three texts. (1) FR-082's own unchanged paragraph: the model checker refuses with the resource-exhaustion cause `ancestor-steps` under `ModelNormalizationLimitsV1`. (2) NFR-012 L57: `resource_exhausted` naming `ancestor_steps`. (3) QSpec FR-150 and TC-195 N01: exhaustion under `ModelNormalizationLimitsV1` is `resource_exhausted`/`insufficient-next-charge` at stage `model-normalization`. §1's classification rule, "a ceiling that a compiler stage... reads is B-3", would also reclassify QSpec accounting-contract meters charged at S3, which redecides QSpec. Fix: scope the rule so that a limit carried in a `quire.value.accounting/v1` limits type (`ModelNormalizationLimitsV1`, `PopulationAdmissionLimitsV1`) is B-2 whichever stage reads it. Restrict the FR-082 amendment to the expression checker's type-environment admission, which matches §1. | ADR-014 §1 classification rule, §12; FR-082 L110-127 and the added paragraph; NFR-012 L57; QSpec FR-150 L102-108, TC-195 N01 |
| FND-007 | medium | §11 and the `spec.md` FR-082 row give the FR-082 check-time `LimitExceeded` code change to QSL-140 (ADR-013 S-6: bounds, modes, capability). `LimitExceeded`/`LimitKind` and "every S3 ceiling" belong to S-5b (QSL-160, FR-096), and FR-082's remaining work is #120's. Fix: assign the reclassification to QSL-160, or to #120 if QSL-160 has landed, in §11, §12 and the `spec.md` FR-082 row. | ADR-014 §11, §12; `spec/spec.md` FR-082 row; ADR-013 §7 S-5b, S-6; FR-096 |
| FND-008 | low | TR-2 names two owners for the single type `TemporalPosition`: "layer 5 evaluator, layer 3 checked form". §11 puts it in the layer-5 evaluator. One type lives in one crate. Fix: name one owner in TR-2, the layer-5 TemporalTrace evaluator as §11 says, or the layer-3 family `check` module if the checked form must carry it. | ADR-014 §3 TR-2, §11 QSL-43; ADR-011 §6.1 "Families are modules, not crates" |

## Resolution

Resolved by the author in the follow-up commit on `spec/17-boundedness-adr`.

FND-001 fixed: no new kernel types; `Option` field shapes (N-3, Alternatives). FND-002 fixed: §4 names QSL-140's request writer as the evaluator; the O-20 row separates IR's form predicate. FND-003 fixed: `Outcome::category()` in `qsl-eval`. FND-004 fixed: `TemporalCounterexample` in `qsl-replay`. FND-005 fixed: guard dropped. FND-006 fixed: classification by limits type. FND-007 fixed: QSL-160. FND-008 fixed: TR-2 names the layer-5 evaluator.
