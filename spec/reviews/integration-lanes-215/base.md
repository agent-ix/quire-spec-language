---
id: SR-507
title: "base review of the #215 current-head integration lane and T-12 arch-lint checks"
type: SpecReview
analysis: base
scope: "FR-058, FR-059, FR-060, FR-061, IT-013, TC-156, TC-157, TC-158, TC-159"
review_set: subset
evaluated_revision: "4cd5174"
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

**PASS.** No blocking finding. Two real defects were found and fixed during
this review pass itself (not deferred): an EARS-grammar/passive-voice defect
in FR-059/FR-061 (fixed at `4cd5174`), and an overclaim in FR-060-AC-4/TC-157
that the real T12-B/T12-C run reproduces ADR-013 OBS-018 exactly — the real
run finds one additional genuine site (`src/value/node.rs:322`) OBS-018's
text does not name, and the spec now says so (fixed at `0906f29`).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | `quire validate` flagged two agentless-passive SHALL statements in FR-059 and FR-061 (`ears:missing-subject`). Resolved before this review's evaluated revision. | FR-059; FR-061 |
| FND-002 | low | FR-060-AC-4/TC-157 asserted the real T12-B/T12-C run reproduces ADR-013 OBS-018's documented sites exactly; the real run also finds `src/value/node.rs:322` (a call to `NodeKey::of` inside the constructor's own defining module, via the crate-internal `node_key_of` helper), which OBS-018's text does not name. Resolved: both documents now state the verified finding, including the site beyond OBS-018, rather than the assumption. Whether the constructor's own defining module should be an implicit allowed caller is left as an open design question for the ADR-013/#211/#213 owner, not decided by this check or this review. | FR-060; TC-157; ADR-013 OBS-018 |
| FND-003 | low | The tool crate (`integration/current-head/tool/`) initially shipped with zero unit tests, unlike the sibling `arch-lint` tool's 20 tests with negative controls. Resolved: added unit tests for `resolved_commit`, which also caught and fixed a real bug (a source string with no `#` fragment at all returned the whole string as a fabricated commit sha instead of `None`). | `integration/current-head/tool/src/main.rs` |

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
  Verification column.
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
- **FR-061's own scope boundary is real, not asserted**: independently
  confirmed against the root `Cargo.lock` that `quire-contract-model`
  (consumed as `quire-contract-ir`) and `quire-contract-ir` (consumed as
  `quire-contract-ir-historical`) are two distinct package names, each
  resolved to exactly one source, so FR-061-AC-4's "no duplicate" claim is
  true of the real lockfile, not only of a synthetic one.

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
  site plus one additional real site; `duplicate-revisions` against the real
  root `Cargo.lock` passes clean.
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
