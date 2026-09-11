---
id: SR-375
title: "Native temporal evaluation base review"
type: SpecReview
analysis: base
scope: "FR-043, FR-044, NFR-008, TC-122-124, TM-008"
review_set: subset
evaluated_revision: "4c1eee8646b51e00cd141d15eb110a33354b8475"
review_date: "2026-09-11"
---

## Summary

FR-043, FR-044, NFR-008, TC-122–124 and TM-008 were read against the vendored shared requirements FR-048, FR-090, FR-093 and NFR-040 and the four temporal definition artifacts. Activation and resource bounding are faithfully covered; bounded-truth evaluation is not. Two criteria are unsound: one demands declared clock dimensions the compiler neither parses nor emits, and the set pins no lower-bound convention for the order-sensitive operators.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | high | FR-043-AC-3 and TC-122 step 3 mutate a declared sample period, epoch, timestamp unit and sequence authority; no such declared value exists in the grammar or the emitted artifact, so the control cannot be written. | FR-043-AC-3; TC-122; FR-090-AC-3 | wrong-requirement |
| FND-002 | high | No criterion pins the `until`/`since` lower-bound convention or the `release`/`triggered` duals; FR-043 exercises operator presence and tie refusal only. | FR-043-AC-4; FR-043-AC-5; FR-091-AC-3; FR-092-AC-3 | missing-requirement |
| FND-003 | medium | FR-091, FR-092 and FR-094 own the settlement, decision-support and history semantics FR-043 restates, but are declared `references` or not at all, so their criteria never entered TM-008. | FR-043; TM-008; FR-092-AC-2; FR-094-AC-3; FR-094-AC-7 | missing-requirement |
| FND-004 | medium | TM-008 traces NFR-008 inside `Functional Requirement Coverage` and has no non-functional section; NFR-008's four metrics and NFR-005 get no verification-method row. | TM-008; NFR-008; NFR-005 | correct-requirement-no-evidence |
| FND-005 | medium | Option permutation: only `always[0,1]` crosses all three profiles; the other seven operators run under one unnamed profile, so no past operator is ever evaluated under the finite-window boundary rule. | FR-043-AC-2; TC-122; TM-008 | correct-requirement-no-evidence |
| FND-006 | medium | NFR-008-AC-5 adds a charging and affordability obligation with no row in the Measurement and Evaluation table, no sentence in Verification and no counterpart in NFR-040. | NFR-008-AC-5; NFR-040 | correct-requirement-no-evidence |
| FND-007 | low | TC-123 step 6 asserts ambient `self`/`result`/`pre(expr)` and shadowing refusals that no acceptance criterion states; FR-044-AC-6 covers only the forward read. | FR-044-AC-6; TC-123; FR-048-AC-3 | missing-requirement |

## Traceability against the shared criteria

Twenty-two shared criteria were mapped one by one. Covered and faithful: FR-048-AC-4, FR-048-AC-6 and FR-048-AC-8 by FR-044-AC-1/2/3, FR-043-AC-1 and FR-044-AC-7; FR-090-AC-1 and FR-090-AC-4 by FR-043-AC-2 and FR-043-AC-5; all five FR-093 criteria by FR-044-AC-1 through AC-5, matching clause for clause; all four NFR-040 criteria by NFR-008-AC-1 through AC-4.

Not covered by any criterion of FR-043, FR-044 or NFR-008: FR-048-AC-1, FR-048-AC-2, FR-048-AC-3 in part, FR-048-AC-5 in part, FR-048-AC-7, FR-090-AC-2, FR-090-AC-3 in part and FR-090-AC-5 in part.

The out-of-scope claim was tested against source, not accepted. FR-048-AC-1's declared precedence, retained spans and refusal of unparenthesized binary chains are implemented in `src/parser/composed/temporal.rs`, whose relation level refuses a second chained relation. FR-048-AC-2's negative and fractional bounds are refused by the unsigned interval reader in `src/parser/composed.rs`, unbounded forms by the interval being mandatory for every bounded operator, and reversed bounds by the `lower > upper` admission check in `src/protocol_artifact/native/families.rs`. FR-048-AC-5's clock binding is emitted as an exact `BindingKind::Clock` requirement and re-checked on artifact validation. FR-048-AC-7's non-Boolean `holds` root is refused by the checker's Boolean obligation on the `holds` argument in `src/checking/composed/solver/roots.rs`, and a temporal node cannot appear in a shared value argument because the two grammars are disjoint. FR-090-AC-2's unknown, stale, digest-mismatched, multiply selected and inconsistent definitions are refused by the identity, revision, digest and duplicate causes in `src/linking/composed/definitions.rs`, over the pinned registry in `src/linking/composed/definition_source.rs`, which also establishes the first half of FR-090-AC-5. The selected profile survives into the artifact as the `Declaration.profile` index, so FR-043's profile-retention premises are grounded.

Two claims do not survive the same test. FR-090-AC-3's sample period, epoch, timestamp unit and sequence authority appear nowhere in the language or the wire: the temporal body carries an input binder, one clock index, an activation record, captures and the operation graph, and the binding requirement record has no period, epoch or unit field. FR-043-AC-3 and TC-122 step 3 nevertheless require each of those dimensions to be independently mutated against a fixed declaration, and no requirement in this addition asks FR-042 to emit them. FR-090-AC-5's second half, that a backend capability request cannot silently select a different executable profile, is only partially reached by FR-043-AC-3's refusal of an inserted selection and FR-044-AC-7's retained native subject.

## Checklist and coverage

Identifier formats are correct and sequential: FR-043 and FR-044 follow FR-042, NFR-008 follows NFR-007, TC-122 through TC-124 follow TC-121, and criteria use the `{PARENT}-AC-N` form with no duplicate or missing number. TM-008's claim of twenty criteria matches eight plus seven plus five. Every relative link in the eight authored and edited documents resolves, and no internal reference uses an absolute or `ix://` form where ADR-0007 requires a relative one. The unrelated TC-116 and TC-118 absences predate this addition. `Coverage Status` and `Status` are used as `docs/matrix-status.md` requires and no rename is proposed; FND-004 concerns the missing non-functional section, not either header.

The requirements are atomic and normative. FR-043 owns bounded truth, FR-044 owns activation, and neither leaks into the other. Unhappy paths are strong throughout: refusal, incomplete, pending and the two Booleans stay five distinct outcomes, and TC-122 step 7, TC-123 step 4 and TC-124 steps 2 and 3 each drive the failure directly rather than inferring it.

Of the six coverage rules, coverage, error path, state transition and edge case hold. Every criterion has exactly one case; the activation state machine has a control for each of its five dispositions and for first, repeated and distinct delivery; zero, exact, insufficient and clamped ceilings are explicit. Constraint boundary holds for the ceilings and for the zero-width interval, but not for the profile-specific boundary rules, which are only ever exercised on `always`. Option permutation fails as recorded in FND-005.

## Verdict and provenance

CHANGES REQUESTED. FR-044 and NFR-008 are ready; FR-043 is not, pending FND-001 and FND-002. This review read source at `4c1eee8646b51e00cd141d15eb110a33354b8475`, the four vendored shared requirements, FR-091, FR-092, FR-094, FR-095 and the four temporal definition artifacts, and inspected the parser, linker, checker and protocol-artifact crates to test each out-of-scope claim. No spec file was modified, no build or test run was started, and no test-status advancement is authorized. Implementation and test completion are not claimed.
