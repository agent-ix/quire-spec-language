---
id: SR-20261109101138
title: "code-review and rust-review of QSL-1818 clock references (ref term, Reference<Clock>)"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language; git diff origin/main...qsl-1818-clock-refs (63a0a9d40..ff7916a3c): qsl-cst grammar/token, qsl-forms syntax/value, qsl-semantics check/typing/timed_model/type_form and tests"
review_set: subset
---

## Summary

Ticket: QSL-1818 (partial slice, Refs only). The branch adds the `ref` keyword and term (`ExprNode::Ref`), types `ref(r.f...)` against the model's clock places (`Frame::ClockRef`, `Typer::clock_ref`), resolves `Reference<Clock>` to `ValueType::Reference(clock_type_id())`, lists every clock place (not only guarded ones) with a `presence` flag, and keeps `ref(p)` operands out of clock-read detection. A `ref` of a real clock place is refused `unknown_required_feature`/`unsupported-feature` because no checked node carries a place id yet; a `ref` of a non-place refuses `ill_typed`/`type-mismatch`. The IR carrier, index postfix, `deref` and AC-45 are out of scope by agreement.

The interim refusal of a valid `ref` is sound and honest: `clock_ref` never returns `Ok`, so no `ref` term reaches lowering, and the timed-model change only drops `ref` operands from the clock-read set, which cannot widen acceptance while every `ref` is refused. The native key `OriginalDeclarationKey("quire", "ix://quire/native/Clock")` follows FR-230 ("object_type the effective id of the Clock type") and cannot collide with a package node key. The open defects are the `Reference<Clock>` type form, which is admitted everywhere without the same interim guard, and trace tags that mark unmet ACs as covered.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | `Reference<Clock>` now resolves in every type position (function parameters and results, record fields, aliases, state fields), but nothing downstream knows `clock_type_id()`: lowering's `reference_type` calls `object_node` and then `declaration_key_of`, which faults `InternalFault(UnknownEffectiveId)`. A function `f(r: Reference<Clock>): Boolean` passes typing and is refused as an internal checker fault rather than `unsupported-feature`, which breaks the honest interim the `ref` path uses | qsl-semantics/src/check/type_form.rs:466-470, qsl-semantics/src/check/lowering/model.rs:2504-2514, qsl-semantics/src/check/lowering/model.rs:2538-2543 |
| FND-002 | medium | Trace inflation: `a_ref_names_a_clock_place_of_its_owner` is tagged FR-230-AC-24 and AC-62 but asserts that `ref(self.x)` is refused, while AC-24 requires `ref(self.x) = ref(other.x)` to check as a data comparison. `a_reference_to_clock_names_the_native_clock_type` is tagged AC-44 but checks only type resolution, not storage, read-back, `=`/`<` or the reference value form. The Test Matrix will report AC-24, AC-44 and AC-62 as covered while the checker does not meet them | qsl-semantics/tests/it/clock_places.rs:2858, qsl-semantics/src/check/type_form.rs:1307-1309, qsl-forms/tests/it/model_source_forms.rs:316-318 |
| FND-003 | medium | The `ref` arm strips only `Field` nodes and types the base, and `clock_ref` matches the whole field path against places of the base's type. A place reached through a reference field, `ref(self.r.x)` with `r: Reference<Call>`, is matched as path `[r, x]` on `self`'s type, finds no place and is refused `type-mismatch` ("not a place") although it denotes the place `Call.x` of `self.r`. When the IR slice turns the interim refusal into an accept, this path stays wrongly refused | qsl-semantics/src/check/check/typing.rs:1417-1432, qsl-semantics/src/check/check.rs:4384-4400 |
| FND-004 | low | Only state clauses of a timed model get presence places. Every other typer (operation clauses, type invariants, terminal and state-graph claims) is built with `presence_places: &[]`, so `ref(self.x)` of a real clock place there is refused `type-mismatch` instead of `unsupported-feature` | qsl-semantics/src/check/operation_clause.rs:75, qsl-semantics/src/check/type_invariant.rs:88, qsl-semantics/src/check/terminal.rs:180, qsl-semantics/src/check/state_graph_claim.rs:491 |
| FND-005 | low | The typing test asserts only each refusal's cause, not its location, so a refusal at the field or the clause would also pass, although FR-230 requires both refusals "at the `ref`" | qsl-semantics/tests/it/clock_places.rs:2872-2887 |

## Verdict

Changes requested on FND-001 and FND-002; FND-003 should be fixed now or recorded on the IR ticket. Clean: the `ref` token, grammar alternative and forms mapping (arity checked like `deref`); `place_ref` builder; the exhaustive `ExprNode::Ref` arms in family encoding, hyper clause, layer and assemble; the owner-location fold in the `Ref` arm (one `child(0)` per stripped field); `named_by` refusing slot steps (FR-230-AC-62, no place id under a `Map` value); the `presence` flag, which gates every presence reader in `check.rs`; and moving `presence_places` to the top of `read_clause` (all readers are gated on `presence`, and the full `qsl-semantics` integration suite passes with `--all-features`, 1403 passed, 2 failed only for the unset `QSPEC_DIR`). Rust style: no new panics on input, closed-enum matches exhaustive, new items documented. quoin gap-analysis was not run (brief scoped to code-review plus rust-review); the AC-to-test check was done by hand for FR-230-AC-24/44/62 (FND-002).

## Scope and evidence

- FR-230 "Clock references" and "Place identity" (spec/functional/FR-230-check-time-declarations-clocks-and-clock-constraints.md:772-830): examined.
- FR-230-AC-24: examined (FND-002, FND-003).
- FR-230-AC-44: examined (FND-001, FND-002).
- FR-230-AC-62: examined; the `Map` refusal of `ref` holds through `named_by`.
