---
id: SR-16110
title: "Spec review — quire-spec parse contract over qsl_cst::parse (FR-002, FR-256, FR-010)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language; spec/functional/FR-002-parse-native-units.md, spec/functional/FR-010-report-native-outcomes.md, spec/functional/FR-256-parse-source-at-any-nesting-depth.md, spec/test-cases/TC-012-native-readiness.md, spec/test-cases/TC-015-native-readiness.md, spec/test-cases/TC-749-root-native-parser-parses-deep-sources-on-a-small-stack.md, spec/test-cases/TC-750-quire-spec-parse-takes-caller-limits.md at 6530ab661"
review_set: subset
evaluated_revision: "6530ab661c73df9a5e356b7e427543842c3bc9ea"
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-002
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-010
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-256
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-012
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-750
    type: reviews
---

## Summary

Ticket: STD-286 (unblocks QSL-837). PR quire-spec-language#1273 rewrites FR-002
over `qsl_cst::parse` and the `parse` lifecycle operation, adds FR-002-AC-7/AC-8 for the
`quire-spec parse` command, drops FR-002-AC-3, FR-256-AC-4 and TC-749, restates FR-256-AC-5
over `--limit s1.*`, and restates FR-010-AC-1/5/7/11. Reviewed `git diff origin/main...HEAD`
at 6530ab661 against FR-286, FR-285, FR-275, FR-255, FR-287 (main and #1274's head),
ADR-029 CB-1 and Alternatives, and QSL main code: `qsl-replay/src/outcome.rs`
(`OutcomeDocument::from_parse`), `qsl-replay/src/spine/lifecycle.rs` (`parse`,
`FrontEndFailure`), `qsl-replay/src/spine.rs` (re-exports), `Cargo.toml` (`[[bin]]
quire-spec`, `default-run`), `Makefile` (`ci-clean-build`) and the tests tagged with the
removed ids.

Verified against FR-286 and the code: `OutcomeDocument::from_parse` exists at
`qsl_replay::outcome`, takes `Result<Staged<ParsedSource>, FrontEndFailure>`, gives success
at S2 with no items, diagnostics, artifacts or result, and maps a failure through
`from_failure`; `qsl_replay::spine::parse` is re-exported; `FrontEndFailure` is
`StageFailure<Box<CompileRefusal>>`, so FR-002's Outputs are accurate. FR-285 maps
`refusal` to 20 and stage limits to `incomplete` 22, as AC-7 and FR-256-AC-5 say.
FR-002-AC-7/AC-8 are unique across main and all 24 open PR heads; no other open PR
references TC-749, FR-256-AC-4 or FR-002-AC-3. `make check-spec-validation` exits 0.
Wording is behaviour-only. No textual conflict with any open PR.

## Verdict

NOT MERGEABLE as written. FND-001 (high) is a real contradiction that must be
resolved in this PR: FR-002 now specifies, as durable behaviour with Test ACs, a QSL-built
`quire-spec` binary and wires `make ci-clean-build` to it, while FR-287 (on main and as
#1274 restates it) says the QSL workspace builds no user binary and the root package
declares no `[[bin]]` or `default-run`, and ADR-029 CB-1 deletes `quire-spec` and rejects
keeping it as a second user CLI. FR-002-AC-7 and FR-287-AC-1 cannot both pass on any tree,
so whichever of QSL-837 and LC7 lands second gets a red AC by construction. Reviewer's
default for the fix (not an owner answer): state FR-002's QSL half as the library contract
the driver's `quire parse` reaches, that is the `parse` operation, `OutcomeDocument::from_parse`
and FR-285's exit function, with the `--limit`/usage-refusal behaviour left to the driver as
FR-287's Overlap section assigns argument refusal. If the owner wants `quire-spec parse`
specified for the transition window, that is an amendment to FR-287 and ADR-029 CB-1, an
owner decision this PR cannot take. FND-002 (medium) and FND-003 (medium) also need fixing
in-PR; FND-004 and FND-005 are wording.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | FR-002 (CLI paragraph, the five `quire-spec parse` bullets, AC-7, AC-8) and the restated FR-256-AC-5 specify a `quire-spec` executable that QSL builds, and AC-7 makes `make ci-clean-build`'s parse step that command. FR-287 says the QSL workspace builds no user binary ("shall build no executable target"; #1274: the root package declares no `[[bin]]` and no `default-run`), and ADR-029 CB-1 deletes `quire-spec` and rejects keeping it as a second user CLI. FR-002-AC-7 and FR-287-AC-1 cannot both pass. State QSL's half as the library contract the driver's `parse` verb reaches (the `parse` operation, `OutcomeDocument::from_parse`, FR-285's exit function) and leave `--limit` and usage refusal to the driver (FR-287 Overlap), or get an owner amendment of FR-287 and ADR-029 CB-1 for the window. | spec/functional/FR-002-parse-native-units.md:29-32,87,88; spec/functional/FR-256-parse-source-at-any-nesting-depth.md:54; spec/functional/FR-287-reach-qsl-through-the-driver-cli.md; ADR-029 CB-1; Cargo.toml:28,146-148 |
| FND-002 | medium | FR-010-AC-11 drives `parse agent-ix specs/a.quire <file>` through the command, but the command now writes only `OutcomeDocument::from_parse`, which carries no source, so "the `ParsedSource` it parses names its source with authority `agent-ix`, identity `specs/a.quire` and the file's digest" is not observable from the command. FR-010-AC-6's "parses with the exact labels and byte digest" has the same problem. State these clauses over the `parse` operation's `ParsedSource` in process (as AC-5 now does) or put the source reference in the document (an FR-286 change, which the PR body names). | spec/functional/FR-010-report-native-outcomes.md:84 |
| FND-003 | medium | The PR deletes FR-002-AC-3, FR-256-AC-4 and TC-749, but tests on main still carry those ids: `tests/it/parser.rs:167` is tagged `#[trace("TC-012", "FR-002-AC-3")]` and `tests/it/deep_sources.rs` is tagged with TC-749/FR-256-AC-4. After merge they trace to criteria that do not exist. Either delete or retag those tests in this PR, or name the follow-up that does (QSL-837) in the PR body, so the computed matrix does not carry orphan tags. | tests/it/parser.rs:167; tests/it/deep_sources.rs |
| FND-004 | low | FR-002-AC-7 says the `a < b < c` document has one `invalid_syntax` diagnostic "at that operator", but the unit holds two comparison operators. Say "at the second `<`", as FR-002-AC-2 does. | spec/functional/FR-002-parse-native-units.md:87 |
| FND-005 | low | "or another limit flag is given" leaves the set of limit flags open; FR-002-AC-8 lists only `--source-bytes`, `--tokens` and `--nodes`. Name the refused flags, or refuse every option other than `--limit` in this position. | spec/functional/FR-002-parse-native-units.md:69-72 |

## Dispositions

Round 1, checked against fix commit 5ed1a4cb5. `make check-spec-validation` exits 0; main is an ancestor; no textual conflict with any current open PR. No spec file on the head names `quire-spec parse` any more (outside `spec/reviews/`). FR-002 now states only the library contract (the `parse` operation, `OutcomeDocument::from_parse`, FR-285's exit code) and assigns the frontend's arguments and their refusal to the driver (FR-287 Overlap), so FR-002-AC-7 and #1274's FR-287-AC-1 can both pass: the #1273/#1274 contradiction is resolved. TC-750 keeps its id in a renamed file; no other file links the old name. FR-010's surviving CLI clauses (AC-2/3/4/7/9/10, AC-11's second half) are on main unchanged and outside this PR's change.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | FR-002 drops every `quire-spec` clause; the frontend paragraph assigns arguments to the driver; AC-7 is stated over `OutcomeDocument::from_parse` and FR-285, with `ci-clean-build` running a library test; AC-8 and FR-256-AC-5 are stated over the `parse` operation and FR-255's settings operation. |
| FND-002 | fixed | FR-010-AC-11's first clause and AC-6 now read the `ParsedSource` the `parse` operation returns in process. |
| FND-003 | deferred | QSL-837 (the code ticket that deletes the 0-draft parser): the PR body now states that QSL-837 must delete or retag `tests/it/parser.rs:167` (FR-002-AC-3) and `tests/it/deep_sources.rs` (TC-749/FR-256-AC-4). This PR is spec-only. |
| FND-004 | fixed | AC-7 says "at the second `<`". |
| FND-005 | fixed | The "another limit flag" clause is deleted with the rest of the CLI argument clauses; argument refusal is the driver's. |
