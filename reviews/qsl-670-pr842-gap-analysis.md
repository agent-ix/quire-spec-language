---
id: SR-9120
title: "Gap analysis of quire-spec-language PR #842: chain-link model identity"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@2719844b3826b23067e57f51d811dfed7e36717d; FR-081-AC-9..AC-13, NFR-012, TC-1022..TC-1025, FR-084-AC-7/TC-410, TC-434; QSpec FR-150-AC-12..16, TC-195, TC-196, TC-198 (ids only)"
review_set: subset
---
# Gap analysis of quire-spec-language PR #842

## Summary

Ticket: QSL-670. Diff-scoped, planless check of each criterion against the
tests that claim it, with a judgement of where each expected value comes
from (oracle versus regenerated from the implementation).

Examined:
- FR-081-AC-9 (TC-1022): `links_expand_to_independent_enumeration` compares
  the production expansion of every declaration of a chain, a diamond and a
  hidden-redefinition fixture with a test-local enumeration over the original
  declarations, and the inherit paths with a second test-local walk over the
  links; `the_strict_reader_refuses_every_mis_wired_link` covers wrong kind,
  wrong original, wrong owner, repeated parent, missing parent, a link on a
  qualify fact and an unlinked inherit fact (met)
- FR-081-AC-10 (TC-1023): `identity_binds_ancestry_and_is_unique` rebuilds
  with the root key changed, a middle edge rewired, the redefinition target
  and a subset target changed, asserting the exact changed set and that no
  identity or preimage is shared (met)
- FR-081-AC-11 (TC-1024): `cost_follows_closed_formulas` checks chains,
  a 500-layer ladder and a 1,000-type dense shape against a hashed-bytes
  formula built from the TC-1024 byte shapes, independent of the encoder;
  `the_formula_reproduces_the_ten_thousand_edge_figure` ties that formula to
  the published 15,104,845; `n13_counters_equal_the_published_figures` pins
  the three published N13 triples; N12 pins 13 / 24 / 76 (met)
- FR-081-AC-12/13 (TC-1025): `ten_thousand_edge_chain` runs at full size in
  the default test run (no `#[ignore]`), in a child process on a 512 KiB
  thread, asserting 10,001 declarations, 20,001 facts, 15,104,845 bytes,
  60,006 work units, peak RSS and wall time; the ci log shows it passing
  (met; measured in the test profile, not release on Terra)
- Oracle versus regenerated: the type and member digests of N01/N02, both
  views and universes of N01/N02/N09, the N01 JCS lengths, charge sequence
  and three boundary records, N02 counters, the N06 retained fact shapes,
  N12 and N13 figures and N19 are copied from the published oracle, not
  computed by the code. The regenerated values are the golden universe and
  view digests under a 0x11 selection, the `model_normalization` N06 work
  units (93, 51/14, 41) and the `model_dispatch` D03 order over `model.*`
  keys; each is backed by an oracle value of the same rule elsewhere
  (N01/N09 digests; the N12 phase-4 charges, recomputed by hand for N06), so
  no semantic claim rests only on a regenerated value (examined)
- Deleted or rewritten tests: `n06_conflict_check_charges_order_by_effective_identity_not_producer_key`
  is deleted; the effective-identity refusal order stays covered by
  `n06_conflict_refusal_order_matches_effective_identity_not_producer_key`
  and the declaration-charge order by the rewritten
  `n08_declaration_charges_order_by_effective_identity_not_producer_key`.
  `n02_charges_fifteen_facts_and_six_cycle_checks` became the 13 / 4 / 40 /
  13088 test; `inherited_facts_share_their_ancestor_path_at_the_defaults`
  was replaced by a per-direct-parent link test; the `FactInputs`
  path-sharing check went with the type. No criterion lost its only test
  (examined)
- NFR-012 measurement rows (negative-abuse-testing): `n01_boundaries_deny_at_the_published_charge`
  and `fifty_parents_per_type_is_refused_at_hashed_bytes` exercise denial
  at the charge (examined)

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Verdict

**PASS.** Every new criterion is tested, and each semantic claim is pinned by
an oracle-derived value.

Recorded, not reported:
- low: the NFR-012 "Default derivation" probes (584 / 585 edges on hashed
  bytes, 1786 on the declaration ceiling) are not pinned by a test. They are
  derivation prose, not acceptance rows. The prose also says 1786 is refused
  on the effective-declaration ceiling, but `derivation_facts` is charged
  first and is what refuses it.
- low: N12's other three mutations (A's key, `D.supertypes = [B]`,
  `[B, C, A]`) are not run on the N12 fixture; TC-1023 covers the same rules
  on its own fixture.
- low: TC-196 D01/D02 dispatch work units and the D03 subtype order, and
  TC-198 L04/L09 values, are not asserted in QSL. Dispatch charges a flat
  unit (pre-existing), and the QSL D03 fixture uses `model.*` keys.
