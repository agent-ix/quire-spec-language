---
id: SR-20261110070323860002
title: QSL-1826 bounded raw-state identity gap-analysis
type: SpecReview
analysis: gap-analysis
scope: agent-ix/quire-spec-language; QSL-1826 PR2654; 17-file raw state-key owner
  API delta
review_set: subset
---

## Summary

Independent review of frozen agent-ix/quire-spec-language; QSL-1826 PR2654; 17-file raw state-key owner API delta at b70f8fcc869e06fe5631d438839325499881a632. Ticket QSL-1826.

## Verdict

**CONDITIONAL** — new nested-value path needs a real constrained-stack control. Repository matrix qualification unavailable.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-002 | medium | The new bounded input-depth value projector lacks an actual nested kernel-value constrained-stack control; existing TC-729 bypasses it through a synthetic Encode implementation. | qsl-eval/src/simulation/model/encode/projection.rs:899-1190 |

## Assurance Context

No applicable active AssuranceProfile or exception found in the local spec context. No workflow acceptance claimed.

## Coverage

All 17 changed files inspected at frozen b70f8fcc869e06fe5631d438839325499881a632 relative to tested base bcb64e73424d1780a4f86127532a39f09818e35f. Main ba09deb6dd255b8e4d7142a47f95ba669c3cfba9 adds the unrelated PR2653 replay/spine delta; no owner-path overlap. No spec/plan or CI changes, so no spec-review submethod applies. Cargo.lock changes only normal quire-exact branch main bd2991e7 to merged 4fe7df6 plus one whitespace line; no manifest, pin or override.

Read reviewer/code-review/rust-review/gap-analysis skills, Rust style API/implementation/safety/wire/testing references, repository AGENTS/CLAUDE/README, FR-259, FR-120 key obligations, FR-196 complete-key context and FR-201 AC30 context. No active applicable AssuranceProfile or exception found. No Tokio/unsafe path introduced. No production edits or new Cargo runs by reviewer.

Shared projection/byte compatibility: old reference/population/record/tuple/union/enum/option/collection/normalized-decimal/quantity-unit/UUID/timestamp forms retain schema. Structural node/declaration/effective metadata lookups use borrowed views and metered actual equality, without hidden NodeKey Ord or cloning identity contents. First-view/validated reverse-index reference selection is preserved. Clock-shell suppression matches exact type/field identity. Stable map-key ordering retains repeated pairs; set/bag ordering uses canonical element bytes. No additional byte-compatibility defect found. However full-container canonical serialization itself is duplicated (FND-001), despite leaf Writer use.

Admission: explicit value/path/container vectors reserve before growth; scalar integer admission precedes the merged kernel decimal primitive; fixed scalar formatting charges before atomic formatting; source scans, text comparisons, task visits, key copies and state sorting charge caller work and poll the original cancel token. Raw identity.input_bytes remains separate from caller storage/work/representation and certificate transport caps. Denied callbacks retain caller-owned typed cause; cancellation, identity byte limit, actual allocator refusal, unsupported encoding and overflow remain distinct. Scoped RefCell state/denial side-channel belongs to one call and finish consumes the refusal; no persistent system key policy is installed. Qualified actions/weighted calls route current, undecided candidate and post-state identity construction through that scope. No additional typed-denial/current-post routing defect found in this pass.

Inspected supplied final-head checks from /tmp/qsl1826-bounded-raw-key-checks.json and logs: encode21, raw models2, undecided1, bare/optional clocks2, borrowed foundation identity1, existing native cancellation1, touched-three-package all-target/all-feature Clippy and fmt/diff checks pass. Raw model controls include unchanged three literal fixtures; exact/one-short/zero cumulative work/storage/identity caps; a 512KiB object identity rejected under 32KiB caller storage while legacy16MiB succeeds; small initial to 256KiB post refusal; fresh/legacy calls after denial; actual integer-phase cancellation. The large fixture was reduced from1MiB to512KiB to remain below the unchanged observation cap, not to change raw identity policy. Existing encode literal oracles also call the new bounded value sink. These are focused supplied results, not full-workspace or all-expansion qualification.

Scope boundary: existing ModelTransition::new/embed private canonical-reader allocations and ordinary interpreter state construction predate this raw-state API and are explicitly excluded by its docs/body/evidence. Original2648FND001 specifically concerns raw state-key scratch and internal candidate/post keys. This narrow owner PR need not invent a second transition schema or claim an unavailable upstream seam. Complete FR201AC30/all-expansion storage assurance remains unproven and is not granted by this review.

Computed repository matrix command `quire matrix --scope <owner-worktree> --format json` produced duplicate-module diagnostics but zero JSON after several minutes; it was interrupted. Full repository matrix/traceability status is unavailable, not clean. Manual scoped trace-to-code/test inspection is separate; no hand-written matrix or percentage. Actual input-depth bounded value coverage is missing (FND-002); the generic TC729 synthetic NestedList control and NodeKey equality depth128 test do not execute that changed path. No optional repository-wide semantic campaign or plan requested. Plan completion: not assessed.

## Examined scope

```yaml
scope:
- id: FR-259
  path: spec/functional/FR-259-encode-identities-and-read-json-through-quire-canonical-at-any-depth.md
  role: examined
  excerpt: "2. **Input-depth keys.** QSL SHALL encode every key over data whose depth\n\
    \   follows the input (a simulation state key, a replay value, an intake\n   document)\
    \ through the event API or the tree encoder, driven from an\n   explicit heap\
    \ stack."
- id: FR-259-AC-3
  path: spec/functional/FR-259-encode-identities-and-read-json-through-quire-canonical-at-any-depth.md
  role: examined
  excerpt: On a thread with a 512 KiB stack, the simulation state key QSL mints for
    a 100,000-long recursive list value, and the intake read of a 100,000-deep JSON
    package document, each equal the result on a thread with an 8 MiB stack, and the
    state key equals the RFC 8785 text of the state.
- id: FR-120-AC-4
  path: spec/functional/FR-120-simulate-a-checked-package-s-state-family.md
  role: examined
  excerpt: '`s0`''s state key has `semantic` equal to `{"type":"map","entries":[{"key":{"type":"text","value":"{\"node\":\"test/counters/counters\",\"package\":\"test/counters\"}"},"value":{"type":"map","entries":[{"key":{"type":"reference","universe":"<U>","object_type":"<E>","identity":"6331"},"value":{"type":"record","name":"{\"node\":\"test/counters/Counter\",\"package\":\"test/counters\"}","fields":[{"name":"value","value":{"type":"integer","value":"0"}},{"name":"label","value":{"type":"integer","value":"0"}},{"name":"next","value":null}]}}]}}]}`,
    where `<U>` and `<E>` are the universe and effective t'
- id: FR-120-AC-27
  path: spec/functional/FR-120-simulate-a-checked-package-s-state-family.md
  role: examined
  excerpt: 'Over a model whose `Counter` has the relationship `link` (both ends `0..2`,
    neither `unique`), operations `remove` (frame `deletes Counter`) and `spawn` (frame
    `creates Counter`) and universe `{c1, c2}`: from `s0` holding `c1`, `c2` and the
    listed pair (`c1`, `c2`), `remove c1` then `spawn` yields a state with `s0`''s
    objects and no pair, and its key differs from `s0`''s. A snapshot listing the
    pair once and one listing it twice are both admitted, have different keys, and
    exploration from the two counts 2 states. (For context, a state that lists no
    extents keeps FR-120-AC-4''s key, with no `relat'
- id: NodeIdentitySink
  path: qsl-foundation/src/node_wire.rs
  role: examined
  excerpt: 'Emit the shared structural node shape directly into a canonical writer.


    The writer admits its own storage and this traversal''s stack before growth.

    No intermediate identity text or JSON tree is retained.'
- id: FR-201-AC-30
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: context_only
  excerpt: "Source/document admission, exact key construction, complete graph\n  expansion,\
    \ predicate/workload evaluation, SCC and membership passes,\n  bias/value storage\
    \ and exact arithmetic SHALL obey actual caller byte,\n  state/edge, work/storage\
    \ and scalar/rational limits and cancellation."
```
