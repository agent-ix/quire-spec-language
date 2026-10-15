---
id: SR-9136
title: "Gap analysis of quire-spec-language PR #850 against QSL-780 acceptance criteria"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@4ac5e4283db82aeedcc0c5343c2970997f415257; PR #850: qsl-replay/tests/it/compile_protocol_profile.rs, qsl-replay/src/spine/lifecycle.rs, qsl-semantics/src/check/lowering.rs; ticket QSL-780 AC-1..AC-4"
review_set: subset
---
# Gap analysis of quire-spec-language PR #850

## Summary

Ticket: QSL-780. Each of the ticket's four acceptance criteria was checked against the new public-path tests and the code. Plan completion: not assessed.

- AC-1 (comes from the authoritative catalog, no fallback, no injection): met. See SR-9135.
- AC-2 (positive source -> IR vector): met in substance. `profile_selections` holds a `protocol_profile` row only when a written node body names that law (`qsl-package/src/emit.rs:25-27`, `split_laws`). So the asserted lock row proves the `Fill` protocol node was emitted under the header's definition. The compile also succeeding proves the reference lowered without `missing-selection`. The test was red before the fix. Nothing asserts the carrier node's wire shape on this path. That shape, and IR v2 reader admission, are covered at emit level by TC-1021 (`qsl-package/src/emit/tests/protocol_reference.rs`, `a_reference_emits_the_target_node_and_one_carrier_the_reader_admits`). Recorded, medium.
- AC-3 (changed binding): only partly met. See FND-001.
- AC-4 (source without protocol use is unaffected): met by `a_unit_without_a_protocol_reference_selects_no_protocol_profile`, with and without the header profile.

The two changed-binding vectors also passed before the fix. They are still honest regression guards against a default or a fallback: if v1 were defaulted, the no-header case would compile, and if v1 were matched loosely, the v2 case would compile.

## Verdict

**FAIL: one blocking finding (FND-001).** The gate (`make ci`) is reported green on the head, and the log shows all three new tests passing. That does not cover the missing assertion.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | AC-3 says "changing the catalog's ProtocolProfile binding changes the emitted package/lock identity". Nothing asserts this. `a_changed_profile_binding_changes_the_selection_or_refuses` only changes the header spelling to a profile that is in no catalog (refuses `UnknownProfile`) or drops the header (refuses `MissingDeclaration`). Neither vector produces a package under a different binding to compare against, so no package_id, node key or lock delta is ever asserted. The coder's report also lists "package identity change on binding not asserted". The public catalog holds only one protocol profile, so the vector has to live inside the crate: in qsl-semantics, use the `pub(crate)` `with_protocol_profile` to lower the same protocol and reference under two different `DefinitionReference`s, then assert the protocol node key (and so the package identity) differs, and that the old binding's key does not match. | qsl-replay/tests/it/compile_protocol_profile.rs:76-95; qsl-semantics/src/check/profile.rs:27-32 |
