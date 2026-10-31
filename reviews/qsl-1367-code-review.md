---
id: SR-1948001
title: "Code review of PR #1948 (QSL-1367): model state keys carry relationship extents, code-review"
type: SpecReview
analysis: code-review
date: 2026-10-31
scope: "agent-ix/quire-spec-language; PR #1948 git diff origin/main...a8823ee74 (branch qsl-1367-extents): qsl-eval/src/simulation/model/encode.rs, qsl-eval/tests/it/model_simulation.rs; ticket QSL-1367"
review_set: subset
---
# SR-1948001: code review of PR #1948 (QSL-1367)

## Summary

Ticket: QSL-1367. PR: quire-spec-language#1948, head a8823ee74. Method: code-review with the rust-review lane. The diff adds `extents_enc` and a `relationships` entry to `semantic_enc`, so a model state's key carries its `ModelState.extents`. It also adds one integration test. The original bug is real and the fix closes it: with encode.rs reverted to origin/main, `states_that_differ_only_in_extents_have_different_keys` fails at model_simulation.rs:1889 ("two states, two keys"). It passes at the head. One soundness defect remains. The encoder dedups instance pairs, but admission keeps repeated pairs of a repeatable relationship.

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| semantic_enc relationships entry | qsl-eval/src/simulation/model/encode.rs:737-760 | examined |
| extents_enc | qsl-eval/src/simulation/model/encode.rs:762-803 | examined |
| ModelState | qsl-semantics/src/model/state.rs:34-40 | context_only |
| extent admission 6.9 (repeatable) | qsl-semantics/src/model/observation/relationships.rs:485-544 | context_only |
| RelationshipExtentStore links/targets | quire-semantic-value@74ab6f8 src/relationship_extent.rs:338-395 | context_only |
| frames.rs extent retain on delete | qsl-eval/src/simulation/model/frames.rs:857-873 | context_only |
| symmetry.rs extent renaming | qsl-eval/src/simulation/model/symmetry.rs:243-258 | context_only |
| states_that_differ_only_in_extents_have_different_keys | qsl-eval/tests/it/model_simulation.rs:1793-1890 | examined |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | `extents_enc` dedups instance pairs. FR-106 admission keeps a repeated pair when neither end is unique (`repeatable`, relationships.rs:485,530), and the upstream store keeps it in `links`, so navigation returns it twice. Two distinct states, one listing (c1,c2) once and one listing it twice, therefore share one key. Reproduced: both initial snapshots are admitted (1 and 2 instances), `state_key` is equal, and exploration counts 1 state. Removing the `dedup_by` line gives distinct keys and 2 explored states, and the 26 model_simulation tests still pass. Sorting alone is the canonical multiset form, and unique-end relationships already refuse repeats, so the dedup has no case where it is needed. | qsl-eval/src/simulation/model/encode.rs:764,786 |
| FND-002 | low | The new test's doc comment says "the explorer counts both", but the test asserts only that the two keys differ. It never runs `explore` or checks a state count. It is also the only test in the file without a `#[trace(...)]` tag (24 of the other 25 tests carry one). | qsl-eval/tests/it/model_simulation.rs:1829-1835 |

## Verdict

**Changes requested (one medium).** I checked these and found them sound.

- Absent vs empty extents. `extents == []` adds no entry, so states without extents keep their bytes. The exact-byte tests pass: FR-120-AC-4 `the_state_key_is_the_simulation_state_envelope` and the encode.rs unit tests in `simulation::` (71 passed).
- A relationship listed with zero instances vs one not listed. The first writes `{complete, instances: []}` and the second writes nothing. They differ, as `ExtentState::Complete` and `Unlisted` differ.
- `complete` is in the key.
- Instance order is normalised by the sort, and relationship order by `map_enc`. Admission refuses a relationship listed twice (`conflicting-identity`).
- No key collision. A population key is the JCS text of a `{node, package}` object, so it always starts with `{`. The bare text `relationships` can never equal one.
- `ModelState` has only `populations` and `extents`. The anchor is left out of the key on purpose (FR-120). No other field is missing.
- No shipped fixture or golden output uses snapshot extents with a model state key. The only snapshot `relationships` listing in qsl-eval and qsl-analyze tests is the new test. Package-level `"relationships": []` entries are type declarations, not extents.
- Rust idioms. The limits and sort pattern matches `map_enc`. `unwrap_or_default` on `to_vec` mirrors `map_enc`, and a reference triple cannot fail at u64::MAX limits. Clippy (`-p qsl-eval --all-targets --all-features -D warnings`) and `cargo fmt --check` are clean.

## Dispositions

Disposition pass 1, head 0ce03bf5f (fix commit 0ce03bf5f over a8823ee74).

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | `dedup_by` removed (encode.rs:786 now only sorts) and the doc comment at encode.rs:764 says repeats are kept. The new test `a_repeated_extent_pair_is_a_different_state` (model_simulation.rs:1902-1930) checks the 1x vs 2x listing: 2 admitted initial states, different keys, 2 explored states. It fails with a8823ee74's encode.rs (model_simulation.rs:1928) and passes at 0ce03bf5f. |
| FND-002 | fixed | The doc comment no longer claims the explorer counts both (model_simulation.rs:1839), and both extent tests carry `#[trace("TC-471", "FR-120-AC-27")]` (model_simulation.rs:1840,1902). The changed `link` fixture (0..2, neither end unique) is used only by these two tests, so no other test relied on the old multiplicity. |
