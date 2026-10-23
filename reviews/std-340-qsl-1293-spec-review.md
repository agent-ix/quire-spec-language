---
id: SR-16200
title: "spec-review of FR-315 false-extension items, internal faults and violation order (quire-spec-language PR 1293)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@c33edaa2; spec/functional/FR-315-encode-the-smt-lib-transition-relation.md; spec/test-cases/TC-900-the-smt-lib-encoding-is-canonical.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-315
    type: references
  - target: ix://agent-ix/quire-spec-language/TC-900
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-123
    type: references
  - target: ix://agent-ix/quire-spec-language/ADR-014
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-182
    type: references
---

# Spec review of FR-315 false-extension items, internal faults and violation order (quire-spec-language PR 1293)

## Summary

Ticket: STD-340, a follow-up to the merged STD-338 (#1289). This is one
spec-review pass over `git diff origin/main...c33edaa2`, which touches FR-315
and TC-900 only. The oracles were:

- ADR-018's property-form table (TP-1 to TP-4) and CX-1;
- ADR-014 TR-3 and TR-4;
- FR-123 "Property form" and its implementation in
  `qsl-semantics/src/check/temporal_clause.rs`, `property_form` (lines
  1193-1216) and `checked_operators` (lines 739-744);
- FR-125-AC-2;
- FR-127's replay settlement table;
- FR-182's `EncodingRefused` and `ItemIdentity`;
- FR-096 `InternalFault`.

Gates: `make check-spec-validation` (QSPEC_DIR=~/dev/quire-specification)
exits 0 with no warning on FR-315 or TC-900. origin/main is an ancestor of
the head. FR-315-AC-29 to AC-32 appear in no other open PR.

**Owner question, checked against the sources.** Under event-position
false-extension an unbounded `always` is not admitted. ADR-014 TR-3 refuses
"a missing interval under a bounded profile", and S3 does so in
`checked_operators`: every temporal operator without an interval under a
bounded reading pushes `TemporalCause::MissingInterval`. The only `always`
under false-extension is therefore `always[0,u]`. For it, FR-125-AC-2 does
fix the closed-boundary reading: `always[0,5] holds(c.value <= 3)` is false
under false-extension when the behaviour ends at position 3. The question of
how an unbounded `always` reads past the end does not arise.

Examined, clean:

- **Distinctness that ignores `h<i>` is sound.** The step case asserts the
  property at positions `0..depth`, and that property excludes `h<p>`, so
  those copies are live. Only copy `depth` can have ended. Take a reachable
  failure at distance `d > k`. Its shortest path has `d` distinct live
  states, so the state space has more than `k` values, and the ended copy
  can take a value that differs from the `k` live copies of the window. If
  `d <= k`, the base case covers the failure. An unsat base and step
  therefore still imply that no failure is reachable.
- **The `always[0,u]` violation order is deterministic.** `h<p>` is its own
  disjunct, followed by `(not I)` per instance, with no `(or h<p> ...)`. The
  folding rules (no disjunct past `min(depth, u)`, none for a failure kind
  with no term, `false` for an empty negated property) leave each script
  with one printing. `eventually[0,u]` is unchanged.
- **The internal-fault rule keeps the refusal-code set closed.** The named
  invariants are ones S3 and FR-120 guarantee, and a broken invariant is not
  read as `item-form`.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | high | TP-1 and TP-3 items under event-position false-extension cannot exist, yet the PR encodes them. ADR-018's property-form table gives TP-1 and TP-3 the infinite-trace profile, and makes "any bounded-profile clause" TP-2. FR-123 classifies `ReachableInvariant` only for `always holds(I)` with no interval (`is_reachable_invariant` requires `interval: None`). Under false-extension every temporal operator must carry an interval: ADR-014 TR-3, enforced by S3's `MissingInterval`. FR-123 also tries `BoundedMltl` before `Safety`, so no false-extension clause can be TP-3. The new "TP-1 under false-extension", "TP-3 under false-extension" and "Induction under false-extension" bullets, the rule 4b widening, AC-29 and AC-30 therefore specify inputs that a well-formed request (as the PR's own Internal faults section defines it) never carries. #1287 would have to build encodings no request reaches, and AC-29 and AC-30 can only be met with ill-formed items. Fix: add the invariant "a TP-1, TP-3 or deadlock-freedom item binds infinite-trace" to Internal faults, so that breaking it is `InternalFault`. Remove the TP-1/TP-3 false-extension encoding and the TP-1/TP-3 part of rule 4b. Turn AC-29 and AC-30 into `InternalFault` vectors, or drop them. Keep `h<i>` for TP-2 `on origin` only. | spec/functional/FR-315-encode-the-smt-lib-transition-relation.md:166-169, :276-292, :355-374, :881-882; spec/functional/FR-123-check-fairness-and-interval-operators-of-infinite-trace-clauses.md:120-128; spec/decisions/ADR-014-temporal-trace-and-boundedness-architecture.md:172; qsl-semantics/src/check/temporal_clause.rs:739-744, :1207-1228 | wrong-requirement |
| FND-002 | medium | The internal fault carried in `SmtEncodingRefusal` has no settlement. FR-315 says "FR-127 settles a call that returns an `InternalFault` `failed`", but FR-127's only `InternalFault` row is for a replay that refuses with `InternalFault`. `SmtEncodingRefusal` is also the error of `encode_hyper_query` and `HyperProductIdentity::of`. FR-182 settles `EncodingRefused { refusal }` and `ItemIdentity(SmtEncodingRefusal)` by refusal code, and has no row for a carried fault. A coder must guess whether an encoder fault becomes `failed`, V-8 `unsupported`, or "no query". Fix: give the error two arms, a code and FR-096's `InternalFault`. State in FR-315, or by amending FR-127 and FR-182, that CG passes the fault through unchanged and that it settles `failed`, O-16 internal failure, for single-subject and HP-2 items alike. | spec/functional/FR-315-encode-the-smt-lib-transition-relation.md:131-154; spec/functional/FR-127-settle-a-model-check-verdict-as-a-terminal-record.md:136; spec/functional/FR-182-settle-a-hyper-verdict.md:89, :112, :167 | missing-requirement |
| FND-003 | low | AC-29 and AC-31 expect `inc`'s postcondition failure `(and (not h<p>) F)` at every position. FR-315 prints a postcondition failure only when a clause holds a partial construct. So the ACs assume that `self.value = pre(self.value) + 1` carries a range coercion into `Int[0, 3]`, as TC-1084's equality-typing rule gives. Neither FR-315 nor the `Counter` fixture's definition says so. Fix: state in AC-31 that `Inc` carries that range coercion, citing TC-1084/FR-104-AC-12, so that the expected disjunct is grounded. | spec/functional/FR-315-encode-the-smt-lib-transition-relation.md:881-883 | correct-requirement-no-evidence |

## Verdict

Not mergeable. The violation-order change, the folding rules and the
internal-fault rule are sound and deterministic. The distinctness argument
holds, but it is moot.

The main change is FND-001. It gives an encoding for TP-1 and TP-3 items
under false-extension, and that combination is refused at S3 (ADR-014 TR-3)
and cannot be classified by FR-123. The fix is to make it a named invariant
whose violation is `InternalFault`, not a new encoding. Because of the same
finding, the owner question has a sourced answer and needs no ruling: an
unbounded `always` is not admitted under false-extension, and FR-125-AC-2
fixes `always[0,u]`'s closed-boundary reading.

FND-002 leaves the internal fault without a settlement row. Fix FND-001 and
FND-002 in this PR; FND-003 may be deferred. For #1287: do not encode a TP-1
or TP-3 item under false-extension; treat it as an internal fault.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-004 | medium | When `HyperProductIdentity::of` faults, CG cannot hand any outcome to QSL, so the HP-2 fault rule never runs. FR-182's `HyperSmtOutcome.product` is a required `HyperProductIdentity`. FR-315's Outcome handoff makes it the queries' `product`, or `HyperProductIdentity::of` over the same request when `encode_hyper_query` refused. `of` is deterministic, so whenever `settle_hyper_smt`'s own `of(item)` returns `Fault`, CG's `of` returned `Fault` too. CG then has no product to put in an outcome. The new rule in FR-182, "when `of` returns `SmtEncodingRefusal::Fault` it SHALL write a `failed` record", and FR-315-AC-30's vector can therefore not be reached through `settle_hyper_smt`. The same holds for an `encode_hyper_query` fault raised in its first four (identity) checks. Fix: let the outcome carry no product for a fault, for example `product: Result<HyperProductIdentity, InternalFault>`, or add a settlement entry that takes the item and the fault. State in Outcome handoff what CG returns in that case. | spec/functional/FR-315-encode-the-smt-lib-transition-relation.md:177-180, :847-853; spec/functional/FR-182-settle-a-hyper-verdict.md:120-123, :199-206 |

## Dispositions

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | The TP-1/TP-3/Induction false-extension encodings and the widening of rule 4b are removed, and `h<i>` and `(not h0)` are for TP-2 only. The invariant "a TP-1, TP-3 or deadlock-freedom item binds infinite-trace" is added, citing FR-123, ADR-014 TR-3 and ADR-018 §1. AC-29 is now a `Fault` vector, and the owner question is withdrawn. |
| FND-002 | fixed | `SmtEncodingRefusal` now has the two arms `Refused{code, locus}` and `Fault(InternalFault)` for every call. A fault settles `failed`/`Failed`. FR-314 gains `SmtProofCheck::Fault`, FR-127 gains a row for it, and FR-182 gains rows for `EncodingRefused`(`Fault`) and for `of` returning `Fault`. These rows agree with FR-127's replay `InternalFault` row and FR-182's `BackendFault` row. The HP-2 case where `of` faults has no outcome, which raises FND-004. |
| FND-003 | fixed | AC-31 now grounds `inc`'s failure term: `pre(self.value) + 1` compared by `=` with an `Int[0, 3]` field is coerced by FR-104's equality typing (TC-1084). |
