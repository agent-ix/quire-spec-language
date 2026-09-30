---
id: SR-818
title: "QSL-329 code review (with rust-review lane) of PR 537"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@5a08d72f78773410fac8c7b62978d814edd86f5a; Cargo.lock; qsl-replay/src/spine/clause/tests.rs; tests/it/config_version_spine.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-105
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-108
    type: reviews
---
## Summary

Ticket: QSL-329. PR: quire-spec-language#537 at 5a08d72f, base origin/main
(merge base e4ac2e8d; main now a8e15b77, which merges with this head without
conflict). Methods: code-review with the rust-review lane folded in.

Checks run:

1. Cargo.lock. Exactly one line changed: the `quire-contract-model` source
   revision (0ca720e9 to 7c700411f). No other package entry moved. Cargo.toml
   and qsl-package/Cargo.toml keep `branch = "main"` with no rev pin, so the
   lock is the only thing that selects the revision. `cargo test --locked`
   passes, so the lock is self-consistent.
2. Right reason. With the main-side Cargo.lock restored, both tests fail
   with IllTyped/OperatorIneligible at the reaches_field node: TC-463 at
   /semantic_graph/nodes/25/body/arguments/0, TC-469 step 6 at
   /semantic_graph/nodes/8/body/arguments/0. With the PR lock both pass.
   The only change between the two runs is that lock line.
3. Test oracles. TC-469 step 6 asserts AdmittedV2 and that the recomputed
   package_id equals the emitted one. TC-463 asserts the I2 read returns Ok;
   `read_import_view` pins the read to the recomputed package_id
   internally, so the AC-3 identity half is enforced by the reader.
4. rust-review lane. The Rust change is two `#[ignore]` removals and doc
   comment rewrites. No new production code, no unwrap, no unsafe.

## Verdict

The code change is correct and the tests pass for the right reason. One
stale doc comment on the neighbouring test in the same file still says the
I04 test is ignored (FND-001).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The doc on `tc_469_step_6_package_id_is_pinned_across_every_case` still says the I04 read half is "a separate, currently-`#[ignore]`d test below, blocked on IR-370 (the pinned `quire-contract-model` reader's gap)" and cites a pin bump to `2a28643`. This PR un-ignored that test, so the doc is now false and carries pin wording. Fix: say the I04 half is the test below, and drop the IR-370/pin history. | tests/it/config_version_spine.rs:844-851 |
