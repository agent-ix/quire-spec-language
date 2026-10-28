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
EN-1 (`temporal-satisfaction`) and EN-4 (`probabilistic-satisfaction`) under
FR-281. Neither engine advertises either refinement-witness capability used
by FR-345. The `ValueValidity` witness has no production provider; QSL-1200
tracks its provider. The `OperationContract` witness has no production
provider; QSL-1201 tracks its provider. QSL-905 supplies the operation-contract
record needed by that path, but does not itself register or implement a
provider. Until those provider tickets land, the production snapshot SHALL
return the empty candidate outcome for the corresponding witness capability,
and FR-345 SHALL report the empty candidate set according to its existing
requirement. The registry builder SHALL NOT insert a placeholder to change
that outcome.

## Inputs

- The linked engine collection returned by `qsl_analyze::Engines::qsl()`.
- Each engine's existing registered descriptor.
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
  `qsl_analyze::Engines::qsl()`. The production source SHALL NOT mint a
  `BackendDescriptor` from a capability label alone.
- `qsl_route::Registry` SHALL admit each descriptor and compute candidates
  under FR-075; this requirement adds no candidate-matching rule.
- The production factory SHALL return a typed error when an engine descriptor
  cannot be constructed.
- The production factory SHALL return a typed error when `Registry::register`
  refuses a descriptor.
- The CLI SHALL report tool failure when the production factory returns an
  error.
- The CLI SHALL NOT report the empty candidate set or FR-345's expected
  unsupported result when the production factory returns an error.
- A production registry SHALL contain no registration solely to make a
  refinement-layering witness candidate set nonempty. QSL-1200 and QSL-1201
  are dependencies for the respective real providers; their absence remains
  visible to FR-345.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-489-AC-1 | A production snapshot built from `qsl_analyze::Engines::qsl()` contains exactly the engine descriptors that collection exposes, including EN-1 and EN-4, and no descriptor without its engine implementation. | Test (TC-1272) |
| FR-489-AC-2 | Candidate queries over the production snapshot include EN-1 for `temporal-satisfaction` and EN-4 for `probabilistic-satisfaction`; where QSL has no production provider implementation for the FR-345 `ValueValidity` or `OperationContract` witness capability, the corresponding candidate outcome is empty. | Test (TC-1272) |
| FR-489-AC-3 | If an engine descriptor fails to construct or a production registration is refused, the factory returns a typed build error and the CLI reports tool failure; it never drops the descriptor and continues with a partial snapshot. | Test (TC-1272) |
| FR-489-AC-4 | Two independently built production snapshots are equal, and the production builder uses no process-global registry state. | Test (TC-1272) |

## Dependencies

- [FR-075](./FR-075-compute-candidates-from-registered-backends.md) owns
  registry admission and candidate computation.
- [FR-281](./FR-281-analyze-claims-with-in-process-engines.md) specifies the
  in-process engine collection and its current provider set.
- [FR-288](./FR-288-build-the-registry-from-provider-manifests.md) specifies
  the provider descriptor source used by each engine.
- [FR-345](./FR-345-run-the-profile-layering-refinement-gate.md) consumes the
  resulting snapshot for its layering witness requests.
- [QSL-1200: Production provider for ValueValidity witnesses](https://linear.app/agent-ix/issue/QSL-1200/production-provider-for-valuevalidity-witnesses)
  tracks the absent `ValueValidity` provider.
- [QSL-1201: Production provider for OperationContract witnesses](https://linear.app/agent-ix/issue/QSL-1201/production-provider-for-operationcontract-witnesses)
  tracks the absent `OperationContract` provider; [QSL-905](https://linear.app/agent-ix/issue/QSL-905)
  is its related operation-contract record producer.
