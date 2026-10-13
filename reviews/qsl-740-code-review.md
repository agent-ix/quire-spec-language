---
id: SR-8703
title: "Code review of quire-spec-language PR #804: effective identity for variant types"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@2dff29e41112bae401b988287d5a4db06e9fd632; base ffaefa335 (impl/business-model-meanings); qsl-semantics/src/model/{index,normalize}.rs, qsl-semantics/src/check/{assemble,check,mod}.rs, qsl-semantics/tests/it/variant_type.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-056
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-081
    type: reviews
---
# Code review of quire-spec-language PR #804

## Summary

Ticket: QSL-740. Code review with the Rust lane (rust-review) folded in, over
the one commit of PR #804 on top of `impl/business-model-meanings` (the PR
base is current: merge-base equals the base tip). Variant types join
`normalizable_types`, a `Variant` record's owner makes the case a phase-2
member of its variant type, and the assembler reads each variant type and
its cases from the normalized view into `PackageDeclarations.variant_types`
and `Scope`, keyed by effective identity and never by name.

Gates were not run (reviewer-only brief: no builds). Behaviour was checked
by reading the code and the tests.

## Verdict

**PASS with two low findings.** Nothing blocks the merge.

Checked and clean:

- Effective identity follows FR-154 and FR-208:43 (QSpec FR-208 row
  `variant-type/v1`: "Normalization admits it as an effective type"). The type
  entry has owner null and derivation `[qualify [type key]]`. A variant type
  has no supertypes in the index, so it gets no `inherit` fact. Each case's
  `original` is the `VariantRecord.key`, which is the case node key
  `.../variant/<Name>-<value>`, never the `(owner, case)` export key. Reordering
  the `variants` list changes no key, so it changes no identity. `Gold` and
  `gold` are distinct because their node keys differ.
- No clause, field or operation can be owned by a variant type:
  `read_variant_type` reads no `clauses`, so the assembler's
  "every visible member is a `Variant`" check cannot fire on a well-formed
  package.
- The assembler names each type `M::<artifact id>` from `EnumRecord.name`, which
  intake sets from `type_identity_segment`. That is the same artifact id the
  composites use. Cases are sorted by `String` `Ord`, which is UTF-8 byte
  order. The list is sorted by effective identity.
- FR-150 is unchanged. Nothing is added to `TypeNames`/`Names`, and
  `AssemblyCause::UnsupportedVariantType` still fires before this code.
- The QSL-738 placeholder (`Field.typeRef` naming a variant type refuses
  `malformed-declaration`) in `validate_references` is unchanged and not
  implemented here.
- No regression found in record or event emission, or in the PR #803 areas.
  Every non-test consumer of `type_identities()` either looks up a known key,
  filters by record kind (`ModelPackage::object_types`), or already holds
  non-object types (persistence interfaces, record and event types).
  `dispatch.rs` skips variant types through `conforms`.
- No new unmarked string reader. The case-name sort in
  `VariantTypeDeclaration::new` is a canonical ordering. It is not a dispatch,
  and the same sort in `read_variant_type` is the precedent. The
  `xtask string-edge` scanner flags only literal or const operands.
- No `unwrap`/`panic`/`unsafe` in production code. Integer conversions are
  unchanged.
- No vendoring, no file or version tracking, nothing that must not be in a
  public repo.
- Test oracles are typed (`EffectiveId`, `DeclarationKey`, `RULE_QUALIFY`,
  `ModelRefusalCause`, `MemberPath`, span line). Reverting `index.rs` or
  `normalize.rs` fails TC-1003 step 1 (no effective entry, or a case owner
  of `None`). Reverting the assembler block fails TC-1004.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | `systems_member_owners` now also returns a variant case's owner. Its name and the phase-2 comment ("Systems end owners are resolved through the admitted declarations") still describe only systems records, so a reader looking for where variant cases become members will not find it. Rename the function to something like `record_member_owners`, or give variant cases their own member source next to `sorted_direct_fields`. | qsl-semantics/src/model/normalize.rs:2535; qsl-semantics/src/model/normalize.rs:2567; qsl-semantics/src/model/normalize.rs:2628 |
| FND-002 | low | No test reaches the `Scope` half of the type environment. The TC-1004 test reads `PackageDeclarations.variant_types` only. Deleting `.with_variant_types(self.variant_types)` from `into_scope` fails no test, and `Scope::variant_types`/`Scope::variant_type(key)` (the by-identity lookup FR-056 says QSL-738 will use) have no caller and no test. The list is also sorted twice, once in assemble.rs and again in `with_variant_types`. Assert `scope.variant_type(view_identity)` in the TC-1004 test. | qsl-semantics/src/check/mod.rs:743; qsl-semantics/src/check/check.rs:647-713; qsl-semantics/tests/it/variant_type.rs:943 |
