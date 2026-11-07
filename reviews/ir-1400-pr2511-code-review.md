---
id: SR-2511001
title: code-review review of QSL PR 2511
type: SpecReview
analysis: code-review
scope: agent-ix/quire-spec-language; IR-1400; Cargo.lock, qsl-eval/tests/it/collection_algebra.rs,
  qsl-eval/tests/it/equality_matrix.rs, qsl-replay/src/spine.rs, qsl-replay/src/spine/refinement.rs,
  qsl-semantics/Cargo.toml, qsl-semantics/src/check/check.rs, qsl-semantics/src/check/check/declared_type_tests.rs,
  qsl-semantics/src/check/mod.rs, qsl-semantics/src/check/refusal.rs, qsl-semantics/src/lib.rs,
  qsl-semantics/tests/it/variant_type.rs, spec/functional/FR-534-preserve-fallible-declared-type-check-failures.md,
  spec/test-cases/TC-1339-fallible-declared-type-check-consumption.md
review_set: subset
relationships:
- target: ix://agent-ix/quire-spec-language/FR-534
  type: references
---

## Summary

Review of IR-1400 consumer PR 2511 against its frozen source and specification. No applicable AssuranceProfile is declared in the repository. Rust and language-independent checks examined native StorageRequest ownership, allocation-free derived hashing, exact semantic forwarding, enum/variant continuation, exhaustive refinement handling, replay fields and actual one-shot owner denial. No production mirror types or compatibility shim were introduced. The dev-only allocator reuses the owner fixture, with thread-local state and no unsafe allowance in QSL. No CI gate changed. The nested competing-failure control has indistinguishable public causes; it establishes unchanged observable semantics. A stronger ordering oracle would require a distinguishable valid representation, which this unary owner walk does not expose. Existing semantic errors continue through the unchanged from_ill_typed mapper.

## Verdict

**PASS after disposition round 1** — FND-001 is fixed by inspected code. The lead’s final combined-tree gate remains required.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Nested StateGraph predicate, timed predicate and protocol key/slot checking refusals retain the storage code/cause but lose the native request diagnostic fields because replay only unwraps TypeInvariant/Terminal/Refinement. Recurse through the actual nested checking carriers when projecting storage fields. | qsl-replay/src/spine.rs:444; qsl-semantics/src/check/refusal.rs:3280 |

## Coverage

Rust and language-independent checks examined native StorageRequest ownership, allocation-free derived hashing, exact semantic forwarding, enum/variant continuation, exhaustive refinement handling, replay fields and actual one-shot owner denial. No production mirror types or compatibility shim were introduced. The dev-only allocator reuses the owner fixture, with thread-local state and no unsafe allowance in QSL. No CI gate changed. The nested competing-failure control has indistinguishable public causes; it establishes unchanged observable semantics. A stronger ordering oracle would require a distinguishable valid representation, which this unary owner walk does not expose. Existing semantic errors continue through the unchanged from_ill_typed mapper.

## Examined scope

```yaml
scope:
- id: FR-534-AC-1
  path: spec/functional/FR-534-preserve-fallible-declared-type-check-failures.md
  role: examined
  excerpt: With allocation enabled, valid declared scalar, nested collection, composite,
    reference and enum/variant fixtures keep their previous outcomes; missing declarations
    and IEEE-bearing set elements retain their existing semantic cause, original source
    location and first-refusal priority, including a fixture with more than one possible
    semantic failure. Evaluator collection/equality owner-API checks retain their
    original assertions.
- id: FR-534-AC-2
  path: spec/functional/FR-534-preserve-fallible-declared-type-check-failures.md
  role: examined
  excerpt: A real allocator denial during the owner's production type-check traversal
    reaches the QSL declared-type consumer as a located typed allocation refusal at
    Typing; the diagnostic is resource_exhausted/allocation-failed with exactly requested
    containing the owner's actual native unit and amount. No bound, setting, type-mismatch
    or stage-limit replacement appears.
- id: FR-534-AC-3
  path: spec/functional/FR-534-preserve-fallible-declared-type-check-failures.md
  role: examined
  excerpt: Allocation and capacity owner errors carrying both native units preserve
    class, amount and unit independently through the QSL mapping and diagnostic projection;
    capacity projects resource_exhausted/reservation-size-overflow with exactly upstream
    and no invented configured bound or allocator-denial claim.
- id: FR-534-AC-4
  path: spec/functional/FR-534-preserve-fallible-declared-type-check-failures.md
  role: examined
  excerpt: After a real owner-traversal allocation failure, the admitted environment
    and input type equal their pre-check values, no successful checked result escapes,
    and the identical check after restoring allocation agrees with a fresh successful
    check.
- id: FR-534 Description
  path: spec/functional/FR-534-preserve-fallible-declared-type-check-failures.md
  role: examined
  excerpt: 'When the expression checker validates a declared value type against its
    shared

    `TypeEnvironment`, the checker SHALL consume the owner''s fallible

    `try_check_type` result without changing semantic refusal behavior or losing a

    storage failure''s class, native request amount or native unit.'
- id: FR-534 Inputs
  path: spec/functional/FR-534-preserve-fallible-declared-type-check-failures.md
  role: examined
  excerpt: 'The admitted shared type environment, declared `ValueType` and existing
    source

    `Location` at `Typer::check_declared_type`.'
- id: FR-534 Outputs
  path: spec/functional/FR-534-preserve-fallible-declared-type-check-failures.md
  role: examined
  excerpt: 'Successful validation, the existing located semantic `CheckRefusal`, or
    a

    located typed storage refusal at the existing `CheckingStage::Typing` stage.

    No checked result is produced by a failed check.'
- id: FR-534 Behavior
  path: spec/functional/FR-534-preserve-fallible-declared-type-check-failures.md
  role: examined
  excerpt: 'When the owner returns `Ok(())`, the checker SHALL continue its existing

    QSL-owned enum and variant binding validation.

    When the owner returns `TypeCheckError::Refused(IllTyped)`, the checker SHALL

    apply the existing `CheckRefusal::from_ill_typed` mapping at the original

    location, preserving diagnostic code, cause, refusal priority and order.


    When the owner returns `TypeCheckError::Allocation(StorageRequest)`, the

    checker SHALL retain the allocator-denial class and exact native request in

    its typed refusal and diagnostic projection.

    When the owner returns `TypeCheckError::Capacity(StorageRequ'
- id: FR-534 Dependencies
  path: spec/functional/FR-534-preserve-fallible-declared-type-check-failures.md
  role: examined
  excerpt: '- [QSV FR-132](ix://agent-ix/quire-semantic-value/FR-132) owns fallible
    type traversal, reservation measurement and semantic ordering.

    - [TC-1339](../test-cases/TC-1339-fallible-declared-type-check-consumption.md)
    defines consumer verification.'
- id: TC-1339
  path: spec/test-cases/TC-1339-fallible-declared-type-check-consumption.md
  role: examined
  excerpt: 'Verify [FR-534](../functional/FR-534-preserve-fallible-declared-type-check-failures.md)

    through QSL''s actual declared-type consumer and diagnostic projection. The

    semantic controls run with allocation enabled and sufficient existing limits.

    The resource controls distinguish a real production allocator denial from

    small, explicit owner-error mapping controls.'
- id: Cargo.lock
  path: Cargo.lock
  role: examined
  excerpt: "# This file is automatically @generated by Cargo.\n# It is not intended\
    \ for manual editing.\nversion = 4\n\n[[package]]\nname = \"agent-ix-extraction-frontend\"\
    \nversion = \"0.2.1\"\nsource = \"git+https://github.com/agent-ix/filament-core-data?branch=main#df5e55796db57a62e5b5984c8ec60d70049cebe0\"\
    \ndependencies = [\n \"agent-ix-semantic-ir\",\n \"clap\",\n \"quire-rs\",\n \"\
    serde\",\n \"serde_json\",\n]\n\n[[package]]\nname = \"agent-ix-semantic-ir\"\n\
    version = \"0.1.1\"\nsource = \"git+https://github.com/agent-ix/filament-core-data?branch=main#df5e55796db57a62e5b5984c8ec60d70049cebe0\"\
    \n\n[[package]]\nname = \"agent-ix-semant"
- id: qsl-eval/tests/it/collection_algebra.rs
  path: qsl-eval/tests/it/collection_algebra.rs
  role: examined
  excerpt: "// SPDX-License-Identifier: AGPL-3.0-or-later\n//! TC-189 collection kind\
    \ algebra over the real `value` boundary (FR-144).\n\nuse ix_trace_rs::trace;\n\
    use quire_eval::FamilyOutcome;\nuse quire_exact::EffectiveId;\nuse quire_exact::FloatType;\n\
    use quire_exact::NodeKey;\nuse quire_exact::{\n    admit_text, BoundViolation,\
    \ IeeeWidth, IllTyped, IllTypedCause, Presence, TextPayload,\n    TextProfile,\
    \ TextType,\n};\nuse quire_exact::{\n    construct_collection, form_collection,\
    \ CollectionType, Deferred, FieldValue, OptionValue,\n    Value, ValueType,\n\
    };\nuse quire_exact::{\n    CardinalityBound, ChargePoi"
- id: qsl-eval/tests/it/equality_matrix.rs
  path: qsl-eval/tests/it/equality_matrix.rs
  role: examined
  excerpt: '// SPDX-License-Identifier: AGPL-3.0-or-later

    //! TC-194 complete equality matrix over the real `value` boundary (FR-149).

    //!

    //! Every equality is type-checked from the operands'' declared types by

    //! `TypeEnvironment::check_equality` and then evaluated over completed values.

    //! Unit and enum node keys come from an independent RFC 8785 canonicalizer;

    //! composite and object-type keys are opaque producer-assigned fixture keys.

    //! E20''s source-order row and E26''s `let`-bound rows run through the FR-146

    //! expression checker and evaluator, as do the scalar operator rows.


    use std:'
- id: qsl-replay/src/spine.rs
  path: qsl-replay/src/spine.rs
  role: examined
  excerpt: "// SPDX-License-Identifier: AGPL-3.0-or-later\n//! FR-027, FR-098 and\
    \ FR-278: the spine (ADR-011 \xA75, \xA77.3 M-6a; ADR-013\n//! O-26). Complete-V1\
    \ source enters S1 and goes through the stage APIs in\n//! DAG order: S1 (`qsl_cst::parse`),\
    \ S2 (`qsl_forms::build_unit`), I1\n//! (`model::intake::admit_unit`, the unit's\
    \ `model` declarations against\n//! the supplied domain packages, FR-056), the\
    \ S4 source resolution of the\n//! unit's `import`s against the dependency input\
    \ (ADR-015 D-1, FR-099), the\n//! FR-091 assembler, S3 (`PackageDeclarations::check`),\
    \ E4\n//! (`CheckedPackage::link_with`) and t"
- id: qsl-replay/src/spine/refinement.rs
  path: qsl-replay/src/spine/refinement.rs
  role: examined
  excerpt: '// SPDX-License-Identifier: AGPL-3.0-or-later

    //! FR-341 and FR-342: the refinement class of a spine compile result and of a

    //! clause-run disposition (ADR-017 RF-2), and [`CompileRefusal::causes`], the

    //! all-causes accessor FR-341 reads.

    //!

    //! Both classifications are one exhaustive `match` with no wildcard arm, so a

    //! new refusal or disposition variant does not compile until it is

    //! classified. They read typed codes, causes and limits, never message text

    //! (ADR-011 FB-02).


    use std::borrow::Borrow;


    use qsl_foundation::diagnostic::{LimitExceeded, StageFailure};

    use qsl_f'
- id: qsl-semantics/Cargo.toml
  path: qsl-semantics/Cargo.toml
  role: examined
  excerpt: "[package]\nname = \"qsl-semantics\"\nversion = \"0.1.0\"\npublish = false\n\
    edition = \"2021\"\nrust-version = \"1.98\"\nlicense = \"AGPL-3.0-or-later\"\n\
    description = \"ADR-011 \xA76.1 layer 3: semantic values, the model and library\
    \ layers, and the S3 check core with its family checkers.\"\n\n[features]\ndefault\
    \ = []\n# Test-only fixtures and views (`ReaderAuthority::fixture`,\n# `DeclarationKey::fixture`,\
    \ `DomainPackageRef::fixture{,_with_version}`,\n# `check::family::fixtures`, `model::refusal::fixtures`\
    \ and the items widened\n# to `pub` only for tests). `ReaderAuthority::fixture`\
    \ forges a crate-issued\n# capab"
- id: qsl-semantics/src/check/check.rs
  path: qsl-semantics/src/check/check.rs
  role: examined
  excerpt: '// SPDX-License-Identifier: AGPL-3.0-or-later

    //! Name resolution and typing of Complete-V1 value expressions and function

    //! declarations into the typed tree, before any definedness or termination

    //! judgment.

    //!

    //! Every refusal here is made before any charge. Typing is bounded by the

    //! declared [`CheckingLimits`] node ceiling, not by nesting depth (ADR-030

    //! D-1); reaching a declared limit is `stage_limit_exceeded`, never an

    //! admission verdict. The typer itself (`typing`) walks an expression over

    //! an explicit heap stack whose frames the node ceiling charges, so its h'
- id: qsl-semantics/src/check/check/declared_type_tests.rs
  path: qsl-semantics/src/check/check/declared_type_tests.rs
  role: examined
  excerpt: "// SPDX-License-Identifier: AGPL-3.0-or-later\n//! Private-consumer controls\
    \ use the owner's thread-local allocator fixture.\nuse super::*;\nuse ix_trace_rs::trace;\n\
    use quire_semantic_value::declaration::{StorageRequest, StorageUnit, TypeCheckError};\n\
    \nfn scope() -> Scope {\n    scope_with_types(TypeEnvironment::new([], []).unwrap())\n\
    }\n\nfn scope_with_types(types: TypeEnvironment) -> Scope {\n    Scope::new(\n\
    \        types,\n        OperationTable::default(),\n        vec![],\n       \
    \ vec![],\n        vec![],\n        None,\n        vec![],\n    )\n}\n\nfn key(name:\
    \ &str) -> NodeKey {\n    NodeKey::new"
- id: qsl-semantics/src/check/mod.rs
  path: qsl-semantics/src/check/mod.rs
  role: examined
  excerpt: "// SPDX-License-Identifier: AGPL-3.0-or-later\n//! ADR-011 \xA77.3 M-5\
    \ (FR-068): the layer-3 checking half of what was\n//! `value::expression`. This\
    \ module owns name resolution, typing, static\n//! definedness and termination\
    \ checking, and the checked-output types\n//! ([`CheckedGraph`], [`CheckedExpression`],\
    \ `CheckedFunction`) whose\n//! constructors are private here (ADR-011 \xA74's\
    \ private-constructor/\n//! public-accessor mechanism). The evaluation half --\
    \ `evaluate.rs`,\n//! `CheckedPackage::call`, `CheckedPackage::evaluate` and the\n\
    //! argument-admission logic -- stays at layer 5 in\n//! `"
- id: qsl-semantics/src/check/refusal.rs
  path: qsl-semantics/src/check/refusal.rs
  role: examined
  excerpt: '// SPDX-License-Identifier: AGPL-3.0-or-later

    //! Located checking refusals and their closed codes and causes.


    use super::protocol_memory::AccessClass;

    use qsl_forms::AccessOrdering;

    use qsl_foundation::diagnostic::Code;

    use qsl_foundation::source::provenance::SourceRegion;

    use qsl_foundation::MemoryModelName;

    use qsl_foundation::Setting;

    use qsl_foundation::Span;

    use quire_checked::node::EndSide;

    use quire_exact::{EffectiveId, NodeKey};

    use quire_exact::{IllTyped, IllTypedCause};

    use quire_exact::{Integer, TextProfile, ValueType};

    use quire_semantic_value::call::WrongSnapshotCause;'
- id: qsl-semantics/src/lib.rs
  path: qsl-semantics/src/lib.rs
  role: examined
  excerpt: "// SPDX-License-Identifier: AGPL-3.0-or-later\n//! ADR-011 \xA76.1 layer\
    \ 3: QSL's semantic core (ADR-011 \xA77.3 X-6).\n//!\n//! In \xA76.1's order,\
    \ `semantic_value < model < library < check core`:\n//!\n//! - [`value`]: the\
    \ \xA76.2 `semantic_value` modules (definitions, enumerations,\n//!   units, quantities,\
    \ declarations, containment, semantic nodes), plus\n//!   `value::model_query`\
    \ (\xA76.2 `model`);\n//! - [`model`]: domain packages, keys, normalization, dispatch,\
    \ population\n//!   and conformance, with `model::intake`, the only module that\
    \ names the\n//!   FCD crates;\n//! - [`library`]: library resolut"
- id: qsl-semantics/tests/it/variant_type.rs
  path: qsl-semantics/tests/it/variant_type.rs
  role: examined
  excerpt: '// SPDX-License-Identifier: AGPL-3.0-or-later

    //! FR-056 "Variant types" (QSL-734): a variant type is admitted as one `enum`

    //! export and one `variant` export per case (TC-994), admits supported

    //! payloads as union members (TC-995), refuses malformed and duplicate cases
    with their causes

    //! (TC-996), and admits a clause, field, parameter or result that names it

    //! while its own declaration stays unchanged (TC-997; FR-360 states how a

    //! member typed by it is assembled and checked).

    //!

    //! Every assertion compares typed code, cause variant and node identity.

    //! The typed memb'
bindings:
- test_id: declared_type_semantics_keep_owner_refusal_priority_and_location
  ac_id: FR-534-AC-1
  trace: correct
- test_id: enum_declaration_becomes_a_real_checked_type_node
  ac_id: FR-534-AC-1
  trace: correct
- test_id: a_unit_naming_a_variant_type_leaves_the_declaration_and_exports_unchanged
  ac_id: FR-534-AC-1
  trace: correct
- test_id: c06_reference_holders_are_keyed_and_ieee_elements_are_ineligible
  ac_id: FR-534-AC-1
  trace: correct
- test_id: e28_equality_on_ieee_bearing_types_is_operator_ineligible
  ac_id: FR-534-AC-1
  trace: correct
- test_id: owner_traversal_denial_retains_request_and_allows_identical_retry
  ac_id: FR-534-AC-2
  trace: correct
- test_id: owner_storage_classes_and_units_survive_conversion
  ac_id: FR-534-AC-3
  trace: correct
- test_id: declared_type_storage_requests_reach_compile_diagnostics
  ac_id: FR-534-AC-3
  trace: correct
- test_id: owner_traversal_denial_retains_request_and_allows_identical_retry
  ac_id: FR-534-AC-4
  trace: correct
```

## Validation evidence

Focused declared-type tests, replay native-field projection, enum binding and variant checking controls passed in the coder’s local logs. Scoped three-crate check and Clippy passed. The reviewer inspected these logs and the production call chain; no competing Cargo or aggregate gate was started. The lead retains the final combined-tree gate.

## Finding evidence

The frozen replay fields selector matches refusal.cause.unwrapped(); unwrapped handles TypeInvariant, Terminal and Refinement only. StateGraphCause::Predicate, TemporalCause::TimedPredicate and ProtocolAnchorCause::{KeyClause,SlotClause} carry a nested CheckCause and forward code/cause, but are left wrapped by that selector. Their allocation/capacity result therefore lacks requested/upstream. The coder independently identified the same gap during final caller inspection.

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | Storage-only iterative projection now follows all nested CheckCause carriers, preserves first Temporal Multiple cause and first refusal selection, and reaches the native request fields. Positive wrapper controls and negative semantic-first controls exercise the projection. |

### Disposition round 1 evidence

Inspected all changes after the initial frozen candidate, including StateModel, StateGraph, temporal Multiple and timed predicates, protocol key/slot clauses, type invariants, terminal and refinement wrappers. The helper loops without new allocation; only diagnostic rendering formats native unit/amount. Existing DispatchLink fallback is retained. No new finding. Focused wrapper execution is queued with the coder; final combined-tree verification remains the lead’s gate.

```rust
Self::StateGraph(cause) => match cause.as_ref() {
    StateGraphCause::Predicate { cause, .. } => cause,
    _ => return None,
},
```

## Fresh-main interaction review

Fresh-main interaction review: PASS, no new finding. Consumed the merged model-document unit-scope and carried-quantity equality changes. They establish the shared environment and resolve model units before declared-type checking; carried equality preserves actual source conversion metadata. The owner try_check_type call, semantic mapping, native StorageRequest carriers, storage-only nested projection, first-refusal selection and refinement classification remain intact. No source or Cargo command was changed or run by the reviewer.
