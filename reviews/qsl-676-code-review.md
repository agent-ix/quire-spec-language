---
id: SR-17011
title: "Code review of quire-spec-language PR #1443: frame occurrence region at the first naming clause's operation reference"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@f6219be1e28138f73bbaa7de2189a867577e67c4; PR #1443 (QSL-676) diff origin/main...HEAD: qsl-semantics/src/check/assemble.rs, qsl-semantics/src/check/lowering.rs, qsl-semantics/src/check/lowering/state.rs, qsl-semantics/src/check/mod.rs, qsl-replay/src/spine/clause/tests.rs, qsl-replay/src/spine/clause/tests/frame_occurrence_region.rs, tests/it/formatter_identity.rs"
review_set: subset
---
# Code review of quire-spec-language PR #1443

## Summary

Ticket: QSL-676. Code review with the rust-review lane folded in. Base is
origin/main 0f238e5f9.

What changed:

- `assemble.rs` records, for each `pre`/`post` clause, the span from the
  start of its context to the end of its operation name (`M::T::op`). The
  span goes into `declared_type_spans` under a spaced synthetic name,
  `state clause {index} operation`. Temporal and hybrid sites already use
  this pattern.
- `AnchorInput` gains `reference`. `register_frame_occurrences` places the
  frame's `generated` occurrence at the `reference` of the first anchor that
  names the operation (`BTreeMap::entry().or_insert`, in anchor-vector
  order). The frame node's key and the occurrence ordinal still come from
  `anchor.location` and the (DeclarationKey, name) sort, so the change only
  moves the region.

Premise check:

- FR-105 Outputs, FR-104 Requirements (lines 278-280) and FR-105-AC-9 on
  main (added by STD-279, 4ac5935ad, #1251) say what the coder claims: one
  region, the span of the qualified operation reference in the first naming
  clause or attempt.
- Reproduced on origin/main: with main's qsl-semantics and the PR's test,
  the region is (548, "self.versionNumber = pre(self.versionNumber)"),
  which is the clause body. AC-9 expects (509,
  "Config::ConfigVersion::attemptUpdate").
- The ticket's original failure (UnlocatedOccurrence, Generated ordinal 0)
  is gone on main. It was fixed by c93bc1481, the squash of GitHub PR #676,
  in the commit "fix: locate clause population type occurrences". That
  commit makes `lower_state_clause` record a `type` occurrence for each
  population object node.
- Reverting that hunk on main makes
  `state_subtype_population_keeps_checked_identity` fail ("population type
  is recorded": `generated` instead of `type`). The subtype population's
  object node falls back to a `generated` occurrence at
  `generated_location`, which has no region. That is the reported failure.

Consumers checked:

- `package_id`: the identity preimage in `qsl-package/src/emit.rs` covers
  the nodes and selections, not `source_map`. Node ids and `package_id` do
  not change. Only the `source_map` region bytes of the frame entry change.
- No golden fixture pins those bytes, and the workspace tests pass.
- `declared_type_spans` is read only by region resolution (region.rs). The
  new entries are inserted after `type_spans.extend(...)` at assemble.rs:3295,
  so name resolution never sees them. A spaced name cannot collide with a
  declared type.
- Frame records key on the occurrence `Origin` (node, role, ordinal), not
  on the location. Replay and frame records are unaffected.
- The clause index spaces in assemble.rs and mod.rs line up: `state_clauses`
  returns Err rather than drop a clause, so success keeps the index.
- API: everything new is `pub(crate)`.

Rust lane:

- The production change adds no `unwrap`, `expect` or panic.
- Integer conversions are confined to tests.
- string-edge is clean.

Checks run by the reviewer at f6219be1e in its own target directory, one
cargo job at a time:

- `cargo fmt --all -- --check`: clean.
- `cargo run --locked -p xtask -- string-edge`: clean.
- `cargo clippy --locked --workspace --all-targets -- -D warnings`: clean.
- `cargo test --locked --workspace --no-fail-fast`: everything passes
  except two failures, both unrelated to this PR:
  - `qsl-route` `cargo_deny_refuses_a_graph_that_depends_on_a_denied_crate`.
    This is known env noise: local cargo-deny 0.20.2 rejects `--config`.
  - `xtask` `typestate_scan::tests::tc_260_value_type_ref_is_the_two_member_union`.
    It fails identically on origin/main, so it predates the PR.
- All 18 formatter-identity state cases pass, along with
  `state_generated_occurrences_have_authored_regions` and the new AC-9 test.

Mutations, re-run by the reviewer against the head (each reverted):

| Mutant | AC-9 test | formatter test |
| --- | --- | --- |
| Last clause wins (`insert` instead of `or_insert`) | killed (579 vs 500) | survives |
| Reference starts at the operation name | killed | killed |
| Reference ends at the context end | killed | killed |
| Main's placement (main qsl-semantics) | killed (548 vs 509) | survives (FND-001) |

## Verdict

**PASS with one low finding.** The region rule matches FR-105-AC-9 as
amended by STD-279. The ordinal and identity rules are untouched. Node ids
and `package_id` are unchanged.

The edit to the formatter test does not weaken it. Every assertion it held
on main still runs for the clause-rooted occurrences. It does not, however,
pin the new placement (FND-001).

The protocol-attempt half and the order of clauses before attempts are not
covered. Both are deliberately out of scope and tracked by QSL-1055.

On scope: the original QSL-676 failure was fixed by #676, and the 2026-10-22
spec-coverage audit blocked QSL-676 on STD-279. This PR therefore carries
QSL-676 as re-scoped. The ticket title and description should be updated to
the STD-279 rule rather than a new ticket being split off.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The edited `state_generated_occurrences_have_authored_regions` branch only runs when a generated occurrence's origin is `TypeDeclaration`. With main's placement (frame at the clause body root) the branch never runs and the test still passes, so the edit tolerates the new placement rather than pinning it. FR-105-AC-9 itself is pinned by the new qsl-replay test. Fix: count the `TypeDeclaration` branch hits and assert exactly one (the `attemptUpdate` frame occurrence). | tests/it/formatter_identity.rs:662-681 |
