---
id: TC-1337
title: "analyze settles an external provider's verdict only after replaying its counterexample, and leaves bounded items unchanged"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-281
    type: verifies
---
# TC-1337: analyze settles an external provider's verdict only after replaying its counterexample, and leaves bounded items unchanged

## Description

Verify the external provider arm of `analyze` with a deterministic test provider that returns a fixed verdict. The test provider is a test double: it exercises QSL's arm and is not a cvc5 or Z3 run.

Scope: FR-281-AC-14 to FR-281-AC-19.

## Test Procedure

1. Check `function q using v(a: Integer, b: Integer): Integer pure { a div b }` and its guarded form `if b != 0 then a div b else 0` under `quire.value.complete/v1`. Build an engine from a test provider and a descriptor advertising (`value-validity`, `unbounded`), and register it beside the value-validity arm.
2. Run the unguarded item settled `Supported` on the test provider, which reports `refuted` with `a` = 2^100, `b` = 0. Run the guarded item, whose provider reports `proved`.
3. Run the unguarded item with the provider reporting `refuted` at `a` = 7, `b` = 2; at a one-element assignment; at a Boolean and an integer; then reporting `unknown`.
4. Run the unguarded item settled `Supported` on the value-validity arm, with the provider registered and with no provider registered, and count the provider's calls.
5. Build an engine from the test provider with a descriptor advertising only (`value-validity`, `bounded`), then with (`value-validity`, `unbounded`).
6. Run the AC-14 `proved` item under a descriptor of origin `Linked`, then `Process`.
7. Run the AC-14 `refuted` item under a descriptor publishing `max_cases` 1; with the run cancelled before `analyze`; and with a provider that cancels the run during its check.

Tag the tests `#[trace("TC-1337", "<AC id>")]`. A test double does not carry QSpec FR-147-AC-7, FR-147-AC-8 or TC-266.

## Expected Results

- Step 2: the first record is `refuted`, category violation, naming the test provider, carrying the counterexample 2^100 and 0 with the `division-by-zero` violation at the `div`; the second is `proved`, basis `inductive` at depth 0, `uncertified`, naming the test provider.
- Step 3: the first four items settle `inconclusive` with cause `replay_parity` and carry no counterexample; the last settles `inconclusive` with cause `solver-inconclusive`.
- Step 4: both runs return the same record, `unsupported` with `unsupported_projection`/`unbounded-extent` naming the value-validity arm, and the provider is called zero times.
- Step 5: the bounded-only descriptor is refused naming its `BackendId`, and the provider is never called; the unbounded descriptor yields an engine.
- Step 6: `uncertified` for `Linked`, `trusted` for `Process`, basis `inductive` at depth 0 in both.
- Step 7: `incomplete` with the `ValueCheckMaxCases` limit; `incomplete` `cancelled` with zero provider calls; `incomplete` `cancelled`, never `refuted`.
