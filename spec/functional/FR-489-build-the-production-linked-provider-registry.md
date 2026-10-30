---
id: FR-489
title: "Build the production linked-provider registry for refinement witnesses"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-075
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-281
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-345
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/FR-288
    type: depends_on
---
# FR-489: Build the production linked-provider registry for refinement witnesses

## Description

QSL SHALL build one production candidate registry from the real linked provider
engines exposed by `qsl_analyze::Engines::qsl()`. The `qsl-route::Registry`
remains the owner of registration admission and candidate computation under
FR-075. The engine collection supplies registrations from each engine's own
FR-288 descriptor; a descriptor without its corresponding implementation is
not a production provider registration.

For each FR-345 CLI invocation, the layering command SHALL obtain provider
registrations from `qsl_analyze::Engines::qsl()`, register each descriptor
once in a fresh `qsl_route::Registry`, and freeze the result as the invocation's
snapshot. The command SHALL NOT use a default empty snapshot, test-only
descriptors, or a process-global registry.

The current linked production sources in `qsl_analyze::Engines::qsl()` are
EN-1 (`temporal-satisfaction`), EN-4 (`probabilistic-satisfaction`) and EN-6
(`temporal-satisfaction` over a dense-time subject, FR-243) under FR-281, and
the two refinement-witness providers FR-345 uses. The `ValueValidity` witness
is supplied by the value-validity check arm (`value-checker`, QSpec FR-483 and
FR-484). The `OperationContract` witness is supplied by the operation-contract
check arm (`contract-checker`, QSpec FR-483 and FR-485), which advertises
`operation-contract` in `bounded` mode. QSL-905 supplies the operation-contract
record the arm settles; it does not itself register or implement a provider.
The production snapshot SHALL return each arm's descriptor as the candidate
outcome for its witness capability. The registry builder SHALL NOT insert a
placeholder for either capability.

| Witness capability | Registration owner/source | Current production status |
|---|---|---|
| `temporal-satisfaction` | EN-1 and EN-6 in `qsl_analyze::Engines::qsl()`; FR-281 owns the engine behavior and FR-288 owns conversion of each provider manifest into a descriptor. | Present; expose and register the existing EN-1 and EN-6 descriptors. |
| `probabilistic-satisfaction` | EN-4 in `qsl_analyze::Engines::qsl()`; FR-281 owns the engine behavior and FR-288 owns conversion of its provider manifest into a descriptor. | Present; expose and register the existing EN-4 descriptor. |
| `ValueValidity` | The value-validity arm in `qsl_analyze::Engines::qsl()`; QSpec FR-483 and FR-484 own its behavior. | Present; expose and register its descriptor. |
| `OperationContract` | The operation-contract arm in `qsl_analyze::Engines::qsl()`; QSpec FR-483 and FR-485 own its behavior. QSL-905 supplies the operation-contract record it settles, not a provider. | Present; expose and register its descriptor. |

The current `qsl_analyze::Engines::qsl()` API does not expose its engine
descriptors, and `xtask` does not yet depend on `qsl-analyze`. QSL SHALL add a
public `Engines::registrations()` accessor that yields the descriptors owned
by the contained engines without constructing replacements. `xtask` SHALL add
`qsl-analyze` as a normal dependency and build each production snapshot by
iterating that accessor, registering each yielded descriptor exactly once in
a fresh `qsl_route::Registry`, and freezing the registry. These are required
API and dependency changes, not claims about existing code.

## Inputs

- The linked engine collection returned by `qsl_analyze::Engines::qsl()`.
- The descriptors exposed by the required public
  `qsl_analyze::Engines::registrations()` accessor.
- The `xtask` normal dependency on `qsl-analyze`.
- A fresh `qsl_route::Registry` for the current CLI invocation.

## Outputs

- A `RegistrySnapshot` containing every admitted linked provider descriptor
  supplied by the engine collection.
- A typed registry-build error when construction or registration fails; a
  failed provider registration is not silently omitted.

## Behavior

- For each CLI invocation, `xtask` SHALL build a fresh registry snapshot. A
  later invocation SHALL read no registry state from an earlier invocation.
- The production source SHALL include exactly the registrations exposed by
  `qsl_analyze::Engines::registrations()` on
  `qsl_analyze::Engines::qsl()`. The production source SHALL NOT mint a
  `BackendDescriptor` from a capability label alone.
- `qsl_route::Registry` SHALL admit each descriptor and compute candidates
  under FR-075; this requirement adds no candidate-matching rule.
- A test-only factory constructor SHALL accept an injectable registration
  source and registry-registration operation so tests can cause, separately,
  a descriptor-source error and a refused registration without changing the
  production engine collection or registry.
- The production factory SHALL return a typed error when the descriptor
  source fails or `Registry::register` refuses a descriptor.
- The CLI SHALL report FR-344 `tool failure`, exit 30, when the production
  factory returns an error.
- The CLI SHALL NOT report the empty candidate set or FR-345's expected
  unsupported result when the production factory returns an error.
- A production registry SHALL contain no registration solely to make a
  refinement-layering witness candidate set nonempty. Each witness
  capability's candidate is its real provider: `value-checker` for
  `ValueValidity` and `contract-checker` for `OperationContract`.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-489-AC-1 | A production snapshot built from `qsl_analyze::Engines::qsl()` contains exactly the engine descriptors that collection exposes, including EN-1, EN-4, EN-6, `value-checker` and `contract-checker`, and no descriptor without its engine implementation. | Test (TC-1272) |
| FR-489-AC-2 | Candidate queries over the production snapshot include EN-1 and EN-6 for `temporal-satisfaction` and EN-4 for `probabilistic-satisfaction` when the query supplies the evidence kind EN-4 advertises, as FR-075-AC-13 requires; the FR-345 `ValueValidity` witness capability gives exactly `value-checker` and the `OperationContract` witness capability gives exactly `contract-checker`. | Test (TC-1272) |
| FR-489-AC-3 | If the injected descriptor source fails or a production registration is refused, the factory returns a typed build error and the CLI reports tool failure; it never drops the descriptor and continues with a partial snapshot. | Test (TC-1272) |
| FR-489-AC-4 | Two independently built production snapshots are equal, and the production builder uses no process-global registry state. | Test (TC-1272) |
| FR-489-AC-5 | `Engines::registrations()` yields exactly the descriptors owned by the EN-1, EN-4, EN-6, `value-checker` and `contract-checker` engines in `Engines::qsl()`; `xtask` declares `qsl-analyze` as a normal dependency and its production factory registers each yielded descriptor once. | Test (TC-1272) |
| FR-489-AC-6 | Test-only injected descriptor-source failure and registration refusal each return their corresponding typed build error; the CLI maps either to FR-344 tool failure, exit 30, without returning a partial snapshot or the FR-345 expected unsupported result. | Test (TC-1272) |

## Dependencies

- [FR-075](./FR-075-compute-candidates-from-registered-backends.md) owns
  registry admission and candidate computation.
- [FR-281](./FR-281-analyze-claims-with-in-process-engines.md) specifies the
  in-process engine collection and its current provider set.
- [FR-288](./FR-288-build-the-registry-from-provider-manifests.md) specifies
  the provider descriptor source used by each engine.
- [FR-345](./FR-345-run-the-profile-layering-refinement-gate.md) consumes the
  resulting snapshot for its layering witness requests.
- FR-345-AC-9, FR-345-AC-12 and FR-345-AC-13 read the witness candidates
  this snapshot gives.
- QSpec FR-483, FR-484 and FR-485 specify the value-validity and
  operation-contract check arms.
