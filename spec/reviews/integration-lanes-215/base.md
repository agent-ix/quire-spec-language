---
id: SR-507
title: "base review of the #215 current-head integration lane and T-12 arch-lint checks"
type: SpecReview
analysis: base
scope: "FR-058, FR-059, FR-060, FR-061, IT-013, TC-156, TC-157, TC-158, TC-159"
review_set: subset
evaluated_revision: "6f287b3"
---

## Summary

Reviewed FR-058 through FR-061, IT-013 and TC-156 through TC-159 (all new)
plus the `spec/spec.md` frontmatter/table additions, against the Checklist and
the six test-coverage rules, and against the real implementation
(`tools/arch-lint/`, `integration/current-head/`) at `4cd5174`. This is a
`subset` review (base plus `dependency` and `evidence`, the two analyses most
applicable to a change whose entire content is a dependency-graph lane and a
set of checks whose central requirement is honest, non-vacuous reporting);
`failure-domain`, `integrity`, `risk-complexity`, `scope-boundary` and
`ears-conformance` were not run as separate analyses, proportionate to a
scoped architecture-tooling change, not a new domain model.

## Verdict

**PASS.** No blocking finding. Three real defects were found and fixed during
this review pass itself (not deferred): an EARS-grammar/passive-voice defect
in FR-059/FR-061 (fixed at `4cd5174`); an overclaim in FR-060-AC-4/TC-157 that
the real T12-B/T12-C run reproduces ADR-013 OBS-018 exactly — the real run
finds one additional genuine site (`src/value/node.rs:322`) OBS-018's text
does not name, and the spec now says so (fixed at `0906f29`); and a
test-tracing-tag gap (FND-004) where new tests used a stale doc-comment
convention instead of this repo's real `#[trace(...)]` attribute, which left
all 18 of FR-058 through FR-061's ACs showing as `unbacked_rows` in `quire
coverage` despite real, passing tests existing.

## Revision update (#249 review round 2)

A second review round (external PR review, REVISE verdict) found real gaps
this base review's PASS verdict had not caught, all fixed in the same PR
rather than deferred, plus three owner rulings (R1, R2, R3) this section
records verbatim rather than re-litigating:

- **R1 (T12-B minting sites)**: `node_key_of(` was added to T12-B's call
  patterns and `value::node` (the helper's own defining module) to its
  allowed-caller prefixes. Run against real QSL head, T12-B now finds **five**
  real sites, not the one this review's FND-002 recorded:
  `value/enumeration.rs:117,162`, `value/unit.rs:198,311` and the OBS-018 site
  `value/model_query.rs:108`. None of these five are among the three modules
  ADR-011 §1's S3 "today" mapping names for the `check` stage
  (`value::expression::check`, `model::checked_dispatch`, `value::library`) --
  a real discrepancy between that mapping and QSL's real head, reported here
  and in FR-060's Status section, not resolved. Remaining work: #211.
- **R2 (duplicate-revisions keying)**: the check now groups by ecosystem
  *repository* (source URL, via a `graph::classify` function shared with
  FR-059's edge extraction), not by crate name. Run against QSL's real root
  `Cargo.lock`, this now reports a real duplicate: `quire-contract-ir` and
  `quire-contract-model` are the same repository at two different revisions.
  `arch-lint duplicate-revisions --lockfile Cargo.lock` exits `1`. This
  review's Dependency analysis section and FND-004/FR-061-AC-4 below
  previously asserted the opposite ("no duplicate," "true of the real
  lockfile"); both are corrected below rather than left standing. This PR does
  not change the root lock to make the check pass.
- **R3 (the lane's own lock)**: FR-061 now also covers
  `integration/current-head/Cargo.lock`. Three new `[patch]` entries in
  `integration/current-head/Cargo.toml` (quire-contract-ir/-model via the
  existing vendored clone, quire-contract-runtime via a new vendored clone,
  quire-spec-language via a direct path) plus a real `cargo update` converge
  that lock to exactly one revision per repository. `make
  arch-lint-duplicate-revisions-lane` (new target) confirms: exit `0`, "PASS:
  one revision per quire-ecosystem crate." Building the lane's real manifest
  at real current heads (a byproduct of this convergence work, not a designed
  fixture) surfaced a genuine, new cross-repository incompatibility: real
  quire-contract-codegen head fails against real quire-contract-**ir** head
  (`error[E0560]: struct CounterexamplePacket has no field named witness`,
  `bounded_kani_corpus.rs:196`; `CounterexamplePacket` is defined in
  quire-contract-ir's `src/kani/replay.rs:45`, not in quire-contract-runtime
  -- corrected in #249 review round 3 M-1, which had misattributed this to
  CG/RT), reported in FR-058's Status section as real, current evidence, not
  remediated here.

Also fixed this round: the lane's `prepare` step now runs `cargo update`
against the lane's own manifest and `revision-log` fails on a
resolved-sha/`git ls-remote` mismatch (HIGH-1); `arch-lint api-surface` and
`api_surface.rs` rules now carry an explicit role (`--qsl`/`--cg`) so a
CG-role rule (T12-A) is evaluated against a CG tree instead of vacuously
against QSL's own, with a negative-control test over a synthetic
consumer-shaped tree (HIGH-2); `check-incompatible-fixture` now requires a
real `error[E0` compiler diagnostic in stderr before emitting the FR-058-AC-3
marker, with a negative-control test for a missing-manifest build failure
that is not that diagnostic (HIGH-3); T12-A now has its own CG scan root
instead of silently flipping to PASS against QSL's own tree once
`src/replay.rs` exists (MEDIUM-4); the check's own report now states both its
renamed-import limitation and a second, previously-undocumented one (a call
pattern found inside a comment or string literal is not excluded) (MEDIUM-5);
T12-A's `pending_reason` now names `src/replay.rs` explicitly, so
`tc_arch_lint_api_surface_002` asserts that name rather than a value it set
itself (MEDIUM-6); **MEDIUM-8 withdrawn (reviewer error, #249 review round
4)**: the finding claimed the PR body's "`make ci` runs Kani proofs" was
false; it is not false -- the reviewer's own grep of `Makefile`/`.github`
missed a proof invoked from test code. `make ci` runs `cargo test --locked
--workspace` twice, once per feature lane, which includes
`tests/configversion_backends.rs`, whose plain `#[test]` functions shell out
to `cargo kani` (cargo-kani 0.67.0, pinned by version and SHA-256 at
`:509-524`; the proof runs via `execute_kani` at `:807`, invoked from the
test at `:861`), so Kani/CBMC proofs do run under `make ci`, and `make ci`
therefore depends on a locally installed, SHA-pinned cargo-kani 0.67.0 on
PATH and fails on a machine without it. What `make ci` does **not** run is
any `arch-lint` target (`arch-lint-api-surface`,
`arch-lint-duplicate-revisions`, `arch-lint-duplicate-revisions-lane`,
`arch-lint-direction`) or the `integration-current-head` lane, so a green
`make ci` says nothing about FR-058/059/060/061 or ADR-011 T-12's
enforcement. The PR body's claim was not removed as a fix; the claim was
correct all along; `integration/current-head/tool/src/
main.rs`'s `cargo metadata` call now passes `--locked`; the dead
`is_ecosystem_crate`/`"quire_contract_model"` underscore check was removed
along with the crate-name-keyed logic it supported; `RuleOutcome::rule_id` is
now read (used in the printed report); the tautological `const _: () =
assert!(MAX_ANALYSIS_BYTES > 0)` in `tests/contract.rs` was dropped (a
`const` bound provable at compile time is not a runtime gate); IT-013-SC-04
now has a `#[trace(...)]` tag and a real automated check
(`backend_crates_resolve_from_head_not_a_registry_release`) instead of only
an inspection step; and the Makefile now states, next to `arch-lint`'s own
target, why it exits 1 by design today and that it joins `ci:` once #211/#213
remediate (Remaining work: #211).

FND-004's coverage-overclaim is corrected below rather than left standing.
Before this round's fixes, real per-document backed count (an AC backed by an
automated, symbol-tagged test, per `quire coverage --scope . --json`'s own
`minted_targets[].backed` field) was FR-058 1/4, FR-059 5/6, FR-060 3/4,
FR-061 3/4 = 12/18, not 18/18. This round added a real negative-control test
tagged `FR-058-AC-3` (the missing-manifest case, HIGH-3), which
`quire coverage` now reports as backed; the real, current count, re-measured
against this document's `minted_targets` output rather than assumed, is
**FR-058 2/4, FR-059 5/6, FR-060 3/4, FR-061 3/4 = 13/18**. The remaining five
unbacked (`FR-058-AC-1`, `FR-058-AC-4`, `FR-059-AC-6`, `FR-060-AC-4`,
`FR-061-AC-4`) are verified by each requirement's TC's own documented
**manual** real-data-run step (TC-159's, TC-156's, TC-157's and TC-158's own
`Automation` fields already say so), which this round ran for real and
recorded real output for (FR-058's and FR-060's Status sections, this
document's Revision update section above) -- a real but non-automated
verification, not an automation gap silently passed over. `unbacked_rows`/
`status_lies` count is a different, narrower claim than "has a passing test":
a manual TC that a symbol correctly cites is not `unbacked_rows` even though
no automated test executes it, so the 0-`unbacked_rows`/0-`status_lies`
figures in FND-004's Resolved clause below remain accurate on their own,
narrower terms.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | `quire validate` flagged two agentless-passive SHALL statements in FR-059 and FR-061 (`ears:missing-subject`). Resolved before this review's evaluated revision. | FR-059; FR-061 |
| FND-002 | low | FR-060-AC-4/TC-157 asserted the real T12-B/T12-C run reproduces ADR-013 OBS-018's documented sites exactly; the real run also finds real call sites OBS-018's text does not name. Resolved: both documents now state the verified finding rather than the assumption. **Superseded by the Revision update above (R1)**: after `node_key_of(` was added to T12-B's call patterns and `value::node` exempted as the helper's own defining module, the real, current count is five sites (`value/enumeration.rs:117,162`, `value/unit.rs:198,311`, `value/model_query.rs:108`), not the one site this finding originally recorded; FR-060 and TC-157 state the five-site count, not the one-site count. Whether the constructor's own defining module should be an implicit allowed caller is left as an open design question for the ADR-013/#211/#213 owner, not decided by this check or this review. | FR-060; TC-157; ADR-013 OBS-018 |
| FND-003 | low | The tool crate (`integration/current-head/tool/`) initially shipped with zero unit tests, unlike the sibling `arch-lint` tool's 20 tests with negative controls. Resolved: added unit tests for `resolved_commit`, which also caught and fixed a real bug (a source string with no `#` fragment at all returned the whole string as a fabricated commit sha instead of `None`). | `integration/current-head/tool/src/main.rs` |
| FND-004 | low | None of the new tests in `tools/arch-lint/*.rs`, `integration/current-head/tool/src/main.rs` or `integration/current-head/tests/contract.rs` carried this repo's real, current test-tracing tag (`use ix_trace_rs::trace; #[trace("TC-...", "FR-...-AC-...")]`, confirmed via real, current examples such as `tests/text_enum_identity.rs:72` and `tests/integer_lowering.rs:68`); a stale, free-text doc-comment naming a test id in prose is not the convention (NFR-005's own `## Verification` text says so explicitly: "Legacy doc-comment tags are not the convention for new tests"). `quire coverage --scope . --json` correctly reported all 18 of FR-058 through FR-061's ACs as `unbacked_rows` as a result. Resolved: added `#[trace(...)]` attributes (and the `ix-trace-rs` dev-dependency, where a manifest did not already carry it) to every relevant test. **Correction (superseding this finding's closing clause, #249 review MEDIUM-7)**: the original text of this finding claimed "real, passing tests existing for every one of them" for all 18 ACs; that overclaimed. The real, per-document *backed* count (an AC covered by an automated, symbol-tagged test that `quire coverage`'s own `minted_targets[].backed` field recognizes) is now, after this round's fixes, FR-058 2/4, FR-059 5/6, FR-060 3/4, FR-061 3/4 = 13/18, not 18/18 -- see the Revision update section above for the remaining five (`FR-058-AC-1`, `FR-058-AC-4`, `FR-059-AC-6`, `FR-060-AC-4`, `FR-061-AC-4`), each genuinely verified only by its TC's own documented manual real-data-run step, not an automated test. | `tools/arch-lint/graph.rs`; `tools/arch-lint/metadata.rs`; `tools/arch-lint/api_surface.rs`; `tools/arch-lint/duplicate_revisions.rs`; `integration/current-head/tool/src/main.rs`; `integration/current-head/tests/contract.rs` |

## Scope and provenance

This review covers only the artifacts #215 adds: the four new FRs, one new
IT, four new TCs, and the `spec.md` relationship/table entries that register
them. It does not reopen FR-057 or any other existing requirement, and it does
not re-review ADR-011 or ADR-013 themselves (both are Proposed, unchanged by
this PR; this PR implements enforcement mechanisms ADR-011 §7.1 T-12 already
names).

Evaluated against real data, not only synthetic fixtures: `arch-lint
direction` was run against real local checkouts of quire-contract-ir,
quire-contract-runtime and quire-contract-codegen; `arch-lint api-surface` and
`arch-lint duplicate-revisions` were run against this repository's own real
`src/` tree and `Cargo.lock` at `4cd5174`. `cargo fmt --check` and `cargo
clippy --all-targets -- -D warnings` were run and passed clean (after fixes,
see Checklist and Test Coverage below) over the root workspace (`arch-lint`)
and each of the current-head lane's three separate manifests (lane, tool,
fixture). `cargo test` passed for all four: 20/20 (`arch-lint`), 4/4 (lane
contract tests), 3/3 (tool unit tests), and the intentionally incompatible
fixture (a fifth, deliberately-failing build, verified via `cargo build`
directly and via `current-head-lane check-incompatible-fixture`).

## Checklist and dispositions

ID formats: FR-058 through FR-061 (3-digit, sequential, no gap since FR-057);
IT-013 (sequential since IT-012); TC-156 through TC-159 (sequential since
TC-155). All four ids were allocated after scanning every remote branch's
frontmatter `id:` fields (not local `main` alone, and not a naive substring
grep, which a range-mention like "SR-479 to SR-489" would corrupt), so none
collides with an id already allocated on an unmerged branch.

This repository's own FR template (every existing FR under `spec/functional/`)
does not link a `US-XXX` user story; the generic Checklist's "Related US
linked" item does not apply here, consistent with the repo's own established
convention (§0 of `/rust-review`'s "project conventions outrank generic skill"
principle, applied the same way to spec conventions).

FR quality: each of FR-058 through FR-061 has Description, Inputs, Outputs,
Behavior, an Acceptance Criteria table and a Status section naming what is
implemented (`tools/arch-lint/*.rs`, `integration/current-head/`) and, for
FR-058, the two design questions this change does not settle (where hosted CI
runs the lane; how QSpec artifacts would be sourced at head). Error conditions
are documented as typed outcomes (`Code` enum in `tools/arch-lint/error.rs`;
`pending`/`Live`/violation-list in `api_surface.rs`), not prose-only.

Test coverage (six rules), scoped to what applies to an architecture-check
requirement set (no state machine, no numeric option matrix):

- Coverage: every AC in FR-058..061 traces to at least one TC (FR-058 AC-1..4
  → TC-159; FR-059 AC-1..6 → TC-156; FR-060 AC-1..4 → TC-157; FR-061 AC-1..4 →
  TC-158). Verified by reading each FR's Acceptance Criteria table's
  Verification column, and machine-verified via `#[trace(...)]` tags plus a
  clean `quire coverage --scope . --json` run (see FND-004 and the Revision
  update above): 0 of these 18 ACs remain in `unbacked_rows`, 0 `status_lies`,
  and 13 of the 18 (all but `FR-058-AC-1`, `FR-058-AC-4`, `FR-059-AC-6`,
  `FR-060-AC-4`, `FR-061-AC-4`) show `backed: true` in the same run's
  `minted_targets`; repo-wide, 528/578 rows are `backed: true`.
- Option permutation: not applicable; none of the four FRs declares an
  `-OPT-` option set.
- Constraint boundary: FR-061's path-dependency-vs-git-source boundary (no
  `source` field is its own distinct value, never merged) is tested (TC-158
  step 4); FR-060's `::`-segment-boundary-vs-string-prefix boundary
  (`model_query` under a `model` allow-list) is tested (TC-157 step 4,
  `tc_arch_lint_api_surface_005`).
- Error path: FR-059's FB-05/FB-11 violation reporting, FR-060's
  pending/failing reporting and FR-061's duplicate-source reporting are each
  tested with a real negative control, not only a positive case.
- State transition: not applicable; none of the four checks is stateful
  across runs.
- Edge case: FR-059's dev-vs-normal edge-kind distinction and the
  CG-normal-only exception; FR-060's pending-vs-vacuous-pass distinction;
  FR-061's non-ecosystem-crate-is-ignored case. All tested with a named
  negative control in the corresponding `tools/arch-lint/*.rs` unit test
  module.
- TC fields complete: TC-156 through TC-159 each carry Priority, Target
  Integration, Automation and Dependencies (Upstream/Downstream).

Cross-referencing: every new FR's `relationships` frontmatter targets
`ix://agent-ix/quire-spec-language/ADR-011` (and FR-060 also targets ADR-013);
every new TC's `relationships` frontmatter verifies its FR; IT-013 verifies
FR-058. `spec/spec.md`'s `relationships` list and Requirements table were both
updated (not only one of the two, a gap this review specifically checked for
since it is easy to update one and miss the other). Full ids are used
throughout; markdown links resolve to files this PR adds. Terminology is
consistent with FR-057's existing pattern (Description / Inputs / Outputs /
Behavior / Acceptance Criteria / Dependencies / Status) and with ADR-011's own
vocabulary (FB-05, FB-11, T-12, layer K) and ADR-013's (O-04, O-05, OBS-018).

## Dependency analysis

The PR's entire second deliverable (FR-058/IT-013) exists to police a
dependency graph, and its third/fourth/fifth (FR-059/060/061) exist to police
crate- and module-level dependency direction and revision uniqueness, so
dependency correctness is this PR's central claim, not an incidental property
to spot-check.

- **Root `Cargo.lock` untouched**: verified directly, not only asserted.
  `git diff --stat Cargo.lock` against the commit this branch started from is
  empty; the only root `Cargo.toml` change is the additive `[[bin]] name =
  "arch-lint"` entry `tools/arch-lint/main.rs` needs, which does not add a new
  crate dependency (arch-lint depends only on `serde_json`, `thiserror` and
  `tempfile`/`dev-dependencies`, all already present).
- **The `[patch]` mechanism is real, not asserted**: independently confirmed
  this session that Cargo refuses a `[patch]` whose replacement is a
  different branch/rev of the *same* git URL ("patches must point to
  different sources"), which is why the lane redirects
  quire-contract-ir to a local vendored *path* clone rather than a second git
  reference, and why `tool/` is a separate, dependency-free crate (a manifest
  with a `[patch]` resolves its whole graph, including that patch, before
  building anything in it, so the tool that creates the patch's prerequisite
  cannot live inside the manifest the patch belongs to).
- **Three separate lockfiles, one root lockfile**: `integration/current-head/`,
  `integration/current-head/tool/` and
  `integration/current-head/fixtures/incompatible/` each declare their own
  `[workspace]` and carry their own committed `Cargo.lock`; none is a member
  of the root `[workspace]`. The `stub-quire-contract-model` fixture crate
  initially had no `[workspace]` table of its own (FND found and fixed this
  session: without one, a direct tool invocation against it — `cargo fmt`,
  `cargo metadata` — errors "believes it's in a workspace when it's not",
  since it sits under the incompatible fixture's directory tree without being
  declared a member or excluded).
- **FR-061's own scope boundary, corrected (superseded by R2, #249 review)**:
  this bullet originally read that `quire-contract-model` and
  `quire-contract-ir` are "two distinct package names... not a duplicate of
  one name," and that FR-061-AC-4's "no duplicate" claim held against the real
  lockfile. Under R2's repository-keyed rule, that is now known to be false:
  both packages classify to the IR *repository*, resolved to two different
  revisions, and `arch-lint duplicate-revisions --lockfile Cargo.lock` exits
  `1` against the real root lockfile. FR-061-AC-4 is rewritten to state the
  repository rule instead of the (now-known-false) "no duplicate" outcome;
  this PR reports the root lock's real failure rather than changing the lock
  to make the check pass.

## Evidence analysis

FR-060's central requirement is that the check "report honestly," including a
pre-existing finding it must not tune away. This makes the honesty of this
PR's own reported evidence a first-class thing to check, not a formality.

- Real command output was captured for all three arch-lint subcommands
  against real inputs (not only against the unit-test fixtures): `direction`
  against real local checkouts of the three backend repositories reproduces
  ADR-011 OBS-029's FB-05 violation and its FB-11 cycles; `api-surface`
  against this repository's own `src/` at `4cd5174` reports T12-A pending,
  T12-C failing at both OBS-018 sites, and T12-B failing at the one OBS-018
  site plus one additional real site (**superseded by R1, #249 review**: with
  `node_key_of(` added to T12-B's call patterns, the real, current count is
  five sites, not two -- see the Revision update section above);
  `duplicate-revisions` against the real root `Cargo.lock` passes clean
  (**superseded by R2, #249 review**: keyed on repository rather than crate
  name, the same real lockfile now fails -- see the Revision update section
  above and the corrected Dependency analysis bullet).
- Every exit code quoted above was re-captured without piping through
  `tail`/`head`, after this session caught itself doing exactly that once (a
  `tail`-piped exit code silently reflects `tail`'s own status, not the
  command being measured) — the corrected, direct capture is what this
  review and the PR body report.
- FR-060-AC-4/TC-157's fix (FND-002 above) is itself an instance of the
  evidence discipline FR-060 requires: the first-written version of both
  documents claimed an exact match to ADR-013 OBS-018's documented sites
  before the real check had actually been run against real QSL head; running
  it surfaced one more genuine site, and both documents were corrected to
  state what the check actually found rather than what was assumed it would
  find.
- The intentionally incompatible fixture (FR-058-AC-3) was verified
  end-to-end, not only by inspection: `cargo build` on the fixture manifest
  directly fails with real unresolved-import compiler errors naming QSL's own
  imported types, and `current-head-lane check-incompatible-fixture` reports
  the stable marker line and exits successfully (it correctly detected the
  expected failure). An earlier version of this fixture (patching to a real
  historical git revision) was abandoned mid-session after direct
  verification showed it unexpectedly *compiled* — a false premise this
  session caught before shipping it, replaced with a deterministic,
  deliberately empty stub crate that cannot pass by accident.

## Reviewed implementation clarification

`tools/arch-lint/` and `integration/current-head/` (including its `tool/` and
`fixtures/incompatible/` sub-crates) were read in full and reviewed against
`agent-skills/rust-review`'s checklist during this same session, ahead of this
spec review, per this repository's CLAUDE.md ("`/code-review` includes the
actual `agent-skills/rust-review/SKILL.md`... at substantial PR checkpoints").
That pass found and fixed: a real logic bug in `resolved_commit` (FND-003
above); two clippy findings (`assertions_on_constants` in
`tests/contract.rs`, `items_after_test_module` in `tool/src/main.rs`); and a
`cargo fmt` gap across six files, none of which changed any behavior. No
`unwrap`/`panic!`/`unsafe` was found on a non-test path; both binaries
(`arch-lint`, `current-head-lane`) carry `#![forbid(unsafe_code)]` and typed
or explicit `Result` error handling throughout.
