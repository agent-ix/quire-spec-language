---
id: SR-359
title: "Gap analysis of native received-Boolean choice admission"
type: SpecReview
analysis: gap-analysis
scope: "src/protocol_artifact/native/families.rs; src/protocol_artifact/native/families/decisions.rs; src/protocol_artifact/native/families/decisions/formula.rs; src/protocol_artifact/native/families/decisions/received.rs; tests/native_choice_emission.rs; tests/native_protocol_emission.rs; spec/functional/FR-042-publish-compiled-protocol-artifacts.md; spec/test-cases/TC-121-publish-compiled-protocol-artifacts.md; docs/compiled-protocol-v1.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-042
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-121
    type: references
---

## Summary

QUOIN `gap-analysis` examined `origin/main...cefc34e` for the received-Boolean
choice increment. Plan completion is not applicable: `plan/` has Plan-001
through Plan-009 but no protocol-artifact-emission bundle, consistent with
SR-355. Current `quire coverage --scope` reports 374/383 backed rows, six raw
unbacked rows (two `no_source_symbol` vocabulary exemptions), 20 inherited
untracked `NFR-007-M-*` symbols, three unmatched `IT-004` tags, and zero status
lies. The new nine-test lane supplies
TC-121 traces. The nine-test Boolean lane includes its required twelve-atom
`References` exhaustion, original locus and fresh retry scenario; no new
implementation defect is demonstrated.

## Verdict

**FAIL** — the corpus-wide matrix rule fails on inherited matrix debt, none in
FR-042/TC-121 scope. Four rows lack ordinary test backing; the other two are
`no_source_symbol` vocabulary exemptions, not unbacked-test failures. No finding makes the Boolean increment fail
independently. This is not a claim that the broader test, build or audit gates
passed.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | medium | Four inherited rows lack ordinary test backing and force the strict gap-analysis verdict; two additional raw rows are no-source-symbol vocabulary exemptions. All are outside this Boolean diff. | TC-115; FR-036-AC-5; FR-036-AC-6; FR-036-AC-8; TC-010 (exempt); FR-017-AC-2 (exempt) | correct-requirement-no-evidence |
| FND-002 | low | The inherited coverage report still has 20 untracked NFR-007 symbols and three unmatched IT-004 tags, all outside the increment. | src/package/encoding/tests.rs; tests/package_construction_cases/limits.rs; tests/fixture_audit.rs | correct-requirement-no-evidence |
| FND-003 | low | Full AC-9 per-dimension zero/exact/one-short vectors remain broader corpus debt, but this increment supplies its specified twelve-atom References/locus/retry case. | FR-042-AC-9; TC-121:174-176; native_choice_emission.rs:1035-1110 | correct-requirement-no-evidence |

### Scope and reverse-gap check

The source change has no public API: `decisions`, `formula`, and `received` are
private native-emission modules. Its concrete callers are the native family
lowerer and `tests/native_choice_emission.rs`; no stub, `todo!`, placeholder,
or re-export-only surface was found in the scoped diff. The retained original
handles and source loci are directly asserted by the focused tests. This review
does not audit protocol emission outside FR-042/TC-121 and does not substitute
for the pending repository gates.

The existing `native_owned_choice_proves_nonliteral_constant_guards_and_preserves_both_branches`
fixture was intentionally tightened: its positive path now uses fully closed
literals, while hidden dynamic operands in a short-circuited or unchosen position
are asserted as `Unsupported::FamilyProof`. That matches FR-042's updated
closed-classification rule; it is not a production-code regression.

### Capture boundary recheck

No positive capture test is missing. Protocol captures are evaluated at
activation, before a protocol run can receive a record; compensation captures
are locally scoped and are not visible at a choice. No admissible source shape
can therefore present a captured same-run received atom to this fragment.
FR-042's capture wording preserves authority and forbids scope bypass; it does
not promise Boolean-atom eligibility. Let-only alias traversal is consistent
with this language topology.

### FND-003 — retained work-vector debt

The twelve-atom fixture correctly proves that a finite valuation scan exhausts
the `References` budget before emission and that a fresh default retry succeeds.
The complete AC-9 matrix across all artifact dimensions is not supplied by this
increment, but it predates it as corpus-level acceptance debt. No evidence here
shows a changed counter, wrong dimension, lost locus, or partial admission; it
is not a Boolean implementation blocker.

## Coverage

`quire coverage --scope /home/peter/dev/worktrees/quire-language-native-boolean-choices
--json` (quire 0.31.0, engine `ca7362d4`; raw report
`/tmp/quire-boolean-coverage.json`) reports 374/383 raw backed rows. Four
non-exempt source-backed-required rows remain unbacked: TC-115 and
FR-036-AC-5/-AC-6/-AC-8. TC-010 is Manual and FR-017-AC-2 is Inspection, so
they are `no_source_symbol` vocabulary exemptions rather than test-backing
failures. FR-042 and TC-121 are backed by the scoped focused tests. Coverage
reports no `status_lies`, but inherited `status-column-matches-nothing`
diagnostics mean that result is not evidence of a fully reconciled corpus.

No plan bundle covers protocol-artifact emission; plan reconciliation is
therefore inapplicable (SR-355 precedent). The optional semantic review was not
selected. This scoped analysis does not claim full corpus acceptance.
