---
id: SR-707
title: "Code and Rust review of the value.rs module-edge scan"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language; qsl-forms/tests/it/identity_free_forms.rs; qsl-forms/src/value.rs; qsl-forms/src/lib.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-091
    type: reviews
---
## Summary

Ticket: QSL-141. PR: quire-spec-language#484. Code review with the
rust-review lane, scoped to `git diff origin/main...HEAD`. The only Rust change
is the new test `value_module_has_edges_only_to_the_forms_core_and_the_lower_crates`
and its `ModuleEdges` visitor in `qsl-forms/tests/it/identity_free_forms.rs`.

Sound: the allow-list matches `value.rs`'s real imports (lines 17-29:
`qsl_cst`, `qsl_foundation`, `quire_exact`, `super::dispatch`, `super::spans`,
`super::syntax`), and its seven inline paths are `self::items`, which the
visitor admits. The test passes at this head. It is not tautological: in a
scratch worktree, adding `use crate::dispatch::FormsLimits as _Mut;`
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

## Dispositions

Transcribed verbatim from the reviewer's Linear comment https://linear.app/agent-ix/issue/QSL-141/adr-011-m-3b-per-family-parsed-form-types-incremental-with-m-6a-m-6e#comment-19403785 (SR-707 dispositions).

<!-- reviewer-dispositions repo=agent-ix/quire-spec-language visibility=public id=SR-707 pr=quire-spec-language#484 date=2026-09-26 -->

| FND | Outcome |
| --- | --- |
| FND-001 | fixed |
| FND-002 | fixed |

Verified by mutation in a scratch worktree: appending `type _T = crate::syntax::TypeForm;` and `stringify!(crate::syntax::TypeForm)` to `qsl-forms/src/value.rs` keeps `value_module_has_edges_only_to_the_forms_core_and_the_lower_crates` green (FND-001). Appending `const _M: &str = stringify!(crate::build_unit);` makes it fail with `"1627: crate::build_unit"`, so macro-body paths are judged (FND-002).

+++ [reviewer data]

```yaml
dispositions:
  - fnd: FND-001
    outcome: fixed
    path: qsl-forms/tests/it/identity_free_forms.rs
    lines: "224-226"
    after_excerpt: |-
      "crate" => segments
          .get(1)
          .is_some_and(|second| FORMS_CORE.contains(&second.as_str())),
    evidence: "mutation: crate::syntax::TypeForm (inline and in stringify!) appended to value.rs -> test passes"
  - fnd: FND-002
    outcome: fixed
    path: qsl-forms/tests/it/identity_free_forms.rs
    lines: "283-325"
    after_excerpt: |-
      /// Macro bodies (`matches!`, `format!`, ...) are token streams the parser
      /// does not read as paths, so scan their tokens for `root::name` runs.
      fn visit_macro(&mut self, mac: &'ast syn::Macro) {
          ...
                  let rooted = matches!(name.as_str(), "crate" | "super" | "self")
                      || name.starts_with("qsl_")
                      || name.starts_with("quire_");
          ...
                  if rooted && segments.len() > 1 {
                      edges.judge(&segments, root.span().start().line);
                  }
    evidence: "mutation: const _M: &str = stringify!(crate::build_unit); appended to value.rs -> test fails with '1627: crate::build_unit'"
```

+++

