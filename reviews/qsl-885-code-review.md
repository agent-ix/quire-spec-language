---
id: SR-12171
title: "Code review of quire-spec-language PR #1014: C1b3 protocol checker remaining constructs"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@5b258e4a32cb3bf0a8d9b4a1373d0991bd95cdf8; PR #1014 (base 2009fd339): qsl-semantics/src/check/{protocol_requirement.rs, protocol_requirement/tests.rs, protocol_relationship.rs, protocol_relationship/tests.rs, protocol_slots.rs, protocol_slots/tests.rs, refusal.rs, mod.rs, check.rs, lowering/model.rs, protocol_clause/{awaits.rs, coverage.rs, mod.rs, types.rs, tests/coverage.rs}, protocol_controls.rs, protocol_obligation/tests/recorded.rs}, qsl-semantics/tests/fixtures/protocol_obligations.rs, qsl-replay/src/spine.rs, qsl-package/src/emit/tests.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-218
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-211
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-208
    type: reviews
---
# Code review of quire-spec-language PR #1014

## Summary

Ticket: QSL-885. Code review with the rust-review lane folded in. Read-only
review of `git diff origin/main...HEAD` (20 files). No cargo build or test was
run; the coder's PR body reports check, all-features clippy, fmt and the
affected crates' tests.

Soundness, checked against the code:

- Nothing unchecked compiles. `covered_construct` now marks `TerminalWhen`,
  `Requirement`, `Relationship`, `Related` and `ReplicatedRole` checked, and
  each has a checker that runs on every protocol whose anchors resolve.
  `JoinPredicate`, `MemoryClause` and `AccessOrdering` still return false, and
  the TC-1065 test still asserts the predicate join and the three memory
  constructs refuse (only the `terminal when` row was removed). A relationship
  whose ends are not both object types (port ends) refuses `Unimplemented`.
  A refused relationship drops all checked relationships, but the refusal is
  already recorded, so the `related by` skip cannot compile.
- Requirements: the name resolves once, temporal clauses first, then the
  protocols and the other unit declarations, with the first declaration of a
  name kept. Aliases (model, profile, import) are not in the table, so they
  refuse `missing-name`. A protocol name refuses `operator-ineligible`
  naming `protocol`. Binding is by position in `temporal_clauses`, and two
  clauses of one position cannot be confused. Duplicates are recorded once at
  the first requirement.
- Replicated roles: every role that passes `protocol_clause::content` and
  `protocol_controls` has an object-typed context (otherwise `MissingRoleType`
  or `RoleModel` refuses), so `role_binders` always has an entry and the
  `Until(_) => None` arm cannot drop an unchecked predicate silently. `max 0`
  refuses and the role is not recorded.
- `terminal when` reads only the `over` input and, under `on origin`, the
  captures (the `BinderOrigin` filter), and its reads are not recorded as a
  node's reads. The tests check both activations, matching TC-947 TM10.
- `related by` ends are typed over `flow.input(node)` plus the node binder,
  compared with the relationship end's type, and record one clause per
  written clause in source order.
- Fields: `RelatedEnd` keeps relationship, `end`, expected and actual;
  `MissingRelated` keeps node and name; `RelationshipExport` keeps required,
  actual and the export's key (the second key is the declared spec gap);
  `Ambiguous` keeps the name and both loci; `RequirementCycle` keeps protocol
  and clause.
- Fixture changes do not weaken earlier tests. The shared obligation header
  now selects `profile t`. The one binder named `t` was renamed `taken` so it
  does not clash with the new alias, and that test's assertions are
  unchanged. No obligation-fixture test asserted the no-temporal-profile
  refusal. The `await` refusal test in `protocol_clause/tests/graph.rs` uses
  its own header, and the new compensate test removes `profile t` and
  asserts that it was there. Every integration fixture and artifact with a
  `compensate` already selects a temporal profile. In the qsl-package
  garbage-content test, the relationship case still has to refuse, and it
  now expects the specific `invalid_model_binding` code. The rewritten
  TC-511 test still refuses at the `related by` clause, and the fence and
  attempt-memory tests still guard the unchecked-construct path.

Trace tags (each checked for whether the test can fail on the AC's
behaviour): `FR-218-AC-1` (all four reference protocols must check, and the
Fill and Chatter edges are asserted), `FR-218-AC-3` (only reader, out-of-scope
read, `scope` region `S` then `Main`), `FR-218-AC-15` (both types, missing
name, swapped ends with every field, scalar id, three clauses in order) and
`FR-218-AC-25` (direct and indirect cycles, `Drain` checks) are all correct.
`QSpec-FR-464` is FR-level, which is right because FR-464-AC-11 also covers
law checks. The partly covered ACs (FR-218-AC-14, AC-16, FR-211-AC-1, FR-208-AC-1) carry TC ids
only.

Rust: no new `unwrap`/`expect`/panic or `unsafe` on production paths;
`kind_label` and the refusal code/tag matches are exhaustive; the unit
declaration table is built once per unit, outside the protocol loop.

## Verdict

Mergeable. No high finding. One medium (a requirement cycle reached through a
function body or another protocol's body is not seen; latent while no
temporal clause is emitted) and two lows.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | `reaches` finds a requirement cycle only through `ExprNode::ProtocolReference` nodes written directly in the required clause's captures and `holds` conditions, and through other protocols' `requires temporal` members. A protocol reference is typable in any expression: `function refers using v(): Boolean pure { protocol Flow }` is an existing fixture, and `holds(Allowed())` calls a function. So `temporal Due ... { always holds(refers()) }` with `refers` holding `protocol Fill`, and `Fill` requiring `Due`, is a definition cycle that FR-218 "Requirements" says to refuse ("reaches the requiring protocol through a protocol reference, directly or through other clauses"), but this check misses it. The same happens when a reached protocol's own slot (a capture, a body) holds `protocol Fill`. Today every unit with a temporal clause refuses `not-emitted`, so no package is unsound yet. The miss becomes live when FR-337 emits temporal clauses. Fix: follow function/predicate call targets and the protocol references in a reached protocol's slots, or reuse the package dependency graph's cycle check, and add a test with the cycle going through a function. | qsl-semantics/src/check/protocol_requirement.rs:150-221 |
| FND-002 | low | `RoleMax` keeps `minimum` and `actual`, but QSpec FR-465 "Role" 2 says the `max` refusal retains `expected` and `actual`. A consumer reading the field by its spec name does not find it. Also, the qsl-replay message hard-codes "at least 1" instead of reading `minimum`. | qsl-semantics/src/check/refusal.rs:408-417; qsl-replay/src/spine.rs:856-859 |
| FND-003 | low | `a_resolving_protocol_with_unchecked_content_still_refuses` (TC-511/FR-113) now asserts the FR-218 `MissingRelated` refusal, not unchecked content, so its name no longer says what it tests. | qsl-semantics/src/check/protocol_clause/tests/coverage.rs:18 |

The findings table holds defects only.
