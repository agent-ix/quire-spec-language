---
id: SR-17011
title: "Code review of quire-spec-language PR #1448: nested-reference-row refusal and node-key digest population order"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@8d8060bdf43c3742e6f4c3aacb94fe3cfbd2bc1d; PR #1448 (QSL-985) diff origin/main...HEAD: qsl-replay/src/refinement/map.rs, qsl-semantics/src/check/{mod.rs,refinement.rs,refinement/protocol.rs}, qsl-semantics/tests/it/refinement_declarations.rs, qsl-replay/tests/it/{refinement_history,refinement_unmapped}.rs, qsl-eval/tests/it/refinement_populations.rs, tests/support/{ring_queue,ring_world}.rs"
review_set: subset
---
# Code review of quire-spec-language PR #1448

## Summary

Ticket: QSL-985. Code review with the rust-review lane folded in.

What changed:

- `UnsupportedFeature::NestedReferenceRow` (`nested-reference-row`). After a
  field row is typed and checked against the lifted abstract type, the row is
  dropped and refused `unsupported_construct`/`not-yet-implemented` at the span
  of its expression when `nests_reference` holds. `nests_reference` exempts a
  plain `Reference<T>` and a `PopulationObjects` row (the whole-population
  `Set<Reference<T>>`). For any other type it walks option, collection element
  and composite (record field, tuple position, union member position), with a
  `seen` guard for self-naming composites. It is not in
  `RefinementSupport::BUILD`. The payload carries the selected profile
  identity and no `subject`.
- `PopulationRow.abstract_node`. It is computed with `population_node`, the
  same `relation`/`population` content (`ModelOwner` from the declaration's
  package and node, no body) that lowering's `population_reference_node` and
  the abstraction relation's `population_target` key. It is therefore
  consistent with the node every population read lowers to. It does not depend
  on the still-partial QSL-548/563 minting.
- `map_state` sorts populations by `abstract_node` bytes, ascending, with the
  `DeclarationKey` as a tie-break. This matches FR-140's "ascending node-key
  digest order of the row's first named declaration, the abstract population".

Identity and compatibility: `PopulationRow` is an in-memory part of
`CheckedRefinement` and is not emitted, since S4 refinement emission is not
built. No golden fixture, `package_id` or node identity changes. The only
observable change is which `MappingFailure` row, object and cause `map_state`
reports when several populations fail. Refinement replay is unreleased, so no
stored verdict is affected. `PopulationRow` has pub fields, so the new field
breaks struct literals, but the only literal is in the checker.

Panic and string surface: no new `unwrap`, `expect` or indexing in production
code. `population_node` failure and a missing span become catalog faults.
string-edge is clean.

Checks run by the reviewer at the head, in their own target directory, one
cargo job at a time:

- `cargo test --locked -p qsl-semantics -p qsl-replay -p qsl-eval --features qsl-semantics/test-support`: all pass.
- `cargo clippy --locked -p qsl-semantics -p qsl-replay -p qsl-eval --features qsl-semantics/test-support --all-targets -- -D warnings`: clean.
- `cargo fmt --all --check`: clean.
- `cargo run --locked -p xtask -- string-edge`: clean.

Coder mutations re-run by the reviewer, each in its own worktree and target
directory:

- M1, the digest-order revert (`map_state` sorts by `abstract_population`): killed by `populations_are_read_in_node_key_digest_order_not_declaration_key_order`.
- M2, the Option arm of `holds_reference` removed: killed by `a_row_nesting_a_reference_other_than_a_whole_population_is_unsupported`.
- M3, the whole-population exemption removed: killed by the same test (`cells = R::slots` refused).
- M4, the collection element not walked: killed by the same test.

Reviewer probes:

- P1. Adding the row `chain = self.link` to the FR-135-AC-6 test refuses
  `ill_typed`/`type-mismatch` and not `nested-reference-row`. Its type is
  `Option<Reference<Ring>>` and the field is a `Sequence<Reference<Queue>>`.
  See FND-001.
- P2. Reverting the PR's change to the FR-140-AC-9 fixture still passes. See
  FND-005.

## Verdict

**CONDITIONAL.** The digest ordering is correct and consistent with how
lowering keys population nodes. Its test kills the revert. The
nested-reference refusal is correct for option and collection nesting, which
is all FR-135-AC-6 asserts, and the four coder mutations are killed.

FND-001 is medium. The refusal is decided after typing and conformance, so a
row nesting a reference in a record, tuple or union, and any ill-typed nested
row, reports `type-mismatch` instead of the feature refusal FR-135 requires.
The rest are low.

Mergeable once FND-001 is fixed or explicitly deferred to a ticket.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The `nested-reference-row` refusal is decided after `check_row` has typed the row and checked it against the lifted abstract type, and only on a row that passed. `lift` rewrites references through option and collection only, never through a composite, and a concrete record, tuple or union never conforms to an abstract one. So a row whose type nests a reference in a record, tuple or union is always refused `ill_typed`/`type-mismatch`, and the composite arms of `holds_reference` are unreachable for a well-typed row. An ill-typed nested row is refused the same way: the reviewer probe `chain = self.link` (`Option<Reference<Ring>>` into `Sequence<Reference<Queue>>`) gives `type-mismatch`. FR-135 says such a row "SHALL use the feature `nested-reference-row`" and that the refusal "is decided before the feature's own checks, so no other diagnostic about that row is reported". No test covers a record, tuple or union nesting. Fix: decide nesting from the row's type before the conformance check (or from the abstract field's declared type), refuse before `check_row`'s type-mismatch, and add a record- or union-nested row to the test. | qsl-semantics/src/check/refinement.rs:1019-1066; qsl-semantics/src/check/refinement.rs:1071-1086; qsl-semantics/src/check/refinement.rs:1885-1913 |
| FND-002 | low | The doc comment on `UnsupportedFeature::NestedReferenceRow` says "option, record, union or collection", and so does FR-135. `nests_reference` and the PR body also include tuple. Tuple nesting is a nested value under FR-135's rationale, but the code and the spec text disagree. Fix: add tuple to the doc comment and ask the FR-135 owner to name it, or state that a tuple is a record here. | qsl-semantics/src/check/refinement.rs:126-130; qsl-semantics/src/check/refinement.rs:1052-1055; spec/functional/FR-135-check-a-refinement-declaration-s-subjects-and-state-mapping.md:148-154 |
| FND-003 | low | `holds_reference` returns `false` when `types.composite(key)` has no declaration for a `ValueType::Composite`. That is a broken checker invariant, and the rest of the checker faults on one (`self.fault(..)`). Here the row is silently admitted as holding no reference. Fix: return a tri-state, or fault (`composite-type-has-a-declaration`) from the caller. | qsl-semantics/src/check/refinement.rs:1046-1048 |
| FND-004 | low | `population_node` returning `Err(NodeKeyRefusal)` (the identity budget exceeded while keying the abstract population) is reported as the internal fault `abstract-population-has-a-node-key`. Lowering reports a keying refusal for the same content as a limit refusal. Reachability is doubtful, since the abstract package's identity was admitted under the same limits. Fix: map `Err` to the same limit refusal lowering gives, and keep the fault for `Ok(None)`. | qsl-semantics/src/check/refinement.rs:1618-1624 |
| FND-005 | low | The PR changes the fixture of the existing FR-140-AC-9 test `an_undefined_row_outranks_an_unmapped_type_that_sorts_first`: ring `a` gets `next = b`, a ring the state does not hold. The PR body does not explain it. Reverting it at the head still passes (probe P2), so the change is unnecessary and adds a dangling reference to an AC fixture. Fix: revert it, or state why it is needed. | qsl-replay/tests/it/refinement_unmapped.rs:123 |
