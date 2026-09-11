---
id: SR-344
title: "Gap analysis of ordered query proofs and native query emission against FR-040 and FR-042"
type: SpecReview
analysis: gap-analysis
scope: "src/checking/composed/proofs/engine/queries.rs; src/checking/composed/proofs/engine/walk.rs; src/protocol_artifact/native/layout.rs; src/protocol_artifact/native/values.rs; tests/composed_query_proofs.rs; tests/composed_proofs.rs; tests/native_query_emission.rs; spec/functional/FR-040-check-composed-values.md; spec/functional/FR-042-publish-compiled-protocol-artifacts.md; spec/test-cases/TC-119-check-composed-values.md; spec/test-cases/TC-121-publish-compiled-protocol-artifacts.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-040
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-042
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-119
    type: references
  - target: ix://agent-ix/quire-spec-language/TC-121
    type: references
---

## Summary

QUOIN `gap-analysis` over the ordered-query increment at `667226b`, with a fresh
`quire coverage --scope /home/peter/dev/worktrees/quire-language-native-query-proofs --json`
(quire 0.31.0, engine 0.46.0). No plan bundle targets FR-040/FR-042 —
`plan/` holds Plan-001..009, none covering composed value checking — so Step 1
(plan completion) is not applicable and matrix reconciliation is the operative
gate. The corpus reconciliation is byte-identical to the previous run: 367/376
rows backed, the same six unbacked rows and twenty untracked symbols, all
inherited from FR-036/FR-017/NFR-005/NFR-007 and all outside this change. The
optional semantic review (Step 4) was declined by the requester.

## Verdict

**FAIL** — retained solely by the skill's own rule that any matrix Test Case with
no backing tagged test fails the gate. TC-115 and three FR-036 acceptance rows,
TC-010/NFR-005-M-1 and FR-017-AC-2 are still unbacked, inherited and untouched
here. Read as a delivery signal this increment is CONDITIONAL: one medium
verification gap it introduces (FND-001), nothing regressed.

## Findings

| ID      | Severity | Summary                                                                                 | Refs                                                                    | Escape Cause                    |
| ------- | -------- | ---------------------------------------------------------------------------------------- | ----------------------------------------------------------------------- | ------------------------------- |
| FND-001 | medium   | `sum` at the declared 10,000 maximum is unverified and provably unreachable under clamped limits | src/checking/composed/proofs/engine/queries.rs:274; tests/composed_query_proofs.rs:492 | correct-requirement-no-evidence |
| FND-002 | medium   | Six inherited unbacked matrix rows remain; none is in FR-040/FR-042 scope                | spec/model-linking/tests.md:107; spec/tests.md:45; spec/functional/FR-036-link-composed-native-packages.md:129 | missing-requirement             |
| FND-003 | low      | Twenty inherited untracked NFR-007-M-* trace tags on package-encoding tests              | src/package/encoding/tests.rs:67; tests/package_construction_cases/limits.rs:55 | correct-requirement-no-evidence |
| FND-004 | low      | FR-042-AC-10 real producer-to-B handoff is still unmet; library fixtures use a synthetic baseline | spec/functional/FR-042-publish-compiled-protocol-artifacts.md:168; tests/support/native_protocol/mod.rs | correct-requirement-no-evidence |

## Coverage

`quire coverage --scope /home/peter/dev/worktrees/quire-language-native-query-proofs --json`:

- `totals`: 367 backed of 376 rows; 258 criteria; 83 property-shaped; 13 specific-shaped.
- `unbacked_rows`: 6 — `TC-115` (spec/model-linking/tests.md:107), `TC-010`
  (spec/tests.md:45), `FR-017-AC-2`, `FR-036-AC-5`, `FR-036-AC-6`, `FR-036-AC-8`.
  All inherited from prior increments; none touches FR-040, FR-042, TC-119 or
  TC-121.
- `status_lies`: 0.
- `untracked_symbols`: 20, all `NFR-007-M-2..5` on `src/package/encoding/tests.rs`
  and `tests/package_construction_cases/limits.rs`. Inherited.
- `quire validate --scope <worktree> 'spec/**/*.md'`: 398/398 docs grammar-clean,
  0 grammar findings. The `DuplicateArchetype` / `DuplicateInverseEdge` lines are
  first-wins module notices from the installed `spec-artifacts-process` module,
  not findings against this repo.

### What this increment adds to the matrix

Nine new `tests/composed_query_proofs.rs` cases tagged `TC-119` with
`FR-040-AC-1/3/4/5/6/8/9`, three new `tests/native_query_emission.rs` cases tagged
`TC-119`+`TC-121` with `FR-040-AC-5`, `FR-042-AC-3/4/7`, and one retagged
`tests/composed_proofs.rs` case moved from `FR-040-AC-4` to `FR-040-AC-5`. Every
cited id resolves; the binder minted no new unbacked row and no status lie.

### Reverse gap (underspecified code)

`Unsupported::SumDomainTransfer` is a new public enum variant. It has an owning
requirement — FR-040's narrative now names broader rational sum-domain transfer as
an explicit unsupported prerequisite — and a backing test
(`rational_sum_separates_supported_prefixes_from_missing_domain_transfer`). Its
sibling `Unsupported::OrderedQuery` is now the reverse case: retained public
surface with no producer and no owning behaviour. Carried as FND-002 in the
code review rather than duplicated here.

No stub masquerading as complete was found in the changed source: `queries.rs`
contains no `todo!`/`unimplemented!`/`TODO`/`FIXME`, no placeholder returns, and
every new path terminates in either a real IR goal, a typed cause or a bounded
`Exhaustion`.

### Remaining acceptance for this scope

Not inferable from these controls and still open: query runtime execution,
B consumer conformance (FR-042-AC-10), recovery admission, general protocol
decision proofs, and full FR-040/FR-042 acceptance. `sum` at N=10,000
(FND-001) is the one item this increment newly puts in reach of a test and
leaves untested.
