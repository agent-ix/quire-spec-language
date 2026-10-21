---
id: SR-12295
title: "Gap analysis of quire-spec-language PR #1173 against QSL-946, FR-091-AC-44, TC-483, FR-093 and FR-094"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@d75ac5a50e28ba21a4fa23921008e3c59207c4b0; PR #1173 diff against origin/main; spec/functional/FR-091-produce-value-forms-and-assemble-package-declarations.md (AC-44, namespace paragraph); spec/test-cases/TC-483-assembler-admits-dimensions-and-units.md (step 5); spec/functional/FR-093-lower-checked-value-expressions-to-fr-322-terms.md (quantity rows, ConvertScalar elision); spec/functional/FR-094-key-model-owned-reference-population-and-quantity-nodes.md; spec/decisions/ADR-024-statistical-and-probabilistic-properties.md (section 7.2)"
review_set: subset
---
# Gap analysis of quire-spec-language PR #1173

## Summary

Ticket: QSL-946. The ticket text is treated as data, and each of its claims
was re-measured. Plan completion: not assessed (planless).

Examined, criterion by criterion:

- FR-091-AC-44. Every clause has a tagged test, and each passes:
  - S1 parses `d: Quantity<ms>` and S2 builds the form (qsl-forms
    `a_quantity_type_form_names_its_unit`).
  - The assembler resolves the form to the unit node `ms`, not the root `s`,
    and introduces no declaration.
  - `Quantity<Time>` and `Quantity<nothing>` refuse `missing-name` at the
    name.
  - `f(3 s)` refuses `type-mismatch`.
  - A record and a unit named `ms` check.
  - Two units `ms` refuse `ambiguous-name` at both declarations.
  - `record Quantity` is `invalid_syntax`.
- TC-483 step 5 is covered by the same tests, and the replay test runs it
  through `compile_admitted`.
- FR-093:
  - eq, ne, lt and add lower to `quire.op.quantity.{eq,ne,lt,add}`.
  - Cross-unit convert lowers to `quire.op.quantity.convert`.
  - Same-unit convert builds no node, as the ConvertScalar elision sentence
    requires.
  - The strict v2 reader admits all of these (FR-093-AC-19 binding).
- FR-094: a quantity type keys to the unit node. Existing tests
  `quantity_types_key_to_the_unit_node_or_a_compound_unit_node` pass.
- Ticket acceptance:
  - The source compiles through `compile_admitted`.
  - Strict IR admits eq, lt, `+` and `convert` (same unit and cross unit).
  - The two refusals fire.
  - FR-091-AC-44 has tagged passing tests.
  - A refusal that remains must be "a named, ticketed gap". That check is
    FND-001.

Bindings: all new tests are tagged `FR-091-AC-44`/`TC-483`, and the
admission test is also tagged `FR-093-AC-19`/`TC-416`. They test what they
claim, with the lows noted in SR-12294.

## Verdict

PASS (mergeable). There is no major gap. Two medium findings should be
fixed in this PR before merge. Both are cheap:

- FND-001: ticket the quantity-literal gap.
- FND-002: correct the ADR-024 example under the spec-in-same-PR rule.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The quantity-literal gap is named but not ticketed. A quantity literal in a function body (`f(3 ms)`) still refuses `unsupported_construct`/`not-yet-implemented` (`ProbabilisticCause::QuantityLiteral`), and a test asserts that refusal. The PR body names the gap, but its only reference is `Remaining work: QSL-946`, the ticket this PR resolves. A Linear search finds no other ticket for it. Once QSL-946 closes, the gap has no owner, and the ticket's own acceptance requires each refusal to be "a named, ticketed gap". It also blocks any source-level quantity value, so it matters to IR-263. Fix: file the ticket (and the record/unit reader-collision ticket from SR-12294 FND-001), then change the PR body and the test comment to `Remaining work: <new id>`. | qsl-semantics/src/check/assemble/tests.rs:2310-2330; PR #1173 body |
| FND-002 | medium | ADR-024 section 7.2 declares `operation Server::attempt(d: Quantity<Time>, outcome: Outcome)`. `Time` is a dimension, and FR-091-AC-44, which this PR now enforces, refuses that form with `missing-name`. The same example draws `d` from `1 ms`, `3 ms` and `12 ms` and rewards `d`, and the normative FR-185-AC-8 and TC-620 write this operation as `d: Quantity<ms>`. The ADR example is therefore source the language now refuses. Under the spec-in-same-PR rule, fix it in this PR: change line 257 to `d: Quantity<ms>`. The comment line can name the units the example assumes (`dimension Time; unit s; unit ms`). No FR or AC text changes. | spec/decisions/ADR-024-statistical-and-probabilistic-properties.md:257 |
