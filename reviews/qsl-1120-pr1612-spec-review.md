---
id: SR-60037
title: "Review of PR #1612 (QSL-1120, FR-485 checker-issued nested call site)"
type: SpecReview
analysis: code-review
date: 2026-10-26
scope: "agent-ix/quire-spec-language; PR #1612 head ec17ca494 over b052a1b7f: qsl-replay/src/application.rs, qsl-replay/src/lib.rs, FR-485 (Nested call sites, items 9-10, AC-9..AC-12), TC-1263 steps 7-8. code-review, rust-review, quoin gap-analysis and spec-review in one pass."
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-485
    type: reviews
---
# SR-60037: review of PR #1612 (QSL-1120, FR-485 nested call site)

## Summary

Ticket: QSL-1120. Verdict: **PASS with one test gap (FND-001, medium) and low findings.** Nothing blocks the merged SHA that luna:IR:codex2 (IR-595) waits on, but FND-001 should be fixed inside this PR because it leaves the one behaviour the spec calls out as "never attribute to an arbitrary owner" unpinned.

Acceptance (ticket comments, luna's four checks):

- AC1 met. `nested_call_site` returns the location the checker recorded for the call occurrence, and the facade accepts it only if a `NodeKind::Call` or `ImportedCall` of the selected caller's own checked body sits at exactly that `Location`. The tests compare every issued site with an independent walk of the body, require a non-empty child path and `Origin::Body` naming the caller, and run a generated `ChildRequest` at the site under a work limit that admits only the root `function.call`. The incomplete outcome carries `location == site` and no nested charge.
- AC2 met. For the imported `outer`, `owner_package_id()` is the dependency id, the site is the dependency body's location, and it equals no unit function body location. No `Origin::Expression` is built anywhere in the diff.
- AC3 met for the facade: seven refusals with seven distinct causes (`ForeignCall` carries the actual owner), all before registration or charge; `CheckedCallSite` has private fields and no constructor. Residual, not a defect of this PR: a CG consumer can still hand `ChildRequest::site` a `Location` built with the public `Location::at`; the leaf cannot tell it from an issued one. The facade proves issuance, not use (FND-004).
- AC4 met. Four tests tagged TC-1263 / FR-485-AC-9..12 use a real checked source, a registered body, `begin_call` and the leaf scheduler.

Identity (review question 2). CG can legitimately hold `(call_node, ordinal)`. In `quire-contract-ir` the V2 package has `CheckedNodeId{domain, digest}` for the node and `CheckedOccurrence{role: CheckedOccurrenceRole::Expression, ordinal: u64}` in its source map, encoded from the same checker occurrence map (`qsl-package/src/checked_v2.rs` maps `Expression` to `"expression"`). The ordinal is assigned per (node key, role) in insertion order across the whole package, not per body. I probed this with a hand-built input: in the fixture, `g`'s `f(x)` and `twice`'s two `f(x)` share one node key and take ordinals 0, 1, 2. Asking for `twice` with `g`'s ordinal, and for `g` with either of `twice`'s ordinals, returns `Unreachable`; `u64::MAX` returns `UnknownOccurrence`; a unit call node with the imported `outer` caller returns `ForeignCall(unit id)`. The lookup is unambiguous because `Location` carries the full child-index path and the owning function, so two calls in one body, or in two bodies, never share a location. FR-485 item 9 states that the ordinal is package-wide and the body selects; CG must carry the source-map ordinal, not a per-body count.

Mutation testing (each mutant applied alone to the PR head, then `cargo test -p qsl-replay --lib application::tests::`): the owner check (use the unit package for every caller) is killed by the imported-caller test and the refusal test. Killed: owner replaced by the unit package; ForeignCall reported as UnknownCall; WrongKind guard removed; acquisition guard removed; reachability removed; reachability that ignores `ImportedCall`; reachability that matches any call instead of the exact location; reachability matched at the root location; result `owner_package_id` taken from the handle instead of the caller; occurrence role `"expression"` changed; ordinal forced to 0. Survived: FND-001, and the equivalent mutant noted in FND-002.

Gate: `flock /tmp/qsl-cargo.lock make ci` at ec17ca494 is red only at `string-edge`: qsl-semantics/src/check/terminal.rs:235, 236 and 255 (inherited from #1595; fix is #1608, still open at review time). The remaining lanes were run with `make -k ci-internal` and all passed (the `fixture` compile error and the `no-such-package` message in the log are the negative-case fixtures). `quire validate` of FR-485 and TC-1263: no errors.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The `UnknownCall` branch "several other packages own the node" has no test. Mutating it to `ForeignCall(first owner)` passes all four tests, yet FR-485 item 10 and the FR-485 lookup rule both say a node found in several other packages must not be attributed to an arbitrary owner. The fixture has one dependency, so only absent node gives `UnknownCall`. Add a closure with two dependencies that hold an identical call node, and assert `UnknownCall` for a third caller. | qsl-replay/src/application.rs `nested_call_site` (`others.next(), others.next()`); AC-11 |
| FND-002 | low | Two `UnknownCaller` guards (owner not in the closure; `function_by_identity` is `None`) cannot be reached through the public API: a selection is minted only by `lookup_callable` for an owner and key of this closure, and `WrongAcquisition` runs first. AC-11 lists no case for `UnknownCaller`, and no test or mutation can exercise it. Keep it as defense in depth but say so in item 10, or drop the cause from the stated seven-way set. The `*id != caller.owner_package_id` filter in the `others` scan is redundant (the node was just not found in that package), an equivalent mutant. | qsl-replay/src/application.rs; FR-485 item 10 |
| FND-003 | low | AC-9 and AC-10 say `Origin::Body` names the caller's function and index. The tests match only `Origin::Body { function, .. }`; the index is covered only by equality with the independent body walk. Add an explicit index assertion or relax the AC text. | application.rs tests; FR-485 AC-9, AC-10 |
| FND-004 | low | The `compile_fail` doctest accesses a private field, so it also passes if `CheckedCallSite` were not exported at all, and it does not try a struct literal. AC-11 says the site "cannot be constructed outside `qsl_replay`". Add a literal construction doctest. State in FR-485 that `ChildRequest::site` stays a public `Location`, so the facade proves issuance, not that a body used the issued site. | application.rs `CheckedCallSite` doc; FR-485 AC-11 |
| FND-005 | low | Naming: `CheckedCallSite` and `CheckedCallSiteRefusal` sit beside the existing `CallSite<S>`, `CallSiteRefusal` and `FunctionSite` of call_site.rs, which address a replay call target and mean something else. The method is `nested_call_site`; `NestedCallSite` and `NestedCallSiteRefusal` would not collide in a reader's head or in grep. The `Role::new("expression")` literal duplicates a spelling owned by the private `OccurrenceRole`; the mutation run shows the tests catch a change. The doc comment on the test helper `issued` is garbled. | qsl-replay/src/lib.rs exports; call_site.rs |
| FND-006 | low | Each call scans every node of the owner (`nodes().find`), then every other package (`has_node`), then walks the caller body: O(nodes) per query. A CG pass that asks for every call site is O(calls x nodes). Acceptable now; index by key if IR-595 shows cost. | qsl-replay/src/application.rs |

## Verdict

PASS with FND-001 to fix in this PR. Production code: no panics, no wildcard, no `#[allow]`, one string compare (`is_call_form`) carrying `#[ix_trace_rs::string_edge]`, the body walk is iterative. Trace tags TC-1263 / FR-485-AC-9..12 each name the AC the test asserts; spec AC text matches behaviour except FND-003 and the unreachable cause of FND-002.

## Coverage

Methods: code-review, rust-review, quoin gap-analysis (every new public item and AC has a tagged test; no untested code except FND-001, FND-002) and spec-review, in one pass.
