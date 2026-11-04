---
id: SR-1578101
title: "Code review of PR #2240: repeated model aliases resolve to one declaration, code-review"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@2b4bbc1a5; git diff origin/main...2b4bbc1a5 (PR #2240): qsl-semantics/src/check/{assemble.rs,check.rs,mod.rs,model_spelling.rs,lowering/model.rs} and their tests; ticket QSL-1578"
review_set: subset
date: '2026-11-04'
---

## Summary

Ticket: QSL-1578. PR: quire-spec-language#2240, head 2b4bbc1a5. Method: code-review with the
rust-review lane. The PR projects a package selected under several `model` aliases once, under
its first alias, and adds `ModelSpellings`, which binds each further-alias spelling `N::T` to the
first alias's entry in the assembler name tables and the check `Scope`. Relationship roots are
deduplicated by declaration key in `relationship_nodes`.

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| first-alias projection loop | qsl-semantics/src/check/assemble.rs:3414-3487 | examined |
| object_names / composite names / names.types spellings | qsl-semantics/src/check/assemble.rs:3488-3510, 3932-3955 | examined |
| ModelSpellings add / bind_first / bind_all | qsl-semantics/src/check/model_spelling.rs:27-79 | examined |
| Scope::with_model_spellings | qsl-semantics/src/check/check.rs:1308-1321 | examined |
| relationship_nodes dedup | qsl-semantics/src/check/lowering/model.rs:2376-2395 | examined |
| alias uniqueness check | qsl-semantics/src/check/assemble.rs:3520-3566 | context_only |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | `ModelSpellings` is package-wide, but FR-480 lets two units bind one alias to two identities. With unit A `model M = X; model N = X;` and unit B `model N = Y;`, where X and Y both declare `T`, `bind_all` appends X's `M::T` candidate to Y's `N::T` entry, so `N::T` becomes ambiguous in both units. `bind_first` keeps Y's entry, so in unit A `N::T` in an object-type context resolves to Y. main already got this wrong (its `object_names.insert` overwrote the entry), so this PR does not regress it. Fix: skip a spelling whose alias also names a different identity, or key spellings by source unit. | qsl-semantics/src/check/model_spelling.rs:69-79; qsl-semantics/src/check/assemble.rs:3955 |

## Verdict

Mergeable. The design is sound. Type and node identity stay alias-independent, which is what
QSpec FR-322-AC-71, AC-132 and AC-142 require. Spellings only add keys that point at existing
entries. `or_insert` keeps declared names authoritative, and `bind_all` keeps a true ambiguity
ambiguous. `Scope::with_model_spellings` is called after `with_variant_types`, as its doc comment
requires (mod.rs:2163), so variant cases gain spellings through `variant_cases`, which is keyed
by type name. The rust-review lane found no idiom or safety issues: no new unwrap, no unbounded
growth beyond aliases x names, and only `pub(crate)` surface. FND-001 is a contrived cross-unit
edge that is already wrong on main.

Evidence: focused tests run in the review worktree (see the gap-analysis artifact SR-1578102).
