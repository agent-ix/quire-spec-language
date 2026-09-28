---
id: SR-765
title: "QSL-313 code review of PR 508 (FR-105-AC-6 emission fault-injection seam)"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@b0da45cc4b99b37a32b0f025b1e7881b0933369b; qsl-package/src/emit.rs; qsl-package/src/emit/tests.rs; qsl-package/src/lib.rs; qsl-package/Cargo.toml; qsl-replay/src/spine.rs; qsl-replay/src/spine/clause/tests.rs; .cargo/config.toml; Makefile; Cargo.toml"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-105
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-463
    type: references
---
## Summary

Ticket: QSL-313 (titled "QSL-308d: FR-105 AC-6 fault-injected all-or-nothing
emission hook"). PR: quire-spec-language#508 at b0da45cc, diff scope
`origin/main...task/313-fr105-ac6-fault-injection` (two files). This review
includes the rust-review lane.

What was checked, and what was clean:

- **The seam compiles out of every non-test build.** `emit_package_with_fault`
  is `#[cfg(test)]`. No `--cfg test` is set anywhere: `.cargo/config.toml`
  holds only an `xtask` alias; the Makefile's only `RUSTFLAGS` value is
  `--cfg seam_probe`; neither Cargo.toml sets rustflags. `lib.rs` re-exports
  `emit_checked` and the emission types, not `emit_package*`. A `pub(crate)`
  item cannot be re-exported as `pub`. The seam is not part of the public
  API.
- **Production behaviour is unchanged.** Every production path reaches
  `emit_package_inner` through `emit_checked` then `emit_package` with
  `|_| None`. For that closure, the new `match` arm always takes
  `None => candidate.wire_node()`, which is exactly the old `map` body.
  Iteration order, the error type and every step after `nodes` are the same.
  The other in-crate callers (`extent_agreement.rs:165`, `emit/tests.rs`) are
  tests.
- **All-or-nothing holds structurally, as the PR claims.** Everything in
  `emit_package_inner` before `EmittedPackage::new` is pure in-memory work:
  `recorded_occurrences`, `omissions`, `graph_order`, `source_map`, and the
  `nodes` collect. Nothing writes to disk, a buffer or a sink. The wire bytes
  exist only inside `EmittedPackage::new`, which runs after `nodes` has fully
  collected. `collect::<Result<Vec<_>, _>>()?` drops the partial `Vec` on the
  first `Err`. The only production caller above it,
  `qsl-replay/src/spine.rs:1012` (`compile_unit`), maps the refusal and
  returns with `?`, and writes nothing before or after. No partial node
  list or byte string can escape.
- **The counter is real.** `attempts` is incremented inside the fault hook,
  which runs in the same `map` closure as `wire_node`. `attempts == target + 1`
  therefore proves that later nodes were never reached, and the test does not
  just infer this from the `Result`. The fault fires at index `total / 2` of a
  graph order with at least 3 nodes, so never the first or the last node.

## Verdict

Changes requested. The mechanism is sound and the production refactor is
behaviour-neutral. The problem is scope: the PR tests a generic mid-list fault
over three plain function nodes, while FR-105-AC-6 and TC-463 step 4 specify a
fault at the `frame` node of a state clause. The PR defers that to QSL-308,
which is Canceled. QSL-313 is itself QSL-308d, the ticket that owns the
frame-node case. The `#[cfg(test)] pub(crate)` seam also cannot be reached
from qsl-replay, where the ConfigVersion frame fixture already exists
(`config_version_compiled()`). So the split is not a genuine separation of
work: the seam as built rules out the case the ticket asks for (FND-001). The
trace tag names the wrong TC (FND-002).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The frame-node case is deferred to a canceled ticket, and the seam cannot reach it. QSL-313's description says the hook must be tripped "on the frame node specifically". FR-105-AC-6 says "fault injected at the `frame` node ... emits no `state` node and no package bytes", and TC-463 step 4 says the same. The test's fixture is `package(vec![both(), nb(), h()])`, three function declarations with no state or frame node. The doc comment hands the frame fixture to "QSL-308's own remaining work", but QSL-308 is Canceled and QSL-313 is titled QSL-308d. The frame fixture already exists: `config_version_compiled()` in qsl-replay/src/spine/clause/tests.rs:2639. What blocks the test is visibility: `#[cfg(test)]` is set only for qsl-package's own test harness, so qsl-replay's tests can never call `emit_package_with_fault`. Frame nodes go through this same loop (`SemanticTerm::Frame`, emit.rs:319), so the fix is small. Expose the seam to dependent test builds, e.g. a qsl-package `test-support` feature (the pattern qsl-semantics already uses) gating a `#[doc(hidden)] pub fn emit_checked_with_fault`, enabled from qsl-replay's `[dev-dependencies]`. Then add a qsl-replay test: build `config_version_compiled()`, find the frame node's `CheckedNodeId` in its graph, fault on that id, and assert the `Encoding` refusal and that no `Emission` is returned. | qsl-package/src/emit.rs:785-799; qsl-package/src/emit/tests.rs:1947-1955; qsl-replay/src/spine/clause/tests.rs:2639-2651; spec/test-cases/TC-463-s4-state-package-reads-back-and-is-stable.md:31-32 |
| FND-002 | medium | Wrong TC in the trace tag. The test carries `#[trace("TC-462", "FR-105-AC-6")]`. TC-462's scope does not include AC-6. TC-463 does: its Scope line lists FR-105-AC-6, step 4 is the fault injection, and it says to tag `#[trace("TC-463", "FR-105-AC-n")]`. spec/tests.md's TC-463 row also lists FR-105-AC-6. As tagged, the matrix credits AC-6 to a TC that never claims it, and TC-463 step 4 stays untraced. Fix: `#[trace("TC-463", "FR-105-AC-6")]`. | qsl-package/src/emit/tests.rs:1961; spec/test-cases/TC-463-s4-state-package-reads-back-and-is-stable.md:17,34; spec/tests.md:245 |
| FND-003 | low | Rust idiom nits. (a) `emit_package_with_fault` is `pub(crate)`, but its only caller is the child module `emit::tests`, which can already see the parent's private items, so a private `fn` is enough. This is moot if FND-001 moves the seam behind a feature. (b) The test casts `total as u32` to fit `Cell<u32>`. Use `Cell<usize>` and compare against `total / 2` directly, with no cast. | qsl-package/src/emit.rs:793; qsl-package/src/emit/tests.rs:1976-1977 |
