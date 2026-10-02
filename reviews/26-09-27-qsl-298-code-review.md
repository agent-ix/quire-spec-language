---
id: SR-761
title: "QSL-298 code review (with rust-review lane) of PR 503"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@cc535653df24672b569727cf4bcd7d7a5ef11a50; qsl-forms/src/lib.rs; qsl-forms/src/protocol_clause.rs; qsl-forms/src/syntax.rs; qsl-replay/src/spine.rs; qsl-semantics/src/check/assemble.rs; qsl-semantics/src/check/assemble/tests.rs; qsl-semantics/src/check/check.rs; qsl-semantics/src/check/mod.rs; qsl-semantics/src/check/protocol_clause.rs; qsl-semantics/src/check/refusal.rs; qsl-semantics/src/check/region.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-113
    type: reviews
---
## Summary

Ticket: QSL-298. PR: quire-spec-language#503 at cc535653, base 68f2dfed.
Methods: code-review with the rust-review lane folded in.

Gates, run by the reviewer at cc535653 in a fresh detached worktree with a
fresh `CARGO_TARGET_DIR`: `make ci` exit 0 (93 `test result: ok` blocks, 0
failed). `cargo fmt --check` clean. `cargo clippy -p qsl-forms -p
qsl-semantics -p qsl-replay --all-targets --all-features -- -D warnings`
clean. `cargo test -p qsl-forms -p qsl-semantics -p qsl-replay` all pass.
QSL-297's S2 tests (`qsl-forms/tests/it/protocol_clause_forms.rs`) are
unchanged by the diff and pass.

Checked and sound. Resolution goes innermost scope first, then outward, and
the first scope that declares the name decides. Later segments resolve in
the target's own child scope. Duplicate declarations refuse at each locus
and at any anchor that decides on the name. The site table matches FR-113's
table for all six sites. The channel check runs only for `receive-of`.
Refusals are sorted stably by span start. All anchors are in the one
`Anchored` builder state (ADR-012 §4.2), so sorting by source position is
enough. The codes reuse `missing_declaration/missing-name`,
`ambiguous_declaration/ambiguous-name` and `ill_typed/type-mismatch`. No
catalog code is added. The S2 extension (`ProtocolNodeKind`,
`ProtocolNodeDeclaration`, `declarations`, `ScopedAnchorForm::channel`) is
needed: FR-113 Inputs asks for a declaration collection, QSL-297 built only
anchors, and the old composed-lane checker is due for deletion (QSL-303), so
it cannot be reused. The extension does not duplicate anything QSL-297
built. The `region.rs` branch reads the cause's own span through the same
`region()` helper that other refusals use. Every `ProtocolAnchorCause`
carries a span in the unit's source, so no protocol refusal is left without
a location. `spine.rs` is correct. The removed `UnimplementedProtocol` arm
was required. The new message arm covers all four variants.
`check_message` keeps its `_ => {}` fallback, so exhaustiveness is not
compiler-enforced there. That matches the existing arms.

Scratch probes, run in the review worktree and reverted afterwards (tree
clean): one spine `compile` probe and seven mutations. The results are cited
in the findings.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | Removing `UnimplementedProtocol` brings back QSL-297's silent-acceptance defect (SR-753 FND-002). FR-113 checks only anchor resolution. Nothing checks the rest of the protocol: types, roles, operation names, binder types, block bodies. Nothing emits it either (emission is QSL-299, and no code reads `CheckedGraph::protocols()`). Probe: `tests/fixtures/spine-run.native` plus `protocol Flow using v over (input: Nope::Input) ... role R on Nope::Actor; ... attempt Tried by R on Nope::Actor::op ... as (tried: Undeclared) { 1 + true }; effect Applied of Tried ...`, run through `qsl_replay::spine::compile` (the CLI `compile` path). It returns `Ok`. The emitted package does not contain `Flow`, and there is no diagnostic. `emission.omitted()` is empty because emission never sees protocols. The old refusal was removed before its replacement works. Keep a refusal for a protocol that passes FR-113, for example `unsupported_construct/not-yet-implemented` at the protocol span after `protocol_clause::check` succeeds, or report it through `omitted`, until emission and the rest of protocol checking land. Add a spine-level test that a protocol does not compile to a package without it. | qsl-semantics/src/check/mod.rs:1036-1049; qsl-semantics/src/check/assemble.rs:474; qsl-semantics/src/check/assemble.rs:1572; qsl-replay/src/spine.rs:1008-1014 |
| FND-002 | medium | The AC-1 and AC-2 tests only assert that no refusal is raised. They never read `CheckedProtocol::resolved_anchors`, and the test helper throws the `CheckedGraph` away. Mutation: resolving outermost-first (`for depth in 0..=len` instead of `.rev()`) passes all 7 tests. The inner `await ... after Applied` then resolves to the outer `effect Applied`, which is an admitted kind, so nothing refuses. The shadowing rule, which is the point of FR-113-AC-2, has no oracle. AC-1's "each recorded by the node's identity" is not checked either. Assert each anchor's resolved `ProtocolNodeId` against the expected declaration (its kind and name span). | qsl-semantics/src/check/protocol_clause.rs:290-313; qsl-semantics/src/check/protocol_clause.rs:366-393; qsl-semantics/src/check/protocol_clause.rs:173 |
| FND-003 | medium | The new S2 declaration collection has no direct test, and half of it has no test at all. Mutation: removing the `declare(...)` calls for `Choice`, `Case`, `Parallel`, `Branch`, `Repeat` and `Check` passes every qsl-forms test and all 7 protocol_clause tests. Removing the `finish` declaration also passes. A dropped declaration would make a valid `Both::left::X` path refuse `missing-name`, or let a duplicate name inside a `choice` through. No test asserts a `ProtocolNodeDeclaration`'s kind, scope, span or channel, or `ScopedAnchorForm::channel`. Add qsl-forms tests over a fixture that uses every control kind. | qsl-forms/src/protocol_clause.rs:124-140; qsl-forms/src/protocol_clause.rs:226-310; qsl-forms/src/protocol_clause.rs:343 |
| FND-004 | medium | The AC-6 substitute (missing + wrong-kind) shows that one refusal does not stop another, and that two runs are deterministic. Its ordering assertion is vacuous. Mutation: deleting the `sort_by_key` passes this test, because both anchor refusals are already pushed in source order. Only the AC-4 test catches the missing sort. Also, `#[trace("TC-512", "FR-113-AC-6")]` marks AC-6 as verified even though its shadowing half is not built (QSL-306). A coverage matrix will show AC-6 as done. Assert the exact cause sequence, with a pair that is pushed out of source order (for example a duplicate declaration inside `run` plus an earlier `compensate` anchor refusal). Mark the tag or the doc comment as partial so that QSL-306 still owns AC-6. | qsl-semantics/src/check/protocol_clause.rs:489-508; qsl-semantics/src/check/protocol_clause.rs:152 |
| FND-005 | low | Several tests differ from the fixtures FR-113 names. AC-3 does not assert the empty scope for `Main::Missing` and `Other::Applied`. AC-4 leaves out the swap step and never asserts loci order (it checks only `loci.len() == 2`). AC-7 uses `effect Applied of Applied` (an `effect`) instead of the spec's `of Recovered` (an `event`), so no test has `event` as the actual kind. The channel fixture does not declare channels `C`/`D`. | qsl-semantics/src/check/protocol_clause.rs:399-417; qsl-semantics/src/check/protocol_clause.rs:450-462; qsl-semantics/src/check/protocol_clause.rs:520-524; qsl-semantics/src/check/protocol_clause.rs:627-638 |
| FND-006 | low | `ProtocolAnchorCause::Missing` keeps the failed segment and the scope, but not the anchor's segments. FR-113 Refusals says "retaining the segments, the scope, and the first segment that failed". | qsl-semantics/src/check/refusal.rs:22-31 |
| FND-007 | low | `a_wrong_kind_refusal_is_not_vacuous_over_an_otherwise_valid_protocol` repeats the `effect-of` case of the AC-7 test exactly (the same `replacen`), so it covers no new path. Its doc comment describes a mutation it does not perform. The coder's mutation claim is correct (disabling the wrong-kind check turns exactly 3 tests red: AC-7, AC-6 and this one). Only 2 of those 3 are separate oracles. | qsl-semantics/src/check/protocol_clause.rs:672-688 |
| FND-008 | low | Rust idiom. The 50-line hand-written `Hash` impl exists only because `qsl_foundation::Span` does not derive `Hash`. Adding `Hash` to `Span`'s derive list and deriving it on `ProtocolAnchorCause` removes the impl and the risk that it drifts. `anchor.anchor.segments[0]` panics on a hand-built form with no segments, because `ScopedAnchorForm` has public fields. `split_first()` with a refusal avoids that. | qsl-semantics/src/check/refusal.rs:71-120; qsl-semantics/src/check/protocol_clause.rs:170 |
| FND-009 | low | No test checks the region reported for a protocol refusal (`refusal_region` on a `ProtocolAnchor` cause), or the new `spine.rs` message text. The refusal's `Location` is a placeholder (`Origin::Expression`, empty path) that any consumer reading `location` directly would misreport. Only `refusal_region` is used today. FR-113's `## Status` still says "Not yet implemented". | qsl-semantics/src/check/region.rs:138-140; qsl-semantics/src/check/protocol_clause.rs:57-69; qsl-replay/src/spine.rs:428-456; spec/functional/FR-113-resolve-scoped-anchors-in-nested-control-scopes.md:137 |

## Verdict

Request changes. FND-001 is a wrong result that blocks merge: a protocol
with ill-typed content compiles to a package that silently leaves it out.
FND-002, FND-003 and FND-004 are test-oracle gaps that should be fixed in
this PR. FND-005 to FND-009 are low and should also be fixed in this PR.
The resolution logic itself is correct as far as the reviewer could see.
