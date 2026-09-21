---
id: Plan-013
title: "Complete-V1 native compiler, runtime and tooling delivery"
type: Plan
status: active
relationships:
  - { target: ix://agent-ix/quire-spec-language/FR-055, type: references }
  - { target: ix://agent-ix/quire-spec-language/IT-011, type: references }
  - { target: ix://agent-ix/quire-specification/AD-003, type: references }
  - { target: ix://agent-ix/quire-specification/AD-005, type: references }
  - { target: ix://agent-ix/quire-specification/AD-006, type: references }
  - { target: ix://agent-ix/quire-specification/AD-008, type: references }
  - { target: ix://agent-ix/quire-specification/FR-001, type: references }
  - { target: ix://agent-ix/quire-specification/FR-002, type: references }
  - { target: ix://agent-ix/quire-specification/FR-003, type: references }
  - { target: ix://agent-ix/quire-specification/FR-005, type: references }
  - { target: ix://agent-ix/quire-specification/FR-006, type: references }
  - { target: ix://agent-ix/quire-specification/FR-007, type: references }
  - { target: ix://agent-ix/quire-specification/FR-008, type: references }
  - { target: ix://agent-ix/quire-specification/FR-009, type: references }
  - { target: ix://agent-ix/quire-specification/FR-010, type: references }
  - { target: ix://agent-ix/quire-specification/FR-011, type: references }
  - { target: ix://agent-ix/quire-specification/FR-012, type: references }
  - { target: ix://agent-ix/quire-specification/FR-013, type: references }
  - { target: ix://agent-ix/quire-specification/FR-016, type: references }
  - { target: ix://agent-ix/quire-specification/FR-019, type: references }
  - { target: ix://agent-ix/quire-specification/FR-033, type: references }
  - { target: ix://agent-ix/quire-specification/FR-039, type: references }
  - { target: ix://agent-ix/quire-specification/FR-041, type: references }
  - { target: ix://agent-ix/quire-specification/FR-131, type: references }
  - { target: ix://agent-ix/quire-specification/FR-132, type: references }
  - { target: ix://agent-ix/quire-specification/FR-133, type: references }
  - { target: ix://agent-ix/quire-specification/FR-134, type: references }
  - { target: ix://agent-ix/quire-specification/FR-140, type: references }
  - { target: ix://agent-ix/quire-specification/FR-141, type: references }
  - { target: ix://agent-ix/quire-specification/FR-142, type: references }
  - { target: ix://agent-ix/quire-specification/FR-143, type: references }
  - { target: ix://agent-ix/quire-specification/FR-144, type: references }
  - { target: ix://agent-ix/quire-specification/FR-145, type: references }
  - { target: ix://agent-ix/quire-specification/FR-146, type: references }
  - { target: ix://agent-ix/quire-specification/FR-147, type: references }
  - { target: ix://agent-ix/quire-specification/FR-148, type: references }
  - { target: ix://agent-ix/quire-specification/FR-149, type: references }
  - { target: ix://agent-ix/quire-specification/FR-150, type: references }
  - { target: ix://agent-ix/quire-specification/FR-151, type: references }
  - { target: ix://agent-ix/quire-specification/FR-152, type: references }
  - { target: ix://agent-ix/quire-specification/FR-153, type: references }
  - { target: ix://agent-ix/quire-specification/FR-180, type: references }
  - { target: ix://agent-ix/quire-specification/FR-181, type: references }
  - { target: ix://agent-ix/quire-specification/FR-270, type: references }
  - { target: ix://agent-ix/quire-specification/FR-271, type: references }
  - { target: ix://agent-ix/quire-specification/FR-272, type: references }
  - { target: ix://agent-ix/quire-specification/FR-300, type: references }
  - { target: ix://agent-ix/quire-specification/FR-301, type: references }
  - { target: ix://agent-ix/quire-specification/FR-302, type: references }
  - { target: ix://agent-ix/quire-specification/FR-303, type: references }
  - { target: ix://agent-ix/quire-specification/FR-304, type: references }
  - { target: ix://agent-ix/quire-specification/FR-305, type: references }
  - { target: ix://agent-ix/quire-specification/FR-306, type: references }
  - { target: ix://agent-ix/quire-specification/FR-307, type: references }
  - { target: ix://agent-ix/quire-specification/FR-308, type: references }
  - { target: ix://agent-ix/quire-specification/FR-309, type: references }
  - { target: ix://agent-ix/quire-specification/FR-311, type: references }
  - { target: ix://agent-ix/quire-specification/FR-336, type: references }
  - { target: ix://agent-ix/quire-specification/FR-339, type: references }
  - { target: ix://agent-ix/quire-specification/TM-009, type: references }
---
# Plan-013: Complete-V1 native compiler, runtime and tooling delivery

## Scope

Adopt the QSpec complete-V1 baseline `d49bbdc4a97ae0dca27b4f1eec2826446520fc28` (the QSpec PR #75 merge, which after PRs #73 and #74
amended the QSpec #68 complete scalar contracts over the PR #64 baseline),
then execute QSL #116 through #123 and `quire-wasm` #6. This is the single
repository plan for Agent-A's complete source/package, type/expression, model,
reference-runtime, tooling, WASM and qualification lane. Central QSpec retains
semantic authority; this plan owns implementation order, local tests and honest
evidence state.

## Requirements Summary

- [x] **FR-055 / IT-011 / QSL #116:** install the frozen contract, exact allocation,
  evidence reconciliation and serial issue graph.
- [x] **AD-003, FR-131/134, FR-302/303 and FR-339:** complete source packages,
  grammar, identities, lossless CST and incremental source tooling foundations
  in QSL #117. FR-132/133 and the authority-bound diagnostic/semantic repeats
  remain in QSL #123.
- [ ] **AD-005, FR-140–149:** complete exact scalar, composite, collection and
  pure-expression semantics in QSL #118 and #119.
- [ ] **AD-006, FR-150–153:** complete immutable model graph, lookup,
  specialization and dispatch semantics in QSL #120.
- [ ] **AD-008, FR-180/181:** complete native reference execution and bounded
  deterministic finite simulation in QSL #121.
- [ ] **FR-300/301/305/306:** expose lifecycle APIs, CLI, isolated Rust plugins
  and exact compilation/cache behavior in QSL #122.
- [ ] **FR-307/308:** deliver reusable semantic libraries in QSL #119 and pure
  bounded WASM parity in `quire-wasm` #6.
- [ ] **FR-311/336, I18/I19 and TM-009:** qualify every Agent-A inventory row in
  QSL #123 without promoting missing consumer evidence.

The central baseline has no FR-135 through FR-139 artifacts. FR-304/TC-224 and
FR-309/TC-229 are referenced interface contracts whose primary capability rows
belong to Agent-C; they are not silently reassigned to this lane.

## Dependency Graph

- `Task-046 -> Task-047 -> Task-048 -> Task-049 -> Task-050 -> Task-051 ->
  Task-052 -> Task-053 -> Task-054`.
  Reason: the user-assigned and central-manifest campaign is serial across one
  repository owner, with WASM consuming the merged lifecycle before final QSL
  qualification.
- QSL #131 and #120 consume the merged filament-core-data semantic IR crates and
  quire-rs extraction by exact git revision; they edit neither repository.
- QSL #121 consumes merged temporal/protocol artifacts and preserves their
  semantic authority rather than importing their evaluators.
- QSL #123 consumes merged temporal, protocol, IR and integration results under
  the exact I18 ecosystem lock; absent results remain missing qualification.

## Existing Evidence Reconciliation

The accepted central inventory snapshot credits the 83 Agent-A rows as follows:
implementation is **18 complete, 36 partial and 29 missing**; direct verification
is **16 complete, 38 partial and 29 missing**; integration is 52 partial and 31
missing; qualification is 83 missing. Plan adoption changes none of those states.

| Existing layer | Credited local evidence | Complete-V1 remainder |
| --- | --- | --- |
| L1 source | `source_map`, parser/formatter/CLI tests; TC-011–019 | exact edition/profile packages and complete grammar/CST in #117 |
| L2 package/type | composed parse/link/check suites; TC-113–121 | complete identities and type matrix in #117–120 |
| L3 expressions | predicate/query suites; TC-126–128 | total functions and complete collection algebra in #118/#119 |
| L4 model/state | graph/state suites; TC-129–131 and TC-136/137 | normalization, closed lookup, inheritance and dispatch in #120 |
| L5 temporal handoff | TC-122–125 and authenticated owner controls TC-138–143 | merged external consumer artifacts before #121/#123 |
| L6 protocol handoff | TC-121 and TC-132–138 | protocol conformance remains with its owner; QSL preserves exact source and domain-package inputs |

## Test Plan

### Plan adoption

- [x] **TC-144** (IT-011): Rust structural audit of the 83-row allocation,
  task graph, evidence-state counts and delivery constraints.

### Source/package and tooling (#117)

- [x] **TC-180/184:** profile accounting and source-locus preservation. TC-181
  remains with Task-049; TC-182/183 remain with Task-054 because manifest and
  extension compatibility consume the final checked semantic package.
- [x] **TC-222:** lossless CST identity and exact round-trip in QSL #117.
  QSL #117 also supplies formatter/editor reparse, exact-token, limit, parity
  and refusal foundations; Task-054 supplies the unforgeable checked-package
  integration and runs concrete TC-223.
  TC-220/221 remain with Task-052.

### Scalar/type semantics (#118)

- [x] **TC-185–187:** decimal, text/enum and unit semantics over all positive,
  boundary and refusal vectors.
- [x] **TC-192/193:** integer division and IEEE exceptions/rounding. Task-048
  supplies scalar equality primitives without claiming complete TC-194.

### Composite/expression semantics (#119)

- [ ] **TC-188–191:** record/tuple/recursive values, collection kinds and
  transforms, and termination-checked pure functions.
- [ ] **TC-194 and TC-227:** composite equality plus exact reusable-library
  resolution, conflict, cycle and migration refusals.

### Model graph semantics (#120)

- [ ] **TC-195–198:** provenance, specialization/dispatch, systems-model binding
  and complete/unknown/foreign/over-bound lookup.

### Reference execution (#121)

- [ ] **TC-209/210:** complete workflow replay and reproducible finite
  enumeration/sampling with explicit exhaustion.

### Lifecycle tooling (#122)

- [ ] **TC-220/221 and TC-225/226:** API/CLI parity, plugin containment and exact
  native/AOT/JIT/cache parity.
- [ ] **TC-180 backend-install repeat:** complement Task-047's authority-free
  parser type proof with a before/after installation-state integration vector.

### WASM and qualification

- [ ] **TC-228 / WASM #6:** native/WASM accepted-value, refusal and limit parity.
- [ ] **TC-047, TC-182/183, TC-230/231, IT-070/071/076 / QSL #123:**
  authority-bound diagnostic catalog/causes, canonical manifest and extension
  compatibility, failure-stage and complete attributable qualification over all
  Agent-A rows and exact dependency pins.

## Atomic capability allocation

Each row is copied by identity from QSpec's accepted Agent-A manifest and TM-009.
`Delivery test` selects exactly one existing central TestCase as the row's
concrete evidence owner; task files may require additional boundary cases.

| Capability | Central requirement | Primary ticket | Delivery test | Qualification ticket |
| --- | --- | --- | --- | --- |
| V1-SRC-001 | FR-001 | QSL #117 | TC-231 | QSL #123 |
| V1-SRC-002 | FR-001 | QSL #117 | TC-231 | QSL #123 |
| V1-SRC-003 | FR-131 | QSL #117 | TC-180 | QSL #123 |
| V1-SRC-004 | FR-131 | QSL #117 | TC-180 | QSL #123 |
| V1-SRC-005 | FR-003 | QSL #117 | TC-231 | QSL #123 |
| V1-SRC-006 | FR-003 | QSL #117 | TC-231 | QSL #123 |
| V1-SRC-007 | FR-134 | QSL #117 | TC-184 | QSL #123 |
| V1-SRC-008 | FR-002 | QSL #117 | TC-231 | QSL #123 |
| V1-SRC-009 | FR-002 | QSL #117 | TC-231 | QSL #123 |
| V1-SRC-010 | FR-132 | QSL #123 | TC-182 | QSL #123 |
| V1-SRC-011 | FR-132 | QSL #123 | TC-182 | QSL #123 |
| V1-SRC-012 | FR-016 | QSL #117 | TC-231 | QSL #123 |
| V1-SRC-013 | FR-132 | QSL #123 | TC-182 | QSL #123 |
| V1-SRC-014 | FR-133 | QSL #123 | TC-183 | QSL #123 |
| V1-SRC-015 | FR-001 | QSL #117 | TC-231 | QSL #123 |
| V1-TYPE-001 | FR-002 | QSL #118 | TC-231 | QSL #123 |
| V1-TYPE-002 | FR-005 | QSL #118 | TC-231 | QSL #123 |
| V1-TYPE-003 | FR-007 | QSL #118 | TC-231 | QSL #123 |
| V1-TYPE-004 | FR-039 | QSL #118 | TC-231 | QSL #123 |
| V1-TYPE-005 | FR-140 | QSL #118 | TC-185 | QSL #123 |
| V1-TYPE-006 | FR-141 | QSL #118 | TC-186 | QSL #123 |
| V1-TYPE-007 | FR-141 | QSL #118 | TC-186 | QSL #123 |
| V1-TYPE-008 | FR-142 | QSL #118 | TC-187 | QSL #123 |
| V1-TYPE-009 | FR-142 | QSL #118 | TC-187 | QSL #123 |
| V1-TYPE-010 | FR-019 | QSL #119 | TC-231 | QSL #123 |
| V1-TYPE-011 | FR-019 | QSL #119 | TC-231 | QSL #123 |
| V1-TYPE-012 | FR-143 | QSL #119 | TC-188 | QSL #123 |
| V1-TYPE-013 | FR-149 | QSL #119 | TC-194 | QSL #123 |
| V1-TYPE-014 | FR-008 | QSL #119 | TC-231 | QSL #123 |
| V1-TYPE-015 | FR-144 | QSL #119 | TC-189 | QSL #123 |
| V1-TYPE-016 | FR-144 | QSL #119 | TC-189 | QSL #123 |
| V1-TYPE-017 | FR-144 | QSL #119 | TC-189 | QSL #123 |
| V1-TYPE-018 | FR-145 | QSL #119 | TC-190 | QSL #123 |
| V1-TYPE-019 | FR-019 | QSL #119 | TC-231 | QSL #123 |
| V1-TYPE-020 | FR-143 | QSL #119 | TC-188 | QSL #123 |
| V1-TYPE-021 | FR-011 | QSL #120 | TC-231 | QSL #123 |
| V1-TYPE-022 | FR-009 | QSL #120 | TC-231 | QSL #123 |
| V1-TYPE-023 | FR-153 | QSL #120 | TC-198 | QSL #123 |
| V1-TYPE-024 | FR-010 | QSL #120 | TC-231 | QSL #123 |
| V1-TYPE-025 | FR-150 | QSL #120 | TC-195 | QSL #123 |
| V1-TYPE-026 | FR-151 | QSL #120 | TC-196 | QSL #123 |
| V1-TYPE-027 | FR-151 | QSL #120 | TC-196 | QSL #123 |
| V1-TYPE-028 | FR-151 | QSL #120 | TC-196 | QSL #123 |
| V1-TYPE-029 | FR-152 | QSL #120 | TC-197 | QSL #123 |
| V1-TYPE-030 | FR-019 | QSL #118 | TC-231 | QSL #123 |
| V1-TYPE-031 | FR-147 | QSL #118 | TC-192 | QSL #123 |
| V1-EXPR-001 | FR-006 | QSL #119 | TC-231 | QSL #123 |
| V1-EXPR-002 | FR-033 | QSL #119 | TC-231 | QSL #123 |
| V1-EXPR-003 | FR-146 | QSL #119 | TC-191 | QSL #123 |
| V1-EXPR-004 | FR-146 | QSL #119 | TC-191 | QSL #123 |
| V1-EXPR-005 | FR-007 | QSL #118 | TC-231 | QSL #123 |
| V1-EXPR-006 | FR-147 | QSL #118 | TC-192 | QSL #123 |
| V1-EXPR-007 | FR-039 | QSL #118 | TC-231 | QSL #123 |
| V1-EXPR-008 | FR-148 | QSL #118 | TC-193 | QSL #123 |
| V1-EXPR-009 | FR-148 | QSL #118 | TC-193 | QSL #123 |
| V1-EXPR-010 | FR-041 | QSL #119 | TC-231 | QSL #123 |
| V1-EXPR-011 | FR-008 | QSL #119 | TC-231 | QSL #123 |
| V1-EXPR-012 | FR-145 | QSL #119 | TC-190 | QSL #123 |
| V1-EXPR-013 | FR-145 | QSL #119 | TC-190 | QSL #123 |
| V1-EXPR-014 | FR-145 | QSL #119 | TC-190 | QSL #123 |
| V1-EXPR-015 | FR-145 | QSL #119 | TC-190 | QSL #123 |
| V1-EXPR-016 | FR-149 | QSL #119 | TC-194 | QSL #123 |
| V1-EXPR-017 | FR-009 | QSL #120 | TC-231 | QSL #123 |
| V1-EXPR-018 | FR-012 | QSL #120 | TC-231 | QSL #123 |
| V1-EXPR-019 | FR-012 | QSL #120 | TC-231 | QSL #123 |
| V1-EXPR-020 | FR-012 | QSL #120 | TC-231 | QSL #123 |
| V1-EXPR-021 | FR-006 | QSL #120 | TC-231 | QSL #123 |
| V1-EXPR-022 | FR-013 | QSL #120 | TC-231 | QSL #123 |
| V1-EXPR-023 | FR-271 | QSL #123 | TC-047 | QSL #123 |
| V1-EXPR-024 | FR-019 | QSL #119 | TC-231 | QSL #123 |
| V1-EXPR-025 | FR-019 | QSL #119 | TC-231 | QSL #123 |
| V1-EXPR-026 | FR-007 | QSL #119 | TC-231 | QSL #123 |
| V1-RUN-001 | FR-005 | QSL #121 | TC-231 | QSL #123 |
| V1-RUN-010 | FR-005 | QSL #121 | TC-231 | QSL #123 |
| V1-RUN-013 | FR-181 | QSL #121 | TC-210 | QSL #123 |
| V1-TOOL-001 | FR-300 | QSL #122 | TC-220 | QSL #123 |
| V1-TOOL-002 | FR-301 | QSL #122 | TC-221 | QSL #123 |
| V1-TOOL-003 | FR-302 | QSL #117 | TC-222 | QSL #123 |
| V1-TOOL-004 | FR-303 | QSL #117 | TC-223 | QSL #123 |
| V1-TOOL-006 | FR-305 | QSL #122 | TC-225 | QSL #123 |
| V1-TOOL-007 | FR-306 | QSL #122 | TC-226 | QSL #123 |
| V1-TOOL-008 | FR-307 | QSL #119 | TC-227 | QSL #123 |
| V1-TOOL-009 | FR-308 | WASM #6 | TC-228 | QSL #123 |

## Remaining Work

### Track A: Critical path (serial)

- **A00 = Task-046** adoption — exit: the frozen plan is validated and audited.
- **A01 = Task-047** source packages — hard; exit: complete syntax and CST round-trip exactly.
- **A02 = Task-048** scalar semantics — hard; exit: exact numeric/text/unit matrices pass.
- **A03 = Task-049** composites/functions — hard; exit: collection and totality properties pass.
- **A04 = Task-050** domain-package intake and model graph — hard; exit: domain-package intake, closed lookup and dispatch pass.
- **A05 = Task-051** runtime — hard; exit: all admitted clauses execute and simulations replay.
- **A06 = Task-052** tooling — hard; exit: API/CLI/plugin/AOT/JIT parity passes.
- **A07 = Task-053** WASM — hard; exit: the pure bounded WASM surface matches native.
- **A08 = Task-054** qualification gate — hard; exit: all 83 rows have attributable evidence.

## Parallel Execution Summary

```text
A00 -> A01 -> A02 -> A03 -> A04 -> A05 -> A06 -> A07 -> A08
        \------ external producer/consumer pins merge before use ------/
```

## Task File Mapping

| Task | Track | Owns (references) | Verified by (verifies) | Status |
| --- | --- | --- | --- | --- |
| Task-046 | A00 | FR-055, IT-011 | TC-144 | done |
| Task-047 | A01 | FR-131/134, FR-302/303, FR-339 | TC-180/184, TC-222 | done |
| Task-048 | A02 | FR-140–142, FR-147/148 | TC-185–187, TC-192/193 | done |
| Task-049 | A03 | FR-143–146, FR-149, FR-307 | TC-188–191, TC-194, TC-227 | not_started |
| Task-050 | A04 | FR-056, IT-012, FR-150–153 | TC-145–148, TC-195–198 | not_started |
| Task-051 | A05 | FR-180/181 | TC-209/210 | not_started |
| Task-052 | A06 | FR-300/301/305/306, FR-339 | TC-180, TC-220/221/225/226 | not_started |
| Task-053 | A07 | FR-308 | TC-228 | not_started |
| Task-054 | A08 | FR-132/133, FR-270–272, FR-303, FR-311/336, TM-009 | TC-047, TC-182/183, TC-223, TC-230/231, IT-070/071/076 | not_started |

## Coordination Rules

- One issue and one PR at a time from a fresh default-branch worktree.
- Shared interfaces are consumed only from merged pinned revisions; no lane edits
  another producer's repository or redefines a central payload.
- Rust only for first-party implementation, tests, fixtures, generators and
  audits. Crates remain `publish = false`.
- Run Cargo locally and serially with literal `--target-dir
  target-codex-backends`; No hosted CI is dispatched.
- Do not modify `resources/native-v1`, move/delete tags, infer completion from a
  self-report, or replace an unsupported capability with smaller semantics.
- After every merge, reconcile the child, parent, QSpec #9 inventory/matrix and
  maintained Research plan before taking the next ticket.
