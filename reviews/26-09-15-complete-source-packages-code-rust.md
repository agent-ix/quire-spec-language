---
id: SR-453
title: "Code and Rust review of complete source packages"
type: SpecReview
analysis: code-review
scope: "quire-spec-language#117; FR-131/134/302/303/339; TC-180/184/222; src/complete; tests/complete_*"
review_set: all
relationships:
  - target: ix://agent-ix/quire-specification/FR-131
    type: reviews
  - target: ix://agent-ix/quire-specification/FR-134
    type: reviews
  - target: ix://agent-ix/quire-specification/FR-302
    type: reviews
  - target: ix://agent-ix/quire-specification/FR-303
    type: reviews
  - target: ix://agent-ix/quire-specification/FR-339
    type: reviews
  - target: ix://agent-ix/quire-specification/TC-180
    type: reviews
  - target: ix://agent-ix/quire-specification/TC-184
    type: reviews
  - target: ix://agent-ix/quire-specification/TC-222
    type: reviews
---

## Summary

`/code-review` dispatched the QSL #117 Rust delta through `/rust-review` and
three independent review passes. The implementation now uses one declarative
grammar authority and bounded interpreter, retains a byte-exact recovering CST,
validates exact source/profile/model selections, and preserves source authority
through the internal FR-131 package graph without granting raw callers checked
semantic authority.

## Verdict

**PASS** — every actionable Rust, API-boundary, resource, diagnostic-locus and
test-adequacy finding was fixed and re-reviewed. No scoped panic, unsafe,
caller-mintable authority, handwritten grammar transcription, stub or tautology
remains.

## Resolved findings

| Finding | Resolution |
| --- | --- |
| Long imperative parser paths duplicated grammar authority | replaced with a declarative `Production -> Rule` table and one bounded interpreter; `/rust-review` was also updated upstream to detect imperative grammar transcription |
| Grammar choices, repetition minima and fixed-width terminals lacked direct boundary vectors | added table-driven positive/negative vectors with exact failing-token loci |
| CST identity and incremental whitespace updates could accept the wrong node or retain a stale root range | match exact node identity and preserve the full revised document range, with clone/foreign and leading/trailing edit regressions |
| Token limits were charged after allocating all raw leaves | preflight retained CST leaves before vector construction and stop at the first excess leaf, including a 900,000-digit lexeme regression |
| Hyphenated compound tokens duplicated declarative terminals | derive compound spellings from the grammar authority and prove each is one budgeted CST leaf |
| Catalog-aware editor fast paths could skip unknown/stale profiles | validate every selected profile before binding, revision checks or incremental early return, preserving exact selection loci |
| Formatter comments/newlines could exceed a lowered output limit before refusal | route every append through one checked writer and test exact/one-short trailing comment/newline capacities |
| Package refusals retained a bare span only | retain boxed source identity, path and raw-byte digest with every refusal |
| Resolver artifact-byte limits and identity exhaustion were ineffective/misclassified | charge resolved definition plus model bytes, stream canonical identity into SHA-256, and classify exhaustion as `ResourceExhausted` |
| Typed graph interpretation was not represented in package identity | bind role, dependency references, per-definition capabilities, model references and aggregate capabilities in the streamed identity |
| Raw callers could construct semantic definitions/models | require an unconstructible `ReaderAuthority`; only a crate-private test mint exists, while public authoring APIs consume authority-free exact-reference `ProfileCatalog` values |
| Capability inventory and validation duplicated family maxima | use one family table and round-trip all 176 values plus zero, one-past and unknown-family refusals |
| Model component diagnostics had a broad future fallback | validate identity/version through exhaustive `InvalidModelComponent`; parse digest independently at its own locus |
| Large typed refusal variants failed strict Clippy | box conflict/source-authority records rather than suppressing `result_large_err` |

## Rust review

- Complete parsing is iterative over a bounded grammar interpreter; untrusted
  source bytes, retained leaves, syntax nodes and nesting all have explicit
  ceilings and exact first-excess refusals.
- Source, definition, model and semantic digests remain distinct types. Source
  and artifact digests hash exact raw bytes; linked identity uses framed,
  domain-separated streaming fields.
- Recovery evidence never makes a source admissible. Catalog, revision and
  formatter correspondence checks all fail before publishing an editor result.
- `ReaderAuthority` has a private field and no production/public constructor.
  The concrete checked reader and checked-package identity remain Task-054
  work; QSL #117 does not self-attest FR-132/133 or TC-223.
- New production code contains no `unsafe`, blocking/async bridge, lock, lossy
  integer conversion, panic contract or dynamic callback seam.

## Gates

| Gate | Result |
| --- | --- |
| `cargo fmt --all -- --check` | pass after remediation |
| Clippy, all targets/features, warnings denied | pass after remediation |
| Focused complete CST/editor/grammar/package tests | pass after remediation |
| Full all-target/all-feature suite, serial | pass after remediation; only documented environment-dependent examples/private-packet tests ignored |
| All-feature doctests | 12/12 pass |
| `git diff --check` | pass |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No unresolved code or Rust finding remains after remediation. | #117; FR-131/134/302/303/339; TC-180/184/222 |
