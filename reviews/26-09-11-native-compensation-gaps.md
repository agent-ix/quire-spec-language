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

QUOIN `gap-analysis` over the compensation-emission increment at `c7275f5`, with
a fresh `quire coverage --scope
/home/peter/dev/worktrees/quire-language-native-compensation --json` (quire
0.31.0, engine 0.46.0). No plan bundle targets FR-042 — `plan/` holds
Plan-001..009, none covering protocol-artifact emission — so Step 1 (plan
completion) is not applicable and matrix reconciliation is the operative gate.
The corpus reconciliation is byte-identical to the previous increment: 367/376
rows backed, the same six unbacked rows and twenty untracked symbols, all
inherited from FR-036/FR-017/NFR-005/NFR-007 and all outside this change. The
optional semantic review (Step 4) was declined by the requester. The one new
reverse gap is the typed compensation-effect lane: TC-121 step 6 and
`docs/compiled-protocol-v1.md` both make a claim about it that no test and no
reader check back.

## Verdict

**FAIL** — retained by the skill's own rule that any matrix Test Case with no
backing tagged test fails the gate, and additionally by FND-001, which is a
`high` finding of this increment (detailed as FND-001 in SR-347). Read as a
delivery signal: the inherited corpus debt is unchanged and untouched here, but
this increment does carry one high defect of its own, so it is not a clean
merge as it stands.

## Findings

| ID      | Severity | Summary                                                                                                       | Refs                                                                                                     | Escape Cause                        |
| ------- | -------- | --------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------- | ----------------------------------- |
| FND-001 | high     | The typed compensation-effect lane asserted by TC-121 step 6 is neither tested nor enforced; a value type disables the identity gate | src/protocol_artifact/validate.rs:620; spec/test-cases/TC-121-publish-compiled-protocol-artifacts.md:94  | implementation-bug-despite-evidence |
| FND-002 | medium   | Six inherited unbacked matrix rows remain; none is in FR-042/TC-121 scope                                      | spec/model-linking/tests.md:107; spec/tests.md:45; spec/functional/FR-036-link-composed-native-packages.md:129 | missing-requirement                 |
| FND-003 | medium   | Compensation clock and recovery authorities are emitted with exact prerequisites but no requirement states the reader obligation, and none is enforced | spec/functional/FR-042-publish-compiled-protocol-artifacts.md:155; src/protocol_artifact/validate/control.rs:911 | missing-requirement                 |
| FND-004 | low      | Twenty inherited untracked `NFR-007-M-*` trace tags on package-encoding tests                                  | src/package/encoding/tests.rs:66; tests/package_construction_cases/limits.rs:54                              | correct-requirement-no-evidence     |
| FND-005 | low      | The protocol `relationships` explicit-Unsupported branch — the other half of the guard this change split — has no test | src/protocol_artifact/native/runtime.rs:563                                                                  | correct-requirement-no-evidence     |

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
- `suspicions`: 4, two of them new and both the same item — the `integer` helper
  at `tests/native_compensation_emission.rs:149` resembles `integer_value` in
  `tests/native_protocol_emission.rs` (similarity 0.86). It unwraps a checked
  wire number for comparison against authored literals, so it is not a
  re-implemented oracle; recorded, not filed as a finding.

### Matrix backing for this increment

The seven new tests all carry `#[trace("TC-121", ...)]` with resolving ACs:
FR-042-AC-3/4/5/6/7/8 plus FR-036-AC-4 and FR-040-AC-5 on the adverse cases. No
new untracked test symbol appeared, and FR-042-AC-6 remains backed. TC-121 steps
newly claimed by this increment are backed by real assertions except the typed
effect view (FND-001) and the recovery-authority swaps (FND-003).

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

### Remaining acceptance (not defects of this increment)

- **FR-042-AC-10** — the real producer-to-B handoff is still open. The library
  fixtures here use an explicitly synthetic producer/baseline; the landed
  producer recipe in PR55 does not close the consumer side.
- **FR-042-AC-6, relationships/correspondence half** — protocol `relationships`
  remain `Unsupported::Export` (`runtime.rs:563`), and general family proof
  support stays Unsupported. Explicit and intended; FND-005 is only that the
  refusal has no test.
- **FR-042-AC-9** — no compensation-specific work-dimension vector was added;
  the new charges (`Entries` per binding, `References` per binder scan, `bytes`
  per name) are charged before the work but have no zero/exact/one-short case of
  their own.
