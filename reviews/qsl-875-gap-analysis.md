---
id: SR-12166
title: "Gap analysis of quire-spec-language PR #1005: library chain memory and time"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@d191db7c102846b24c9a92121ebe59e650885b2e; PR #1005: qsl-package checked.rs, emit.rs and emit/tests.rs; against QSL-875's work items and acceptance, ADR-013 O-02, ADR-015 D-2, ADR-030 D-1, FR-099-AC-7 (TC-446), FR-276-AC-2 (TC-757), FR-307, FR-322"
review_set: subset
---
# Gap analysis of quire-spec-language PR #1005

## Summary

Ticket: QSL-875. Planless. Plan completion: not assessed. Read-only. The
criterion-to-test check was done by hand over the tests the change touches
or relies on. The PR adds no test and no `#[trace]` tag; it edits three
assertions to read `path(identity)`.

- QSL-875 work item 1 (path storage): done. The first hop plus a shared
  `Arc` replaces the per-entry `Vec`, and the path is rebuilt only for
  display. Covered by `e4_refuses_a_conflicting_diamond` (both conflict
  paths, exact vectors), `a_diamond_selecting_one_package_unifies` and the
  `closure` helper's users.
- QSL-875 work item 2 (memoise the emission): done. ADR-013 O-02 holds: the
  id is minted by the emitter and never accepted from a caller (see
  SR-12165). Identity is unchanged under the existing golden and
  format-identity tests, including
  `dependency_chain_and_order_variants_keep_format_identity`, which compares
  the original and formatted ids at every package.
- QSL-875 work item 3 (`read_import_view`): not touched. The PR body states
  it is an owner decision, and QSL-879 owns the format. Correctly out of
  scope.
- QSL-875 acceptance (peak under 2 GB at 1,000 libraries, measured with the
  locked-build wrapper, before and after reported): the PR body reports
  16.55 GB / 86 s to 1.25 GB / 34-37 s with `/usr/bin/time -v` under the
  wrapper. Not re-measured here (no builds, per the dispatch).
- FR-276-AC-2 (TC-757): `a_cancel_inside_package_stops_within_one_charge`
  runs `package` twice on one checked package. It still passes only because
  `emit_checked_with_cancel` always emits, which is commit `d191db7`.
- FR-307, FR-322: no wire, lock or preimage change. The path is not
  serialized.
- No new production code without an owning requirement.

## Verdict

PASS. No blocking gap. FND-001 is a test gap on the new drop path, recorded
as medium. It does not block.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | ADR-030 D-1 / FR-099-AC-7 (TC-446): the new `release_into` branch that moves each selection's `Via` `Arc` onto the drop stack has no any-length test. `dependency_chain`, the fixture behind `a_dependency_chain_of_any_length_drops_on_a_small_stack` (100,000 links, 128 KiB), builds every package with empty `selections`, so deleting the selection loop would leave that test green. With the loop gone, the dependency `Arc`s stay shared with `Via` entries, `try_unwrap` fails, and the derived field drop recurses once per link. Only qsl-replay's 1,000-library compile at 512 KiB exercises the real shape. Give `dependency_chain` the production shape (each link's `selections` holding `Via` entries into the next package). | qsl-package/src/checked.rs:313-320; qsl-package/src/checked.rs:624-658 |
| FND-002 | low | No test pins the memo directly: a second `emit_checked` equals the first, `link_with`'s recomputed id equals a fresh emit of the import, and `bind_abstraction` clears it. The behaviour is preserved by the existing id and golden tests, but a memo regression that serves a wrong emission would be caught only indirectly. | qsl-package/src/emit.rs:1002-1028; qsl-package/src/checked.rs:447-453 |
| FND-003 | low | QSL-875 says that once the peak is under the lane cap, QSL-828's separate dependency-chain invocation in `make test-deep-format` can be merged back into the light run. The PR leaves the Makefile unchanged and the ticket records no follow-up for it. | QSL-875 Acceptance |
