# Deletion gate evaluation checklist (QSL-15, QSL-462)

This checklist states what the `quoin:spec-architecture-evaluation` review for
gates QSL-15 (ARCH-G4, lanes M-6c to M-6e) and QSL-462 (ARCH-G4E) asserts and
the evidence each assertion reads. The evaluation runs after the last unit in
ADR-011's "Deletion order of the remaining old-architecture units" has
deleted. It writes one `SpecReview` (`analysis: architecture-evaluation`) under
`reviews/`. It is advisory unless a gate ticket declares it a gate.

## QSL-15: old paths are unreachable and the common architecture holds

| # | Assertion | Evidence |
|---|---|---|
| 1 | `src/native_model`, `src/protocol_artifact`, `src/syntax/composed*`, `src/linking/composed`, `src/checking/composed`, `src/parser/composed*` are absent, and no workspace file, example, fixture, Makefile lane or Cargo feature names them. `src/temporal` is absent too, except the public surface of assertion 9 while the quire-protocol repoint (QSL-1549) and IR-1005 are not both merged; with both merged it is absent entirely. | `git ls-files` on those paths is empty; `git grep` for `crate::temporal`, `protocol_artifact`, `native_model`, `parse_native`, `DeclarationKind::Protocol`, `handoff-writer` finds none outside history; FR-499-AC-2 to AC-5 tests (TC-1293). |
| 3 | `quire matrix` lists no criterion of a retained requirement untagged because its test was deleted. | `quire matrix --scope .` output: a criterion untagged at the deleting commit that a deleted test tagged in its parent (FR-499-AC-6, FR-493-AC-5). |
| 4 | The spine is the only producer of a checked package, and no second package, proof or replay path exists. | ADR-011 section 3 forbidden bypasses; `cargo run -p arch-lint -- qualified-core --qsl .`; FR-087-AC-2, AC-9, AC-10 tests. |
| 5 | Identity, provenance, version, capability, bound and structured outcome survive the stages for the nine G4 scenarios: (1) exact expression/operator extension; (2) sum type and exhaustive case; (3) model-bound lookup/dispatch and finite state transition; (4) temporal operator over bounded and infinite-trace profiles; (5) scoped protocol/frame clause; (6) unbounded declaration and unsupported backend claim; (7) spec/profile refinement regression; (8) model-to-implementation relation with an unbound refusal; (9) nested witness/trace replay. | One traced scenario per item, each following the files that carry it (S1 to S6a), with the successor tests tagged to that family's requirements in `quire matrix`. |
| 6 | Unsupported, incomplete and tool-failure outcomes are never promoted to success or refusal; finite evidence is not presented as unbounded proof. | FR-501 and FR-502 tests (finite window, regressing watermark), FR-329 `Lasso::Model` refusal test, FR-327-AC-8 to AC-13 tests. |
| 7 | The deletion left no stringly typed semantic dispatch or feature-local exception to the common architecture. | `git grep -nE '"(StateModel|SumCase|TemporalTrace|ProtocolClause)"' -- ':(glob)qsl-*/src/**' src` finds no string-matched family name in dispatch code (a hit that only renders an enum variant as its name, never matches a string to choose a family, is not dispatch), and the same command run on a scratch file holding `"StateModel"` under `qsl-eval/src/` and under `src/` finds it in both (a positive control: a grep that cannot match proves nothing); the ADR-012 contract tests and `cargo run -p arch-lint -- qualified-core --qsl .` pass. |
| 8 | quire-protocol consumes the checked package, not a QSL handoff writer. | quire-protocol default branch: pins a QSL revision containing the successor, has no `handoff-writer` feature, `cargo check --locked --all-targets` and IT-001, IT-002, TC-003, TC-011 pass (FR-521-AC-3). |
| 9 | The native owner-report route (`temporal::{evaluate_v2, Report, Trace}`) is deleted after FR-052-AC-15, AC-16, AC-19 and AC-20 have their FR-526-AC-20 to AC-28 successors and AC-17, AC-18 and AC-21 are retired with its carrier. | The root crate has no `temporal` module (TC-1293) and the tests tagged FR-526-AC-20 to AC-28 pass. |

Deleted units also leave documentation behind: `docs/native-model-checking.md`,
`docs/compiled-protocol-v1.md`, `docs/compiled-protocol-v2.md`,
`docs/native-temporal-evaluation.md` (deleted) and the rows of `docs/native-error-codes.md`
for deleted codes are removed or rewritten in the deleting change (assertion 1
reads `git ls-files docs/`).

Blocking findings reopen the owning Layer 1, 2 or 3 ticket; no gate passes by
accepting a feature-local exception.

## QSL-462: native model checkers are sound and replayable

Precondition: QSL-15 has passed. The deletions change no `qsl-analyze` code, so
the evaluation confirms that nothing it depends on went with them.

| # | Assertion | Evidence |
|---|---|---|
| 1 | EN-1 safety and liveness verdicts each carry a replayed counterexample. | `qsl-analyze` EN-1 tests; `qsl-analyze/tests/it/lasso_replay.rs`. |
| 2 | A tampered certificate is rejected by its core checker. | Certificate-checker tests for zone (E20b-chk) and exact probability (E18b). |
| 3 | An engine without a checker carries the `uncertified` label. | Outcome tests for E22-H and the hybrid solver. |
| 4 | Undefined settles `refuted` for proofs and `measured: rejected` for statistical claims; a reached budget settles `inconclusive` and states its default. | FR-283, FR-330 and E17d tests; FR-523 stop tests. |
| 5 | No core crate depends on `qsl-analyze`. | `arch-lint qualified-core` run. |
| 6 | The POR reduced-versus-unreduced differential gate is green. | E12c differential test in `qsl-analyze`. |
| 7 | Every engine sub-issue is Done, or the evaluation names it as open scope, not as a pass. | Linear state of the QSL-462 sub-issues. |
