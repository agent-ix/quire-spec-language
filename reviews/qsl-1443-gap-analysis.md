---
id: SR-202610310152
title: "Acceptance-to-test gap analysis of QSL-1443 PR #2040"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@b202293619b944fee512bcc6384f2e61ec4667ad; QSL-1443: replay QSpec CL-57..62 in both QSL consumers; FR-230-AC-54 and QSpec FR-322 clause-node acceptance"
review_set: subset
---
## Summary

Ticket: QSL-1443. Plan completion: not assessed. `quoin gap-analysis` is not an installed command in Quoin 0.29.1, and `quoin matrix --repo ... --json` produced no output and timed out after 60 seconds. I therefore performed a manual acceptance-to-test trace using the QSL-1443 acceptance, test trace attributes, the CL-57..62 vector cases, and the typed projection/writer code. Current-main CL-01 `/temporal_bindings` failures were reproduced identically on clean base and PR, so they are an IR-1230 baseline and excluded from this delta.

- `checked_v2::conformance_timed_clause_vectors_keep_their_published_outcomes` traces FR-322-AC-261..263, AC-295 and TC-367 and iterates the QSpec clause vectors. The filter in the emitter consumer includes all CL-57..62; the checked reader consumer loops all cases. Both consumers are run successfully with QSpec #564 and IR #785; neither skips an admitted binder vector or typed refusal.
- CL-58, CL-60 and CL-62 exercise typed refusals, including schema-invalid CL-61 at its execution-point node. The common refusal assertion checks code, cause and locus; the schema-invalid branch checks `InvalidSemanticGraph` and the node pointer.
- CL-57 and CL-59 assert only that the package is admitted. They do not inspect the projected typed binder IDs or assert that writing the typed model retains binder order. The existing typed round-trip test uses the unquantified positive fixture. This leaves a coverage gap for the central new behavior despite the lower-level vector readers passing.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | CL-57/CL-59 do not verify that guard/invariant binders survive typed projection and serialization. | qsl-package/src/checked_v2/tests.rs:2608-2610; qsl-package/src/checked_v2/timed_model.rs:1390-1440 |

## Verdict

Not mergeable until the admitted binder cases assert the typed projection and typed writer behavior. The two consumer tests establish that IR admits the published vectors and that QSL's I2 reader accepts them, but `Read::Verified` alone does not establish that `CheckedGuardPart.binders` and `CheckedTimeInvariant.binders` contain the input binders or that `write_guard`/`write_invariant` emit them. The pre-existing typed round-trip does not cover this gap because its positive fixture is unquantified.

Required oracle: replay CL-57 and CL-59 through typed admission, assert the exact nonempty ordered `CheckedNodeId` list for each guard/invariant, serialize with `write_nodes`, assert the exact ordered `binders` references, and compare the typed round-trip. Keep the CL-58/60/61/62 typed refusal assertions and both consumer loops intact.

## Coverage trace

| Obligation | Test / evidence | Result |
| --- | --- | --- |
| CL-57 and CL-59 are admitted by both consumers | `checked_v2::conformance_timed_clause_vectors_keep_their_published_outcomes`; `emit::timed_order::conformance_the_clause_member_key_vectors_replay` | Covered for admission; typed projection and writer values are not asserted (FND-001) |
| CL-58, CL-60 and CL-62 retain typed refusal code, cause and locus | checked-v2 consumer calls `assert_timed_refusal`; emitter consumer replays refusal vectors | Covered |
| CL-61 schema-invalid execution-point refusal stays at its node | checked-v2 consumer asserts `InvalidSemanticGraph` and node pointer; emitter consumer includes invalid semantic graph cases | Covered |
| Both consumers replay published QSpec #564 vectors without skipping the binder cases | Focused tests passed against QSpec #564 + IR #785; emitter filter explicitly includes CL-57..62 | Covered |
| Current QSpec main `/temporal_bindings` behavior | Both consumers on clean base and PR fail identically at CL-01 `UnknownMember /temporal_bindings` | Known external baseline, IR-1230; no PR delta |

## Finding details

### FND-001 — medium

CL-57/CL-59 enter the `"admitted"` branch at `qsl-package/src/checked_v2/tests.rs:2608-2610`, which only asserts `Read::Verified`. The only existing typed fixture round-trip at `qsl-package/src/checked_v2/timed_model.rs:1390-1440` uses the unquantified positive clause fixture. Therefore a regression that drops or reorders binder IDs in `project_guard` / `project_invariant`, or omits them in `write_guard` / `write_invariant`, would not be caught by these tests. Assert exact nonempty binder IDs after typed projection, then exact output binding order and typed round-trip equality for the CL-57/59 packages.
