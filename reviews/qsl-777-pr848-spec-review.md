---
id: SR-9132
title: "Spec review (base) of quire-spec-language PR #848: repeated protocol references as occurrences of one node"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@5cfb21ab7ebb808d852f7675428e4923c394bf2c; spec/functional/FR-092-key-type-parameter-and-declared-nodes.md, spec/functional/FR-218-check-every-protocol-control-construct-at-s3.md, spec/test-cases/TC-1021-a-protocol-reference-resolves-by-typed-key-and-emits-one-carrier.md"
review_set: subset
---
# Spec review (base) of quire-spec-language PR #848

## Summary

Ticket: QSL-777. Base review of the amendment that admits a repeated
protocol reference as a further `expression` occurrence of one
`protocol_reference` node. Oracle: QSpec FR-322 and TC-931 as amended
upstream (cited by id only), and the merged QSL #844 code, which still
refuses a repeated reference until the IR change lands (target design, not a
contradiction).

Examined:
- FR-092 "Protocol reference nodes" occurrence and identity text,
  FR-092-AC-16, FR-092-AC-17 (examined)
- FR-218 repeated-reference bullet, FR-218-AC-8, FR-218-AC-9 (examined)
- TC-1021 description, steps 4, 6, 7 and their expected results (examined)
- qsl-package `emit/tests/protocol_reference.rs` and qsl-semantics
  `check/protocol_reference.rs`, `check/refusal.rs` (context_only)
- FR-308-AC-6, spec/tests.md TC-1021 row (context_only)

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | FR-092-AC-17 (and TC-1021 step 6) require the package identity of a unit that holds `protocol Fill` twice to equal the one-reference unit's. No source unit can meet that. A reference is an expression, so a second one needs an enclosing construct: `protocol Fill and protocol Fill` in the function body (as the #844 test writes it) or a second function. Either changes another node's body or adds a node, so the identity projection and `package_id` differ. QSpec FR-322-AC-66 and TC-931 V1/V2 state the equality at wire level: the same package with only an occurrence and a `source_map` entry added. The carrier node key, body and dependencies claim holds. The FR-092 statement ("changes neither the node key nor the package identity") and the FR-218 bullet repeat the same claim. Fix: keep the carrier key, body and dependencies equality. State identity exclusion at the right level, for example: the package identity equals that of the same graph with the second occurrence and its source-map row removed. Or: moving one reference (regions change, graph unchanged) leaves `package_id` unchanged. | FR-092-AC-17; FR-092:430-433; FR-218:146-153; TC-1021 step 6 |

## Verdict

**FAIL: 1 blocking finding (FND-001).**

Matches the oracle:
- One or more `expression` occurrences, one per source reference, each with
  its own source-map row keyed (node id, `expression`, ordinal), and no
  other role. This is FR-322 as amended.
- Ordinals 0 to n-1 in source order. This is a producer-side narrowing of
  FR-322, which requires only distinct ordinals. It is compatible.
- Occurrences enter neither the structural key nor the identity projection.
  This is FR-322 as amended.
- AC-16 ordinal 0 is consistent.

Step 4's replacement case matches #844 `ProtocolReferenceCause::UnsupportedRoot`
(`unsupported_construct`/`declaration-form`) and FR-218-AC-8's existing
clause.

Spec-code gate: `make ci` does not fail.
- The stale test `unresolved_non_protocol_and_repeated_references_refuse_by_cause`
  (`#[trace("FR-218-AC-8", "TC-1021")]`) still passes, because the code
  still refuses. It is expected to stay until the code PR (IR-785).
- `ix-trace-rs` checks id syntax only.
- No trace-matrix lane is in `ci`.
- `check-index-completeness` is unaffected, because no file was added.
- The only `include_str!` of these files is qsl-semantics
  `lowering/tests.rs` over FR-092, which reads golden vectors only. No
  parser reads FR-218 or TC-1021.
- The three changed files pass `quire validate` (quire 0.36.2).

Leak check: the diff cites QSpec by id only.

Recorded, not reported:
- low: TC-1021 step 4's expected result names only the refusal at the
  reference. #844 also refuses at the declaration (two refusals), per
  FR-092-AC-18. This is not a contradiction.
- low: test doc comments in `protocol_reference.rs` still say
  "TC-1021 step 6" for AC-17 (now step 7). The code PR will update them.
