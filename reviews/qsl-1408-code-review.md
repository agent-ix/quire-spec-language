---
id: SR-1988001
title: "QSL-1408 PR 1988 code-review"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language; tests/it/formatter_identity.rs, tests/it/source_inventory.rs, tests/it/unbounded_declarations.rs"
review_set: subset
relationships:
  - {target: ix://agent-ix/quire-spec-language/FR-003, type: reviews}
  - {target: ix://agent-ix/quire-spec-language/FR-334, type: reviews}
---

## Summary

Reviewed frozen head 96932a4318ccee707d5e802573be49a7bf440194 against main bdbc95f3ac6e33edd3f8ac17bae2defa79cc9d92. Only tests/it/formatter_identity.rs, tests/it/source_inventory.rs and tests/it/unbounded_declarations.rs change (41 additions, 25 deletions). Ticket: QSL-1408. No production, dependency, CI, skip, budget or test-selection changes.

The sum fixture keeps 40 and 3,000 terms and the unchanged 24-declaration doubling chain. Authored sum literals come from the existing lifecycle fixture, with unique-prefix assertions; no independent grammar/codec copy is introduced. Both compile calls execute the existing real S1–S4 formatter helper. Inventory expected tuples now come directly from checked requirement keys; only actual typed inventory nodes pass through the existing structural projection. Role, ordinal, minimum occurrence count, pending obligation count and total partition assertions remain. Wrong/missing requirement rows or a changed node/role/ordinal still fail the equality. This checks the graph-to-inventory seam; it is not independent validation of the shared identity codec itself.

Each emitted type key is resolved with its own package WireGraph, so indexed operands have document context. Both resolved keys additionally equal the original checked parameter type keys. Existing semantic form, bound absence/presence, exact min/max, key inequality and package inequality assertions remain. A wrong emitted type/operand selection now fails exact checked-key equality. No mocks, guards, skips, smaller fixtures, raised budgets or suppressed warnings were added.

The existing tests/support/front_end.rs helper checks admission, second-pass bytes and retained source, but does not directly compare original/formatted package equality. This pre-existing FR-003-AC-9 limitation is documented in reviews/qsl-1338-gap-analysis.md and unchanged here; no full AC-9 acceptance claim is made. QSpec FR-322 is external indexed-wire context; local QSL FR-322 (union evaluation) is unrelated and is not a claimed binding.

Existing evidence, inspected without new builds: /home/peter/dev/.agents/tmp/qsl-1408-checks/96932a431/focused-tests.log records the three repaired tests PASS plus twelve neighboring passes, with one unchanged external-vector failure (15 pass, 1 fail, exit 101). The missing file is /home/peter/dev/quire-specification/proposals/checked-package-v2/collection-population-type-vectors.json in clean QSpec edccac4e1eacd9fa05222dc880cb4ec7bf85fedc, owned by QSL-1368 / STD-610. clippy.log records focused all-features Clippy exit 0 at this exact head; formatting/diff checks were reported passed by the coder. No full-suite green claim: the prior main run was 998 pass / 4 fail / 6 ignored, and the lead's aggregate gate remains pending.

## Verdict

PASS for the scoped QSL-1408 maintenance diff. No introduced findings, including low-severity findings. This is source-review clearance, with the external-vector failure and existing formatter oracle limitation stated above; it is not whole-repository acceptance or aggregate-gate clearance.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Coverage

Code review includes the Rust lane: repository AGENTS.md, CLAUDE.md, README.md, Rust testing/implementation/wire rules, panic paths in test assertions, shared projection callers, oracle independence and assertion preservation were inspected. No applicable AssuranceProfile exists. Spec review and its sub-analyses do not apply because no spec/plan changes exist.

## Examined units

```yaml
scope:
- id: FR-003-AC-9
  path: spec/functional/FR-003-format-native-source.md
  role: examined
  excerpt: For each complete-V1 fixture unit in the repository's tests that checks,
    checking its formatted source through the S1 to S4 spine yields a checked package
    identity equal to the original's, and formatting the formatted source again yields
    identical bytes.
- id: FR-334-AC-1
  path: spec/functional/FR-334-key-collection-types-by-the-root-definitions-identity-preimage.md
  role: examined
  excerpt: 'A unit whose only difference from another is `s: Set<Integer>` against
    `s: Set<Integer>[0, 18446744073709551615]` emits a different structural node key
    for `s`''s type and a structurally unequal package value; the unbounded node has
    no `collection_bounds` member and the bounded node has one with `min` 0 and `max`
    18446744073709551615.'
- id: TC-885
  path: spec/test-cases/TC-885-formatting-keeps-the-checked-package-identity.md
  role: examined
  excerpt: The second formatting returns identical bytes.
- id: TC-844
  path: spec/test-cases/TC-844-key-collection-types-by-the-root-definitions-identity-preimage.md
  role: examined
  excerpt: 'Verify that bound presence and value enter collection and population node

    identity and the emitted package value, and that structural identities and

    package values are deterministic.'
- id: QSL-767-source-inventory
  path: tests/it/source_inventory.rs
  role: examined
  excerpt: '//! QSL-767: the QSL owner report for Contract IR''s source-obligation
    input.

    //! Rows and the stop cause are asserted as typed fields only.'
- id: source-inventory-traversal
  path: src/checking/source_inventory.rs
  role: context_only
  excerpt: '/// QSL''s own traversal of the named scope: every checked requirement
    record of

    /// the graph, then every retained pending obligation of the type report.'
bindings:
- test_id: lifecycle_generated_bodies_keep_format_identity
  ac_id: FR-003-AC-9
  trace: correct
- test_id: a_collection_bound_adds_minimum_and_maximum_to_the_type_node
  ac_id: FR-334-AC-1
  trace: correct
```
