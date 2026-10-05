---
id: SR-1324
title: "Spec review of quire-spec-language PR #642: ADR-029 PV-1/PV-4/CA-2 and FR-288 against the QSL-637 rulings"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@3ce34b67ac1653e193e7714dc652b5d229406429; spec/decisions/ADR-029-lifecycle-cli-provider-plugin-cache-codegen-boundaries.md (PV-1, PV-4, CA-2, Amendments); spec/functional/FR-288-*.md; spec/test-cases/TC-772-*.md; spec/tests.md; context: agent-ix/quire-specification@2b2dd28 spec/objects/protocol/FR-290-protocol-claim-kind.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/ADR-029
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-288
    type: reviews
---
## Summary

Ticket: QSL-637. Rulings judged as decided (Q1, Q2, Q3, Q5, Q6 and the
ProviderOrigin amendment, option b); this review checks that the PR states
them correctly.

- **PV-1** — origin typed, owned by layer R, set at the conversion, never a
  manifest member; origin-differing pair conflicts, both withdrawn, each
  digest recorded, order-independent, identical idempotent; built-in `kani`
  vs plugin `kani`: neither wins. Matches Q6 and the amendment.
- **PV-4 item 2** — `Process(BackendId)`, `from_identity` unchanged, Process
  only from origin, Linked unknown identity is `UnknownBackend`, no inference
  from identity or executable text. Matches Q2 as amended.
- **PV-4 item 3** — matches Q3 (empty `KindOutput::Process`, no adapter, no
  terminal, PL-7, no panic). Q3's "or an unreachable!" is dropped from the
  ADR's "never a panic"; `unreachable!` is a panic, so nothing is lost.
- **PV-4 item 4** — matches Q5 word for word.
- **PV-4 item 5** — option (b) with the reasons against (a) and (c). Matches.
- **Amendments bullet** — rewritten from "the driver maps every plugin
  BackendId" to settlement by origin. Matches the amendment.
- **CA-2** — the cache-key member change is QSL-636's (PR body says so); it
  reads consistently and is outside these rulings.
- **FR-288, TC-772, spec/tests.md** — consistent with each other; AC ownership
  is SR-1323's FND-001.

## Verdict

Changes requested, spec text only. The rulings are stated faithfully. The
ticket's own Q1 work item (a QSpec FR-290 decline cause for an unadvertised
domain or bound) is missing and deferred without a ticket, so PV-4 cites a
decline that FR-290 does not define. PV-4 item 1 is ambiguous about which
descriptor carries domains and bounds.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | PV-4 items 1 and 4 say an item "declines under FR-290" when the manifest does not advertise its domain, bound or unbounded mode. QSpec FR-290 at quire-specification 2b2dd28 has no such decline cause, and its advertised-mode table gives requires-bound (not a decline) for an unbounded item on a bounded-only candidate with a finite bound. QSL-637 Q1 says FR-290 "gains one in this ticket" if missing; the PR body lists it as "Not in this PR" with no follow-up ticket. Fix: open (or name) the QSpec ticket that adds the cause, and cite it in PV-4 as pending, so CG IR-629 does not implement a cause that does not exist. | spec/decisions/ADR-029-lifecycle-cli-provider-plugin-cache-codegen-boundaries.md:476-500 |
| FND-002 | medium | PV-4 item 1 says "The descriptor CG receives carries the manifest's advertised domains and bounds". The descriptor PV-1 says reaches consumers is qsl_route::BackendDescriptor, which holds only identity, origin, digest and (kind, mode) pairs. Q1 means CG's own descriptor for the Process variant. As written, a reader can take it as a requirement on BackendDescriptor, and nothing says who fills domains and bounds. Fix: say "CG's descriptor for the Process variant" and name its source (the driver, from the FR-331 manifest the BackendDescriptor came from). | spec/decisions/ADR-029-lifecycle-cli-provider-plugin-cache-codegen-boundaries.md:478-480 |
| FND-003 | low | FR-288 Description still says "A BackendDescriptor holds the BackendId and the manifest's advertised capabilities, and no other identity of the tool", which omits the origin member the PR adds. Fix: add "and its ProviderOrigin". | spec/functional/FR-288-build-the-registry-from-provider-manifests.md:41-42 |
