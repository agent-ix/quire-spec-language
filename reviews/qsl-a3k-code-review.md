---
id: SR-1254
title: "Code review of quire-spec-language PR #611: ModelOwner preimage and model declaration key without version (A3k)"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@d21fb55865c68e4fff12aee04a0c8fe8ab571fa3; PR #611 diff against origin/main (merge base 652ae3d5): qsl-semantics/src/check/node_key/mod.rs, qsl-semantics/src/check/node_key/tests.rs, qsl-semantics/src/check/lowering/model.rs, qsl-semantics/src/check/lowering/model/tests.rs, qsl-package/src/emit.rs, qsl-package/src/emit/tests.rs, spec/functional/FR-094-key-model-owned-reference-population-and-quantity-nodes.md (golden vectors), spec/test-cases/TC-417-model-reference-and-population-nodes-match-golden-vectors.md, spec/test-cases/TC-418-clause-function-nodes-carry-model-owner.md; TEMP commit 52750d617 (Cargo.toml, qsl-package/Cargo.toml, Cargo.lock) checked for shape only"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-094
    type: reviews
---
# Code review of quire-spec-language PR #611

## Summary

Ticket: A3k. The Linear id is not yet known (placeholder: `QSL-TBD-A3k`), because
the Linear keyring is locked. PR: quire-spec-language#611, in lockstep with
quire-contract-ir#250 (IR-505). Reviewed head d21fb558, two commits on main
652ae3d5. The Rust lane (rust-review) is folded into this file.

**TEMP commit 52750d617.** It touches only `Cargo.toml`, `qsl-package/Cargo.toml`
and the `quire-contract-model` entry of `Cargo.lock` (branch
`feat/model-owner-content-only` at 81cd8036). The `quire-verification-contracts`
lock entry stays at ead78f3f. It is not reviewed as a finding, per the brief.

**The A3k change is correct and complete on the code side.**

- `node_key::ModelOwner` is now `{identity, kind, node}`. `version`, its
  accessor and `InvalidModelOwner::EmptyVersion` are gone. The fields serialize
  in alphabetical order, so the preimage stays RFC 8785-ordered.
- `Lowering::model_owner` builds the owner from the selection identity and the
  `DeclarationKey` node only. Model declaration nodes and clause-function nodes
  both get their owner from it (model.rs:268, :280, :535), so both are keyed by
  `{kind, identity, node}`.
- `emit::nominal_owner` no longer writes the `version: ""` placeholder into IR's
  `NominalOwner::Model`. IR's 81cd8036 `NominalOwner::Model` is
  `{identity, node}` (identity.rs:62), so the two sides agree.
- No ModelOwner version is left anywhere in QSL. `ModelOwner::new` has one
  production caller. `value::NodeOwner::Model(ModelSubject)`, the replay and
  identity-preimage owner, was already `{identity, node}` with
  `deny_unknown_fields` on main. `tests/it/text_enum_identity.rs` already uses
  the content-only shape. The other `version` hits in `qsl-semantics/src/check`,
  `qsl-package/src/emit*` and `qsl-replay` are lock or selection versions
  (`model_selections`, `dependency_selections`), which the plan ruling keeps.
- **Golden vectors, recomputed independently.** All 35 FR-094 preimages are
  canonical JSON, and SHA-256 over each one equals both its table key and its
  block key. Every digest a preimage names is either an FR-094 key or an
  FR-092/FR-093/QSpec key (T1 to T4, L1, L2, `unit-metre`, `unit-second`). No
  deleted or old key is left anywhere in the tree at d21fb558.
- **Deleted vectors.** Take every old vector, drop `version` from its model
  owner, and remap digests M2 to M1, R2 to R1, P9 to P7 and C3 to C1. The
  result equals the new vector of the same name, or of its twin, byte for byte.
  So M2, R2, P9 and C3 were exactly version-only duplicates, and every changed
  vector differs only by that change. U1 to U4, L4 and E10 are unchanged, as
  expected.
- **QSpec agreement.** M1, M3, M4 and M5 match QSpec origin/main c76c6aed
  `proposals/checked-package-v2/model-member-type-vectors.json`
  `model_declaration_nodes` byte for byte, in both key and JCS preimage.
  QSpec's own M2 is a different node (`acme/billing`). QSL no longer has an M2,
  so the two label sets do not clash. QSpec's `ModelOwner` schema
  (`node-identity-preimage.schema.json`) is closed over `{kind, identity, node}`,
  and FR-322-AC-28 says the same.
- **Deleting the count assertion is right.** `assert_eq!(vectors.len(), 39)` was
  a count pin. All 35 vectors are referenced by at least one test, and a
  vector the parser drops makes `vectors()[name]` panic. So nothing depended on
  the count.

**Test oracle.** The checks are strong. `admitted("2.0.0")` changes both the
selection version and its digest. The AC-1 half asserts R1, M1, M3 and R5 byte
for byte under 2.0.0, and the AC-5 half asserts R1, P7, C1 and C2 byte for byte
plus equality of every graph key. A versioned owner would fail either half,
and it would also fail the 1.0.0 vector asserts. The TC-442 addition hashes a
literal content-only preimage for `Gadget` and `Widget`. It compares the set of
emitted model-node digests, and IR's reader returns `Verified`, so the
emit-to-IR round trip is covered.

**Rust lane.** No new `unwrap`, `expect`, panic or `unsafe` on a production
path, and no wildcard arm. `map_err(|_| ... EmptyNode)` at model.rs:269 still
folds `EmptyIdentity` into `EmptyNode`. That is unreachable, because the
identity comes from an admitted selection and intake refuses an empty identity,
so it is not a finding. There is no new limit, depth cap, pin, compatibility
layer or ceremony.

**Gate (coder's logs, worktree at d21fb558, clean).** `make ci`: fmt and clippy
`-D warnings` pass, then `ci-default-features` stops on
`admission_corpus::every_emitted_node_family_is_admitted_at_its_package_id`
(CyclicEquality, from IR-486 #240, excluded by plan-lead ruling). `cargo test
--workspace --no-fail-fast` has exactly one `test result: FAILED`, that one. The
TC-417, TC-418 and TC-442 tests pass. `make ci` targets after
`ci-default-features` did not run (ci-all-features, ci-clean-build, ci-docs,
arch-lints). They are due on the post-repoint pre-merge gate. Tests were not
re-run for this review.

## Verdict

Approve the code, with one low finding. The diff does what the PR says, and no
ModelOwner version is left on any producer or consumer. Spec-wide agreement
findings are in SR-1255.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The doc comment on `two_admitted_versions_of_one_model_identity_refuse` gives a reason this PR makes false: "refuse as a broken invariant instead of keying owners from whichever comes first". The `ModelOwner` is now `{identity, node}`, so it is the same whichever admitted version comes first. The refusal still matters, because `model_owner` also returns the first matching `AdmittedModel`, and the declaration's record is read from it (model.rs:281-284, state.rs:403-404). Fix: reword it, for example "instead of resolving declarations against whichever admitted view comes first". | qsl-semantics/src/check/lowering/model/tests.rs:945-947 |

## Dispositions

Round 1, reviewed at 1512416d77383b82026b63e86ff8581ec957b665 (fix commit
1512416d7 on d21fb5586; `git diff d21fb5586 1512416d7`). The round adds no new
finding. The fix commit touches only the four text units SR-1254 and SR-1255
name, plus the two SR files under `reviews/`, which are byte-identical to the
review-pass files.

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | 1512416d7 |
