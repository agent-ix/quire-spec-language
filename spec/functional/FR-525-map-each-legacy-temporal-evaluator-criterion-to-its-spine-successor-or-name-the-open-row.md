---
id: FR-525
title: "Map each legacy temporal evaluator criterion to its spine successor or name the open row"
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
# FR-525: Map each legacy temporal evaluator criterion to its spine successor or name the open row

## Description

This requirement names, for each acceptance criterion of the legacy temporal
evaluator (FR-043, FR-044, FR-052, FR-053 and NFR-008), the spine criterion that
now states it, and names each behavior for which the spine has no successor as an
open row that the owner rules on. FR-500 does the same for FR-042. FR-499 gates
the deletion of `src/temporal` on these rows; this requirement supplies the
successor column that FR-499's temporal table leaves as "none".

A row never reads "retired" without a named successor. A legacy criterion with no
successor stays in the legacy code until the owner rules (FR-499).

## Semantic authority and boundary

FR-499 owns dispositions: COVERED (a tagged spine test is green on the head of
the deleting change), PARTIAL, PORT (the behavior moves under the named ticket),
KEEP and RETIRE. This requirement adds **OPEN**: a behavior with no spine
successor, kept in the legacy code, with the question for the owner. The tickets
are QSL-1502 (FR-522, FR-523), QSL-1503 (FR-524, FR-526) and QSL-1504 (FR-527).
The successor criteria of FR-501, FR-502 and FR-504 belong to the finite-window
and progress work under QSL-858's train.

## FR-043 criterion to successor

| Legacy criterion | Successor criteria | Disposition |
| --- | --- | --- |
| FR-043-AC-1 constant and `holds` atom not folded | FR-327-AC-1, FR-327-AC-8 | PORT QSL-1503 (FR-327-AC-8 is new) |
| FR-043-AC-2 three profiles, three identities | FR-501-AC-2, FR-522-AC-12 | PORT QSL-1502 |
| FR-043-AC-3 profile and clock disagreement refuse before any position | FR-522-AC-1 to AC-11 | PORT QSL-1502 |
| FR-043-AC-4 eight operators, `once[1,1]`, `until[1,2]`, duals | FR-327-AC-1, AC-2, AC-9; FR-501-AC-1, AC-6, AC-7 | COVERED per FR-499 for the false-extension profiles; PORT QSL-1503 for FR-327-AC-9 |
| FR-043-AC-5 equal coordinate without order key, mixed order authority | FR-501-AC-7 (`timestamped-event`) | PORT (FR-501); the clause "reversing insertion order changes nothing" is OPEN-1 |
| FR-043-AC-6 pointwise connectives | FR-327-AC-1 (`always[0,3] not holds(c.value > 2)`); FR-501 "Operators" | COVERED per FR-499 |
| FR-043-AC-7 closed-scope, open-scope basis, fact missing inside or outside support | FR-501-AC-4, AC-14; FR-504-AC-5 | PORT (FR-501, FR-504) |
| FR-043-AC-8 incomplete input applies no false extension or empty truth | FR-501-AC-5; FR-504-AC-4 | PORT (FR-501, FR-504) |
| FR-043-AC-9 history cutoff gives missing history; interior gap | FR-501-AC-8; FR-504-AC-6; the interior gap is OPEN-2 | PORT (FR-501, FR-504) |
| FR-043-AC-10 four independent axes; no caller basis | FR-501-AC-15; FR-516-AC-1, AC-5 | PORT (FR-501) and QSL-1545 |
| FR-043-AC-11 watermark settles a silent deadline; event-position does not | FR-501-AC-3, AC-6; FR-504-AC-1, AC-2 | PORT (FR-501, FR-504) |
| FR-043-AC-12 regressing watermark, conflicting completeness | FR-502-AC-1 to AC-8 | PORT (FR-502) |
| FR-043-AC-13 one identity per input set | FR-522-AC-12 to AC-15; FR-501-AC-16 | PORT QSL-1502 |
| FR-043-AC-14 `/2` authenticated clock configuration and report | FR-522-AC-4, AC-5, AC-7, AC-9; the `/1` half is OPEN-3 | PORT QSL-1502 |

## FR-044 criterion to successor

| Legacy criterion | Successor criteria | Disposition |
| --- | --- | --- |
| FR-044-AC-1 distinct triggers with equal payloads, two instances | FR-524-AC-1, AC-4 | PORT QSL-1503 |
| FR-044-AC-2 duplicate delivery, conflicting payload | FR-524-AC-2, AC-3 | PORT QSL-1503 |
| FR-044-AC-3 captures immutable | FR-524-AC-12 | PORT QSL-1503 |
| FR-044-AC-4 capture faults named, siblings unaffected | FR-524-AC-8 to AC-10 | PORT QSL-1503 |
| FR-044-AC-5 four-way trigger-scope disposition | FR-524-AC-15, AC-16 | PORT QSL-1503 |
| FR-044-AC-6 captures once, in order, no later read | FR-524-AC-11, AC-13 | PORT QSL-1503 |
| FR-044-AC-7 guard false, guard unestablished | FR-524-AC-6, AC-7 | PORT QSL-1503 |
| FR-044-AC-8 one instance per execution origin | FR-524-AC-5; FR-327-AC-2 for `on each` | PORT QSL-1503 |
| FR-044-AC-9 evicted capture | FR-524-AC-8, AC-11 | PORT QSL-1503 |

## FR-045, FR-052, FR-053 and NFR-008 criteria to successors

| Legacy criterion | Successor criteria | Disposition |
| --- | --- | --- |
| FR-045-AC-1 every table row reachable | FR-527-AC-1, AC-4, AC-5 | PORT QSL-1504; KEEP the legacy surface until IR-1005 |
| FR-045-AC-2 finite window, mixed direction, past operators | FR-527-AC-2, AC-3 | PORT QSL-1504 |
| FR-045-AC-3 unsupported retains subject, profile, activation | FR-527-AC-7 | PORT QSL-1504 |
| FR-045-AC-4 every unmatched dimension | FR-527-AC-3 | PORT QSL-1504 |
| FR-045-AC-5 supported names target and premises | FR-527-AC-8, AC-9 | PORT QSL-1504 |
| FR-045-AC-6 authenticated selection | FR-527-AC-11 to AC-14 | PORT QSL-1504; the `/1` half is OPEN-3 |
| `Unmatched::TimedProfile` (exact rational time) | none: FR-045 admits four profiles | OPEN-4 |
| FR-052-AC-9 no supplied truth | FR-516-AC-1 | PORT QSL-1545 |
| FR-052-AC-10 independent facts, no default | FR-516-AC-3, AC-5; FR-524-AC-16 | PORT QSL-1545 |
| FR-052-AC-11 stop confined to its instance | FR-523-AC-8, AC-9 | PORT QSL-1502 |
| FR-052-AC-12 correction relation | FR-526-AC-1 to AC-10 | PORT QSL-1503 |
| FR-052-AC-13, AC-14 | FR-502, FR-501 | as FR-052 states |
| `ProtocolLedger` correction must name the latest result of the obligation | none: the evidence call holds no history | OPEN-5 |
| `ProtocolLedger` activation-mapping checks (control to temporal declaration) | none: the evidence call selects a clause, not a control | OPEN-6 |
| FR-053-AC-7, AC-8 distinct identities stay distinct | FR-524-AC-1, AC-4; FR-516-AC-4 | PORT QSL-1503 |
| FR-053-AC-9 empty identity | FR-516-AC-4; FR-526-AC-10 | PORT QSL-1503 |
| FR-053-AC-9 malformed, stale, cross-version identity | none: the legacy API kept an opaque string and checked only emptiness | OPEN-7 |
| NFR-008-AC-1 checked tick arithmetic | FR-523-AC-10; FR-326-AC-3 | PORT QSL-1502 |
| NFR-008-AC-2 reached limit, typed stop, siblings keep results | FR-523-AC-3, AC-8, AC-9 | PORT QSL-1502 |
| NFR-008-AC-3 eviction names the subject | FR-524-AC-8, AC-14 | PORT QSL-1503 |
| NFR-008-AC-4 ceilings in the identity; 100,000-deep formula | FR-522-AC-13; FR-523-AC-12 | PORT QSL-1502 |
| NFR-008-AC-5 first unaffordable operation not performed | FR-523-AC-6, AC-7 | PORT QSL-1502 |
| NFR-008 `instances` ceiling | none | OPEN-8 |
| Hard owner maxima that clamped the native request limits | none: FR-255 states no ceiling beyond the width of the quantity | OPEN-9 |
| Retired `json_depth` and the document byte ceilings of the native request | FR-516 (`request_bytes`, `result_bytes`, `member_bytes`); `json_depth` none | OPEN-10 |

## Open rows

Each row stays in the legacy code until the owner rules. The recommendation is the
requirement author's inference, not a ruling.

| Row | Behavior with no spine successor | Question for the owner | Recommendation |
| --- | --- | --- | --- |
| OPEN-1 | Legacy canonicalized positions by coordinate and order authority, so reversing insertion order changed nothing | The spine takes the admitted order as given and refuses a decreasing stamp (FR-501). Is refusal the successor? | Accept refusal: a caller that sorts is explicit about order |
| OPEN-2 | An omitted interior history interval is incomplete at its exact position, never false padding (FR-043-AC-9) | Event-position and fixed-sample positions are dense indices and the finite-window window is stamp-addressed, so an interior gap has no representation. Is that by design? | Accept: no gap can exist; keep a test that a stamp gap is an empty interval |
| OPEN-3 | The `/1` entry points kept the trace's clock parameters as unauthenticated premises and retained no binding (FR-043-AC-14, FR-045-AC-6) | The checked route has one authenticated form. Does any consumer need the unauthenticated form? | Retire after quire-protocol's repoint (QSL-1549) |
| OPEN-4 | `Unmatched::TimedProfile`, exact rational-time semantics | The spine admits four profiles and FR-045's table has no timed row. Retire the dimension? | Retire with the legacy `/1` classifier |
| OPEN-5 | A correction must name the latest result of its obligation | A stateless evidence call cannot know the latest result. Add a caller-held result ledger like FR-502's, or accept that the caller names the predecessor it holds? | Add the ledger only if quire-protocol relies on the latest-only check |
| OPEN-6 | The legacy ledger refused a control with no admitted temporal mapping, or one mapped to another declaration | Should `evaluate_temporal_evidence` accept a control selection and check FR-449's map? | Yes if quire-protocol selects by control (QSL-1549) |
| OPEN-7 | Malformed, stale or cross-version trigger identity (FR-053-AC-9) | The legacy API never checked these on opaque bytes. Keep the AC text and accept emptiness as the only check? | Accept; amend FR-053-AC-9 when the owner rules |
| OPEN-8 | A ceiling on concurrently active instances | One delivery per call; `on each` activations are charged by `evaluation_steps`. Is a separate ceiling wanted? | No |
| OPEN-9 | Owner maxima clamped caller limits | FR-255's rule is no clamp; is that intended for the temporal counters? | Yes |
| OPEN-10 | `json_depth` ceiling and document byte ceilings | Typed values have fixed nesting; FR-516 names three byte counters. Confirm the three? | Confirm |

The default of `formula_depth` is also an owner choice: the legacy default was 64,
which contradicts ADR-030's rule that nesting is not capped. FR-255's row sets it
equal to the `formula_nodes` default so that the node ceiling is reached first.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-525-AC-1 | Each of FR-043-AC-1 to AC-14, FR-044-AC-1 to AC-9, FR-045-AC-1 to AC-6, FR-052-AC-9 to AC-14, FR-053-AC-7 to AC-9 and NFR-008-AC-1 to AC-5 has a row with a named successor criterion or an OPEN row. | Inspection (TC-1320) |
| FR-525-AC-2 | Every successor criterion a row names exists in this repository's spec. | Test (TC-1320) |
| FR-525-AC-3 | Every OPEN row of the Open rows table is named in a row of the criterion tables, and no row of those tables reads RETIRE. | Inspection (TC-1320) |
| FR-525-AC-4 | A spine test carries each PORT row's successor criterion as a `#[trace(...)]` tag on the head of the change that deletes `src/temporal`, and the legacy tests TC-122, TC-123, TC-125, TC-144, TC-149 and TC-1277 to TC-1279 are not offered as that evidence. | Test (TC-1320) |
| FR-525-AC-5 | No file of `src/temporal` is deleted while a PORT row names a successor criterion that no passing spine test carries, or while an OPEN row has no owner ruling. | Test (TC-1320) |

## Dependencies

- FR-499 (the deletion gate and its dispositions), FR-500 (the same map for
  FR-042), FR-501, FR-502, FR-504 (the finite-window and progress successors),
  FR-516 (the evidence call), FR-522 to FR-524, FR-526 and FR-527 (the ports).

## References

- Linear QSL-1502, QSL-1503, QSL-1504, QSL-858; the deletion table rows of the
  temporal family (B3 to B31).
