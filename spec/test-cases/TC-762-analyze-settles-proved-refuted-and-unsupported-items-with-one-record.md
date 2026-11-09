---
id: TC-762
title: "analyze settles proved, refuted and unsupported items with one record each"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-281
    type: verifies
---
# TC-762: analyze settles proved, refuted and unsupported items with one record each

## Description

Verify `analyze` over QSL's layer-A engines: a `certified` proof, a replayed counterexample, and an item no engine accepts.

Scope: FR-281-AC-1 to FR-281-AC-3, FR-281-AC-8 to FR-281-AC-10, FR-281-AC-13 and FR-281-AC-21.

Every item names a claim of a declaration family its kind applies to (FR-507) unless a step says otherwise: a `value-validity` item names a value claim and a `temporal-satisfaction` item a temporal clause's claim.

## Test Procedure

1. Over ADR-028 §15.4's `Link` model with `probability >= 0.95 [ eventually holds(m.delivered) ]` under every scheduler, run `analyze` with one EN-5 item built from the source bytes and the provision.
2. Over FR-564's Counter subject, run `analyze` with one EN-1 item for the clause `P`, built from the source bytes, the provision and the clause's occurrence key; replay the returned counterexample through `replay` over the same bytes.
3. Run `analyze` over `Engines::qsl()` with three items: one of a claim kind no layer-A engine advertises, settled `Unsupported` with `unsupported_projection`/`unsupported-requested-capability` naming that kind, then step 1's and step 2's items, each settled `Supported` on its engine.
4. Build `engines` with `Engines::with` holding test engines `qsl.test-a` and `qsl.test-b`, both advertising (`value-validity`, `bounded`), each counting its runs. Run one `value-validity` bounded item settled `Supported(qsl.test-b)`; settled `InvalidRequest` with `invalid_capability`/`ambiguous-backend` naming both; settled `Unsupported` with `unsupported_projection`/`unsupported-requested-capability`; settled `RequiresBound`; and settled `Supported(qsl.test-c)`.
5. Run step 3's request with a binding whose `universes` name a population the package does not declare.
6. Call `Engines::with` on step 4's `engines` with a second engine whose manifest's `BackendId` is `qsl.test-a`; then run an item settled `Supported(qsl.test-a)` on the set it was called on. Run a `temporal-satisfaction` item settled `Supported(qsl.test-a)`. Read step 3's first record's members.
7. On step 4's `engines`, with `qsl.test-a` advertising only (`value-validity`, `bounded`), run a `finite-replay` item on a state clause's claim handed `Supported(qsl.test-c)`, then `Supported(qsl.test-a)`, then `InvalidRequest` with `invalid_capability`/`ambiguous-backend`. Run a `temporal-satisfaction` item on a temporal clause's claim handed the same three.
8. Call each `ClaimItem` constructor with a clause's node under an occurrence that is not its `claim` occurrence, with a node the package does not hold as a clause, and with step 2's clause key.

Tag the tests `#[trace("TC-762", "<AC id>")]`.

## Expected Results

- Step 1: the item settles `proved`, category success, naming `en-5`, after the EN-5 checker accepts; the record carries the certificate.
- Step 2: the item settles `refuted`, naming `en-1`, and the outcome carries the counterexample as an `ItemCounterexample`; `replay` reproduces it.
- Step 3: three records return in `request_index` order 0, 1, 2; the first is settled by its `unsupported` disposition with the diagnostic naming the claim kind.
- Step 4: `qsl.test-b` alone runs and the record names it; then records settled by `invalid-request` (`ambiguous-backend`, both named), `unsupported` and `requires-bound` (no diagnostic) dispositions with no run; then `invalid-request` with `invalid_capability`/`unknown-backend` naming `qsl.test-c` and no run. Neither test engine runs except in the first case.
- Step 5: `StageFailure::Refused(AnalyzeRefusal::Binding)` holding `invalid_runtime_input`/`wrong-role-mapping`, no record, no engine run.
- Step 6: `invalid_capability`/`duplicate-backend` naming `qsl.test-a`; the first `qsl.test-a` engine runs the `Supported(qsl.test-a)` item. The `temporal-satisfaction` item settles `invalid-request` with `invalid_capability`/`inconsistent-candidates` naming `qsl.test-a`, `temporal-satisfaction` and `bounded`, and no engine runs. Step 3's first record carries the `unsupported-requested-capability` diagnostic naming the claim kind and no `cause` member.
- Step 7: each `finite-replay` record is `invalid-request` with `invalid_capability`/`inapplicable-kind`, payload `capability` `finite-replay` and `family` `state-model`, never `unknown-backend`, `inconsistent-candidates` or `ambiguous-backend`, and no engine runs. The `temporal-satisfaction` records are `unknown-backend` naming `qsl.test-c`, `inconsistent-candidates` naming `qsl.test-a`, and the handed `ambiguous-backend` disposition, with no run.
- Step 8: the first two refuse `UnknownClaim`; the third returns an item whose `claim` is that key.
