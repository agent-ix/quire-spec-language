---
id: SR-752
title: "QSL-302 code review (with rust-review lane) of PR 499"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language; Makefile; qsl-package/src/checked_v2/tests.rs; qsl-package/src/checked_v2.rs (read_checked_package_v2, map_refusal_code; unchanged); quire-contract-ir crates/quire-contract-model/src/checked_package/v2/mod.rs (frame_eligibility, frame_defect, validate_frame_semantics; dependency, unchanged)"
review_set: subset
---
## Summary

Ticket: QSL-302. PR: quire-spec-language#499.
Methods: code-review with the rust-review lane folded in.

The PR adds no admission logic. It adds a `QSPEC_DIR`-gated conformance test
over QSpec's `frame_mutations`, three always-run unit tests, and a
`make conformance` step.

The coder's central claim holds. I read it in the IR dependency myself:

- `frame_eligibility` is FR-340's closed table. `modifies` takes
  `relation/relationship` or `model/field_declaration`. `creates` and `deletes`
  take `model/object_type` or `model/process`. Every other form is `NONE`, and
  the match has no wildcard.
- `frame_defect` refuses `missing_declaration`/`missing-name` for an entry
  that is not in the frame's `dependencies`, and also for a declared
  dependency that names no node. It refuses
  `invalid_model_binding`/`malformed-declaration` for an ineligible kind. A
  meaning-join defect outranks a canonical-order defect. The winner is picked
  by (member order, entry digest). The locus is the entry.
- `validate_frame_semantics` visits frames in ascending digest order and is
  called from `validate_graph` (mod.rs:2122).
- `read_checked_package_v2` (checked_v2.rs:642) hands the whole wire to IR's
  `read_checked_package`. There is no second, weaker path. `map_refusal_code`
  maps `MissingDeclaration` and `InvalidModelBinding` one-to-one
  (checked_v2.rs:507, 509).

The conformance test is faithful. It replays all 26 vectors against the real
frame node in `positive-all-families.json`. It replaces the frame's
`dependencies`, `modifies`, `creates` and `deletes`, appends `second_frame`
where one is given, and rebuilds `identity_projection` and `package_id`. It
asserts the exact IR `code`, `cause` and `locus` digest for each vector.
All 26 vectors are refusals. A vector that referenced a node missing from the
fixture would get `missing_declaration` rather than the expected
`invalid_model_binding`, so the test would catch a broken fixture reference.

Unit tests: the claimed red/green mutation (an ineligible member in the
positive fixture) is exactly what
`tc_053_frame_entry_of_an_ineligible_kind_refuses_as_invalid_model_binding`
keeps as a permanent negative control. The same `frame_fixture` puts
`object_type` into `modifies` and asserts the exact refusal. The structure
supports the claim.

"Bug fix" claim: main has no `source_map_entry` helper. `git grep` on
origin/main finds only the test name
`a_source_map_entry_naming_an_unknown_node_refuses`. The new
`source_map_entry_for_digest` is correct, because it keys each entry by the
node's own `node_id.digest`. No existing caller changes behaviour. The PR
body's "fixed a real bug" describes a defect in the coder's own draft, not in
merged code (FND-005).

Makefile: the `CONFORMANCE_FRAME_TEST` step matches the sibling steps. It has
its own `cargo test --locked ... --exact`, captures and echoes output,
propagates the exit code, and checks an anchored stdout marker. There is no
drift. No QSpec content is copied: vectors and fixture are read at run time
from `$QSPEC_DIR`.

Gates, re-run by me in this worktree with a fresh `CARGO_TARGET_DIR`:
- `cargo clippy -p qsl-package --all-targets --all-features -- -D warnings`
  was clean.
- `cargo test -p qsl-package --lib checked_v2::tests::` gave 42 passed.
- `QSPEC_DIR=<qspec-main> make conformance` exited 0 and printed
  `conformance: 26 frame-body mutation vectors matched`.
- `make ci` exited 0. The log has 93 `test result: ok` lines and 0 FAILED.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The evidence targets a superseded FR-340. QSpec main moved on (#163, STD-111): `frame_mutations` now has 30 vectors, and each `modifies` entry is an object (`{declaration, kind}`), not a bare digest. Pointed at current QSpec main, this test panics at tests.rs:1983. The pre-existing C-14 and I2 conformance tests also fail there, so the drift is lane-wide and this PR did not cause it. But the claim that "FR-340 is already implemented" is true only for QSpec before #163 and the IR then in use. It needs a follow-up ticket: bump IR when IR implements the #163 frame body, then re-spell `frame_entries`. | qsl-package/src/checked_v2/tests.rs:1979-1986; qsl-package/src/checked_v2/tests.rs:2035 |
| FND-002 | low | `frame_entries` uses a bare `digest.as_str().unwrap()`. When the QSpec spelling changes (FND-001), the test panics with `called Option::unwrap() on a None value` and gives no vector name. Use `unwrap_or_else(\|\| panic!("{name}: ..."))`, or pass `name` in, as the rest of the test does. | qsl-package/src/checked_v2/tests.rs:1983 |
| FND-003 | low | The tests assert only IR's raw `CheckedPackageRefusalCode`. They never assert QSL's own mapped `V2ReadRefusal::code()` (`map_refusal_code`). That mapping is the only QSL code on this path, and no test in the crate asserts `Code::MissingDeclaration` or `Code::InvalidModelBinding` from an I2 envelope refusal. A wrong arm, such as `InvalidModelBinding => InvalidPackage`, would pass every test. Add `assert_eq!(refusal_outer.code(), Code::…)` to the two refusal unit tests. | qsl-package/src/checked_v2/tests.rs:2293-2300; qsl-package/src/checked_v2/tests.rs:2320-2330; qsl-package/src/checked_v2.rs:438 |
| FND-004 | low | Minor test hygiene. (a) `frame_node_index` uses `.expect("fixture carries exactly one frame node")` after `position`, which finds the first frame and never checks that it is the only one. (b) `refresh_frame_identity` and `valid_envelope_over` each carry their own copy of the projection logic (clone each node, drop `occurrences`). One `projection_of(&[Value])` helper would serve both. (c) The `tc_053_frame_entry_outside_dependencies…` doc says "whether or not that digest names a real node elsewhere", but the test only covers the case where the node is absent from the graph. The undeclared-real-node case is covered only by the `QSPEC_DIR`-gated vector `undeclared-entry`. | qsl-package/src/checked_v2/tests.rs:1968-1975; qsl-package/src/checked_v2/tests.rs:1995-2012; qsl-package/src/checked_v2/tests.rs:2177-2190; qsl-package/src/checked_v2/tests.rs:2270-2273 |
| FND-005 | low | The PR body says it "fixed a real bug ... a `source_map_entry` test helper was double-hashing". No such helper exists on main. The fix was to the coder's own unmerged draft. Nothing in the diff changes existing behaviour, but the description overstates the change for anyone reading the history. Correct the PR body before merge. | qsl-package/src/checked_v2/tests.rs:2162-2169 |

## Verdict

Approve with findings. There are no high findings. The central claim is
verified in IR's code: FR-340's eligibility table, the refusal split and the
precedence are enforced there. QSL's I2 read delegates to that
code, and the conformance test replays every vector with exact code, cause
and locus. FND-001 needs a follow-up ticket, not an in-PR fix. FND-002 to
FND-005 are cheap to fix in this PR.
