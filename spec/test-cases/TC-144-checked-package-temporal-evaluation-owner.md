---
id: TC-144
title: "Preserve temporal evaluation ownership on the checked-package route"
type: TC
relationships:
  - { target: ix://agent-ix/quire-spec-language/FR-052, type: verifies }
---
# TC-144: Preserve temporal evaluation ownership on the checked-package route

## Description

Verify [FR-052-AC-9 through FR-052-AC-12](../functional/FR-052-publish-native-temporal-evaluation-owner.md)
on Layer-4 checked-package protocol evaluation. This successor is a behavior
test over the checked-package route and does not require the retired native
temporal request/result API.

## Test Procedure

Evaluate checked temporal protocol fixtures through the checked-package
evaluation route using the same admitted obligation and evidence as the
FR-043 reference evaluation. Include a formula whose result differs from an
individual leaf valuation; an obligation with each required evidence category
missing in turn; one unsupported or inconsistent input; an exhausted work
budget alongside a separately settled sibling; and, where the route accepts
corrections, an original result plus valid and invalid correction relations.
Attempt to supply caller-selected truth, settlement and support wherever the
route's inputs permit mutation. Compare all observable outcomes and retained
facts with FR-043/044 and the checked-package contract.

## Expected Results

The route reports the FR-043 formula-level truth, settlement and support and no
caller-authored substitute. Required facts remain independently observable;
missing facts and typed failures produce no partial truth, and a settled
sibling remains settled. Where correction relations exist, only an exact direct
predecessor for the same obligation admits, and prior result content remains
unchanged. The test records no evidence for corrections if that relation is
not present on the route.
