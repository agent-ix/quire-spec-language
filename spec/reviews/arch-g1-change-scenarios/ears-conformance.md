---
id: SR-506
title: "EARS conformance analysis of ADR-010 to ADR-013 (ARCH-G1)"
type: SpecReview
analysis: ears-conformance
scope: "spec/decisions/ADR-010-observed-architecture-baseline.md, spec/decisions/ADR-011-stage-dag-and-dependency-architecture.md, spec/decisions/ADR-012-semantic-family-extension-contracts.md, spec/decisions/ADR-013-canonical-type-package-conversion-ownership.md"
review_set: all
relationships:
  - target: ix://agent-ix/quire-spec-language/ADR-011
    type: reviews
  - target: ix://agent-ix/quire-spec-language/ADR-012
    type: reviews
  - target: ix://agent-ix/quire-spec-language/ADR-013
    type: reviews
---
# SR-506: EARS conformance analysis of ADR-010 to ADR-013 (ARCH-G1)

## Summary

Reviewed QSL `457a131` (branch `task/212-arch-g1-gate`) plus the uncommitted
ADR-011 edits that SR-498 records. The four ADRs were read together as one
`/spec-review all` subject. Line anchors are to that working tree.

The records are ADRs, not FR, NFR or StR, so the EARS scope is narrow. Per
record:

- ADR-010: not applicable. It is an observed baseline. Its three `must` uses
  (ADR-010:223, 229, 679) quote observed code or ticket text. They are not
  requirements.
- ADR-011: applicable. It has one `SHALL` statement, written twice: Decision 8
  (ADR-011:135) and §2.3 (ADR-011:375). It also has three indicative `must`
  uses (ADR-011:480, 545, 886) and the `Must not` header of §10.
- ADR-012: no `SHALL`. Its `must` uses (ADR-012:384, 402) are a table header
  and a Rust fact. Applicable only for statements FR-057 must carry forward.
- ADR-013: no normative modal. Applicable only for statements FR-057 must carry
  forward.

`quire validate --strict --summary` over the four ADRs and FR-057 reports
5/5 docs grammar-clean and 0 grammar findings. The engine does not see the
semantic defects below.

The new ADR-011 rule 8 statement is the only `SHALL` in scope. It has no
acting subject, and "independently" is not defined. FB-10 contradicts it:
FB-10 forbids any shared helper, while §2.3 rule 3 allows a shared helper
whose run mutation fails the proof (FND-001). The rule 8 counting sentence
is ambiguous (FND-003). Nothing publishes the list of shared helpers
(FND-004). Five normative statements in ADR-011 and ADR-012 are not yet
carried by FR-057 or conflict with it (FND-007 to FND-010).

Verdict: BLOCK. FND-001 is the only blocking finding, and it is an editorial
fix. The other findings are not blocking.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | Blocking. FB-10 forbids a proof gate "only propositions whose expectation shares a helper with the code under proof". §2.3 rule 3 and Decision 8 allow a shared helper when a run mutation of it fails the proof. SR-498 scenario 1 relies on the §2.3 reading. A gate that meets §2.3 is still an FB-10 bypass, so the two rules change what gets built. Fix FB-10 column 2 to: "A proof gate claims a module over which it discharges no SUCCESS check, or shares a helper between its expectation and the code under proof without a run mutation of that helper that fails the proof". | ADR-011:423, ADR-011:390-391, ADR-011:135-139 |
| FND-002 | medium | The only `SHALL` in scope, "The proof's expectation SHALL be derived independently of the function under proof", is passive. It names no acting system, which breaks the EARS ubiquitous form "The <system> shall <response>". "Independently" has no test: rule 3 tests shared helpers, not independence. Fix both copies to: "Each counted proof gate SHALL compute its expectation without calling the function under proof." Rule 3 then covers shared helpers. | ADR-011:135-136, ADR-011:375-376 |
| FND-003 | medium | "The floor counts SUCCESS checks only; UNREACHABLE checks are subtracted and never counted as discharged." If the count holds SUCCESS checks only, there is nothing to subtract from. A reader can subtract UNREACHABLE checks from the SUCCESS count a second time. Fix both copies to: "A discharged check is a check whose status is SUCCESS. A check whose status is UNREACHABLE is not discharged and is not counted." | ADR-011:139-141, ADR-011:385-387 |
| FND-004 | medium | Rule 3 says "For each helper that the oracle or expectation shares with the code under proof". Nothing lists those helpers, so a gate that names none passes rule 3 vacuously. §2.3 already makes each gate publish its claimed modules. Fix: after "Each counted gate publishes a checked-in list of the modules it claims." add "It also publishes a checked-in list of the helpers its expectation shares with the code under proof. An empty list is stated as empty." | ADR-011:381-382, ADR-011:390-391 |
| FND-005 | low | One rule uses three names for its target: "the function under proof", "the code under proof" and "the code it claims". It uses two for the check side: "oracle" and "expectation". Fix: say "expectation" and "the function under proof" throughout §2.3 rule 8, and keep "the modules it claims" for the claimed-module list. | ADR-011:135-139, ADR-011:375-377, ADR-011:390 |
| FND-006 | low | Rule 8 is stated five times: Decision 8, the §1.1 skeleton bullet, §2.3, the §9 agent-ix/quire-contract-runtime#53 row and Consequences. Only Decision 8 and §2.3 carry the full rule, so the copies can drift. Fix: keep the full text in §2.3, and cite "§2.3 rule 8" in the skeleton bullet, the agent-ix/quire-contract-runtime#53 row and Consequences. | ADR-011:135-143, ADR-011:221-225, ADR-011:800, ADR-011:942-946 |
| FND-007 | medium | Carry-forward. ADR-011 E3 and ADR-012 §13.5 key each `capability_report` entry by the item's checked identity. The entry holds kind, extent and bound, and never a backend, candidate or disposition. FR-057 keeps the pair by request index, with its `required` flag, in the FR-036 handoff. Candidate sets and accounting join on `request_index` (ADR-012:495, 574). No record states how the checked identity maps to `request_index`. The FR-057 re-homing (SR-498 FND-002) needs one statement. For example: "S3 SHALL record one `capability_report` entry per checked item that has `Requirements`, keyed by checked node id." | ADR-011:320, ADR-011:868-870, ADR-012:863, FR-057:196-198 |
| FND-008 | medium | Carry-forward. ADR-012 §5.2 says a second registration with the same `BackendId` makes "registry construction" refuse. FR-057 refuses only that registration and keeps the earlier one. These are two different behaviours for #185. SR-498 FND-007 records the same conflict. Fix ADR-012:424 to: "the second registration refuses with `invalid_capability`/`duplicate-backend`, naming the identity; the registration already held stands (FR-057)". | ADR-012:424, FR-057:214-218 |
| FND-009 | medium | Carry-forward. ADR-012 §7.2 refuses the whole run when a registered `BackendId` has no CG kind. FR-057 settles that case per item as `invalid-request`, `invalid_capability`/`unknown-backend`. This is SR-498 MD-2. It is a missing decision, not an editorial fix. It blocks the FR-057 re-homing, but not this analysis. | ADR-012:547-551, FR-057:239 |
| FND-010 | low | Carry-forward. ADR-012 §6 says a checked package is "the same bytes whatever backends are registered". FR-057-AC-5 asserts only "identical admitted pairs and static meaning". SR-498 scenario 7 tests byte identity. Fix: when FR-057 is re-homed, state AC-5 as "the checked package bytes are identical whatever backends are registered". | ADR-012:470-471, FR-057 AC-5 |
| FND-011 | low | Carry-forward. When FR-057 is re-homed to S3, `route` and CG (ADR-012 §14.1), several of its statements join two or three `SHALL`s: FR-057:150-151, 220-221, 248-249 and 281-282. The engine allows these, but each re-homed statement names a new actor. Split them into one `SHALL` per statement during the `/specify` pass. | FR-057:150-151, FR-057:220-221, FR-057:248-249, FR-057:281-282 |

## Round 2

- FND-001 is resolved: FB-10 now reads "discharges no SUCCESS check, or
  shares a helper between its expectation and the code under proof without a
  run mutation of that helper that fails the proof".
- FND-003 is resolved: the counting sentence says no status but SUCCESS is
  counted.
- FND-004 is resolved: §2.3 requires a published shared-helper list, and an
  empty list is stated as empty.
- FND-002 stays open: the SHALL keeps the #245 wording the coordinator ruled.

Verdict after Round 2: ACCEPT WITH FINDINGS.

## Round 3

Dispositions after the #212 rulings, 2026-09-19 (issue #212, newest two comments).

- FND-002 is resolved: the SHALL keeps the #245 wording the owner ruled.
- FND-005 and FND-006 are Remaining work: #247.
- FND-007, FND-008, FND-010 and FND-011 are Remaining work: #185, in the
  FR-057 re-homing `/specify` pass.
- FND-009 is resolved (#212 rulings, 2026-09-19, MD-2): the run-level refusal carries
  `invalid_capability`/`unknown-backend`, and FR-290's per-item row stays
  as CG's guard.
- FND-001, FND-003 and FND-004 were resolved in Round 2.

Verdict after Round 3: ACCEPT WITH FINDINGS.

## Round 4

Dispositions after the #212 round-2 rulings, 2026-09-19 (issue #212, round-2
comment), and the #247 editorial alignments.

- FND-005 is resolved (#247): ADR-011 §2.3 rule 8 says "expectation" and
  "the function under proof" throughout, and "the modules it claims" for the
  claimed-module list.
- FND-006 is closed as satisfied (#247 ruling). No text change.
- FND-007 and FND-008 are resolved (#212 round-2 rulings): ADR-011 E3 and
  ADR-012 §13.5 key each `capability_report` entry by occurrence key with
  `request_index` as their bytewise order, and a duplicate `BackendId`
  registration follows FR-057 (the existing registration stands).
- FND-010 and FND-011 stay Remaining work: #185.

Verdict after Round 4: ACCEPT WITH FINDINGS.

## Round 6

Dispositions after the #212 round-4 rulings, 2026-09-19 (issue #212, round-4
comment).

- FND-007 and FND-008 are resolved, as recorded in Round 4.
- FND-010 and FND-011 stay Remaining work: #185.

Verdict after Round 6: ACCEPT WITH FINDINGS.
