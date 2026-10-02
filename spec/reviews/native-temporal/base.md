---
id: SR-375
title: "Native temporal evaluation base review"
type: SpecReview
analysis: base
scope: "FR-043, FR-044, FR-045, NFR-008, TC-122-125, TM-008"
review_set: subset
evaluated_revision: "87bc83f5fad724413737e0a707cf80bdbcba931a"
review_date: "2026-09-11"
---

## Summary

Second cycle. The base checklist was re-applied at `87bc83f` against the shared requirements FR-048, FR-090, FR-091, FR-092, FR-093, FR-094, FR-095 and NFR-040 held as copies in this repo and the four temporal definition artifacts. All seven findings from the first cycle at `4c1eee8` are resolved, six of them substantively and one by recording a limitation the artifact cannot currently close. Four new findings are open, none of them semantic: one broken traceability edge, one contradiction inside NFR-008's own scope text, and two under-specified output and counter vocabularies.

### Disposition of the first-cycle findings

FND-001 is resolved by narrowing, not by closing the underlying gap, and the review accepts that on the stated record. FR-043-AC-3 now mutates only the asserted profile identity, profile revision and clock binding name — the three dimensions the emitted body actually carries — and the declared sample period, epoch, timestamp unit and sequence authority became opaque trace premises that participate in result identity. FR-043's Dependencies, TM-008's Overview, TC-122's Expected Results and `docs/native-temporal-evaluation.md` each state plainly that the artifact does not authenticate those values and that checking them needs an FR-042 wire extension on #38. The criterion is now testable and the claim is now honest; nothing in the artifact changed.

FND-002 is resolved substantively. FR-043's Operators section states the lower-bound convention for `until` and `since` and defines `release` and `triggered` as their duals under that same convention; FR-043-AC-4 pins `p until[1,2] q` true with `p` false at the anchor and `q` true at offset one, matching FR-091-AC-3, and TC-122 group 4 differentially evaluates each dual against its negated expansion on the same traces.

FND-003 is resolved. FR-091, FR-092 and FR-094 are now `depends_on` rather than `references`, and their criteria entered the set: FR-092-AC-2 as FR-043-AC-9, FR-094-AC-1/AC-2 as FR-043-AC-11, FR-094-AC-3 and FR-094-AC-7 as FR-043-AC-10, FR-094-AC-5 and FR-094-AC-8 as FR-043-AC-12, FR-094-AC-4 as FR-043-AC-12's foreign-binding clause.

FND-004 is resolved. TM-008 carries a Non-Functional Requirement Coverage section naming a catalog method per NFR-008 metric, plus rows for NFR-005 and NFR-003, and a Stakeholder and User Story Coverage section.

FND-005 is resolved. TC-122 group 4 now runs all eight bounded operators under each of the three profiles, and group 2 compiles one declaration per profile from identical clause text rather than re-pointing one declaration.

FND-006 is resolved. NFR-008's Measurement table carries a fifth row under `negative-abuse-testing` for the charging obligation, and its Verification paragraph exercises the zero, exact, one-short and clamped ceiling in every dimension.

FND-007 is resolved by relocation. TC-123 group 6 no longer asserts the ambient `self`/`result`/`pre(expr)` and shadowing refusals, so no control now outruns its criterion. Those refusals were confirmed delivered by the existing linker: `src/linking/composed/scopes.rs` carries the distinct issue kinds for a binder shadowing a model or profile alias and for `self`/`result`/`pre` being unavailable at a declaration or anchor.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | high | Resolved by narrowing: FR-043-AC-3 now mutates only profile identity, revision and clock binding name; period, epoch, unit and sequence authority became retained trace premises, with the artifact's inability to authenticate them recorded on #38 in four places. | FR-043-AC-3; FR-043-AC-13; TM-008; TC-122 | wrong-requirement |
| FND-002 | high | Resolved: FR-043 states the `until`/`since` lower-bound convention and the `release`/`triggered` duals; FR-043-AC-4 pins the FR-091-AC-3 vector and TC-122 group 4 checks each dual differentially. | FR-043-AC-4; FR-091-AC-3; FR-092-AC-3 | missing-requirement |
| FND-003 | medium | Resolved: FR-091, FR-092 and FR-094 are now `depends_on`, and their settlement, history, progress and four-axis criteria entered FR-043-AC-7, AC-9, AC-10, AC-11 and AC-12. | FR-043; FR-092-AC-2; FR-094-AC-3; FR-094-AC-7 | missing-requirement |
| FND-004 | medium | Resolved: TM-008 gained a Non-Functional Requirement Coverage section with a catalog method per NFR-008 metric plus NFR-005 and NFR-003 rows, and a Stakeholder and User Story Coverage section. | TM-008; NFR-008; NFR-005 | correct-requirement-no-evidence |
| FND-005 | medium | Resolved: TC-122 group 4 runs all eight bounded operators under all three profiles, and group 2 compiles one declaration per profile from identical clause text. | TC-122; TM-008 | correct-requirement-no-evidence |
| FND-006 | medium | Resolved: NFR-008 added a fifth metric row under `negative-abuse-testing` for the charging obligation and covers it in Verification. | NFR-008-AC-5 | correct-requirement-no-evidence |
| FND-007 | low | Resolved by relocation: TC-123 group 6 dropped the ambient-access and shadowing assertions, which were confirmed delivered by the existing linker's scope issue kinds. | FR-044-AC-6; TC-123; FR-048-AC-3 | missing-requirement |
| FND-008 | low | Open: FR-043's Outputs declares a truth of `true`, `false` or `pending` as always present, while its Behavior and AC-7 require basis `unavailable` with no truth; the set also names no basis for an obligation that was never assessed. | FR-043; FR-043-AC-7; FR-044-AC-5 | wrong-requirement |
| FND-009 | medium | Open: FR-045 declares `implements US-004` and TM-008 asserts the chain US-004 to FR-045 to TC-125, but US-004 carries no `exercises` edge to FR-045 and no FR-045 entry in its traceability list; the same reciprocal edit was made for US-003 last cycle. | FR-045; US-004; TM-008 | missing-requirement |
| FND-010 | medium | Open: NFR-008's Scope says it invents no universal maximum, while its Counter definitions clamp any ceiling above the published defaults, which makes those defaults exactly a universal maximum; `horizon`'s published default also leaves that dimension's ceiling unreachable. | NFR-008; NFR-040 | wrong-requirement |
| FND-011 | low | Open: NFR-008-AC-2 names four ceiling dimensions while the published contract defines eight counters, TC-124 group 2 forces five stops and group 5 charges eight; the criterion's dimension vocabulary does not map onto the counters a result must name. | NFR-008-AC-2; TC-124 | correct-requirement-no-evidence |

## Traceability against the shared criteria

All twenty-two criteria of the first cycle's source set are now reached, sixteen directly and six as verified out-of-scope deliveries. FR-048-AC-4, AC-6 and AC-8 land on FR-044-AC-1/AC-2/AC-3, FR-043-AC-1 and FR-045-AC-3; FR-048-AC-3's capture-scope half lands on FR-044-AC-6 with its ambient and shadowing half owned by the delivered linker. FR-090-AC-1, AC-3 and AC-4 land on FR-043-AC-2, AC-13 and AC-5, AC-3 at premise level only per FND-001. FR-090-AC-5's second half lands on FR-045-AC-1's prohibition on consulting a backend capability report. All five FR-093 criteria land on FR-044-AC-1 through AC-5 clause for clause, and all four NFR-040 criteria on NFR-008-AC-1 through AC-4.

The wider set pulled in this cycle is covered as recorded in the disposition above. FR-095 is now split correctly: FR-045 owns the support classification as a total function of the declaration, and the emission half — formula, valuation request and correspondence record — stays outside the matrix against `quire-contract-ir#63` and `#64`. FR-095-AC-6's epoch, period and unit correspondence appears in FR-045's support table as a condition on the fixed-sample row rather than as a checked artifact property, which is the same limitation FND-001 records.

The first cycle's source verification stands unchanged at this revision: the parser's chain refusal and spans, the unsigned interval reader, the `lower > upper` admission check, the `BindingKind::Clock` requirement, the checker's Boolean obligation on the `holds` argument, and the linker's identity, revision, digest and duplicate causes over the pinned definition registry.

## Checklist and coverage

Identifiers remain correct and sequential: FR-045 follows FR-044, TC-125 follows TC-124, and criteria use the `{PARENT}-AC-N` form with no duplicate or gap. TM-008's claim of thirty-two criteria matches thirteen plus nine plus five plus five. Every relative link in the ten authored and edited documents resolves, including the new `docs/native-temporal-evaluation.md`, and `quire validate` reports the nine subject documents grammar-clean. `Coverage Status` and `Status` are used as `docs/matrix-status.md` requires and no rename is proposed.

Atomicity holds and improved: FR-043 evaluates, FR-044 activates, FR-045 classifies, and each carries a Semantic authority and boundary section naming the owning agent and conceding that the shared rule governs on divergence. The published counter contract makes the NFR-008 controls derivable from a document rather than from the run.

Of the six coverage rules, all six now hold. Option permutation holds through the operator-by-profile cross. Constraint boundary holds for the ceilings, the zero-width interval, the nonzero lower bound, the inclusive upper endpoint and the deadline-adjacent instant. Error path, state transition and edge case hold across two state machines — activation through six dispositions and settlement through five bases with every one-axis substitution refused. The worked examples were checked independently: on one closed-complete position with `p` true, `always[0,1] not holds(p)` is false and `eventually[0,1] not holds(p)` is true under false extension, and `always[0,1] not true` is false, which is the pointwise reading FR-043-AC-6 claims.

Two obligations are outstanding by declaration rather than by omission, and both are stated in the artifacts themselves: mutation-adequacy measurement of the exhaustion paths is not performed by this revision, and the emitted body authenticates no declared clock parameter. Neither is claimed as covered anywhere in TM-008.

## Verdict and provenance

PASS for implementation of this specified scope. FND-009, FND-010, FND-008 and FND-011 are text-level corrections to a traceability edge, an NFR scope sentence, an Outputs sentence and a dimension vocabulary; none blocks implementation and none requires a semantic decision. This review read the spec set at `87bc83f5fad724413737e0a707cf80bdbcba931a`, the eight shared requirements held as copies in this repo, the four temporal definition artifacts, the new evaluation contract, and the parser, linker, checker, protocol-artifact and newly committed `src/temporal` sources. No spec file was modified, no build or test run was started, and no test-status advancement is authorized. Implementation and test completion are not claimed.
