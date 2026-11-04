---
id: SR-20261104190828
title: "Gap analysis of PR #2309: QSL-1641 A bytes-only temporal evidence entry (FR-531 temporal part, FR-532)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@51570af56 trial-merged with origin/main 0642337ef; spec/functional/FR-531, FR-532, FR-515, FR-516, FR-523; tests/it/bare_package.rs, tests/it/temporal_package_route*.rs, qsl-eval/tests/it/temporal_causes.rs; ticket QSL-1641"
review_set: subset
---

## Summary

Ticket: QSL-1641. PR: quire-spec-language#2309, head 51570af56. Method: quoin gap-analysis by hand (the AC-to-test table by reading the tags and the assertions) plus mutation checks on the new logic only.

Criteria due in this PR (FR-532 is delivered here whole except the gated AC-11; FR-531's temporal part is AC-2, AC-6, AC-9, AC-10, AC-11): FR-532-AC-1 to AC-10 and AC-12, FR-531-AC-2, AC-6, AC-9, AC-10, AC-11. Criteria that belong to the identity-file PR (#2311) or part B: FR-531-AC-1, AC-3, AC-5, AC-7 (the slot half), AC-8, AC-12, AC-13, the slot half of AC-4. FR-532-AC-11 is gated on QSL-1643 and QSL-1644 and `plan_node` faults `aggregate-formula-not-emitted` for it, as the spec states.

Mutation checks (each reverted; tests `bare_package` and `temporal_package_route` with `--all-features`, `qsl-eval --test it temporal`, `qsl-package --lib temporal`):

| Mutation | Result |
| --- | --- |
| profile of every package clause forced to event-position (`finite_window_profile`) | killed: two root tests fail |
| atom number keyed by use position, not by node (`atom_numbers`) | SURVIVED |
| `BarePackage::dependencies` without the recursion into child dependencies | SURVIVED |
| `capture_value_types` returns no captures | SURVIVED |
| `instance_bound_atoms` leaf-count guard removed | SURVIVED |
| `release`, `historically` and `triggered` mapped to the non-dual scan in `plan_node` | SURVIVED (root `it`, `qsl-eval` temporal and `qsl-replay` temporal all pass) |

Examined, clean: FR-532-AC-5 (binding names the package, equal on the two routes), AC-6 (`AmbiguousName` only, by source inspection), AC-9 (inspection), AC-10 for the single-atom shape, FR-531-AC-2 (compile-fail probes and signature scan), AC-10 (charge equality), AC-11 (consumer-entry sections). FR-531-AC-9 reads the declarations by indentation scraping where the AC says `syn`.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | No test covers the new refusal and fact-building paths of FR-532-AC-12 and the temporal half of FR-531-AC-6: `clause-name`, `capture-type`, `profile-unknown`, `profile-selection`, absent clock rows, and capture names and value types equal to the in-process clause. With `capture_value_types` returning nothing, every test still passes. Due in this PR. | qsl-eval/src/temporal/evidence/package.rs:367-385, qsl-package/src/checked_views/subject.rs:303-316 |
| FND-002 | medium | No test builds a changed package for FR-532-AC-2, AC-3, AC-7 or AC-8 over `from_package`. All three routes build their facts through the one `subject()` from the same bytes, so the three-route equality of FR-532-AC-1 cannot fail on a facts error; only a mutated-package test (AC-2) shows that the bytes, not the source, drive the result. The `plan_node` operator table is new and untested: mapping `release`, `historically` and `triggered` to the wrong dual survives. Due in this PR. | tests/it/bare_package.rs:20-67, qsl-eval/src/temporal/facts.rs:450-480 |
| FND-003 | medium | The three-route test asserts only `ActivationState::Active` and equal `Debug` strings for 1 and 3 ticks. FR-532-AC-1 names one request per truth outcome (`True`, `False`, `Pending`), per activation state and per `Incomplete` counter, and the canonical byte encoding; none of those variants is requested, and the test carries the FR-532-AC-1 tag. | tests/it/bare_package.rs:20-67 |
| FND-004 | medium | Atom numbering by first use over distinct nodes is tested only for one shared atom alone. A clause with a shared atom followed by a distinct atom (`holds(a) and holds(a) and holds(b)`, atom 1 for `b`), a shared atom read by an instance binder, the swapped order, an arena order that differs from first use, and a request keyed by the arena index (FR-532-AC-4, AC-10) are not tested; numbering by use position survives. | tests/it/bare_package.rs:89-150, qsl-eval/src/temporal/facts.rs:249-262 |
| FND-005 | medium | The `predicate_leaves()` count against `holds` count refusal (FR-516-AC-39, FR-532-AC-7) is now untested: the only test of the branch lost its `None` assertions, and removing the guard in `instance_bound_atoms` survives. The guard is now the only protection, since `bound_atom_indices` no longer checks. | qsl-package/src/checked_views/predicate.rs:282, qsl-eval/tests/it/temporal_causes.rs:294-310 |
| FND-006 | low | The IR-1325 closure test cannot fail on the recursion or the deduplication of `dependencies()`: R's library has no transitive dependency, so no fixture has a chain or a diamond. | tests/it/bare_package.rs:176-188 |
| FND-007 | low | FR-531-AC-4 (R's three clause forms and the non-clause refusal over the bare package) is covered by `a_node_that_is_no_temporal_clause_is_unresolved_over_the_bare_package` and the three-route test but is tagged with neither, and FR-531-AC-9 reads declarations by indentation, not with `syn` as the AC states. | tests/it/bare_package.rs:391-420, tests/it/bare_package.rs:228-268 |

## Verdict

Not mergeable as written. FND-001 to FND-005 are criteria of this PR's own requirement (FR-532) or tests whose removal this PR caused, and each names new logic that survives mutation. The coder's list resolves as: due here and a gap: FR-532-AC-2, AC-3, AC-7, AC-8, AC-12 and FR-531-AC-6 (temporal half); belong to #2311 or part B: FR-531-AC-1, AC-3, AC-5, AC-7 (slot half), AC-8, AC-12, AC-13; gated: FR-532-AC-11. FR-531-AC-7 (temporal half: no source label or span in the encoded result) is due with the PR that delivers the slot value, provided it covers `EvidenceResult` too.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-008 | low | FR-531-AC-6 and FR-532-AC-7/AC-12 name packages (no `temporal_clause_names` row, no `profile_selections` row, no clock row, an unbounded interval) that Contract IR refuses before any evaluation, so `clause-name`, `profile-selection`, the absent-clock `Unavailable` result and the bare `not-finite-window-profile` row cannot be met by an admitted package. The tests assert the reader's refusal by Debug substring (`TimedNodeReference`) and, for the missing profile row, only that the text is non-empty. The spec lane should restate those ACs as reader refusals; the branches in `subject()` are defensive. | tests/it/bare_package.rs:608-632, tests/it/bare_package.rs:806-830, qsl-eval/src/temporal/evidence/package.rs:367-372 |
| FND-009 | low | The third sentence of FR-532-AC-4 (a request keyed by the arena index reads a different atom) is untested on a route; the facts unit test `a_shared_node_is_one_node_and_one_atom_numbered_by_first_use` asserts counts only and survives numbering by use position (only `bound_atom_indices`' unit test kills it); `let _ = EvidenceCounter::FormulaNodes;` is a leftover in `matrix`. | qsl-eval/src/temporal/facts.rs:637-654, tests/it/bare_package.rs:62 |

## Dispositions

Round 1, head 003b53ccd. Mutation re-runs (each reverted): atom numbering by use position KILLED (`a_leaf_count_that_differs_from_the_holds_count_maps_no_atoms`); `dependencies()` without recursion KILLED (chain and diamond test); `capture_value_types` returning none KILLED (`a_record_typed_capture_projects_with_the_source_identities`); `instance_bound_atoms` view guard removed SURVIVES as an equivalent mutant (`bound_atom_indices` returns the same `atom-count` reason); `bound_atom_indices` guard removed KILLED; `release`, `historically`, `triggered` mapped to the non-dual scan KILLED (`each_operation_maps_to_its_scan_family_and_dual`).

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | Record-typed captures project with the source identities and equal the in-process types; the reasons that no admitted package can meet are asserted as reader refusals (FND-008). |
| FND-002 | fixed | Forged-bytes tests: a clock row replaced drives the bare result (`binding-mismatch`, `Bindings` on `from_emission`); a replaced interval changes usage and the source routes refuse `formula-differs`; twins pair by key over `from_package`; the 12 operators agree over three routes and the plan table is unit-tested. The cycle, out-of-range operand and timed-interval shapes are reader refusals and `ClauseFormula` unit tests. |
| FND-003 | fixed | `matrix` runs 4 clauses over truth, activation and `Incomplete` requests per counter; the test asserts True, False, Pending, Active, Inactive, Unavailable and an incomplete row were seen. |
| FND-004 | fixed | `after[2].node = after[1].node` case (distinct atom after a shared one is atom 1), shared instance-bound atom and swapped order are asserted; numbering by use position is killed. Residue: FND-009. |
| FND-005 | fixed | `None` assertions restored; guard removal killed at `bound_atom_indices`. |
| FND-006 | fixed | Chain and diamond fixtures. |
| FND-007 | fixed | `FR-531-AC-4` tagged; AC-9 reads declarations with `syn` and carries a probe that a missing row fails. |
