---
id: SR-7863
title: "QSL-723 PR 791 code review: inherited transition references (TC-235 M19)"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@b467c6780481a06fd4be31e32a0ad4850d20ab24; PR #791; qsl-semantics/src/model/intake.rs; qsl-semantics/src/model/normalize.rs; qsl-semantics/tests/it/tc235_state_machine.rs; qsl-semantics/Cargo.toml; Cargo.lock"
review_set: subset
---

## Summary

Ticket: QSL-723. Code review with the rust-review lane, over the PR diff only. The diff bumps the FCD pin to 72527ab5 (FCD #281 and #282, both on FCD main), widens the state-machine trigger check to ancestor operations, puts the inherited index in the derivation-conflict detail, adds an intake rule that refuses a transition `from`/`to` naming a state declared elsewhere as `malformed-declaration` on the machine, and rewrites the M19 fixtures. No build was run (reviewer-only). The spec text checked is QSpec FR-208 (`spec/objects/foundation/FR-208-quire-meaning-vocabulary.md`) and QSpec FR-154 (`spec/functional/type-model/FR-154-admit-domain-package-model.md`). QSL's own FR-208 and FR-154 are unrelated requirements.

Each M19 run against the row's literal text:

- Run 1: the fixture matches the row. `OrderLifecycle/Placed` is the identity of the inherited state, which is how FCD #281 resolves inherited references.
- Run 2: the fixture matches the row (states `[Expedited, Placed]`). Before this PR the run 2 and run 3 state lists were swapped.
- Run 3: the fixture matches the row. Each "then" is a variant of run 1. Applying run 2 cumulatively would make `Placed` local, so no transition conflict could arise, which contradicts the row. `Draft`, `Placed` and `place` are the inherited declarations. Nothing here needs a spec decision.
- Run 4: the input matches the row. The refusal is raised at intake, though, not in phase 3 (FND-001).
- Run 5: the input does not match the row (FND-001).

## Verdict

FAIL. The trigger widening and the inherited index are correct, and each is red-first: run 3 fails without either change. The pin bump is complete, with no stale rev anywhere in the repo. However, the intake rule classifies FR-208's phase-3 refusal at intake. That gives the literal M19 run 5 input a second refusal, and the run 5 fixture was changed to avoid it.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The new intake rule refuses a transition `from`/`to` that names a state of another machine at intake. FR-208 says that case is `malformed-declaration` "in phase 3", and FR-154 says "Intake reports every declaration refusal in node order, and then no later phase runs". The literal M19 run 5 input (run 1 with `supertypes: [Fulfilment]`, so `[0]` is still `Placed → Expedited`) should give one refusal: the intake refusal at `supertypes[0]`, with the phase-3 `from` refusal suppressed. This PR instead reports a second intake refusal at `transitions[0].from`. The run 5 fixture was changed to `Expedited → Expedited` to hide that. The row is consistent with the spec. The conflict is between FCD `decide`, which flags any `from`/`to` not visible to the type as `UNRESOLVED_CONSTRUCT_REF`, and FR-208, under which only a reference that names nothing is refused at intake. Fix in QSL: filter that diagnostic when the document declares the reference, as is already done for `UNRESOLVED_FRAME_PATH`, and have normalize report `malformed-declaration`. Then restore the literal run 5 input. If that does not land in this PR, keep run 5 literal under `#[ignore = "QSL-NNN"]` with a ticket. | qsl-semantics/src/model/intake.rs:1734-1764; qsl-semantics/tests/it/tc235_state_machine.rs:482-503 |
| FND-002 | medium | The reclassification covers `from`/`to` only. FR-208 says a `trigger` or `guard` that names an operation or clause of another machine, but no effective one of its own, is also `malformed-declaration` in phase 3. Here such a trigger stays `missing-name`, from the intake fallback or from normalize's `missing(...)` when the widened `has_operation` is false. A guard naming a non-ancestor clause is also `missing-name`. No test covers either case. | qsl-semantics/src/model/intake.rs:1737-1739; qsl-semantics/src/model/normalize.rs:1356-1402 |
| FND-003 | medium | The inherited index exists only in `detail` prose. M19 runs 2 and 3 are asserted with `detail.contains("OrderLifecycle.states[1]")` and similar checks. The typed `DerivationConflict { type_, member, redefiners }` is never asserted. The row's "naming `ix://test/orders/ExpressLifecycle`" (`type_`) and the inherited member key (`member` = `OrderLifecycle/Placed` or `OrderLifecycle/transition0`) are therefore unchecked. Assert the typed fields. If the row's member paths must be data, they need a typed path on the cause. | qsl-semantics/src/model/normalize.rs:1546-1557; qsl-semantics/tests/it/tc235_state_machine.rs:412-446 |
| FND-004 | medium | The derivation-conflict loop compares only against direct supertypes (`machine.supertypes` → `parent`). A state or transition that redeclares a grandparent's is not refused. FR-208 refuses one "declared by two distinct owners reaching the subtype". `has_state` and `has_operation` already walk the full `machine_chain`. | qsl-semantics/src/model/normalize.rs:1504-1597 |
| FND-005 | low | No FCD-validated document can now reach normalize's "ancestor transition cannot target subtype-only state" block, or its `from`/`to` `has_state` missing-name path, because intake refuses first. The block duplicates the new intake rule with a different detail. It should become the single phase-3 home (see FND-001) or be deleted. | qsl-semantics/src/model/normalize.rs:1598-1619 |
