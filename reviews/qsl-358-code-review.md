---
id: SR-946
title: "QSL-358 slice 0 code review (with rust-review lane) of PR 555, stack-safe quire-exact Value and rust-version 1.82"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@0983d60a6353a5b6a00db258a59fa089cad8a264; quire-exact/src/value.rs (drop_nested, defer_node, release_children, take_present, Debug for Value, Drop for OptionValue and CompositeValue, new tests); quire-exact/src/collection.rs (take_elements, Drop for CollectionValue); quire-exact/Cargo.toml (rust-version); Makefile (quire-exact-msrv, ci); quire-exact/src/equality.rs and key.rs (context: existing worklists)"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/ADR-011
    type: reviews
---
## Summary

Ticket: QSL-358, slice 0. PR: quire-spec-language#555 at 0983d60a, diff `8b0c1ffe...0983d60a`.

What I checked:

- **Drop covers every recursive variant.** The only `Value` variants that hold a `Value`
  are `Option`, `Composite` and `Collection`. Each node struct has a `Drop` that hands
  its children to `drop_nested`. The scalar arms are exhaustive matches, so a new
  variant is a compile error.
- **Placeholders are valid.** `Option::take` leaves `None`. `mem::take` on
  `Box<[FieldValue]>` and `Box<[Value]>` leaves an empty boxed slice, which does not
  allocate. When the emptied node later drops, its `Drop` calls `drop_nested` on nothing.
- **No leak and no double drop.** Every child is either moved onto the worklist (from a
  node this thread owns alone) or left inside a shared node that is only decremented.
  `#![forbid(unsafe_code)]` holds. Nothing is forgotten. Sharing inside one value (a DAG,
  two slots holding one `Arc`) still works: the first pop fails `get_mut` and
  decrements, and the second pop then owns the node alone.
- **The race comment is correct.** On one thread `get_mut` cannot fail and then the
  decrement reach zero. Another thread has to drop the other handle inside that window.
  When two threads drop two handles to one deep chain, only the root is shared. The
  loser's decrement runs the root's `Drop`, whose worklist then owns every child alone.
  So the nesting is +1. It grows past one only when interior nodes are also shared and
  are dropped concurrently: one level per such coincidence, never per value depth. See
  FND-001 for a simpler approach without the window.
- **Moving payloads out.** The fields of `OptionValue`, `CompositeValue` and
  `CollectionValue` are private, so no code outside quire-exact can move them out.
  Adding `Drop` turns any move-out inside the crate into E0509. The workspace builds, so
  there is none, and nothing changed silently. `Value` itself has no `Drop`, so moving a
  payload out by pattern still works.
- **Debug is bounded by depth.** A list is expanded one item at a time
  (`Step::Items(rest)`), so the worklist holds a fixed number of steps per open level.
  Compact mode passes the caller's formatter to leaves, as the derive does. In alternate
  mode the `Indented` adaptor matches `PadAdapter`, blank lines included. The documented
  alternate-mode limit (leaves get plain `{:#?}`) cannot be fixed on stable Rust 1.82,
  which has no public way to build a `Formatter` with the caller's options over another
  writer. It is accepted, not a finding.
- **Oracle strength.** I replaced the `drop_nested` worklist with plain recursive drops
  in a scratch copy. `a_deep_value_drops_on_a_small_stack` then aborts with "has
  overflowed its stack" (SIGABRT). That test fails without the fix.
- **Public API.** The only API change is that `Value`'s `Debug` is hand-written instead
  of derived. The output is the same, and the trait is still implemented. The new
  helpers are `pub(crate)` or private. The derived `Debug` on the three node structs is
  unchanged and goes through `Value`'s iterative `Debug`.
- **MSRV target.** It touches only the Makefile, no workflow file. The `rustup install`
  in the ci log runs and the crate is rebuilt with the 1.82 rustc. See SR-947 for what
  the target does and does not prove for RT.
- **Main.** origin/main 3260a231 (#553, #554) touches neither the Makefile nor
  quire-exact/, so there is no conflict.

Gate: the coder's `make ci` log on 0983d60a ends `exit=0`, and all five new tests pass
in both test passes. I did not re-run it. The mutation run used a scratch copy with its
own target dir, which is now deleted.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | `release_children` checks `Arc::get_mut` on `&mut Value` and drops the node afterwards. That opens the window the race comment describes, where a concurrent drop of the other handle runs a nested `drop_nested`. The popped node is owned and `Value` has no `Drop`, so `match node { Value::Option(arc) => if let Some(mut inner) = Arc::into_inner(arc) { .. } .. }` works. `Arc::into_inner` is stable since 1.70, within the 1.82 MSRV. It is atomic: exactly one handle gets the inner value. This removes the window, the nested-worklist path and the paragraph explaining it. | quire-exact/src/value.rs:357-363, quire-exact/src/value.rs:394-430 |
| FND-002 | low | The hand-written `Debug` lists each node's fields by hand (`schedule_value` / `schedule_node`: label strings plus `&option.payload_type`, `&composite.slots`, ...). A field added to `OptionValue`, `CompositeValue` or `CollectionValue` would then be missing from `Debug` with no error, which the derive prevented. Bind the fields with an exhaustive destructure (`let OptionValue { payload_type, payload, occ } = &**option;`) so a new field is a compile error. | quire-exact/src/value.rs:637-680 |
| FND-003 | low | The doc of `a_deep_value_debug_formats_on_a_small_stack` says "the output closes every level it opens", but the test only checks `sink.0 > DEEP`. Output that dropped every `Close` step would pass it. Either count opening and closing brackets in the sink and assert they balance, or remove the claim from the doc. | quire-exact/src/value.rs:1679-1690 |
| FND-004 | low | The `Debug` parity fixture never prints the variant names `Rational`, `Decimal`, `Float`, `Quantity`, `Text` or `Reference`. These are hand-typed strings in `schedule_value`, and a typo in one would ship. The fixture also never puts a node inside a collection element or an option payload, and its root is always a composite. Extend `debug_sample`, and the captured derive strings, with one of each scalar and with a composite inside a collection inside an option. | quire-exact/src/value.rs:1452-1482, quire-exact/src/value.rs:637-650 |
| FND-005 | low | `quire-exact-msrv` runs `rustup toolchain install` on every `make ci`, and that syncs the channel manifest over the network even when 1.82 is installed (the ci log shows "syncing channel updates"). So `make ci` now needs static.rust-lang.org on every run. Install only when `rustup which --toolchain $(QUIRE_EXACT_MSRV) rustc` fails. I did not test offline. | Makefile:204-207 |

## Verdict

The drop and `Debug` code is correct. I traced it and found no leak, no double drop and
no depth-proportional recursion, and the core drop test is shown to fail without the
fix. Every finding is low and none blocks the merge. FND-001 is a simplification that
also removes the race nesting. FND-002 to FND-004 strengthen the `Debug` oracle and its
maintenance. FND-005 is a ci ergonomics issue. Mergeable. By repo practice, fix the
lows inside this PR.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-006 | low | On a machine whose cargo cache lacks the crates (a fresh clone or CI host), `cargo +1.82 fetch --offline` prints `error: no matching package named ... found ... you're using offline mode`, then the online fallback succeeds. I reproduced it with an empty `CARGO_HOME`. A passing `make ci` log then carries an `error:` line, which a reader can take for the cause of an unrelated later failure. Silence the offline attempt's stderr, so a failed fallback still shows its own real error, or print one line saying the fallback ran. | Makefile:220 |

## Dispositions

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | e9e462d7: `release_children(node: Value, ..)` takes the popped node by value and calls `Arc::into_inner` on each nesting variant (value.rs:392-414). The get_mut window and its race paragraph are gone. The deep-drop and shared-handle tests pass in the e9e462d7 ci log. |
| FND-002 | fixed | e9e462d7: `schedule_value` destructures `OptionValue { payload_type, payload, occ }` and `CompositeValue { declaration, slots, occ }` exhaustively. The collection arm uses `CollectionValue::debug_fields`, which destructures `Self { collection_type, elements, occ }` (collection.rs:207). A new field in any of the three is now a compile error. |
| FND-003 | fixed | e9e462d7: a `BracketCount` sink counts `([{` and `)]}`. The test asserts `sink.open > DEEP` and `sink.open == sink.close` (value.rs:1872-1945). |
| FND-004 | fixed | e9e462d7: `debug_samples()` returns a composite root and an option root. Between them they cover all ten scalar variants, Absent, Null and Present, and a composite inside a collection inside an option. I checked that the expected strings are the real derive output: in a scratch copy, with `#[derive(Debug)]` restored on `Value` and the hand-written impl disabled, `value_debug_matches_the_derived_format` passes. |
| FND-005 | fixed | e9e462d7: `rustup toolchain install` now runs only when `rustup target list --toolchain 1.82 --installed` lacks thumbv7em-none-eabi (Makefile:213). In the e9e462d7 ci log no install runs and nothing syncs. |
| FND-006 | fixed | e6277add: the offline fetch's stderr goes to `/dev/null`, and on failure the target prints `quire-exact-msrv: cargo 1.82 cache incomplete, fetching online` and then fetches online (Makefile:220). The coder's fresh-cache log shows that line, then the download and a finished build. The warm-cache log goes straight to the build. Neither log has an `error:` line. The only lines matching "error" are `thiserror` crate names. A real failure is still visible: if the offline attempt fails for any other reason, such as a cargo-1.82 manifest regression, the online fetch and the build both print the error unsilenced. |
