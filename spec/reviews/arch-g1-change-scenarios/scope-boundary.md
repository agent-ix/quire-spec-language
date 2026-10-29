---
id: SR-505
title: "Scope-boundary analysis of ADR-010 to ADR-013 (ARCH-G1)"
type: SpecReview
analysis: scope-boundary
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
# SR-505: Scope-boundary analysis of ADR-010 to ADR-013 (ARCH-G1)

## Summary

Reviewed state: QSL `origin/main` 457a131 on branch `task/212-arch-g1-gate`,
plus the uncommitted ADR-011 edits of this PR (#245 rule 8 and the ADR-013
O-03 alignment). The four records were read together. Line anchors are to the
working tree. `ADR-011:134` means line 134 of
`spec/decisions/ADR-011-stage-dag-and-dependency-architecture.md`.

The gate record SR-498 already lists MD-1, MD-2 and its FND-001 to FND-007.
This review does not repeat them. It asks one question: does each
responsibility have exactly one owner, across the three Layer 1 records (#209
stage DAG, #210 families and capability, #211 types and conversions) and
across QSL and the other repositories (IR, RT, CG, QSpec, FCD)?

What holds:

| Boundary | Owner | Assumed or guaranteed | Contract |
|---|---|---|---|
| FCD → QSL | QSL `model::intake`, the only translation point | guaranteed | C-01 intake test over a pinned FCD fixture (ADR-013:677) |
| QSL → IR | QSL `package` emits v2. IR reads it. | guaranteed | FR-322; C-03, C-04 (ADR-013:679-680) |
| IR → CG, RT | layer-owned enums with total maps | guaranteed | C-05, C-06, C-20 (ADR-013:681-696) |
| QSL registry → CG | data only, no crate edge | guaranteed | T-7, C-28, C-29 (ADR-013:666, 704-705) |
| CG → QSL | the layer-6 `replay` facade only | guaranteed | FB-05, T-12 (ADR-011:418, 928) |
| Wire authoring | QSpec | assumed until TK-06 to TK-10 land | QC-1 to QC-20 (ADR-013:809-830) |

- The stage owners in ADR-011 §1 (ADR-011:177-188), the family set in ADR-012
  §1 (ADR-012:120-127) and the object owners in ADR-013 §3 agree on every
  stage. No stage has two owners.
- Each record cites the other two for the decisions it does not own. ADR-012
  §13.1 and §13.2 (ADR-012:814-832) and ADR-013 Q209 and Q210
  (ADR-013:832-852) are answered and applied.
- ADR-010's routing (ADR-010:790-812) puts every finding with one Layer 1
  ticket. The three records close their own items.

What does not hold:

- The routing step is placed in two components (FND-001). The removal of
  composed negotiation and of the QSL `negotiate_*` copies has two owners
  (FND-002). ADR-011 and ADR-012 disagree on the fate of `--target`
  (FND-003).
- The orchestrating binary has duties in all three records but no
  implementing ticket (FND-004).
- The #245 rule this PR adds to ADR-011 has no counterpart in ADR-013's kernel
  consumer rule or in the CG gate ticket T-10 (FND-005).
- Six low findings are stale cells or mismatched owner names.

Verdict: **ACCEPT WITH FINDINGS.** No finding is blocking. None makes a
scenario cell, a pass-condition verdict or a missing decision in SR-498
wrong. FND-005 is the closest. SR-498 scenario 1 cites agent-ix/quire-contract-codegen#88 as "T-10 harness
gate: claimed modules, `unreached`, mutation control", which is accurate for
T-10 as written. The gap is that T-10 was not widened with the #245 rule.
It can be fixed in this PR by the text in FND-005.

## Findings

| ID | Severity | Summary | Refs |
|---|---|---|---|
| FND-001 | medium | The routing step has two owners. ADR-011 puts "the candidate step and the routing step" in QSL `route`, before E7 (ADR-011:281-284). ADR-012 step 4 routes each `supported` item to its generation (ADR-012:532-534). A disposition exists only after CG `negotiate_*` settles it at E7. `route` runs before E7, and no QSL module calls CG. So `route` cannot route a `supported` item. In practice CG's S9 generation arm does it (ADR-012:394, 796). ADR-012 §7.1 still calls the registry the party that "routes each `supported` item" (ADR-012:486). FB-12 checks "#185 alone owns registry/routing" at #216 (ADR-011:425). **Fix:** say that `route` computes candidate sets only. Dispatch of a `supported` item to generation is the CG S9 arm over the one candidate. Reword ADR-012 Decision 6, §7.1 and §7.2 step 4, and the FB-12 evidence text, to match. | ADR-011:76, 281-284, 425; ADR-012:100-104, 486, 532-534, 394, 796 |
| FND-002 | medium | Removal of composed negotiation and of the QSL `negotiate_*` copies has two owners. ADR-011 SEAM-2 says "`requests` backend disposition leaves QSL" under M-6e with #214, #220, #221 and #223 (ADR-011:608). ADR-012 gives the same removal to #185 (ADR-012:683, 879). ADR-013 §7 limits #185 to "Capability registry and routing only" (ADR-013:776). The `value::division` and `value::ieee` `negotiate_*` copies map to "RT" with no owning change (ADR-011:636). ADR-012 gives their removal to #185 (ADR-012:684, 879). But `division` and `ieee` move into `quire-exact` with X-1 (ADR-011:634), and X-1 lands before #185. Either X-1 carries the copies into the kernel, or X-1 removes them. **Fix:** name one owner per removal. For example: X-1 (#213 S-1) drops the `negotiate_*` copies when it moves `division` and `ieee`; #185 rewrites `requests::report` to record data; M-6e deletes what is left of SEAM-2. Drop "only" from ADR-013:776, or move the removals out of #185. | ADR-011:608, 634, 636; ADR-012:683-684, 879; ADR-013:776 |
| FND-003 | medium | ADR-011 and ADR-012 disagree on `--target`. ADR-011 says `lowering::target` "is replaced by the #185 registry in `route`" (ADR-011:631, 849). ADR-012 §9 says its three values "are QSL lowering domains, not backends, and stay a closed QSL enum beside the registry", owned by "QSL lowering" (ADR-012:670). ADR-011 retires `lowering` with SEAM-1 (ADR-011:607). So ADR-012 keeps an enum in a module ADR-011 deletes, and ADR-011 replaces an enum ADR-012 says is not a backend choice. **Fix:** decide once, in ADR-011 (module placement is #209's). If the lowering domains survive, name their target module. If they do not, ADR-012 §9 says `ProjectionTarget` is deleted with SEAM-1, and backend choice is the `BackendId` argument only. | ADR-011:607, 631, 849; ADR-012:670 |
| FND-004 | medium | The orchestrating binary (the driver) has duties in all three records and no implementing ticket. It reads provider manifests and builds the registry value (ADR-011:290, ADR-012:497-499). It calls `route` and CG (ADR-012:503-507). It converts every `BackendId` to a CG kind before negotiation (ADR-012:547-551, the MD-2 site). It resolves the backend CLI argument by registry lookup (ADR-012:426, 670). ADR-011 gives only its placement to #225 (ADR-011:525, 884-886). No row of the ADR-011 implementing-tickets table owns it (ADR-011:72-93). ADR-012 also runs its disposition test "in the test harness downstream of CG" (ADR-012:454-457, 920-921). That home is neither CG nor QI `heads/`, the two homes ADR-011 §7.1 allows (ADR-011:698-701). MD-2's proposed ruling adds a refusal to this unowned component. **Fix:** name the implementing ticket for the driver (for example #232 with #225, or a T-row), and say that the ADR-012 §5.3 harness is that driver's test suite or CG's. | ADR-011:72-93, 290, 525, 698-701, 884-886; ADR-012:426, 454-457, 497-507, 547-551, 670, 920-921 |
| FND-005 | medium | The #245 rule this PR adds has no counterpart in ADR-013 or in the CG gate ticket. Decision 8 and §2.3 rule 3 require the proof expectation to be independent of the function under proof, and a run mutation of each shared helper to fail the proof (ADR-011:134-143, 375-391). The oracle is CG's (ADR-012:277). ADR-013 O-13 lists "CG oracles" as a consumer of `quire-exact` with no limit (ADR-013:294), and TK-03 is the CG adoption (ADR-013:890). For a kernel operation under proof, that makes shared kernel helpers the default. Neither ADR-013 row mentions the rule. T-10, the CG harness-gate ticket, still reads "claimed-module list, `unreached` failure, mutation control" (ADR-011:926). It omits the SUCCESS-only floor and the shared-helper mutation. Decision 8 is also stated for every proof gate, while §2.3 limits it to the #205 gates and sends the wider rule to QSpec (ADR-011:368-370, 400-404, 927). **Fix, editorial, this PR:** ADR-011:926 T-10 reads "CG generated-harness gate: claimed-module list, `unreached` failure, SUCCESS-only discharge floor, mutation control, and a run mutation of each helper the oracle shares with the code under proof (§2.3, #245)". ADR-011:134 begins "A proof gate counted by #205 (§2.3) discharges …". ADR-013 O-13 Owner cell adds "A CG oracle for a kernel operation under proof meets ADR-011 §2.3 rule 3." | ADR-011:134-143, 368-391, 400-404, 926-927; ADR-012:277; ADR-013:294, 890 |
| FND-006 | low | The owner of a family's witness binding schema differs. ADR-012 §8 gives the schema to CG (ADR-012:612). ADR-012 §12.1 and §12.2 place the schema change in `IR:src/kani/witness.rs` (ADR-012:750, 779). ADR-013 puts `decode` over the bindings in IR and reconstruction in CG (ADR-013:606-610, 687). **Fix:** name one owner for the schema and one path in all three rows. | ADR-012:612, 750, 779; ADR-013:606-610, 687 |
| FND-007 | low | ADR-012 misstates ADR-013's answer on the shared refusal part. ADR-012 says "the shared part is the kernel `Refusal`" (ADR-012:214, 829). ADR-013 O-17 says the shared part is `RefusalRecord` in QSL F `diagnostic`, and the kernel `Refusal` carries only kernel causes (ADR-013:403-413). **Fix:** cite `RefusalRecord` in F `diagnostic` in both ADR-012 cells. | ADR-012:214, 829; ADR-013:403-413 |
| FND-008 | low | ADR-011 §10 row 7 omits CG and reads as if the backend negotiates. Its "Other repositories" cell names only the backend repository and QSpec, and its stage cell says the backend "settles dispositions in `negotiate_*`" (ADR-011:817). ADR-012 puts one backend kind variant, its `negotiate_*` arm and its generation arm in CG, also for a backend outside CG (ADR-012:543-547, 796). SR-498 scenario 7 already records the CG owner, so the gate is not affected. **Fix:** add "CG (backend kind, `negotiate_*` arm and generation arm, seam S9)" to the row, and say CG settles the dispositions. | ADR-011:817; ADR-012:543-547, 796 |
| FND-009 | low | The repository that lands the `replay` facade differs. ADR-011 T-2 gives the skeleton spine, which lands the QSL layer-6 `replay` module, to "CG, with QSL M-4" (ADR-011:918, 591, 651). ADR-013 TK-01 gives the same module to "QSL, spine then #214" (ADR-013:888). **Fix:** make T-2's owner "CG and QSL: QSL lands `replay` (TK-01), CG lands the adapter", or split T-2. | ADR-011:591, 651, 918; ADR-013:888 |
| FND-010 | low | ADR-012 never names the `replay` facade as the executor surface. Its §8 Replay row says the `evaluate` hook is "reached through the QSL complete-V1 executor" (ADR-012:613). §12.3 says replay goes "through the existing QSL `evaluate` hooks" (ADR-012:799). ADR-011 makes the layer-6 `replay` facade the only CG-facing surface (ADR-011:418, 584-591). **Fix:** cite the ADR-011 layer-6 `replay` facade in both ADR-012 cells. | ADR-011:418, 584-591; ADR-012:613, 799 |
| FND-011 | low | Stale owner and path cells. ADR-013 §9 says DA-04 "`value::expression::CheckedPackage` canonical for S4" and DA-08 "one checked clause kind in `value::expression`" (ADR-013:908, 912). ADR-013's own T-1 and O-10 place them in layer-4 `package` and the layer-3 `check` core (ADR-013:244, 660; ADR-011:470). ADR-011 still names the S4 type `value::expression::CheckedPackage` (ADR-011:107, 236). ADR-013 O-08 gives frame semantics to "the frame family (decided in #210)" (ADR-013:220), but ADR-012 has no frame family: frames belong to `ProtocolClause` (ADR-012:126). **Fix:** align the four cells to T-1, O-10 and ADR-012 §1. | ADR-011:107, 236, 470; ADR-012:126; ADR-013:220, 244, 660, 908, 912 |

## Round 3

Dispositions after the #212 rulings, 2026-09-19 (issue #212, newest two comments).

- FND-001 is resolved (#212 rulings, 2026-09-19, SR-505 FND-001): `route` computes candidates and
  routes the settled items, and CG `negotiate_*` settles. ADR-011 §2.1 and
  ADR-012 §7.1 and §7.2 say so.
- FND-002: the ADR-011 side is resolved (#212 rulings, 2026-09-19, SR-502 FND-002), since M-6e names
  the family implementation tickets. The ADR-012 #185 claim is Remaining
  work: #247.
- FND-003 is Remaining work: #185.
- FND-004 is Remaining work: #225.
- FND-005 is resolved (#212 rulings, 2026-09-19, SR-500 FND-001): ADR-013 O-13 carries rule 8 for
  CG oracles. ADR-011 T-10 already carried it after Round 2.
- FND-006 is Remaining work: #231.
- FND-007, FND-008, FND-009 and FND-010 are Remaining work: #247.
- FND-011 is resolved (#212 rulings, 2026-09-19, FND-010 and FND-014): ADR-011 names the S4 type
  `CheckedPackage` in layer-4 `package`, and ADR-013 DA-04 and DA-08 match.

Verdict after Round 3: ACCEPT WITH FINDINGS.

## Round 4

Dispositions after the #212 round-2 rulings, 2026-09-19 (issue #212, round-2
comment), and the #247 editorial alignments.

- FND-002 is resolved (#247): #185 removes the `requests` backend
  disposition from QSL, and #213 S-1 removes the QSL `negotiate_*` copies
  (ADR-011 §6.2, ADR-012 §14.1, ADR-013 §7).
- FND-003 is resolved (#212 round-2 ruling): `ProjectionTarget` and
  `--target` are deleted with SEAM-1, and the backend is chosen only by
  `BackendId`.
- FND-004 is resolved (#212 round-2 ruling): the orchestrating driver crate
  is ADR-011 T-13, implemented by #248. #225 accepts its design.
- FND-006 is resolved (#212 round-2 ruling): the #231 envelopes live in the
  layer-6 `replay` public API.
- FND-007 is resolved (#247): ADR-012 names `RefusalRecord` in F
  `diagnostic` as the shared part.
- FND-008 is resolved (#247): ADR-011 §10 row 7 names CG, whose
  `negotiate_*` arm settles every disposition.
- FND-009 is resolved (#247): T-2 names QSL #243 for the `replay` facade and
  agent-ix/quire-contract-codegen#87 for the replay adapter, matching
  ADR-013 TK-01.
- FND-010 is resolved (#247): ADR-012 §8 and §12.3 cite the layer-6
  `replay` facade.

Verdict after Round 4: ACCEPT.
