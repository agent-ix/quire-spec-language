---
id: SR-12294
title: "Code review of quire-spec-language PR #1173: Quantity<u> type form from S1 to the assembler (QSL-946)"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@d75ac5a50e28ba21a4fa23921008e3c59207c4b0; PR #1173 diff against origin/main: qsl-cst/src/grammar.rs; qsl-cst/src/token.rs; qsl-cst/tests/it/complete_grammar.rs; qsl-forms/src/syntax.rs; qsl-forms/src/value.rs; qsl-forms/tests/it/value_forms.rs; qsl-package/src/emit/tests/admission_corpus.rs; qsl-replay/tests/it/main.rs; qsl-replay/tests/it/quantity_type_source.rs; qsl-semantics/src/check/assemble.rs; qsl-semantics/src/check/assemble/tests.rs; qsl-semantics/src/check/check.rs; qsl-semantics/src/check/family.rs; qsl-semantics/src/check/lowering.rs; qsl-semantics/src/check/lowering/model.rs; qsl-semantics/src/check/type_form.rs"
review_set: subset
---
# Code review of quire-spec-language PR #1173

## Summary

Ticket: QSL-946. One code-review pass with the rust-review lane folded in,
over `git diff origin/main...HEAD` (16 files, +428/-14, one commit). Tests
were run, not just read: the `quantity`-filtered tests of qsl-cst, qsl-forms,
qsl-semantics, qsl-package and qsl-replay pass. The full `--all-features`
suites of qsl-cst, qsl-forms, qsl-semantics and qsl-package pass with 0
failures. `cargo clippy --all-features --all-targets -D warnings` over the
five touched crates is clean. Throwaway probes (since
deleted) measured spans, emission and strict-reader admission.

Checked and clean:

- S1/S2: `Quantity '<' qualified-name '>'` matches QSpec shared-grammar
  type-ref (line 321), and `Quantity` is in QSpec's complete-V1 reserved list
  (line 75). The S2 head maps to `BuiltinType::Quantity`, and `family.rs`
  names it.
- Reserved-word collateral: no QSL source, fixture, `.native` file, test
  string or doc example declares or names an identifier `Quantity`. The only
  QSpec hit is TC-187 U32, which expects `record Quantity` to be refused. The
  other `Quantity` hits are JSON type tags (`{"type": "Quantity"}`), not
  source identifiers.
- Assembler: `Quantity<u>` looks up only the unit namespace
  (`Names::units` / `Scope::unit_named`). The probe and the tests show that
  `Quantity<Time>` and `Quantity<nothing>` each give exactly one
  `missing_declaration`/`missing-name` at the name. `check_names` and
  `resolve_builtin` do not raise the same error twice.
- AC-44 refusals, measured at the span. `f(3 kmx)` against `Quantity<m>`
  refuses `ill_typed`/`type-mismatch`, and the region is exactly the argument
  `3 kmx` (bytes 271..276). Two units `m` refuse `DuplicateQuantityName` with
  both declaration spans.
- Strict IR: eq, ne, lt, add and cross-unit convert over `Quantity<m>` emit,
  and the checked-package/v2 strict reader admits them at their
  `package_id`. A same-unit `convert<Quantity<m>>(q)` writes no node: the
  probe shows no Expression family, and the body references the parameter.
  This is what FR-093 requires ("a `ConvertScalar` whose operand type equals
  its target builds no node; its parent references the operand's node"). The
  convert case does not vanish silently, because the m to km row still
  asserts `quire.op.quantity.convert`.
- Declaration-position namespaces: `unit <n>` and `dimension <n>` follow the
  existing spaced `Origin::TypeDeclaration` convention (`bound`,
  `abstraction`, `probabilistic`). No declared type name contains a space,
  so no record, enum or alias key can collide with them, and the prefixes
  are distinct. The probe shows that a dimension and a unit that share a
  name emit and read cleanly.
- Rust lane: there is no `unsafe`. Production code adds no `unwrap`,
  `expect`, index or integer conversion. `unit_ids_by_name` is extracted and
  reused, not duplicated. `panic!` appears only in tests.

## Verdict

PASS (mergeable). There is no major finding. One medium finding: a record
and a unit that share a name check, but the strict reader refuses the
emitted package. The finding needs a ticket, and the AC-44 claim should not
be read as end to end. Two lows are assertion gaps in tests whose behaviour
was measured correct.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | A unit `m` and a record `m` check (AC-44), and `compile_admitted` returns Ok, but the emitted package is refused by the strict checked-package/v2 reader: `AmbiguousDeclaration`/`AmbiguousName` at `/semantic_graph/nodes/3/declaration/qualified_name`. The reader (quire-contract-model `checked_package/v2/mod.rs`, the holders map keyed on `declaration.qualified_name`) treats every node's declaration name as one namespace. Moving the position into `unit <n>` fixed the location, but not the declaration identity. Before this PR this source failed with `UnlocatedOccurrence`, so the case is newly reachable and does not regress anything. The owner is cross-repo: either QSL lowering namespaces the unit node's declaration name, or the IR reader or QSpec FR-322 admits per-kind namespaces. File a ticket and cite it as Remaining work. The AC-44 test checks only `check()`, so it does not detect this. | qsl-semantics/src/check/lowering/model.rs:1229; qsl-semantics/src/check/assemble/tests.rs (a_unit_and_a_record_share_a_name_and_two_units_do_not) |
| FND-002 | low | In `QUANTITY_OPERATIONS`, the same-unit convert row carries `None`, so `is_none_or` asserts nothing about the body. The test would still pass if a `quire.op.quantity.convert` or other node were written. The probe found no node, so the behaviour is correct but unasserted. Assert that no application of `quire.op.quantity.convert` exists, or that the body binding references the parameter node (FR-093). | qsl-package/src/emit/tests/admission_corpus.rs:811-816,836 |
| FND-003 | low | `a_quantity_argument_of_another_unit_is_a_type_mismatch` asserts the code but not AC-44's "at the argument". The duplicate-unit check asserts `spans.len() == 2` but not that the spans are the two declarations. Both were measured correct (region 271..276 = `3 kmx`, spans at both `m` names), but the tests do not hold them. | qsl-semantics/src/check/assemble/tests.rs:2293-2357 |
