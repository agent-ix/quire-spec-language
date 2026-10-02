---
id: SR-348
title: "Gap analysis of source-owned compensation emission against FR-042 and TC-121"
type: SpecReview
analysis: gap-analysis
scope: "src/protocol_artifact/native/runtime.rs; src/protocol_artifact/native/runtime/compensations.rs; src/protocol_artifact/native/layout.rs; src/protocol_artifact/native/controls.rs; src/protocol_artifact/validate.rs; src/protocol_artifact/validate/control.rs; tests/native_compensation_emission.rs; spec/functional/FR-042-publish-compiled-protocol-artifacts.md; spec/test-cases/TC-121-publish-compiled-protocol-artifacts.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-042
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-121
    type: references
---

## Summary

QUOIN `gap-analysis` recheck over the correction commit `b7aafe1`, re-running
`quire coverage --scope /home/peter/dev/worktrees/quire-language-native-compensation
--json` (quire 0.31.0, engine `ca7362d4`). Scope is the `c7275f5..b7aafe1` diff
and the requirements it touches; the rest of the increment is not re-analysed.
Step 1 (plan completion) remains not applicable — `plan/` holds Plan-001..009,
none covering protocol-artifact emission — so matrix reconciliation is the
operative gate. The optional semantic review (Step 4) remains declined by the
requester. The corpus reconciliation is unchanged from `c7275f5`: 367/376 rows
backed, the same six unbacked rows, twenty untracked symbols and three unmatched
tags, all inherited and all outside this change. The previously high typed-effect
gap is closed in both directions. One new reverse gap replaces it: FR-042 now
obliges the reader to accept exactly the declaration's population/closure pairs
"for recovery and contributing captured origins" without saying which
construction defines *contributing*, and the emitter and reader define it
differently.

## Verdict

**FAIL** — retained solely by the skill's own rule that any matrix Test Case
with no backing tagged test fails the gate. That debt is inherited
(FR-036/TC-115/FR-017/NFR-005), byte-identical to the previous increment and
untouched by this change. No finding of this increment is high. Read as a
delivery signal, the increment itself is clean at medium; the FAIL is corpus
debt, not this change.

## Findings

| ID      | Severity | Summary                                                                                                       | Refs                                                                                                     | Escape Cause                        |
| ------- | -------- | --------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------- | ----------------------------------- |
| FND-001 | medium   | Six inherited unbacked matrix rows remain; none is in FR-042/TC-121 scope                                      | spec/model-linking/tests.md:107; spec/tests.md:45; spec/functional/FR-036-link-composed-native-packages.md:129 | missing-requirement                 |
| FND-002 | medium   | FR-042's new `recovery_bindings` SHALL does not say which construction defines a "contributing captured origin"; emitter and reader each define it, differently | spec/functional/FR-042-publish-compiled-protocol-artifacts.md:167; src/protocol_artifact/native/runtime/compensations.rs:462; src/protocol_artifact/validate/control.rs:986 | missing-requirement                 |
| FND-003 | low      | Twenty inherited untracked `NFR-007-M-*` trace tags on package-encoding tests                                  | src/package/encoding/tests.rs:66; tests/package_construction_cases/limits.rs:54                              | correct-requirement-no-evidence     |
| FND-004 | low      | The protocol `relationships` explicit-Unsupported branch still has no test                                     | src/protocol_artifact/native/runtime.rs:563                                                                  | correct-requirement-no-evidence     |
| FND-005 | low      | FR-042-AC-9 still has no compensation work-dimension vector, and the correction adds further charged dimensions without one | spec/functional/FR-042-publish-compiled-protocol-artifacts.md:206; src/protocol_artifact/validate/control.rs:1029 | correct-requirement-no-evidence     |

## Dispositions of the SR-348 findings recorded at c7275f5

| Prior   | Disposition                                                                                                                          |
| ------- | -------------------------------------------------------------------------------------------------------------------------------------- |
| FND-001 | **Resolved.** TC-121 step 6 now names both typed-effect mutations and FR-042 states the `Unsupported::Export` obligation; `adding_a_payload_type_cannot_bypass_compensation_effect_authority` backs both, tagged `TC-121`/`FR-042-AC-6/7/8`. |
| FND-002 | **Open**, unchanged and inherited. Carried above as FND-001.                                                                          |
| FND-003 | **Resolved as a requirement gap** — FR-042 lines 160-171 now state the reader's clock, snapshot, progress/closure and `recovery_bindings` obligations, and `compensation_recovery` enforces them. The residual is the *definition* of the inventory, carried above as FND-002. |
| FND-004 | **Open**, unchanged and inherited. Carried above as FND-003.                                                                          |
| FND-005 | **Open**, unchanged. Carried above as FND-004.                                                                                        |

### FND-002 — the requirement stops one level short

FR-042 now says `recovery_bindings` SHALL contain "exactly those three records
and the declaration's population/closure pairs for recovery and contributing
captured origins". Two implementations answer *which* origins contribute, and
they do not agree:

- the emitter (`compensations.rs:437-523`) selects typed nodes by source-span
  containment inside the `recover` expression regions and admits a `Closure`
  member when its model is non-null **and** its single `requires` names a
  `Population` binding;
- the reader (`control.rs:986-1021`) walks the validated
  operand/initializer/origin `value_edges` graph and admits a `Closure` member
  when its model's export kind is `Population`, with no `requires` constraint.

Because the reader compares the two inventories for exact equality, a divergence
refuses a package this repository's own compiler produced. That is a
requirement-level gap, not only a code defect: nothing in FR-042 or TC-121
nominates an authoritative construction, and no AC would catch the divergence.
Filed as FND-001 in SR-347 on the code side and FND-001 in SR-350 as a failure
mode.

## Coverage

`quire coverage --scope /home/peter/dev/worktrees/quire-language-native-compensation --json`:

- `totals`: 367 backed of 376 rows; 258 criteria; 83 property-shaped; 13
  specific-shaped.
- `unbacked_rows`: 6 — `TC-115` (spec/model-linking/tests.md:107), `TC-010`
  (spec/tests.md:45), `FR-017-AC-2`, `FR-036-AC-5`, `FR-036-AC-6`,
  `FR-036-AC-8`. All inherited; none touches FR-042 or TC-121. `TC-010` and
  `FR-017-AC-2` are the two `no_symbol_rows` (`Manual`, `Inspection`), so only
  the four FR-036/TC-115 rows are non-exempt.
- `status_lies`: 0. `unmatched_tags`: 3, all IT-004 `fixture_audit` symbols in
  the private-packet lane. `binding_census`: 1.
- `untracked_symbols`: 20, all `NFR-007-M-*` on `src/package/encoding/tests.rs`
  and `tests/package_construction_cases/limits.rs`. Inherited.
- `suspicions`: 5, one more than at `c7275f5` and the same item throughout — the
  `integer` helper resembles `integer_value` in
  `tests/native_protocol_emission.rs` (similarity 0.86). The new occurrence is
  the attempt-bound test calling that helper. It unwraps a checked wire number
  for comparison against authored literals, so it is not a re-implemented
  oracle; recorded, not filed as a finding.

### Matrix backing for this increment

The ten compensation tests all carry `#[trace("TC-121", ...)]` with resolving
ACs; the three added by `b7aafe1` carry FR-042-AC-3/6/7/8. No new untracked test
symbol appeared and `status_lies` is still 0, so every cited id resolves.
FR-042-AC-6 remains backed. TC-121 step 6 as rewritten — typed-effect insert,
typed insert plus foreign operation, clock/snapshot/progress/closure swaps, each
prerequisite severed singly, unrelated population pairs, and the signed-64 bound
with its overflow refusal — is backed by real assertions in
`adding_a_payload_type_cannot_bypass_compensation_effect_authority`,
`compensation_clocks_and_recovery_premises_cannot_cross_obligations_or_lose_edges`
and `compensation_attempt_bound_preserves_signed64_maximum_and_refuses_one_beyond`,
each mutation preceded by an assertion that the original edge or member exists
exactly once. Reader checks that exceed what TC-121 asks for remain unexercised;
that is recorded as FND-002 in SR-347, not as a matrix gap.

### Reverse gap — code without an owning requirement

- `AwaitAnchor::Compensation` and compensation-associated domain events
  (`controls.rs:150`, `controls.rs:256`) are emitted and tested, but FR-042
  never names them; `FR-042-AC-5` covers "await branches" generically and
  `FR-042-AC-6` covers compensations without mentioning that an await may anchor
  to one. The seventh test is the only place the ordinal→handle mapping under an
  interleaved temporal requirement is stated at all. Low-severity spec debt, not
  a code defect; recorded here rather than as a separate finding because the
  behaviour is covered by the two ACs jointly.
- `Anchor::Registration/CompensationActivation/Retry/Recovery` now resolve in
  `layout.rs:680` and `runtime.rs:383` where they previously returned
  `Unsupported::Export`. Both new paths are reached by the positive tests.
- `recovery_origins` (`control.rs:1021`) is a second, independent reachability
  construction with no owning requirement. FND-002 above.

### Remaining acceptance (not defects of this increment)

- **FR-042-AC-10** — the real producer-to-B handoff is still open. The library
  fixtures here use an explicitly synthetic producer/baseline; the landed
  producer recipe in PR55 does not close the consumer side. Untouched by
  `b7aafe1`.
- **FR-042-AC-6, relationships/correspondence half** — protocol `relationships`
  remain `Unsupported::Export` (`runtime.rs:563`), and general family proof
  support stays Unsupported. Explicit and intended; FND-004 is only that the
  refusal has no test.
- **FR-042-AC-9** — still no compensation-specific work-dimension vector, and
  `b7aafe1` adds charged dimensions rather than removing the gap:
  `Entries` for the three fixed recovery records and one per admitted population
  member, `References` for each `requires` comparison and for the sorted
  progress/closure triple, plus `Entries` per visited value node and per pushed
  edge in `recovery_origins`. All are charged before the work, and none has a
  zero/exact/one-short case of its own. FND-005 above.
- **Inherited matrix debt** — the six unbacked rows and twenty untracked symbols
  are corpus-wide FR-036/TC-115/FR-017/NFR-005/NFR-007 debt tracked for the later
  testing and assurance campaign. They are the reason the gate reads FAIL and are
  deliberately kept separate from the defects of this increment.
