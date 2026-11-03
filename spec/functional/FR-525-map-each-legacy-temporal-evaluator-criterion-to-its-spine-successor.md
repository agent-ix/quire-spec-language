---
id: FR-525
title: "Map each legacy temporal evaluator criterion to its spine successor"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-033
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/FR-043
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-044
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-045
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-052
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-053
    type: references
  - target: ix://agent-ix/quire-spec-language/NFR-008
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-499
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-501
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-502
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-504
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-516
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-522
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-523
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-524
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-526
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-527
    type: depends_on
---
# FR-525: Map each legacy temporal evaluator criterion to its spine successor

## Description

This requirement names, for each acceptance criterion of the legacy temporal
evaluator (FR-043, FR-044, FR-045, FR-052, FR-053 and NFR-008), the spine
criterion that states it. FR-500 does the same for FR-042. FR-499 gates the
deletion of `src/temporal` on these rows, and this requirement supplies the
successor column that FR-499's temporal table leaves as "none".

Every legacy criterion has a row with a successor criterion. A row reads RETIRE
only where it states the structural reason and the successor criterion, and no live consumer calls the behavior.

## Semantic authority and boundary

FR-499 owns dispositions: COVERED (a tagged spine test is green on the head of
the deleting change), PARTIAL, PORT (the behavior moves under the named ticket),
KEEP and RETIRE. The tickets are QSL-1502 (FR-522, FR-523), QSL-1503 (FR-524,
FR-526, FR-327-AC-8 to AC-13) and QSL-1504 (FR-527). The successor criteria of
FR-501, FR-502 and FR-504 belong to the finite-window and progress work under
QSL-858's train.

## FR-043 criterion to successor

| Legacy criterion | Successor criteria | Disposition |
| --- | --- | --- |
| FR-043-AC-1 constant and `holds` atom not folded | FR-327-AC-1, FR-327-AC-8 | PORT QSL-1503 |
| FR-043-AC-2 three profiles, three identities | FR-501-AC-2; FR-522-AC-16 | PORT QSL-1502 |
| FR-043-AC-3 profile and clock disagreement refuse before any position | FR-522-AC-1 to AC-13 | PORT QSL-1502 |
| FR-043-AC-4 eight operators, `once[1,1]`, `until[1,2]`, duals | FR-327-AC-1, AC-2, AC-9, AC-10, AC-11, AC-12; FR-501-AC-1, AC-6, AC-7 | PORT QSL-1503 |
| FR-043-AC-5 equal coordinate without order key, mixed authority, one position keyed, insertion order not semantic | FR-501-AC-7, FR-501-AC-18, FR-501-AC-19, FR-501-AC-20, FR-501-AC-21 | PORT (FR-501) |
| FR-043-AC-6 pointwise connectives | FR-327-AC-1, FR-327-AC-13 | PORT QSL-1503 |
| FR-043-AC-7 closed-scope and open-scope basis; fact missing inside or outside support | FR-501-AC-4, AC-14; FR-504-AC-5 | PORT (FR-501, FR-504) |
| FR-043-AC-8 incomplete input applies no false extension or empty truth | FR-501-AC-5; FR-504-AC-4 | PORT (FR-501, FR-504) |
| FR-043-AC-9 history cutoff gives missing history | FR-501-AC-8; FR-504-AC-6 | PORT (FR-501, FR-504) |
| FR-043-AC-9 omitted interior history interval is incomplete, never false padding | FR-501-AC-2 and AC-5 (a stamp gap is an empty window, and an incomplete input never reads it as false); FR-504-AC-4 | PORT (FR-501, FR-504) |
| FR-043-AC-10 four independent axes; no caller basis | FR-501-AC-15; FR-516-AC-1, AC-5 | PORT (FR-501) and QSL-1545 |
| FR-043-AC-11 watermark settles a silent deadline; event-position does not | FR-501-AC-3, AC-6; FR-504-AC-1, AC-2 | PORT (FR-501, FR-504) |
| FR-043-AC-12 regressing watermark, conflicting completeness | FR-502-AC-1 to AC-8 | PORT (FR-502) |
| FR-043-AC-13 one identity per input set | FR-522-AC-15 to AC-23; FR-501-AC-16 | PORT QSL-1502 |
| FR-043-AC-14 `/2` authenticated clock configuration and report | FR-522-AC-4, AC-6, AC-9, AC-12 | PORT QSL-1502 |
| FR-043-AC-14 `/1` entry points: parameter mutations not refused, no authenticated binding | none | RETIRE with the `/1` lane (structural reason: no caller of the `/1` entry points exists; Contract IR FR-350 and quire-protocol IT-001 consume the `/2` entry points; successor FR-522-AC-9, the authenticated form) |

## FR-044 criterion to successor

| Legacy criterion | Successor criteria | Disposition |
| --- | --- | --- |
| FR-044-AC-1 distinct triggers with equal payloads, two instances; captures do not cross | FR-524-AC-1, AC-2, AC-7 | PORT QSL-1503 |
| FR-044-AC-2 duplicate delivery, conflicting payload, within a call and across calls | FR-524-AC-3, AC-5, AC-6, AC-19, AC-20 | PORT QSL-1503 |
| FR-044-AC-3 captures and the predicate valuation environment immutable | FR-524-AC-4, AC-22 | PORT QSL-1503 |
| FR-044-AC-4 capture faults named, siblings unaffected | FR-524-AC-12 to AC-17 | PORT QSL-1503 |
| FR-044-AC-5 four-way trigger-scope disposition | FR-524-AC-27 to AC-31 | PORT QSL-1503 |
| FR-044-AC-6 captures once, in order, no later read; count stays one across re-evaluation and restoration | FR-524-AC-18, AC-19, AC-23 | PORT QSL-1503 |
| FR-044-AC-7 guard false, guard unestablished | FR-524-AC-10, AC-11 | PORT QSL-1503 |
| FR-044-AC-8 one instance per execution origin | FR-524-AC-8, AC-9; FR-327-AC-2 for `on each` | PORT QSL-1503 |
| FR-044-AC-9 evicted capture, no recomputation, replay | FR-524-AC-12, AC-19, AC-21 | PORT QSL-1503 |

## FR-045, FR-052, FR-053 and NFR-008 criteria to successors

| Legacy criterion | Successor criteria | Disposition |
| --- | --- | --- |
| FR-045-AC-1 every table row reachable | FR-527-AC-1, AC-5, AC-6 | PORT QSL-1504 |
| FR-045-AC-2 finite window, mixed direction, past operators | FR-527-AC-2, AC-3 | PORT QSL-1504 |
| FR-045-AC-3 unsupported retains subject, profile, activation | FR-527-AC-8 | PORT QSL-1504 |
| FR-045-AC-4 every unmatched dimension | FR-527-AC-4 | PORT QSL-1504 |
| FR-045-AC-5 supported names target and premises | FR-527-AC-9, AC-10 | PORT QSL-1504 |
| FR-045-AC-6 authenticated selection | FR-527-AC-13 to AC-17 | PORT QSL-1504 |
| FR-045-AC-6 `/1` classification retains no selection | none | RETIRE with the `/1` lane (structural reason as above; successor FR-527-AC-13) |
| `Unmatched::TimedProfile` (exact rational time) | FR-527-AC-18: a profile outside the four admitted is refused at check | PORT QSL-1504 |
| `Operators::of` out-of-range index reaches nothing | FR-527-AC-17: the view lists only the clause's operators | PORT QSL-1504 |
| FR-052-AC-9 no supplied truth | FR-516-AC-1 | PORT QSL-1545 |
| FR-052-AC-10 independent facts, no default | FR-516-AC-3, AC-5; FR-524-AC-31 | PORT QSL-1545 |
| FR-052-AC-11 missing evidence | FR-516-AC-3 | PORT QSL-1545 |
| FR-052-AC-11 inconsistent evidence | FR-524-AC-5, AC-9, AC-20 | PORT QSL-1503 |
| FR-052-AC-11 evidence unsupported by the profile | FR-330-AC-3 (`unsupported` and `admit` refusals) and FR-522-AC-1 | PORT QSL-1502 |
| FR-052-AC-11 over-budget evidence; sibling keeps its result | FR-523-AC-5, AC-12 to AC-14 | PORT QSL-1502 |
| FR-052-AC-12 correction relation, direct predecessor, cross-obligation | FR-526-AC-1 to AC-19 | PORT QSL-1503 |
| `ProtocolLedger` correction must name the latest result of the obligation | FR-526-AC-8 (the caller-held `EvidenceLedger`) | PORT QSL-1503 |
| `ProtocolLedger` identifier space exhausted | FR-526-AC-13 (lineage overflow) | PORT QSL-1503 |
| `ProtocolLedger` trigger identity count mismatch | FR-516: the deliveries are a list, each with its identity, so no count can differ | RETIRE (structural reason: the request carries the deliveries as a list, each with its identity, so no count can differ; successor FR-516-AC-10) |
| `ProtocolLedger` activation-mapping checks (control to temporal declaration) | FR-516-AC-15, AC-16 | PORT QSL-1503 |
| FR-052-AC-13 | FR-502-AC-1 to AC-8 | PORT (FR-502) |
| FR-052-AC-14 | FR-501-AC-1 to AC-21 | PORT (FR-501) |
| FR-053-AC-7, AC-8 distinct identities stay distinct | FR-524-AC-1, AC-7; FR-516-AC-4 | PORT QSL-1503 |
| FR-053-AC-9 empty identity | FR-516-AC-4; FR-526-AC-18 | PORT QSL-1503 |
| FR-053-AC-9 mismatched or stale identity | FR-524-AC-20; FR-526-AC-8, AC-10 | PORT QSL-1503 |
| FR-053-AC-9 absent identity | FR-516-AC-2, FR-516-AC-20, FR-516-AC-21 (an absent delivery is `Inactive` or `Unknown`, never an aliased instance); FR-526-AC-18 | PORT QSL-1545 |
| FR-053-AC-9 malformed identity, cross-version translation | none: the identity is opaque non-empty bytes (FR-053) and the request type has one version | RETIRE (structural reason: the identity is opaque non-empty bytes with one version, so neither can occur; successor FR-053-AC-9 as amended, FR-526-AC-18) |
| NFR-008-AC-1 checked tick arithmetic, refused before any position | FR-523-AC-16 | PORT QSL-1502 |
| NFR-008-AC-2 reached limit, typed stop, siblings keep results | FR-523-AC-5, AC-12 to AC-14 | PORT QSL-1502 |
| NFR-008-AC-3 eviction names the subject | FR-524-AC-12, AC-21, AC-24 | PORT QSL-1503 |
| NFR-008-AC-4 ceilings and restoration state in the identity | FR-522-AC-17; the `EvidenceLedger` member of the identity | PORT QSL-1502 |
| NFR-008-AC-4 100,000-deep formula on a small stack | FR-523-AC-17 | PORT QSL-1502 |
| NFR-008-AC-5 first unaffordable operation not performed | FR-523-AC-9, AC-10, AC-11 | PORT QSL-1502 |
| NFR-008 `instances` ceiling | FR-523-AC-4, AC-5 (counter `instances`); FR-524-AC-26 | PORT QSL-1502 |
| NFR-008 retained-state table across calls | FR-524-AC-19 to AC-21 (`EvidenceLedger.instances`) | PORT QSL-1503 |
| Hard owner maxima that clamped the native request limits | FR-523-AC-8: no clamp (FR-255); a reached ceiling is `Incomplete` with its limit kind | PORT QSL-1502 |
| Native request document byte ceilings | FR-516-AC-7 (`request_bytes`, `result_bytes`, `member_bytes`) | PORT QSL-1545 |
| `json_depth` ceiling; `formula_depth` ceiling | none: ADR-030 D-1 and NFR-008 Scope state that nesting is not a limit | RETIRE (structural reason: ADR-030 D-1 forbids a nesting limit and a typed value has fixed nesting; successor FR-523-AC-17) |

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-525-AC-1 | Each of FR-043-AC-1 to AC-14, FR-044-AC-1 to AC-9, FR-045-AC-1 to AC-6, FR-052-AC-9 to AC-14, FR-053-AC-7 to AC-9 and NFR-008-AC-1 to AC-5 has a row with a named successor criterion or a RETIRE row. | Inspection (TC-1320) |
| FR-525-AC-2 | Every successor criterion a row names exists in this repository's spec. | Inspection (TC-1320) |
| FR-525-AC-3 | Every RETIRE row states its structural reason and names a successor criterion. | Inspection (TC-1320) |

## Dependencies

- FR-499 (the deletion gate and its dispositions), FR-500 (the same map for
  FR-042), FR-501, FR-502, FR-504 (the finite-window and progress successors),
  FR-516 (the evidence call), FR-522 to FR-524, FR-526 and FR-527 (the ports).

## References

- Linear QSL-1502, QSL-1503, QSL-1504, QSL-858; the deletion table rows of the
  temporal family (B3 to B31).
