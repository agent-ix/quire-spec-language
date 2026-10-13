---
id: SR-7861
title: "Code review of quire-spec-language PR #793: relationship pointer resolution and the stale intake expectations"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@1e97537704f80527e67247e5f969d7e53fe90e01; PR #793: qsl-semantics/src/model/intake.rs (declared_node_for_pointer, validate_with_semantic_ir, missing_reader_reference), qsl-semantics/tests/it/model_intake.rs, qsl-semantics/tests/it/model_operations.rs"
review_set: subset
---
# Code review of quire-spec-language PR #793

## Summary

Ticket: QSL-59. Code review with the rust-review lane folded in.

The new `relationships` arm of `declared_node_for_pointer` is correct for
every indexed relationship pointer (`/types/N/relationships/M/...`). It
resolves to the relationship member's own identity and origin, which is what
FR-056-AC-5 (TC-145 step 5) and FR-260-AC-3 (TC-731) ask for. The
`reading_fcd_199s_golden_shape_reads_all_12_types` rewrite is right under
FR-056's native UUID rule, and its assertions are typed.

The resolver still maps field, operation and operation-parameter pointers to
the owning type. Until base commit 7fe6fca6a the node was the reader's own
`located.owner`, which was the member. The Decimal-parameter test now asserts
the misattributed locus.

## Verdict

Changes requested: one high finding on the resolver, one high finding on the
test that locks it in, and one low finding.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | `declared_node_for_pointer` resolves only `transitions`, `steps` and (new) `relationships` to a member. Every other member pointer falls to `_ => Some(declaration)`, so a reader diagnostic at `/types/1/operations/0/params/0` (or any `fields/M`, `operations/M`) is retained on the owning type, not on its IR node as FR-056 requires ("retain every reader diagnostic with its IR node, artifact id and span"). Before 7fe6fca6a the node was `located.owner`, the parameter. Fix: resolve to the deepest object on the pointer path that carries an `identity` (one generic walk replaces the three index arms), and add tests for a field and a parameter pointer. | qsl-semantics/src/model/intake.rs:1857-1922; qsl-semantics/src/model/intake.rs:1430-1444 |
| FND-002 | high | `a_decimal_typed_parameter_refuses_at_intake_not_assembly` now asserts the upstream `DECIMAL_POLICY_MISSING` refusal at `ConfigVersion`, which locks in FND-001. Its comment ("on the declaring type") contradicts the diagnostic's own pointer `/ir/types/1/operations/0/params/0`. It also uses `any` twice, so it does not pin the refusal count or the reader's diagnostic code. Expected per FR-056 plus FR-103-AC-3: exactly the reader's refusal and `unsupported_construct`/`declaration-form`, both at `attemptUpdate/note`. Fixing FND-001 then makes `read_domain_package`'s per-node dedup (`semantic_refused_nodes`) drop the declaration-form refusal. That dedup must key on more than the node, or the spec needs a ruling on which single refusal FR-103-AC-3 means. | qsl-semantics/tests/it/model_operations.rs:2279-2303 |
| FND-003 | low | The rename missed one reference. The doc comment at `model_intake.rs:678` still cites `reading_fcd_199s_golden_shape_admits_the_schema_and_refuses_3_of_its_12_types`, which no longer exists. `intake.rs:6925` likewise cites the removed `refuses_uuid_as_malformed_declaration_r5_holds_until_plat_836`. | qsl-semantics/tests/it/model_intake.rs:678; qsl-semantics/src/model/intake.rs:6925 |

## Dispositions

Round 1, reviewed at `0a634f55d0954004c174e94fbdf674f98818b575`.

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | 0a634f55d0954004c174e94fbdf674f98818b575 |
| FND-002 | fixed | 0a634f55d0954004c174e94fbdf674f98818b575 |
| FND-003 | fixed | 0a634f55d0954004c174e94fbdf674f98818b575 |

FND-001: `declared_node_for_pointer` walks the pointer to the deepest object
with a string `identity`. It stops at `constraints`, `origin`, `extensions` and
`roles`, and returns `None` for a bare collection. Checked against the pinned
semantic-IR schema (b3defd7): the identity-bearing objects under a type are
fields, operation params (field-shaped), operations, variants, relationships,
clauses, states, transitions and steps, which are all member declarations,
plus constraints and extensions, which are excluded. Inline pre/post clauses,
`returns`, relationship ends and frames carry no `identity`. The exclusions
are sufficient. The unit test
`a_member_pointer_resolves_to_the_member_not_the_owning_type` pins field,
operation, parameter, relationship, a string leaf (`supertypes/0`) and a bare
collection.

FND-002: the test asserts exactly two refusals, `invalid_model_binding` and
`unsupported_construct`/`declaration-form`, both at `attemptUpdate/note`.
`read_domain_package` keeps a reader `UnsupportedDeclarationForm` unless an IR
refusal of the same family is already at that node. That satisfies
FR-103-AC-3 and leaves FR-056-AC-5's `missing-name` path unchanged.

FND-003: both references were updated. `resolves_a_known_native_type_ref`
exists and calls `read_value_type_ref` directly.
