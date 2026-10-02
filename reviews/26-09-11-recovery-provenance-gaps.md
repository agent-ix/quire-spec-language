---
id: SR-355
title: "Gap analysis of the shared recovery provenance rule against FR-042 and TC-121"
type: SpecReview
analysis: gap-analysis
scope: "src/protocol_artifact/recovery.rs; src/protocol_artifact/value_graph.rs; src/protocol_artifact/validate.rs; src/protocol_artifact/validate/control.rs; src/protocol_artifact/native/metadata.rs; tests/native_compensation_emission.rs; spec/functional/FR-042-publish-compiled-protocol-artifacts.md; spec/test-cases/TC-121-publish-compiled-protocol-artifacts.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-042
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-121
    type: references
---

## Summary

QUOIN `gap-analysis` over `origin/main...a33a3c1`, re-running `quire coverage
--scope /home/peter/dev/worktrees/quire-language-recovery-provenance --json`
(quire 0.31.0, engine `ca7362d4`). Step 1 (plan completion) remains not
applicable — `plan/` holds Plan-001..009, none covering protocol-artifact
emission — so matrix reconciliation is the operative gate. The optional semantic
review (Step 4) was declined by the requester. Reconciliation is unchanged from
the previous increment: 367/376 rows backed, the same six unbacked rows, twenty
untracked symbols, three unmatched `IT-004` tags, zero status lies. FR-042's
ten ACs and TC-121 are all backed, and the new test carries
`#[trace("TC-121", "FR-042-AC-4", "FR-042-AC-6", "FR-042-AC-7")]`. The reverse
gap SR-348 FND-002 recorded — FR-042 not saying which construction defines a
contributing origin — is closed by the new normative paragraph. One new gap
replaces it: the paragraph's phase-anchor refusal has no test on either path.

## Verdict

**FAIL** — retained solely by the skill's own rule that any matrix Test Case
with no backing tagged test fails the gate. That debt is inherited
(FR-036/TC-115/TC-010/FR-017), byte-identical to the previous increment and
untouched by this change. No finding of this increment is high; read as a
delivery signal the increment is clean at medium, and the FAIL is corpus debt.

## Findings

| ID      | Severity | Summary                                                                                                        | Refs                                                                                          | Escape Cause                    |
| ------- | -------- | ---------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------- | ------------------------------- |
| FND-001 | medium   | Six inherited unbacked matrix rows remain; none is in FR-042/TC-121 scope and none is touched by this change     | TC-115; TC-010; FR-017-AC-2; FR-036-AC-5; FR-036-AC-6; FR-036-AC-8                              | correct-requirement-no-evidence |
| FND-002 | medium   | The new "another compensation's phase anchor refuses" obligation has no test on either path, and no case shows whether it is reachable from source at all | src/protocol_artifact/recovery.rs:36; spec/functional/FR-042-publish-compiled-protocol-artifacts.md:181 | correct-requirement-no-evidence |
| FND-003 | medium   | FR-042-AC-9 still has no compensation or value-graph work-dimension vector, and this increment adds a second charged traversal of every declaration's values | spec/functional/FR-042-publish-compiled-protocol-artifacts.md:219; src/protocol_artifact/value_graph.rs:14 | missing-requirement             |
| FND-004 | low      | Twenty inherited untracked `NFR-007-M-*` tags and three unmatched `IT-004` tags, all outside this change          | src/package/encoding/tests.rs; tests/package_construction_cases/limits.rs; tests/fixture_audit.rs:231 | correct-requirement-no-evidence |
| FND-005 | low      | The two new modules cite FR-042 in their `//!` headers but carry no declared `implements` marker; `coverage.implements` is 0 of 2924 corpus-wide | src/protocol_artifact/recovery.rs:2; src/protocol_artifact/value_graph.rs:2                     | correct-requirement-no-evidence |

### Matrix reconciliation

`totals`: backed 367, total 376, criteria 258, property-shaped 83,
specific-shaped 13. `status_lies` 0. `binding_census`: 572 Rust candidates, 572
tagged, 572 bound, 62 self-named and bound. Two of the six unbacked rows are in
the module's `no_source_symbol` vocabulary. Raw report retained at
`/tmp/quire-recovery-provenance-review-coverage.json`.

### FND-002 — the one new obligation with no evidence

FR-042 now says an origin naming another compensation's phase anchor refuses,
and TC-121 step 6 was extended without naming that mutation. The implementation
is `recovery.rs:36-44`: a reachable `Origin::Anchor` whose anchor kind is
`Registration`, `CompensationActivation`, `Retry` or `Recovery` and whose
`owner` is not this compensation's subject returns `Invalid::Binding`. No test
reaches it. The existing cross-obligation mutations
(native_compensation_emission.rs:1494-1560) substitute *binding indices* between
`Full` and `Partial`; they never rewrite an origin anchor inside the `recover`
subgraph. Nor does any case establish whether an authored recovery can reference
another obligation's capture at all, or whether binder scoping forbids it first
— which decides whether this is a wire-only defence or a reachable source
refusal. This check is inherited from the deleted `recovery_origins` and was
already flagged unexercised in SR-347 FND-002; it is now the only unevidenced
clause of a requirement sentence this PR wrote.

### FND-003 — the work vector gap grew

The read path now resolves and charges every declaration's operand handles
twice (see SR-354 FND-002), and native emission charges a graph construction it
did not perform before. The work is not wasted — the graph's terminating
`validate::acyclic` call is the cycle check every declaration already received —
but it is an unbounded-by-requirement cost increase. AC-9 asks for
"independently counted ... vectors" per dimension; there is still no
compensation or value-graph vector, and the exact one-short test covers eight
dimensions, excluding `Entries` and `References`. Carried from SR-348 FND-005,
stronger here.

### Reverse gap (Step 3)

No new public surface: `recovery` and `value_graph` are private modules,
`population_members`, `attach` and every `ValueGraph` method are `pub(super)` or
narrower, and both have callers and test coverage. No stub, placeholder return,
`todo!`, `TODO` or re-export-only module appears in the diff. Every behavior the
two modules add is named by the FR-042 paragraph and the wire contract, except
the pair-identity predicate — which the crate implements consistently in two
places while no requirement states it (SR-357 FND-001, low).
