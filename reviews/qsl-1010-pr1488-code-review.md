---
id: SR-17080
title: "Code review of PR #1488: move SubjectIdentity into qsl-replay (QSL-1010, FR-336-AC-6), code-review with rust-review lane"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@8a5a09595cdaabe41727532129b9efd7ff0a74af; git diff origin/main...8a5a09595 (PR #1488, branch qsl-1010-subject-identity): qsl-replay/src/model_check/subject_identity.rs, qsl-replay/src/model_check.rs, qsl-replay/src/lib.rs, qsl-analyze/src/model_check/request.rs, qsl-analyze/src/model_check/mod.rs, qsl-analyze/tests/it/universe_obligation.rs; context qsl-analyze/tests/it/protocol_roles.rs, qsl-foundation/src/bound.rs, qsl-replay/Cargo.toml, spec/functional/FR-336-bound-a-model-subject-by-its-universes.md, spec/test-cases/TC-1190-*.md"
review_set: subset
date: '2026-10-25'
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-336
    type: reviews
---
## Summary

Ticket: QSL-1010 (FR-336's own tickets are QSL-385 spec and QSL-42
implementation). PR: quire-spec-language#1488. The rust-review lane is folded
into this file.

**The move is faithful.** The struct, its `InitialState` tuple, its five
fields and their collection types (`BTreeSet`, `BTreeMap<_, BTreeSet>`,
`BTreeSet<[u8; 32]>`, `HashSet<ProofBound>`), the body of `of` and
`universe` are byte-identical to the removed code in
qsl-analyze/src/model_check/request.rs. The derived `Eq`/`PartialEq` is
unchanged. Only the doc gains "and of the proof bounds" in the
order-immaterial sentence (true before as well: `HashSet` equality). No
behaviour drifts on the request path.

**Re-exports.** `qsl_analyze::model_check::{SubjectIdentity,
SubjectIdentityRefusal}` re-export the qsl-replay items, so the existing
caller `qsl-analyze/tests/it/protocol_roles.rs:15,1464` compiles unchanged
(it is in the same `it` binary that was built and run). `request.rs`
intra-doc links resolve through the new `use`. A workspace grep finds no
other `SubjectIdentity` user.

**Refusal mapping.** `RequestRefusal` is unchanged (`NoRequirementRecord`,
`Package(#[from] EmitRefusal)`, both `#[error]` strings as before).
`SubjectContext::new` maps the single `SubjectIdentityRefusal::Package(e)`
onto `RequestRefusal::Package(e)` with an exhaustive `match`, so the same
`EmitRefusal` value reaches callers as before, and a future variant is a
compile error, not a silent catch-all. There are no stable error codes on
either enum.

**Hash agrees with Eq.** `package`, `initial`, `universes`, `packages` are
B-tree collections, so their `Hash` is order-deterministic and agrees with
their `Eq`. `ProofBound` derives `Eq` and `Hash` together
(qsl-foundation/src/bound.rs:275) and has no `Ord`, which is why the field
stays a `HashSet`. The set is hashed as `len` plus a `wrapping_add` fold of
each member's fixed-key `DefaultHasher` digest. Equal sets have equal member
multisets, so equal folds: `Hash` agrees with `Eq`. Sum folding is
commutative and, unlike XOR, sets cannot hold duplicates anyway, so there is
no trivial self-cancellation; the `len` prefix separates sizes. A collision
needs two distinct sets whose 64-bit member-digest sums agree, which is not
trivial for non-adversarial input (see FND-002 for the adversarial angle). No
panics, no `unwrap`, no integer casts (`len()` is hashed as `usize`;
`wrapping_add` is the intended modular sum, not an overflow bug).

**Layering.** qsl-replay/Cargo.toml has no qsl-analyze dependency (normal or
dev); every crate the new module imports (`qsl-eval`, `qsl-foundation`,
`qsl-package`, `qsl-semantics`, `quire-checked`, `thiserror`) was already a
qsl-replay dependency, and Cargo.toml is not in the diff. No `#[allow]` in the
diff.

**Tests.** Mutations re-run by the reviewer in a `git archive` copy of the
head, with its own target dir:

- M1, order-dependent proof-bound hash (hash `len` then each member in
  `HashSet` iteration order, instead of the fold): run 40 times,
  `the_facade_identity_is_the_request_path_identity_whatever_the_listing_order`
  failed 15 and **passed 25**. Survives in about 60% of runs (FND-001).
- M2, `of` keeps only the first proof bound (`.take(1)`): the facade test
  fails at universe_obligation.rs:266 (`assert_eq!(reversed, identity)`).
  Killed.

The coder's other two mutations (ordered initial states, swallowed emission
refusal) were not re-run; by reading, the `Vec` change breaks
`assert_eq!(reversed, identity)` and the refusal test's `expect_err` catches
a swallowed refusal.

Gates (the reviewer's copy of 8a5a09595, `CARGO_TARGET_DIR` under
~/.cache/reviewer-pr1488, every cargo call under `flock /tmp/qsl-cargo.lock`):
`cargo test -p qsl-analyze --test it universe_obligation`: 4 passed.
`cargo fmt --all --check`: clean. `cargo clippy -p qsl-replay -p
qsl-analyze --all-targets -- -D warnings`: clean. `RUSTDOCFLAGS="-D
warnings" cargo doc -p qsl-replay -p qsl-analyze --no-deps`: clean.
`cargo run --locked -p xtask -- string-edge`: exit 1 with 5 unmarked
occurrences, all in qsl-replay/src/hybrid/process_request.rs (199, 222, 233,
314, 315), a file this diff does not touch (baseline, not this PR); nothing
in the diff's files is reported.

## Verdict

Changes requested (one medium). The move itself is clean: faithful,
re-exported, the refusal maps to the same `RequestRefusal::Package`,
`Hash` agrees with `Eq`, the layering holds and nothing panics. But the one
part of the new `Hash` that is not order-safe by construction, the
`HashSet<ProofBound>` fold, is gated only by chance: a two-element set in
two independently seeded `HashSet`s iterates in the same order about half
the time, so an order-dependent hash passes the test in most runs (FND-001).
The PR body's "Mutation-checked (order-dependent hash ...)" was a lucky run.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The equal-hash assertion does not reliably kill an order-dependent proof-bound hash. The facade test builds two subjects with two proof bounds each; `HashSet` iteration order depends on each set's own random `RandomState`, not on the listing order, so with two members both sets iterate alike in roughly half the runs. Reviewer mutation M1 (hash `len` then each bound in iteration order, dropping the fold) passed the test in 25 of 40 runs. Listing the bounds "in reverse order" is therefore not what the test exercises, and the equal-hash clause of FR-336-AC-6 is evidenced only probabilistically. Fix: make the check deterministic, for example use 6 to 8 proof bounds (so a same-order coincidence is below 1 in 720) and also rebuild the reversed identity in a loop (say 32 times) asserting every hash equal, or assert hash equality between identities whose sets were built by inserting into a `HashSet` with differing capacities. Re-run M1 many times to confirm it is killed every run. | qsl-analyze/tests/it/universe_obligation.rs:251,254,267; qsl-replay/src/model_check/subject_identity.rs:100-119 |
| FND-002 | low | The proof-bound fold pre-hashes each member with a fixed-key `DefaultHasher::new()` before feeding the 64-bit sum to the caller's hasher, so for that component the outer hasher's random keys no longer protect against constructed collisions: two different proof-bound sets with equal digest sums collide in every `HashMap<SubjectIdentity, _>`, whatever its `RandomState`. Not a correctness issue (Hash still agrees with Eq) and only relevant if proof bounds come from untrusted input and identities key a hash map (FR-315's `HyperProductIdentity` is the expected consumer). Fix, if wanted: derive `Ord` on `FiniteBound` (and `IntegerInterval` if needed) and `ProofBound` (`DomainKey` and `DomainKind` already derive it) and store `BTreeSet<ProofBound>`, which makes the derived `Hash` correct and removes the hand-written impl; or note the trade-off in the comment. | qsl-replay/src/model_check/subject_identity.rs:105-118 |

## New findings (disposition pass 1)

Found reviewing follow-up PR #1498 (head 812890fe3, fix commit b5171a0b5).

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-003 | low | The new hand-written `bound_key` projection (`ProofBound` to `(DomainKey, Option<DomainKind>, Bound)`) is correct by reading: each `FiniteBound` arm maps injectively (`IntegerInterval` is exactly `lower`/`upper`, `NonZeroU64` to `u64` is lossless, and `Cardinality`/`Depth` are separate `Bound` variants), and the `match` is exhaustive so a new variant is a compile error. But only the `Cardinality` arm is exercised: every proof bound in `universe_obligation.rs` is `FiniteBound::Cardinality`. A lossy edit to the `IntegerRange`, `Depth` or `Variants` arm (for example keying `IntegerRange` on `lower` twice, or mapping `Depth` to `Bound::Cardinality`) would make two subjects with different proof bounds share one identity, against FR-336-AC-5, with no test failing. Not mutation-run by the reviewer (shell unavailable during this pass); judged by reading the test file. Fix: in the AC-5 test, add `assert_ne!` pairs for two `IntegerRange` bounds differing only in `upper`, and for a `Depth` versus a `Cardinality` bound with the same maximum. | qsl-replay/src/model_check/subject_identity.rs:52-62; qsl-analyze/tests/it/universe_obligation.rs:219-229 |

## Dispositions

Round 1, against follow-up PR #1498 (branch
qsl-1010-subject-identity-review-fixes, head 812890fe3, fix commit
b5171a0b5). PR #1488 had already merged. At the head, the reviewer's own copy
passed: `cargo test -p qsl-analyze --test it universe_obligation` 4 of 4,
`cargo clippy -p qsl-replay -p qsl-analyze --all-targets -- -D warnings`
clean, `cargo fmt --all --check` clean. `reviews/qsl-1010-pr1488-*.md` on
the branch are byte-identical to the review-pass SR files.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | The proof bounds are a `BTreeSet<BoundKey>`, so no `HashSet` iteration order or per-set seed remains, and the test checks 8 bounds over all 8 rotations and their reversals, each rebuilt identity equal with an equal hash. The original order-dependent-hash mutation no longer applies, because nothing is hashed in an unspecified order. |
| FND-002 | fixed | The hand-written `Hash` and its fixed-key `DefaultHasher` fold are removed; `SubjectIdentity` derives `Hash` over B-tree fields, so the caller's hasher sees every member. `Ord` is on a private `Bound` mirror because `IntegerInterval` (quire-exact) has no `Ord`. |
