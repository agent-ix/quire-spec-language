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

The linked production sources in `qsl_analyze::Engines::qsl()` are EN-1
(`temporal-satisfaction`), EN-4 (`probabilistic-satisfaction` with statistical
evidence) and EN-5 (`probabilistic-satisfaction` with exact evidence) under
FR-281. None advertises either refinement-witness capability used by FR-345.
The `ValueValidity` witness has no production provider; QSL-1200
tracks its provider. The `OperationContract` witness has no production
provider; QSL-1201 tracks its provider. QSL-905 supplies the operation-contract
record needed by that path, but does not itself register or implement a
provider. Until QSL-1200 and QSL-1201 land, the production snapshot SHALL
return the empty candidate outcome for the corresponding witness capability,
and FR-345 SHALL report the empty candidate set according to its existing
requirement. In particular, FR-345-AC-9, FR-345-AC-12 and FR-345-AC-13 are
satisfied only after QSL-1200 supplies `ValueValidity` and QSL-1201 supplies
`OperationContract`; until then, `make refinement-layering` SHALL NOT be added
to `make ci`. QSL-518 remains blocked by QSL-1200 and QSL-1201. This
requirement does not change FR-345's empty-candidate outcome. The registry
builder SHALL NOT insert a placeholder to change that outcome.

| Witness capability | Registration owner/source | Current production status |
|---|---|---|
| `temporal-satisfaction` | EN-1 in `qsl_analyze::Engines::qsl()`; FR-281 owns the engine behavior and FR-288 owns conversion of its provider manifest into a descriptor. | Present; expose and register the existing EN-1 descriptor. |
| `probabilistic-satisfaction` | EN-4 and EN-5 in `qsl_analyze::Engines::qsl()`; FR-281 owns engine behavior and FR-288 owns conversion of each provider manifest into a descriptor. FR-075 owns evidence-specific candidate selection. | Present; EN-4 advertises statistical evidence and EN-5 advertises exact evidence as defined by FR-192 and FR-203. |
| `ValueValidity` | No current provider registration. QSL-1200 owns supplying one. | Absent until QSL-1200 lands. |
| `OperationContract` | No current provider registration. QSL-1201 owns supplying one; QSL-905 supplies a related operation-contract record, not a provider. | Absent until QSL-1201 lands. |

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
  refinement-layering witness candidate set nonempty. QSL-1200 and QSL-1201
  are dependencies for the respective real providers; their absence remains
  visible to FR-345.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-489-AC-1 | A production snapshot built from `qsl_analyze::Engines::qsl()` contains exactly the engine descriptors that collection exposes, including EN-1, EN-4 and EN-5, and no descriptor without its engine implementation. | Test (TC-1272) |
| FR-489-AC-2 | Candidate queries over the production snapshot include EN-1 for `temporal-satisfaction`, EN-4 for `probabilistic-satisfaction` with statistical evidence and EN-5 for that capability with exact evidence, as FR-075-AC-13 requires; where QSL has no production provider implementation for the FR-345 `ValueValidity` or `OperationContract` witness capability, the corresponding candidate outcome is empty. | Test (TC-1272) |
| FR-489-AC-3 | If the injected descriptor source fails or a production registration is refused, the factory returns a typed build error and the CLI reports tool failure; it never drops the descriptor and continues with a partial snapshot. | Test (TC-1272) |
| FR-489-AC-4 | Two independently built production snapshots are equal, and the production builder uses no process-global registry state. | Test (TC-1272) |
| FR-489-AC-5 | `Engines::registrations()` yields exactly the descriptors owned by the EN-1, EN-4 and EN-5 engines in `Engines::qsl()`; `xtask` declares `qsl-analyze` as a normal dependency and its production factory registers each yielded descriptor once. | Test (TC-1272) |
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
- FR-345-AC-9, FR-345-AC-12 and FR-345-AC-13 require the production provider
  coverage owned by QSL-1200 and QSL-1201. Until both tickets land, the target
  is not part of `make ci`, and QSL-518 remains blocked by those tickets.
- [QSL-1200: Production provider for ValueValidity witnesses](https://linear.app/agent-ix/issue/QSL-1200/production-provider-for-valuevalidity-witnesses)
  tracks the absent `ValueValidity` provider.
- [QSL-1201: Production provider for OperationContract witnesses](https://linear.app/agent-ix/issue/QSL-1201/production-provider-for-operationcontract-witnesses)
  tracks the absent `OperationContract` provider; [QSL-905](https://linear.app/agent-ix/issue/QSL-905)
  is its related operation-contract record producer.
