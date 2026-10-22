---
id: SR-15492
title: "failure-domain review of STD-262 FR-290 refusal order and replay context (QSL PR #1217)"
type: SpecReview
analysis: failure-domain
scope: "agent-ix/quire-spec-language@8e62c04067d50955f5388df4071fb2077f0a16b2; spec/functional/FR-290-settle-plugin-results-as-typed-terminal-records.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-290
    type: reviews
---
# SR-15492: failure-domain review of STD-262 FR-290 refusal order and replay context (QSL PR #1217)

## Summary

Ticket: STD-262. This review covers the 8-step reader refusal order, the payload domains, the refusal
codes, the context's limits and dependencies, and the three ordered `request` refusals. Each payload
domain was checked against `proposals/backend-provider-v1/schema.json` (QSpec main 429c59d). The
spelling-first rule (step 3) is sound and matches FR-289. The FR-272 codes exist (SR-15490).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | minor: step 3 does not name the member it applies to. As written, every reader refuses a QSL-only cause spelling and a core-only basis spelling. So it is unclear what a `basis` `{"type":"replay-parity"}` or a `cause` `{"type":"closed-form"}` returns: `QslOnlyCause`/`CoreOnlyBasis` (step 3) or `UnknownSpelling` (step 4, not in that member's FR-331 set). Two implementers would differ, and the refusal table's member list (`cause`, `basis` or `certification`) does not settle it. State that the QSL-only check is the cause reader's and the core-only check is the basis reader's, and add one cross-member vector to AC-6 and AC-9. | spec/functional/FR-290-settle-plugin-results-as-typed-terminal-records.md:116-120 |
| FND-002 | medium | minor: the domain rules do not separate "valid FR-331 that QSL cannot hold" from "not FR-331" consistently, and they do not "accept exactly the FR-331 wire spellings". An unknown non-empty `method` is `Unsupported`, but an unknown non-empty `form` (FR-331: `Nonempty`) is `Malformed`. In the other direction, `state_key` is accepted as "a string" although FR-331 requires a `StateKey` hex digest, and the items of `prefix` are not checked as `ModelStep`. So the reader accepts values that FR-331 refuses. Make `form` `Unsupported`, as `method` is, and check `state_key` and `prefix` to the FR-331 domain, or say why the reader is looser. | spec/functional/FR-290-settle-plugin-results-as-typed-terminal-records.md:160,165 |
| FND-003 | medium | minor: `stage_limits` carries only the `CallerLimits::spine` settings, and `supplied_limits` is `None`. The library-resolution limits (`CallerLimits::library`) and the supplied admission ceilings the compile ran under are not carried, so the replay recompile runs them at the published defaults. The spec's claim "never under QSL's defaults" therefore holds only for the spine. If a caller raised those limits to compile, its package can fail to recompile in replay, and a real plugin counterexample then settles `inconclusive`/`replay-refused`. FR-098 also refuses before the recompile any `s1.input_bytes` above `replay.input_bytes`, and copying a raised `s1.input_bytes` can trigger that. Carry the other limits the compile used, or say that replay runs them at defaults and what follows. | spec/functional/FR-290-settle-plugin-results-as-typed-terminal-records.md:258-259 |
| FND-004 | low | minor: the `dependencies` row writes one entry per lock `dependency_selections` entry, with the library source that `check` resolved (`CheckedUnit::libraries`). `libraries()` is documented as "never the full dependency input": a lock entry the unit's imports never reached has no source. The spec does not say what such an entry carries, or whether it is left out. | spec/functional/FR-290-settle-plugin-results-as-typed-terminal-records.md:251 |
| FND-005 | low | minor: `request` refusal 3 (`Identity`, `noncanonical_wire`/`inexact-integer`) cannot happen. Every JSON integer in the `process-provider` preimage is a `u32` (`ordinal`, the path elements, a population `ordinal`), and every bound is a decimal string (O-09). So no test can exercise it, and AC-14 has none. The preimage's real failure mode has no refusal: two proof bounds with an equal `DomainKey`, which O-09 forbids ("no two entries having an equal `key`"). Replace step 3 with the duplicate-key refusal, or state what step 3 guards. | spec/functional/FR-290-settle-plugin-results-as-typed-terminal-records.md:289-291 |
| FND-006 | low | minor: `read_plugin_assignments` does not say whether it checks the entry's `request_index` and `backend` against the item and the plugin (FR-331's "originating run identity"). It also does not say where these fall in the order: an absent or non-array `values`, an unknown member at the entry or counterexample level, or a `counterexample` with no `type`, which could be `MissingCounterexample` or `MissingMember`. | spec/functional/FR-290-settle-plugin-results-as-typed-terminal-records.md:180-193 |

## Verdict

No major findings in this method. FND-001 to FND-006 are minor. FND-001 and FND-002 are the ones an
implementer would trip on.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-007 | low | minor: the new `request` refusal 3 (`UnresolvedDependency`) and its FR-290-AC-14 vector cannot be reached from consistent units. The FR-322 lock writes only the unit's direct imports (`CheckedPackage::direct_dependency_selections`). Every direct import is in the resolved closure, and both the lock and `CheckedUnit::libraries` come from the same `CheckedUnit`. The spec does not say how AC-14's "unit whose lock dependency has no resolved library source" is built. One possibility is a library that the dependency input supplies with no source; if so, name that case. Otherwise, call the step a defensive check and drop the vector, or test it with a constructed context. | spec/functional/FR-290-settle-plugin-results-as-typed-terminal-records.md:359-362 |

## Dispositions

Round 1, reviewed QSL #1217 at 6ce0503e2.

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | - |
| FND-002 | fixed | - |
| FND-003 | fixed | - |
| FND-004 | fixed | - |
| FND-005 | fixed | - |
| FND-006 | fixed | - |
