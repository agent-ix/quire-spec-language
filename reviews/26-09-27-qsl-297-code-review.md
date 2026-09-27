---
id: SR-753
title: "QSL-297 code review (with rust-review lane) of PR 500"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@6a093e9d0a531e9154a9f3d8c0e21075673aa5d6; qsl-forms/src/protocol_clause.rs; qsl-forms/src/syntax.rs; qsl-forms/src/dispatch.rs; qsl-forms/src/lib.rs; qsl-forms/src/value.rs; qsl-forms/tests/it/protocol_clause_forms.rs; qsl-forms/tests/it/value_forms.rs; qsl-semantics/src/check/assemble.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-112
    type: reviews
---
## Summary

Ticket: QSL-297. PR: quire-spec-language#500 at 6a093e9d, base a62bd8b4.
Methods: code-review with the rust-review lane folded in.

Gates, run by the reviewer at 6a093e9d with a fresh `CARGO_TARGET_DIR`:
`make ci` exit 0 (93 `test result: ok` blocks, 0 failures). `cargo fmt
--check` clean. `cargo clippy -p qsl-forms -p qsl-semantics --all-targets
--all-features -- -D warnings` clean. `cargo test -p qsl-forms -p
qsl-semantics` all pass.

Checked and sound. The new types are plain data with derived
`Clone/Debug/Eq/PartialEq`, no identity and no resolved target. The walk
handles all nine `Control` alternatives in the grammar (Sequence, Choice,
Parallel, Repetition, AwaitControl, EventNode, Check, Commit), plus Case
and Branch. All six sites are built. `commit never` builds no anchor. Source
order holds: compensations come before `run` in the grammar, and inside
`await` the `after` reference comes before the matched event and the
branches. Only direct-child `NodeReference`s are read, so no reference is
counted twice. The `DeclarationForm::Protocol(_) => {}` arm is commented as
deferred to QSL-298. No FR-113 resolution logic is in this PR: the form is
inert past S2. The ticket's minimal `ProtocolDeclarationForm` follows the
team-leader ruling.

Scratch probes, run in the worktree and reverted afterwards (tree clean):
an await-scope probe, a deep-nesting probe, a spine `compile` probe and
four mutations. Their results are cited in the findings.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The name of an `await` control is never pushed onto the scope. References inside its matched event, its `then` control or its `timeout` control therefore leave out the await that encloses them. Probe: `await Wait after Tried ... match receive Got via C of Sent ... then sequence Then { effect E of Got ... }` builds `effect-of [Got]` with scope `[Main, Then]`, and `receive-of [Sent]` with scope `[Main]`. `Got` is declared by `Wait`, since FR-113 Inputs says a control declares its direct child controls and event nodes, and FR-113's `await-after` row lists `await` as a structural control. FR-113 resolves innermost-out over this scope list (`Then`, then `Main`, then the top level), so it never looks in `Wait`. It refuses `missing-name`, or picks the wrong target when an outer node is also named `Got`. The legacy checker that FR-113 replaces puts await children under the await's symbol (`src/linking/composed/scopes/protocol.rs:289-298`). FR-112 Outputs asks for "the named controls that enclose the reference", and `await Wait` is one. Keep the await's own `after` reference outside it, as FR-112-AC-2 requires. | qsl-forms/src/protocol_clause.rs:285-305 |
| FND-002 | high | A `protocol` declaration is now accepted and then silently dropped. On main, S2 refused it (`NoDispatchEntry`, `unsupported_construct`, "no form reads a `protocol` declaration"). Now S2 builds it and the assembler throws it away. Probe: the spine-run fixture plus `protocol Flow ... { ... effect Applied of Missing ... }` goes through `qsl_replay::spine::compile`, which the CLI `compile` command calls (`src/command.rs:595`). It returns Ok with a 62,710-byte package that does not contain `Flow`, and gives no diagnostic, even though the protocol has a dangling reference and undeclared types. The comment says why nothing is assembled. It does not stop a user's declaration from vanishing out of a package that is reported as good. Until QSL-298 lands, refuse at S3 (for example `unsupported_construct` at the protocol's span), not with an empty arm. | qsl-semantics/src/check/assemble.rs:470-474; qsl-forms/src/dispatch.rs:472 |
| FND-003 | medium | The control walk recurses once per nesting level and charges nothing against `FormsLimits::nesting_depth`. `await ... then <Control>` and `repeat ... exhausted <Control>` nest with no bracket, so the CST's nesting ceiling of 64 does not bound them. Probe: 500 chained awaits, within the default CST limits, abort the process with a stack overflow on a 2 MiB thread (debug build). It survives on an 8 MiB stack up to the token ceiling. It overflows on any stack once a caller raises `Limits.tokens` or `Limits.nodes`, which qsl-cst documents as safe ("a raised ceiling cannot overflow the host stack"). Untrusted source should get a refusal, not an abort. Charge the depth against `nesting_depth` (the FR-091 S2 depth limit), or walk with an explicit stack. With the walk disabled, the same input builds fine, so the new code is the cause. | qsl-forms/src/protocol_clause.rs:160-235; qsl-forms/src/protocol_clause.rs:285-305 |
| FND-004 | medium | Four of the walk's arms have no test. Mutations tried against the PR's tests: dropping the `receive` site (`ReceiveOf` becomes `None`) passes, 7 of 7. Removing `Commit` from the no-op arm, so a `commit` control reaches `_ => Err(unexpected)`, passes, 7 of 7. Skipping the `choice` scope push passes, 7 of 7. No test builds a `choice`, `case` or `repeat`, or anything inside an await's `then` or `timeout`. No test asserts a `ScopeName` span, although FR-112 Outputs requires "names, with their spans". The coder's own mutation (skipping the `branch` scope push) does fail FR-112-AC-2. That test asserts the exact scope `[Main, Both, left]`, so it is a real oracle. | qsl-forms/tests/it/protocol_clause_forms.rs:180-278 |
| FND-005 | low | The test fixture differs from TC-510's `RecoveryFlow`. It leaves out the `commit Committed by R ...` control that TC-510 puts in `run sequence Main`, as the legacy `COMPENSATION` constant does. That leaves the `Commit` arm untested (FND-004), and the fixture no longer matches the one FR-113-AC-1 relies on (`Main::Committed` resolving to that commit node). The PR describes TC-510's fixture pointer as nonexistent or wrong. It is not: `tests/it/composed_scopes.rs:269` defines `COMPENSATION` with exactly TC-510's shape. Writing inline source in qsl-forms, the way `complete_grammar.rs` does, is a sound substitute because S2 needs no model resolution. Add the commit node back. | qsl-forms/tests/it/protocol_clause_forms.rs:138-170 |
| FND-006 | low | The block that pushes a name, recurses and pops appears six times: in Sequence, Choice, Parallel and Repetition, and in `case_anchors` and `branch_anchors`. A small `with_scope(scope, name, \|scope\| ...)` helper would remove the copies and make a missed push or pop, like the bug in FND-001, harder to write. | qsl-forms/src/protocol_clause.rs:170-283 |

## Verdict

Request changes. FND-001 and FND-002 are wrong results and block merge.
FND-003 is an abort on untrusted input and should be fixed in this PR.
FND-004 and FND-005 need tests or fixture changes in this PR. FND-006 is
optional.
