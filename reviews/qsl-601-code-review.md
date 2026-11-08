---
id: SR-20261108230650
title: "Code review — qsl-inspect render (JSON, text, DOT) re-land"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language; branch l5/p2-render e508a53e1 vs origin/l5/integration; FR-298; qsl-inspect/src/render.rs, qsl-inspect/src/lib.rs, qsl-inspect/src/result_views.rs, qsl-inspect/Cargo.toml, qsl-replay/src/outcome.rs, Cargo.lock, README.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-298
    type: reviews
---

## Summary

Ticket: QSL-601. Code review with the rust-review lane (rust-style loaded) over
`git diff origin/l5/integration...HEAD` (one commit, e508a53e1). The diff adds
`qsl-inspect::render` (`render_outcome`, `render_view`, `RenderSources`,
`Rendered`, `RenderCause`), an `AssignmentView::declaration_order` field, and
two `OutcomeItem` accessors in `qsl-replay`. Compared with the stale-branch
commit d95e094d6: dropping `package_id` and the digest fields is forced by the
current base types (`PackageView`, `OutcomeView` and `DocumentRef` no longer
carry them; `SourceIdentity` takes authority and identity), and keying
`RenderSources` on `RawSourceRef` matches FR-298's "source reference
(authority, identity)". FR-297/FR-298 fix no view JSON field names beyond
`format`, so the `source_authority`/`source_identity` renames are not a
deviation. No cargo build was run (shared lane); review is by reading.

## Verdict

**Changes requested.** One medium design defect against FR-298's limit and
cancellation text (whole output built before any charge or cancel poll), one
medium wire-contract defect (authoritative view JSON carries `Debug` strings),
and four low items. The `qsl-replay` accessors, the `declaration_order`
projection, the DOT refusal guard, the inclusive byte bound arithmetic and the
`RenderSources` constructor are correct.

## Scope

- **FR-298 — examined** (`spec/functional/FR-298-render-outcomes-and-views-from-typed-values.md`): Limits and cancellation: "`render` writes its output as units: one line of text, or one DOT statement. Before it writes a unit, `render` charges the unit's UTF-8 byte length against `output_bytes` and polls `&Cancel` (FR-276)."
- **FR-298 JSON — examined**: "JSON is the authoritative form: each outcome serializes as FR-286's document and each view with its own `format` identity."
- **FR-277 — context_only**: "Each event SHALL be checked before its work."

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Text and DOT output is formatted in full (`render_view_text`, `render_view_dot`, `render_diagnostic_lines`) before `finish_units` charges or polls `Cancel` even once, then copied into a second `Vec`. `render.output_bytes` therefore bounds nothing but the returned value (a 100,000-state graph is fully formatted, then discarded on a 1-byte limit), cancellation is not observed during the formatting that is the whole cost, and peak memory is about twice the output plus a slice vector. Letter of AC-8/AC-9 holds; FR-298's per-unit "charge before it writes a unit" and FR-277's check-before-work do not. Generate units through a charging writer instead. | qsl-inspect/src/render.rs:214-224, 227-268, 179-187 |
| FND-002 | medium | The authoritative view JSON serializes package, node, state-graph and certification fields as `format!("{:?}")` strings (model selections, requirements, capability rows, node tag/body/owner, occurrence keys, states, transitions, completion, certification). `Debug` output is not a contract, so any `Debug` change (including FR-073 redaction) silently changes JSON bytes; certification is `Trusted` in JSON but `trusted` in text. | qsl-inspect/src/render.rs:471-490, 494-506, 523-537, 548-563, 591-593, 599, 616-627 |
| FND-003 | low | Item text derives the wire label from `format!("{:?}", item.result()).to_lowercase()` instead of the `ItemLabel` serde `snake_case` label; correct only while every variant is one word. | qsl-inspect/src/render.rs:290-294 |
| FND-004 | low | Invariant panics: `Rendered::as_str` `expect`s UTF-8 over a `Vec<u8>`, and `render_view_text` round-trips `String -> Vec<u8> -> String::from_utf8(..).expect`. Holding a `String` in `Rendered` and returning `String` from `render_diagnostic_lines` removes both panic sites. | qsl-inspect/src/render.rs:139-161, 686-691 |
| FND-005 | low | When an outcome document has any diagnostic, its items (and their `proved` certification) are not rendered as text at all; FR-298 requires a `proved` record's certification beside its verdict. Confidence low: depends on whether a document can carry both. | qsl-inspect/src/render.rs:287-301 |
| FND-006 | low | `render_view_dot`'s `_ => Vec::new()` arm silently emits an empty `digraph view {}` for a non-graph view if the guard in `render_view` ever drifts; prefer a typed graph-view split so the unsupported case is unrepresentable. | qsl-inspect/src/render.rs:772-777 |
