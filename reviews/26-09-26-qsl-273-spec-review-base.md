---
id: SR-660
title: "QSL-273 base checklist review of state clauses on the spine"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@d8b74aba7d349ccb3989583cc4e608aad301c38b; spec/decisions/ADR-012-semantic-family-extension-contracts.md (§15); spec/functional/FR-102 to FR-109; spec/test-cases/TC-456 to TC-469; spec/spec.md; spec/tests.md; code evidence qsl-semantics/src/model/intake.rs, qsl-semantics/src/check/assemble.rs, qsl-semantics/src/model/population.rs, qsl-semantics/src/check/facts.rs, qsl-semantics/src/check/lowering.rs, qsl-forms/src/dispatch.rs, qsl-forms/src/value.rs, qsl-cst/src/grammar.rs, examples/config-version/cases.rs, tests/it/config_version.rs, src/runtime/validation/values.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-104
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-106
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-107
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-108
    type: reviews
---

## Summary

Ticket: QSL-273 (PR agent-ix/quire-spec-language#462, spec only). This is the
base checklist over the 25 changed files: ID formats, AC testability, oracles,
coverage, and the author's code claims, measured against the code at the
reviewed sha.

Measured and clean:

- Operations are refused at intake: a frame with any non-empty member returns
  `unsupported_at(.., "operation.frame")` (qsl-semantics/src/model/intake.rs:1263-1289).
- Operations are refused at assembly: every `OperationMember` record gets
  `AssemblyCause::UnsupportedModelMember`. The arm is at
  qsl-semantics/src/check/assemble.rs:405-409. The cited range 353-381 is the
  doc comment and the `unsupported` closure (FND-006).
- The spine frame check compares only reference-valued fields.
  `enforce_frame` compares `PopulationMember::field_values`, which holds
  reference-valued fields only (qsl-semantics/src/model/population.rs:466-475,
  1381ff).
- S2 refuses `self`, `result` and `reaches` today
  (qsl-forms/src/value.rs:1039-1041), and S1 parses the three state clause
  forms (qsl-cst/src/grammar.rs:566-591).
- The native corpus has 13 cases (examples/config-version/cases.rs:110-170),
  and every FR-108 truth value matches the catalog rows. Native checks integer
  bounds at validate with `invalid_runtime_input`
  (src/runtime/validation/values.rs:216-221), so the four boundary rows are
  consistent with native.
- IDs are well formed (FR-102 to FR-109, TC-456 to TC-469). Every AC has a TC.
  The spec.md and tests.md rows are present. `quire validate` over the changed
  files exits 0 with 18 EARS warnings (SR-661).
- FR-108's 17 expected dispositions are independent of the implementation.
  Each is hand-derivable from the case rows.
- FR-106's 11 checks are in a fixed order, and each names a catalog code.
  Their sub-checks are not ordered (FND-002).

Verdict: changes requested. One high finding: the ConfigVersion `ParentOrder`
clause cannot pass S3's definedness check as specified.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | FR-104 says `value(self.parent)` under `present(self.parent) implies` "checks" with the definedness check "it has today". Today it cannot. FR-104 types `self.f` as the `Attribute` node (`deref(self).f`). The definedness walk derives presence facts only for `Local`, `Field` and `Value` stable paths (qsl-semantics/src/check/facts.rs:405-426). `Attribute` returns `None`, so `present(self.parent)` records no fact (facts.rs:569-576), and `value(self.parent)` refuses `Obligation::Presence` (facts.rs:887-890). Failure scenario: TC-459 step 1 gets `undefined_expression`/`unproved-presence` on `ParentOrder`. Then every `ParentOrder` corpus case (10 of 17 in FR-108) reports stage `compile`, not its table row, and FR-107-AC-2's `pre(present(self.parent) implies ...)` refuses too. Fix: add a Behavior bullet and an AC. Presence and interval facts must cover `self.f` and `deref(r).f` paths, keyed by observation, so that a pre fact never discharges a post read (see SR-664 FND-003). | spec/functional/FR-104-check-state-clauses.md:104-107, 126; spec/test-cases/TC-459-s3-checks-configversion-state-clauses.md:27-29 |
| FND-002 | medium | FR-106 fixes the order of its 11 checks, but not the order inside a check. Check 1 has seven conditions (present, limits, format, members, labels, digest) and one unordered failure list. Check 6 applies six conditions across every population and object, in no stated order. Checks 8 and 11 can each find several defects and report one. FR-106 also puts decode and shape before the digest. FR-056, the rule it cites, digests first (check 3, then shape). Failure scenario: a document with an unknown member whose bytes were also edited under the original digest, or a snapshot with `root` at -1 and `child` at 1001. One implementation reports `unknown-member` or `root`, another `byte-digest-mismatch` or `child`. Both pass TC-465, whose rows each carry one defect. Fix: order the sub-checks (following FR-056 for read) and the object walk (document order or key order), and add one multi-defect AC. | spec/functional/FR-106-admit-snapshots-and-invocations.md:132-147, 159-167, 173-176, 186-192 |
| FND-003 | medium | FR-107-AC-3's meter oracle comes from the implementation under test: "every `k` below the expansions it needs ... completes at that number". The number is not stated. FR-107 also charges once per expansion while the budget counts every S6a charge, so the threshold is not the expansion count. Failure scenario: an implementation that charges three times per expansion, or charges `deref` reads too, still passes TC-466 step 3, because the test finds the threshold by search. Fix: state the expected count for `reaches(a, c)` over `chain`. Under FR-107's rule that is 2 expansions (`a`, then `b`, which discovers `c`). Also say which other nodes charge, or assert the exact charge log. | spec/functional/FR-107-evaluate-state-clauses-at-s6a.md:77-83, 102; spec/test-cases/TC-466-s6a-evaluates-state-clauses.md:37-40, 50-51 |
| FND-004 | low | Two refusal oracles name no code. FR-104-AC-3 and TC-460 row 4 say "the code a duplicate function name gets". FR-104-AC-4 and TC-460 row 8 say "the definedness cause it has today". Failure scenario: an implementation that refuses a duplicate clause, or an unguarded `value(self.parent)`, with any code passes. Pin the code and cause: `undefined_expression`/`unproved-presence` for row 8 (qsl-semantics/src/check/refusal.rs:591), and the measured duplicate-declaration code for row 4. | spec/functional/FR-104-check-state-clauses.md:73-74, 128-129; spec/test-cases/TC-460-s3-refuses-ill-formed-state-clauses.md:44, 48 |
| FND-005 | low | FR-106-AC-3 says "each check 1 to 10 has one case", but it lists no case for checks 7 and 8 (AC-4 covers them). No AC case covers `nesting-depth-exceeded`, `missing-member` at read, or `wrong-role-mapping` for an unknown population or type (check 6). `unknown_wire` names no cause, although the catalog pairs it with `unsupported-wire`. Failure scenario: an implementation that reports a missing top-level member as `unknown-member`, or an unknown population as `conflicting-identity`, passes TC-465. | spec/functional/FR-106-admit-snapshots-and-invocations.md:139-146, 164-167, 207; spec/test-cases/TC-465-admission-refuses-each-input-defect.md:28-49 |
| FND-006 | low | FR-103 cites qsl-semantics/src/check/assemble.rs:353-381 for the operation refusal. That range is the doc comment and the `unsupported` closure. The refusal arm is assemble.rs:405-409. Failure scenario: an implementer following the citation edits the closure, not the match arm that refuses `OperationMember`. | spec/functional/FR-103-admit-model-operations-and-frames-on-the-spine.md:30-34 |
| FND-007 | low | FR-108's Corpus table gives `self` for every `Current` case but not for the three invocation cases (unchanged-version, changed-version, forbidden-parent-change). The native catalog's `Input::Update` carries no self key (examples/config-version/cases.rs:60-63). TC-464 fixes `child` for changed-version only. Failure scenario: the fixtures generator picks `root` for unchanged-version. `root` is unchanged by every invocation, so the case still passes, and changed-version's `false` would then rest on a different object than native's. State `self child` in the three rows. | spec/functional/FR-108-run-the-configversion-spine-corpus.md:81-83 |
| FND-008 | low | FR-108 gives the new domain package fixture and the generated unit `AGPL-3.0-only`. The repository's sources and Cargo.toml are `AGPL-3.0-or-later` (571 SPDX headers; Cargo.toml:16). Failure scenario: new fixtures ship under a different licence from the files beside them. Use `AGPL-3.0-or-later`, or say why these differ. | spec/functional/FR-108-run-the-configversion-spine-corpus.md:39-46 |

## Verdict

Changes requested. FND-001 blocks FR-104-AC-1 and most of FR-108.
