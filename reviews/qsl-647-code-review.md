---
id: SR-1380
title: "Code review of quire-spec-language PR #660: composite witness values encode once (QSL-647)"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@2219a6a2b88b5a93156d70bb2af10377da2074a0; PR #660 diff against origin/main: qsl-replay/src/witness/value_text.rs, qsl-replay/src/witness/value_text/tests.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-070
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-263
    type: reviews
---
# Code review of quire-spec-language PR #660

## Summary

Ticket: QSL-647. Code review with the rust-review lane folded in.

The encoder now builds each value bottom-up: `encode_shell` writes a value's own JCS with each child as a bare-number placeholder, and `splice` replaces each placeholder with the child's finished bytes. Set and bag elements are sorted by those finished bytes. The decoder drops the per-collection `to_vec` re-encode and instead runs `check_collections`, one explicit-stack scan of the verified canonical bytes that checks each `set`, `bag` and `ordered-set` by the byte spans of its elements.

Checked and sound:
- Span check soundness. `from_value_text` compares `to_vec(read(bytes))` with `bytes` and refuses `NotJcs` before `build` and `check_collections` run, so only canonical JCS reaches the scan. In canonical JCS an element's own encoding is exactly its span of the parent text, so span comparison equals the old `to_vec` comparison. Only tag members are named `type` (record field names are carried as `"name"` values, never as keys), and `build` has already accepted the whole shape, so the scan cannot mistake a non-collection for a collection or miss one.
- Byte identity. A scratch probe at the head compared `encode_bottom_up` with the old single-writer encoder (main's `sort_orders` + `encode_into`, re-hosted on the PR's `value_steps`) over 20,000 seeded random values of depth up to 5 covering every `WitnessValue` variant, all three slot states, and text with quotes, backslashes, digits, control characters, `%;<>`, non-BMP and private-use characters, plus a 1,234-element sequence and set (multi-digit placeholders). All 20,002 encodings were byte-identical.
- Placeholder safety. Every scalar a form writes is a string, boolean or null, so a bare number in a shell is always a placeholder; `splice` skips string contents including escapes.
- Recursion. Encode uses a pre-order `Vec` and a loop; `splice`, `check_collections` and `string_end` are loops over an explicit stack. No native recursion grows with depth. The 512 KiB-stack tests pass.
- Fault order. A shape fault is now reported before an element-order fault. FR-070's Decode list gives no precedence among faults and `DecodeRefusal::Malformed` carries one; FR-263, FR-358 and TC-736 set none for decode. Allowed.

Gates run at the head (release, through locked-build.sh): `cargo test -p qsl-replay --lib value_text` passes, 11 tests. Timings below.

| Test (release) | base dac0d90fc | head 2219a6a2b |
| --- | --- | --- |
| witness::value_text::tests::tc_736_a_100000_long_list_decodes_and_walks_on_a_small_stack | 1.17 s | 207.09 s |
| execute::tests::composite::tc_736_a_100000_long_list_replays_under_the_raised_input_bound | 1.83 s | 393.70 s |
| execute::tests::composite::tc_736_a_100000_term_source_and_list_replay_to_the_proving_verdict | 46.36 s | 333.72 s |
| witness::value_text::tests::tc_736_nested_sets_encode_each_value_once | n/a | 3.42 s |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | `encode_bottom_up` materialises every value's full bytes and `splice` copies each child's bytes into a new parent buffer, so the bytes copied total the sum of every subtree's size: O(depth x output) for any deep nesting. This applies to every composite (record, option, sequence, tuple), not only sets, where main streamed them through one `Writer` in linear work. The FR-263-AC-1/FR-070-AC-10 100,000-long recursive list now takes 207 s to 394 s per test in release, against 1.2 s to 1.8 s on main (table above). A scratch probe timed `to_value_text` over nested `Option`s, release: depth 5,000 in 11.5 ms, 10,000 in 70.8 ms, 20,000 in 299.4 ms, about 4x per doubling, so quadratic. It also makes `value_text_len` (FND-003) and every replay of a deep value quadratic. The ticket wanted linear work; this moves the depth multiplier from sets to all composites. One fix: keep main's single streaming `Writer` pass for everything and compute each set/bag element's bytes once only where a set or bag needs to sort them, or build a rope of shell segments and child ids and write the output once at the end. | qsl-replay/src/witness/value_text.rs:161-200, qsl-replay/src/witness/value_text.rs:207-256 |
| FND-002 | medium | The `encoded <= 2 * bytes.len()` oracle cannot fail for this encoder. `encoded` sums shell bytes only, and each shell is its value's output with each child replaced by a shorter placeholder, so the sum of shells is below the output length by construction. It counts neither the splice copies nor the sort comparisons, which is where the depth-multiplied work now is (FND-001), and the 20,000-deep set test passes while the 100,000-deep list takes 200x longer than main. A real oracle counts every byte the encoder writes or copies (shell plus splice), or compares that count at depth d and 2d and requires a ratio near 2, not 4, over sets and over non-set composites both. | qsl-replay/src/witness/value_text/tests.rs:476-493 |
| FND-003 | low | `value_text_len` now builds the whole encoding to measure it (`encode_bottom_up` then a count), where main counted through the `EscapedLength` sink without building it. It feeds the `replay.input_bytes` measure (witness.rs:412) and the composite claim size (composite.rs:221), so each measurement allocates the full output and, with FND-001, pays the quadratic copy before the bound can refuse. Measure with a counting sink again once FND-001 is fixed, or reuse the bytes when the caller encodes next. | qsl-replay/src/witness/value_text.rs:475-483 |

## Verdict

One high, one medium and one low finding. The decode side (span-based order check) is correct and is the part of the ticket that is done. The encode side replaces depth-multiplied encoding for nested sets with depth-multiplied copying for every nested composite, a 100x to 200x regression on the TC-736 tests. Not mergeable until FND-001 and FND-002 are fixed; FND-003 should be fixed with them.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-004 | low | Every value without a set or bag now pays the per-node pieces machinery that only sets need: a `Vec<u8>` shell, a `Vec<Piece>` and its `Piece::Bytes` vectors per node, a pointer-keyed `HashMap` over the whole tree, and a `cut` pass, in both `to_value_text` and `value_text_len`. Main streamed set-free values through one `Writer` with no per-node allocation. The author's release timings (pre-rebase, the encoder is unchanged by the rebase) show the cost on the TC-736 set-free list: `tc_736_a_100000_long_list_decodes_and_walks_on_a_small_stack` 1.34 s vs 1.17 s on main, and `tc_736_a_100000_long_list_replays_under_the_raised_input_bound` 2.64 s vs 1.83 s (44% slower). The work is linear, so this is a constant factor, not a correctness defect. One fix: build pieces only for set and bag elements (where sorting needs finished bytes) and stream everything else through one `Writer`, writing a sorted set's elements from their pieces. | qsl-replay/src/witness/value_text.rs:308-359, qsl-replay/src/witness/value_text.rs:557-588 |
| FND-005 | medium | The last assertion of `tc_905_the_element_order_check_reads_spans`, commented "A bad collection nested inside a good one is found", cannot fail if nested collections go unchecked. The outer set is `[a, bad]` with `a` = `{"type":"text","value":"a"}` and `bad` = `{"elements":[...],"type":"set"}`; `bad` starts `{"e`, which sorts below `a`'s `{"t`, so the outer set is itself out of order and refuses `Elements` on its own. A scan that checked only the outermost collection passes this test. The span scan is new code, so its nested path is untested. Fix: put the bad set second inside a correctly ordered outer set (for example `[bad, a]`, or an outer bag/ordered-set), and assert the good outer alone decodes. | qsl-replay/src/witness/value_text/tests.rs:569-574 |

## Dispositions

Round 1, reviewed at `2d987b0db83e89b0c60a5dd344cae16c8911e919`.

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | 6cce4788: `splice` is gone. `encode_pieces` encodes each value's shell once and `cut` turns it into `Piece::Bytes` literals and `Piece::Child(id)` references into an arena, so no child's bytes are copied into an ancestor; output is a single `Chunks` walk, and set/bag elements are sorted by `compare` over their pieces in place. Every step is linear in the output (sort comparisons stop at the first differing byte). The author's release timings at the pieces encoder (log q647b, before the rebase; the rebase changed nothing under `qsl-replay/src/witness/`): 1.34 s, 2.64 s and 46.66 s against main's 1.17 s, 1.83 s and 46.36 s, down from 207 s, 394 s and 334 s. Not re-measured after the rebase onto a360ae02; the remaining constant-factor cost is FND-004. |
| FND-002 | fixed | 6cce4788: the test is now `tc_905_encoding_work_is_linear_in_depth`. `Work` counts every byte the shell writer produces plus every byte `compare` looks at, there is no splice copy left to miss, and the test requires work at depth 10,000 to be at most 3x work at depth 5,000 (a depth-multiplied encoder gives about 4x) and within 3x the output, over nested sets, options and sequences. |
| FND-003 | fixed | 6cce4788: with FND-001 fixed the measure is linear; `value_text_len` counts the escaped length over the pieces without building the escaped `String`. It still builds the pieces (the unescaped bytes) to count them; that cost is part of FND-004. |
| FND-004 | still-open | New this round (see New findings); no fix yet. |
| FND-005 | still-open | New this round (see New findings); no fix yet. |

Round 2, reviewed at `db471524f23aea363f960b34e273f578443eb5f5`.

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-004 | fixed | 503a20e3: `encode_pieces` marks each value with a set or bag at or below it; every maximal set-free subtree is written by `encode_flat`, one `Writer` pass over an explicit task stack (no native recursion), into a single `Piece::Bytes`, and only sets, bags and their ancestors get shells and pieces. Work stays linear. The author's release timings (log q647h, taken with this encoder just before the rebase onto 70ddb73c, which changed nothing under `qsl-replay/src/witness/` or the timed tests' files): 1.15 s, 1.95 s and 45.03 s against main's 1.17 s, 1.83 s and 46.36 s. |
| FND-005 | fixed | 503a20e3: the nested case is now an outer set `[bad, a]` that is in order (`{"e` sorts below `{"t`), so only the nested set can refuse, plus a sound control `[set[a, b], a]` that must decode. A scan of only the outermost collection now fails the test. |
