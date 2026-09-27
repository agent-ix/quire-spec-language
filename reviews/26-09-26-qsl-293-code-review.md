---
id: SR-742
title: "Code and Rust review of embedded-body check locations"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@09e9d18585d4aed678ce681ef824a24b19a24b27; qsl-semantics/src/check/check.rs; qsl-semantics/src/check/mod.rs; qsl-semantics/src/check/region.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-096
    type: reviews
---
## Summary

Ticket: QSL-293 (blocks QSL-160). PR: quire-spec-language#493 at 09e9d185.
Code review with the rust-review lane, scoped to `git diff origin/main...HEAD`
(main 2df75ab6): three Rust files.

Sound: `PackageDeclarations::embedding` is set to `None` in the only
constructor (`PackageDeclarations::new`, check.rs:357). It is moved into
`CheckedGraph` in `check()` (mod.rs:697, :1112) and cloned into
`DeclarationRegions` (region.rs:152). All six resolve sites pass it through
the one `region()` helper. With `embedding: None` the helper's else-branch is
textually the pre-change body (region.rs:61-63), so a non-embedded package
behaves identically. The three QSL-239 tests still pass with `None`. No
`unwrap`/`expect`/`panic!` is added outside tests. There is no `unsafe`. The
`u64` conversions stay `try_from`.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The resolver never checks that the map belongs to this unit. `region()` calls `map.map_regions(map.body(), span)`, which passes the map's own body, so the source-identity check inside `map_regions` (source_map.rs:238-245) always passes. `self.source` (the unit's `RawSourceRef`) is never compared with `map.body().reference()`. `embedding` is a public field with no constructor-enforced invariant. A map for another body therefore produces plausible document regions for the wrong bytes (a wrong result, not a refusal). Fix: resolve to `None` unless `map.body().reference() == source`, or set the map only through a checked setter. | qsl-semantics/src/check/region.rs:57-59; qsl-semantics/src/check/check.rs:321 |
| FND-002 | low | The whole `SourceMap` is cloned on every `regions()` call and held by `CheckedGraph`. It owns both the original and the body `Source` (the full document text and the full body text). Consider `Arc<SourceMap>` so that `regions()` and `check()` share one copy. | qsl-semantics/src/check/region.rs:152; qsl-semantics/src/check/mod.rs:343 |
| FND-003 | low | Readability: `regions.next().filter(\|_\| regions.len() == 0)` expresses "exactly one" through a side-effecting iterator. A slice pattern (`match regions.as_slice() { [one] => Some(one.clone()), _ => None }`) says it directly. The test mixes `k as u64` (region.rs:512-513) with `usize::try_from` in the same function. | qsl-semantics/src/check/region.rs:59; qsl-semantics/src/check/region.rs:512-513 |

## Verdict

Approve with changes. Fix FND-001 in this PR, because it can produce a wrong
result. FND-002 and FND-003 are optional cleanups. The split-span disposition
is a spec question, recorded in SR-744 FND-001. Test coverage is recorded in
SR-743.
