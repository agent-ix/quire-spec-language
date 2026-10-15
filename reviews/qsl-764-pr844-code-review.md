---
id: SR-9123
title: "Code review of quire-spec-language PR #844: protocol reference nodes"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@9309d436002b08a21ead02a6122ceb9662fcdb7a; PR #844 diff vs main: Cargo.lock, qsl-cst/src/grammar.rs, qsl-forms/src/{syntax.rs,value.rs}, qsl-semantics/src/check/{assemble.rs,check.rs,check/typing.rs,checked_dispatch.rs,claims.rs,family.rs,ir.rs,lowering.rs,lowering/protocol.rs,mod.rs,profile.rs,protocol_reference.rs,refusal.rs}, qsl-eval/src/value/expression/{causes.rs,evaluate.rs}, qsl-package/src/emit.rs, tests, tests/fixtures/parser-differential/baseline.txt"
review_set: subset
---
# Code review of quire-spec-language PR #844

## Summary

Ticket: QSL-764. Code review with the rust-review lane, one pass over
9309d4360 vs main (4 commits). Oracles: QSL FR-308, FR-218, FR-092
(AC-16/17/18), FR-110, TC-1021; QSpec FR-463, FR-322-AC-22, FR-322-AC-66,
FR-052 (by id); Contract IR main c9bf2631 v2 reader.

Examined:
- Cargo.lock: only quire-contract-model moves, to c9bf2631; no Cargo.toml
  change (clean).
- S2: `protocol <qualified-name>` is a `primary` alternative; ExprNode
  matches (children, spellings, visit, family encode, assemble, dispatch
  test) all updated with explicit arms, no wildcard absorbs the variant
  (clean).
- Parser baseline: only the 100 `complete` bucket lines change. The
  complete-family generator never writes `protocol`, so no input can flip
  from refused to accepted; only refusal expected-token sets move.
  Historical and composed lines are byte-identical (clean).
- S3: `Scope` indexes local protocols only; unknown and qualified names give
  MissingName, a local function or type gives operator-ineligible (clean).
- Repeated-reference scan: BTreeMap over function bodies in declaration
  order then state clauses, full `descendants()` walk; `NodeKind::children`
  covers every operand. Positions that are typed but never lowered
  (operation requires/ensures, protocol bodies) emit no carrier, so they
  need no scan (clean).
- S4 lowering of the protocol_clause node and protocol_reference carrier:
  application body with the protocol_profile law and profile_operator
  member, Boolean type, no owner (FR-322 owner table: application-keyed
  nodes carry none), one declaration occurrence; carrier body, type, deps
  and occurrence match FR-092 and the IR `validate_protocol_references`
  join (clean).
- Emitter: profile_selections split from definition laws; identity
  preimage carries the same rows; StaleProtocolReference fires before
  omission handling and maps to missing_declaration/missing-name; every
  EmitRefusal match updated (clean).
- Tests read emitted packages back through the real v2 reader
  (`read_v2`, Read::Verified), not a mock (clean).
- `protocol_profile` header resolution in the ProtocolClause family catalog
  (FR-110), marked string edge (clean).
- qsl-eval: a reference refuses unknown_required_feature/unsupported-feature,
  matching how a protocol clause itself evaluates (clean).
- No unwrap/expect/panic added outside tests (clean).
- Gate: qsl764c-ci4.log, exit 0, finished 17:30 after the 16:17 final
  commit; ci3 failed only on string-edge, which the final commit fixes.
- `protocol_reference::inputs` / `run_control` over every protocol of every
  unit (FND-001).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | `run_control` only accepts a sequence/choice/parallel/repeat/await/check root, but the grammar's `Control` also admits an event node (send/receive/attempt/effect/event) or `commit` as the `run` root, and a bare `run attempt ...` root checks at S3 today. For such a protocol `inputs` returns `InternalFault(ProtocolWithoutControl)`, and `check` pushes it unconditionally, before and regardless of any protocol profile or reference. Every complete-V1 unit with an event- or commit-rooted protocol that checked on main now refuses as an internal fault. No test covers this shape, so the gate stayed green | qsl-semantics/src/check/protocol_reference.rs:52-99, qsl-semantics/src/check/mod.rs:1431-1440, qsl-cst/src/grammar.rs:1385-1396 |

## Verdict

**FAIL, 1 blocking finding (FND-001).** Everything else holds. The carrier
and target node match FR-092 and the IR reader, the emitted package is
admitted by the real reader, the repeated-reference scan is deterministic
and covers every lowered position, the baseline re-record changes only
refusal expected-token sets, and the gate is green on the final head.

Fix: do not refuse a protocol with no control root. Only build protocol
nodes when a protocol profile is selected, and either pick a profile
operator for an event/commit root or refuse with a catalogued cause at a
reference to it, never an internal fault. Add a test for a `run attempt`
root unit with and without a reference.

Deviations judged:
- (a) Evaluation refuses unsupported-feature. FR-308 says a reference is
  evaluated exactly as its clause is, and a protocol clause evaluation
  refuses the same way. No AC requires a value. Accepted.
- (b) S3 binds by declaration index and lowering, inside `check`, mints the
  key before any reference is lowered; the stale check is at S4 as FR-218
  says. Accepted.
- (c) No protocol node without a selected protocol profile. FR-092-AC-18
  requires the selected law and FR-322-AC-22 refuses a node without it, so
  no node can be built. Recorded as a spec gap: FR-092 and FR-218-AC-8
  should state the selection precondition.
- (d) Import-only names: the scope holds local protocols only, so any
  qualified or imported name is missing-name. Accepted.
- (e) `profile_operator`: the IR reader does not check the value; QSpec
  FR-322 says one operator the profile defines (the FR-052 bounded
  control constructs), and the QSpec fixtures use `sequence`. The value is
  right for a sequence root. Recorded: `await` and `check` roots are not
  FR-052 control constructs.
- (f) The spec's `ExpressionForm::ProtocolReference{target: QualifiedName}`
  is `ExprNode::ProtocolReference{target: String}`; spec text to align.
  Recorded.

The following are recorded and do not block:
- A unit that selects the protocol profile but holds no protocol node
  writes no profile_selections row (FR-110 says the family writes it).
- The protocol node's law comes from any header protocol profile, not from
  the protocol's own `using` alias. This is the same result today, with one
  protocol profile in the catalog.
- operator-ineligible does not name the resolved kind (FR-218 text).
- The parse-refusal test checks invalid_syntax but not its position.

## Dispositions

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | 626b9ac79 (tests unrolled in 61285b19b). With no protocol profile selected, `protocol_nodes` builds nothing and refuses nothing, so the unit checks as on main. `InternalFault(ProtocolWithoutControl)` and `KeyFault::ProtocolWithoutControl` are deleted, with no consumers left. With a profile selected, a root other than sequence/choice/parallel/repeat refuses `unsupported_construct`/`declaration-form` (`ProtocolReferenceCause::UnsupportedRoot`) at the declaration, and also at a reference. That is the cause FR-218-AC-5 and QSpec FR-052 give an unsupported protocol declaration shape. Refusing `await`/`check` roots is consistent with QSpec FR-322-AC-22 (an operator the profile defines, the FR-052 bounded controls). It is not a regression, because main had no protocol profile selection. The new typed tests fail on the old code (the no-profile case refused there). The FR-092-AC-18, FR-218-AC-8 and FR-308 spec edits match the code. The gate is qsl764c-ci6.log: exit 0, 9114 passed, 0 failed, 22 ignored, finished 19:24, after the 17:50 head commit, and it runs the new tests. |
