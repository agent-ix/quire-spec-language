---
id: TC-761
title: "QSL engine manifests register, and the provider entry returns analyze's records"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-280
    type: verifies
---
# TC-761: QSL engine manifests register, and the provider entry returns analyze's records

## Description

Verify what QSL supplies to the driver's `prove`: each engine's FR-331 manifest and the provider entry.

Scope: FR-280-AC-1, FR-280-AC-2, FR-280-AC-4, FR-280-AC-5, FR-280-AC-6.

## Test Procedure

1. Read each QSL engine's provider manifest with the FR-331 reader and convert it with FR-288's conversion.
2. For each TC-762 item, build a `RoutedItem` routing it to its engine and pass it, with the `CheckedPackage`, `SubjectBinding` and `AnalyzeLimits` of the same run, to `qsl_analyze::provider::run`; call `analyze` over `Engines::qsl()` with the same item settled `Supported` on the same backend.
3. Convert EN-4's manifest, admit the descriptor, register it with a descriptor advertising the same pair with evidence `{exact}`, and compute the candidates of a `statistical` and of an `exact` probabilistic item.
4. Check `Service` (FR-185-AC-7) and confirm `emit_checked` refuses it. Pass its `CheckedPackage` to `qsl_analyze::provider::run` with a `RoutedItem` routing the `2 ms` claim of ADR-024 §7.2 over `Service` to EN-4 (`statistical`, Okamoto, seed 7, `max_witnesses` 3); call `analyze` with the same item, checked package, binding and limits; replay each returned witness with `replay_sampled_witness` over `Service`'s checked subject.
5. Pass step 2's routed items in reverse order to the entry; then pass them with a binding whose `universes` name an undeclared population, and with a binding whose `initial` digest is absent from `snapshots`.

Tag the tests `#[trace("TC-761", "<AC id>")]`.

## Expected Results

- Step 1: each converts to a `BackendDescriptor` whose (kind, mode) pairs equal the claim kinds the engine settles in TC-762.
- Step 2: each pair of terminal records is equal.
- Step 3: the descriptor advertises exactly (`probabilistic-satisfaction`, `bounded`) with `{statistical}` and the `domains` `{population, integer}`, and is admitted; the `statistical` item's candidates are the EN-4 identity alone and the `exact` item's the other identity alone.
- Step 4: `emit_checked` refuses `unsupported_construct`/`not-yet-implemented`; the entry returns one `measured`, `rejected` record equal to `analyze`'s, with the same three witnesses, and each witness replays `Reproduced`. The run holds no `EmittedPackage` of `Service`, and `emit_checked` over `Service` is refused after the entry returns as it was before.
- Step 5: the records return in the reversed order, each equal to step 2's; then one `declined` record per item, cause `invalid-input`, code `invalid_runtime_input`/`wrong-role-mapping`; then one `declined` record per item, cause `incomplete-input`, code `unavailable_observation`/`missing-required-artifact`; no engine runs in either.
