---
id: SR-16308
title: "Code review of quire-spec-language PR #1391: hybrid forms and checked hybrid model (QSL-1017 PR 1 of 4)"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@f6e8991c2f77c5e63556f0ff568de37ff9270d95; PR #1391 diff against origin/main: qsl-cst/src/{cst,grammar}.rs; qsl-cst/tests/it/{complete_cst,complete_grammar}.rs; qsl-forms/src/{dispatch,hybrid_declaration,lib,syntax,value}.rs; qsl-forms/tests/it/{hybrid_forms,main,protocol_clause_forms,terminal_declaration_forms,value_forms}.rs; qsl-replay/src/spine/refinement.rs; qsl-semantics/src/check/{assemble,check,mod,refusal,region,hybrid}.rs; qsl-semantics/src/check/hybrid/{cause,model,term,tests}.rs; tests/it/complete_cst.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-447
    type: reviews
---
# Code review of quire-spec-language PR #1391

## Summary

Ticket: QSL-1017 (PR 1 of a 4-PR stack; base of #1392, #1393, #1394). One
code-review pass with the rust-review lane folded in, over
`git diff origin/main...HEAD` (26 files, +2458/-17). Reviewed against FR-447
"S2 forms", "Modes and transitions", "Initial states", "Continuous variables
and flows", "The clock line" and "Invariants, guards, resets and expression
forms", and FR-447-AC-1, AC-2, AC-6 and AC-8.

Run here: `cargo test -p qsl-forms -p qsl-semantics --features
qsl-semantics/test-support hybrid` (13 tests, all pass) and `cargo fmt
--check` on the touched crates (clean). Clippy and the full gate were not
re-run (the lead's gate).

Checked and clean:

- S1: `hybrid-initial` is `opt` as the first member of the body, `initial`
  a `literal` spelling only there; an `initial` clause after a mode is not
  admitted, and `initial` stays an identifier as a variable name.
- S2: `LeadingTokenKind::Hybrid` dispatches `hybrid` only; the mode builder
  pairs guard and reset blocks by count and refuses a mismatch rather than
  indexing; no `unwrap`, slicing or indexing on a production path.
- S3: duplicate mode, missing mode (transition and initial entry), qualified
  equation name, duplicate and missing equation, rate typing, the clock line
  on the literal `1` as written (no folding), the all-clock refusal,
  Boolean invariant/guard/initial condition, `pre(x)` only in a reset, the
  reset conjunction walk under `let` and the `=`-plus-second-conjunct rule
  (both orders) match FR-447. A declaration with any stop produces no model:
  `check_models` keeps a model only while the refusal list is empty.
- Term lowering is iterative (explicit work stacks), charges every node and
  each `let` substitution against the package node ceiling, and returns
  `ResourceExhausted` at it; the package check returns at that refusal.
- The not-emitted gate refuses every hybrid declaration at its name until
  emission lands; tests bypass it only through the test-support builder.
- The public-API list in the PR body matches the diff: the breaking enum
  variants (`DeclarationForm::Hybrid`, `LeadingTokenKind::Hybrid`,
  `Production::{HybridInitial, InitialEntry}`, `CheckCause::Hybrid`) and the
  two new `PackageDeclarations` fields. The repo keeps no CHANGELOG file.
- No vendoring, duplication, re-export-only module or ceremony.

## Verdict

**CONDITIONAL**: two low findings only; correct against the scoped ACs, no
panic surface on malformed input. Mergeable.

## Findings

| ID      | Severity | Summary | Refs |
| ------- | -------- | ------- | ---- |
| FND-001 | low | `lower_at` clones the whole `outer` binder list (`outer.to_vec()`, each binder a full `HybridTerm`) on every call, uncharged. `reset()` calls it once per conjunct and per `let` value, so `let k = <large term> in v = k and a = k and ...` copies the large term once per conjunct outside the node meter (time and transient memory of node ceiling x conjunct count). Borrow `outer` and keep only the local binders in the owned stack. | qsl-semantics/src/check/hybrid/term.rs:131 |
| FND-002 | low | Dead code: `let _ = index;` with an otherwise unused `enumerate()` in the missing-equation loop, and `let _ = operand;` in the `Pre` arm. | qsl-semantics/src/check/hybrid/model.rs:269-279; qsl-semantics/src/check/hybrid/term.rs:153-154 |
