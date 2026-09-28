---
id: SR-766
title: "QSL-313 gap analysis of PR 508 (FR-105-AC-6 fault-injected all-or-nothing emission)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@b0da45cc4b99b37a32b0f025b1e7881b0933369b; spec/functional/FR-105-emit-state-nodes.md; spec/test-cases/TC-462-s4-emits-state-nodes.md; spec/test-cases/TC-463-s4-state-package-reads-back-and-is-stable.md; spec/tests.md; qsl-package/src/emit.rs; qsl-package/src/emit/tests.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-105
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-463
    type: references
  - target: ix://agent-ix/quire-spec-language/TC-462
    type: references
---
## Summary

Ticket: QSL-313 (QSL-308d). PR: quire-spec-language#508 at b0da45cc. There is
no plan bundle, so the scope is the ticket: FR-105-AC-6, backed by TC-463
step 4.

FR-105-AC-6: "When the emitter cannot emit one of a clause's nodes (fault
injected at the `frame` node), the compile refuses and emits no `state` node
and no package bytes."

| Clause of AC-6 | Status at head |
| --- | --- |
| an injectable fault in node emission | Present: `emit_package_inner`'s `fault` hook, `#[cfg(test)]` entry point. |
| at the `frame` node | Not covered. The fixture holds only function nodes (`both`, `nb`, `h`). |
| of a clause (a state-clause package) | Not covered. No state node exists in the fixture. |
| the compile refuses | Covered at the emitter level only. `compile_unit` propagates the refusal with `?` (qsl-replay/src/spine.rs:1012), which I checked by reading the code. No test drives it. |
| no `state` node and no package bytes | Implied by the `Err` return type, and the call counter proves later nodes are never reached. |

Underspecified code: none. The new production code (the `fault` parameter and
the match arm) traces to FR-105-AC-6.

## Verdict

Changes requested. The PR proves the all-or-nothing mechanism in general, but
AC-6 as written (a frame-node fault in a state-clause package) is still
unverified (FND-001). The spec status text for AC-6 still says no hook exists,
and it names a canceled ticket (FND-002).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | FR-105-AC-6 is not covered as specified. The only test faults a node in a three-function package, which has no `state` or `frame` node. It does not check "fault injected at the `frame` node" or "emits no `state` node". The ticket requires the frame node explicitly. The ConfigVersion frame fixture it needs exists in qsl-replay (`config_version_compiled()`). The remaining work is the frame-node test in TC-463 step 4, described in SR-765 FND-001, and it belongs to this ticket. QSL-308, the ticket the PR defers it to, is Canceled. | qsl-package/src/emit/tests.rs:1961-2011; spec/functional/FR-105-emit-state-nodes.md:135; spec/test-cases/TC-463-s4-state-package-reads-back-and-is-stable.md:31-32,47-48 |
| FND-002 | medium | The spec's status text is stale and points at a canceled ticket. The PR changes no spec file. FR-105's AC table row for AC-6 still reads "Not yet implemented: no fault-injection hook exists; tracked by QSL-308". The hook now exists, and QSL-308 is Canceled. TC-463's Status section says step 4 is "tracked by QSL-308", and spec/tests.md's TC-463 row is unchanged. Fix: when the frame-node test lands, update the FR-105 AC-6 row, TC-463's Status (step 4) and the tests.md TC-463 row to name the backing tests and QSL-313. If only the generic test ships, mark AC-6 as partial, with the mechanism covered and the frame-node case open under a live ticket. | spec/functional/FR-105-emit-state-nodes.md:135,162; spec/test-cases/TC-463-s4-state-package-reads-back-and-is-stable.md:50-60; spec/tests.md:245 |
