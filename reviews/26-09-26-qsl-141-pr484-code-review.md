---
id: SR-707
title: "Code and Rust review of the value.rs module-edge scan"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@5a51beb0e6ccd69b5086c2510cf22d89ed907e15; qsl-forms/tests/it/identity_free_forms.rs; qsl-forms/src/value.rs; qsl-forms/src/lib.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-091
    type: reviews
---
## Summary

Ticket: QSL-141. PR: quire-spec-language#484 at 5a51beb0. Code review with the
rust-review lane, scoped to `git diff origin/main...HEAD`. The only Rust change
is the new test `value_module_has_edges_only_to_the_forms_core_and_the_lower_crates`
and its `ModuleEdges` visitor in `qsl-forms/tests/it/identity_free_forms.rs`.

Sound: the allow-list matches `value.rs`'s real imports (lines 17-29:
`qsl_cst`, `qsl_foundation`, `quire_exact`, `super::dispatch`, `super::spans`,
`super::syntax`), and its seven inline paths are `self::items`, which the
visitor admits. The test passes at this head. It is not tautological: in a
scratch worktree at 5a51beb0, adding `use crate::dispatch::FormsLimits as _Mut;`
to `value.rs` made it fail with `"20: crate::dispatch::FormsLimits"`. The
closing assertion (`use qsl_cst::` and `use super::syntax::` present) guards
against scanning the wrong file. Unrooted third-party inline paths are not
scanned, but `family_outcome_layering.rs` pins `qsl-forms`'s normal
dependencies to exactly three crates, so no such path can compile.

## Verdict

**Approve with nits.** Two low findings on the scanner's precision; neither
lets a real layering break through today.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | `judge` refuses every `crate::` path (`"crate" => false`), including `crate::dispatch`, `crate::spans` and `crate::syntax`, which are the forms core the doc comment and TC-398 allow. The test is stricter than its own oracle: a harmless rewrite of `super::syntax::X` as `crate::syntax::X` fails it. Fix: judge `crate::<second>` against `FORMS_CORE` like `super::`. | qsl-forms/tests/it/identity_free_forms.rs:219-227 |
| FND-002 | low | `syn::visit` does not enter macro token streams, so a path inside `matches!`, `format!`, `vec!` and the like is never judged (`value.rs` has nine such invocations). Today the crate has no other family module to reach, so nothing leaks. It becomes a gap once a sibling family module lands in `qsl-forms`. Fix: parse `Macro::tokens` as expressions or paths where possible, or say in the doc comment that macro bodies are out of scope. | qsl-forms/tests/it/identity_free_forms.rs:260-300 |
