---
id: SR-944
title: "QSL-356 gap analysis of PR 554: ticket items 1-4 against the change"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@f085d7a307f1ffd3504ebb6b3a52d7978b451051; QSL-356 items 1-4; spec/decisions/ADR-011-stage-dag-and-dependency-architecture.md (FB-05, X-10, T-7, §7.1, OBS-040); spec/decisions/ADR-013-canonical-type-package-conversion-ownership.md (§2, O-15, T-7); spec/functional/FR-059-check-backend-dependency-direction.md; spec/test-cases/TC-156-check-backend-dependency-direction.md; tools/arch-lint/graph.rs; tools/arch-lint/metadata.rs; qsl-replay/Cargo.toml; qsl-replay/src/lib.rs; qsl-semantics/src/check/mod.rs; qsl-semantics/src/check/termination.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/ADR-011
    type: reviews
---
## Summary

Ticket: QSL-356. PR: quire-spec-language#554 at f085d7a3.

Each ticket item against the code:

- **1. quire-exact is a leaf outside FB-05.** FB-05 row now names K as outside FB-05 and
  points at §6.1 and §7.1. FR-059 gains the SHALL and FR-059-AC-8; TC-156 gains step 8;
  `tests.md` widens the TC-156 range. The lint exempts `quire-exact` and the test shows
  RT -> quire-exact gives no edge while RT -> qsl-eval is an FB-05 violation. The test
  fails without the fix (measured). The lint change has a collateral effect on FR-061,
  recorded as SR-943 FND-001.
- **1, T-7 sentence: not added, and that is correct.** ADR-011 T-7 is "M-2 and M-2c"
  (QSL) and ADR-013 T-7 is where `BackendDescriptor` and `Capability` live. Neither is
  the parity comparator. ADR-011 §7.1 ("The parity comparator stays in
  agent-ix/quire-contract-codegen#50") and OBS-040 already attribute it to CG, on the
  branch and on main 8b0c1ffe. Writing "T-7 belongs to the parity comparator" into either
  ADR would be false. The ticket's T-7 most likely uses IR-323's numbering.
- **2. ADR-013 §2.** Now reads "QSL encodes through it. IR's adoption (IR-274) and quoin's
  (PLAT-989) are required." No other "adopted" claim about IR-274 remains in `spec/`.
- **3. Measure discharge.** Added to ADR-013 O-15 Invariants. The code backs it:
  `check/mod.rs:1142-1145` returns `termination::check`'s refusals before any lowering,
  and `termination.rs:307` refuses per recursive component. `qsl-package/src` has no
  termination or decreases field, so there is no per-function wire flag. The sentence is
  scoped to v2 packages emitted from a checked package, which is accurate. FR-146 is a
  QSpec requirement: no `spec/functional/FR-146*` exists in this repo. See FND-001.
- **4. X-10.** The `[dependencies]` list matches `qsl-replay/Cargo.toml` exactly
  (qsl-attrs, qsl-cst, qsl-eval, qsl-forms, qsl-foundation, qsl-package, qsl-semantics,
  optional qsl-source under `quire-extraction`, quire-exact, thiserror). The module list
  matches `qsl-replay/src` (bounds, call_site, execute, identity, proof_result, request,
  result, spine, witness). The re-export claim matches `lib.rs` (`pub use` of
  qsl_semantics, qsl_forms, qsl_foundation and quire_exact items). Main's #551 does not
  change qsl-replay's manifest.

Merge with main 8b0c1ffe: `git merge-tree` reports ADR-011, ADR-013 and `tests.md`
changed on both sides, with no conflict markers. #552's edit to the "K is a leaf" bullet
does not touch the FB-05 row or X-10, and the new FB-05 text ("K is a leaf (§6.1)")
stays true against #552's wording.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | Ticket item 3 asks for the sentence in FR-146. FR-146 is QSpec's, so the PR rightly edits only ADR-013, but no QSpec follow-up ticket is recorded for the FR-146 half; the item is silently half-done. | spec/decisions/ADR-013-canonical-type-package-conversion-ownership.md:380 |

## Verdict

Items 1, 2 and 4 are met, and the T-7 skip is the right call. Item 3 is met in QSL; its
FR-146 half needs a QSpec ticket or an explicit note on QSL-356 that it is not wanted.

## Dispositions

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-001 | accepted-no-change | The team leader recorded on QSL-356 (Linear comment 9ec0e7a2) that no QSpec FR-146 follow-up is wanted: the plan lead ruled the measure-discharge rule is QSL admission behaviour, so ADR-013 O-15 is its home. That explicit note is the remedy the finding offered. |
