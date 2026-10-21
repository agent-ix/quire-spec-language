---
id: SR-12285
title: "Code review of quire-spec-language PR #1171: ModelRequest on the EN-1 request path"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@96338902801d6e5a2557b94630025898632d6a96; PR #1171 diff against origin/main: qsl-analyze/Cargo.toml; qsl-analyze/src/model_check/{mod,request}.rs; qsl-analyze/tests/it/universe_obligation.rs"
review_set: subset
---
# Code review of quire-spec-language PR #1171

## Summary

Ticket: QSL-889. One code-review pass with the rust-review lane folded in,
over `git diff origin/main...HEAD` at the reviewed sha.

Run at the reviewed sha (shared target dir, `locked-build.sh`):

- `cargo test -p qsl-analyze --test it universe_obligation`: 2 passed.
- `cargo clippy -p qsl-analyze --all-targets --all-features -- -D warnings`:
  clean.
- `make arch-lint route-lint`: canonical-encoder, qualified-core (direction
  and ambient input), api-surface-qsl and route-lint all PASS.
- A reviewer probe test (not committed, deleted after the run) built two
  `Counter` worlds that differ only in the `post Inc` contract (`+1` against
  `+2`), with the same clause, the same domain package and the same initial
  document. Result: `clause_occurrence` equal, `SubjectIdentity` equal, and
  `author_clause` returned index 0 for both subjects in one `ModelRequest`.
  The clause of the first package written over the second subject was also
  accepted (index 0). This is FND-001.

Checked and clean:

- Layering. `qsl-route` (layer R) moves from a dev-dependency to a normal
  dependency of `qsl-analyze` (layer A). ADR-011 §6.1 lists R in A's
  "Depends on" cell, and the qualified-core direction check passes. No core
  crate gains an edge.
- Universes. They are merged per population into a `BTreeMap` of
  `BTreeSet`s. An empty universe (`Some(∅)`) stays distinct from no
  universe (`None`), which matters: no universe is `RequiresBound`.
  Duplicate populations and duplicate keys are refused by FR-120 admission
  (`qsl-eval/src/simulation/model.rs:509-529`), so merging never hides a
  difference that reaches a verdict.
- Initial states. Each one is its four labels plus its `sha256-jcs` digest,
  so the content is bound and the snapshot provision does not need to be in
  the identity. The set ignores order, which matches FR-120's
  `sorted_initial`.
- Proof bounds. A `HashSet` is used because `ProofBound` is `Hash + Eq` and
  not `Ord`. Equality is correct.
- Limits. Expansion limits, the meter and observation limits are excluded.
  Under ADR-014 §1 a reached ceiling is the caller's budget and settles
  `resource_exhausted`/inconclusive, never a different finished verdict.
  `max_depth` is a `ModelCheckRequest` member, not part of the subject.
  Excluding these limits is right.
- Domain package digests. They are the only digests in the identity, and
  they are load-bearing: they are canonical content identities of the model
  the subject is admitted against. They are not ceremony. The scope problem
  is FND-002.
- `author_clause` reads the requirement record from the subject's own
  package and refuses `NoRequirementRecord` with no item written. The
  refusal is a typed `thiserror` enum with no panic path. There is no
  `unwrap` or `expect` in shipped code.
- Production caller. `author_clause` has no non-test caller, but neither has
  EN-1 itself: `check_model` is called only from `qsl-analyze/tests/it`. The
  driver is the consumer (ADR-011 §6.1, layer A's dependents). The function
  is the seam the ticket asks for, the identity projection the obligation
  join needs. That passes the value test: it is not dead-code ceremony. The
  PR body's "adds the production caller of `ModelRequest`" overstates it,
  since it adds a library entry point and no caller.

## Verdict

FIX. FND-001 blocks. `SubjectIdentity` drops the checked package, and the
module doc's claim that "the checked package is named by the item's
occurrence" is false. Under ADR-013 O-09 a clause node id covers the clause
and the nodes it references, not the whole package, and the reviewer probe
shows two packages with different operation contracts sharing one
obligation and one request index. A verdict over one model then joins the
other. Fix: put the checked package's canonical identity into
`SubjectIdentity`, the one place where a digest is the feature, and add
the "differs only in the checked package" case to FR-336-AC-5 and its test.
FND-002 to FND-005 are recorded and do not block.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | `SubjectIdentity` holds initial states, universes, domain package digests and proof bounds, but not the subject's checked package. ADR-018 SM-2 says a model subject *is* a checked package with those members. The doc says the package "is named by the item's occurrence", but the occurrence is (clause node id, claim origin), and O-09 node ids cover only the clause and the nodes it references. Reproduced: two `Counter` packages differing only in `post Inc` (`+1` against `+2`), with the same clause, domain package and initial document, give equal `clause_occurrence`, equal `SubjectIdentity`, and `author_clause` index 0 for both in one `ModelRequest`. Their successor relations differ, so a `proved` over one joins the other's obligation. `author_clause(subject_2, clause_of_package_1)` is also accepted. Fix: add the checked package's canonical identity to `SubjectIdentity` and drop the false doc claim. | qsl-analyze/src/model_check/request.rs:29-45; qsl-analyze/src/model_check/request.rs:48-75; qsl-analyze/src/model_check/request.rs:119-137 |
| FND-002 | medium | `packages: simulation.packages.keys()` takes every digest in the FR-056 provision map, not the domain packages the subject's model selects. FR-336 says "the domain packages the subject is admitted against". A provision carrying an extra, unselected package therefore splits one subject into two identities. That is two indices for one verdict, a lost dedup, not a collision. The keys are also read before admission checks them against their bytes. Fix: project the selected `DomainPackageRef` digests (the checked package's model selections). | qsl-analyze/src/model_check/request.rs:73 |
| FND-003 | low | The repetition case `over(&fixture, &["b", "a", "b"])` is a subject that FR-120 admission refuses (`conflicting-identity` on the duplicate key), so the test asserts identity equality with a subject that can never reach a verdict. The order half (`["b", "a"]`) is the meaningful case. Use an admissible reordering. | qsl-analyze/tests/it/universe_obligation.rs:69-75; qsl-eval/src/simulation/model.rs:509-517 |
| FND-004 | low | `clause_occurrence` re-derives the occurrence triple that `qsl_replay::ClauseIdentity::resolve` already builds the same way (`WireNodeId::from_digest(node.identity())`, `node.claim()`). Two copies of the O-07 triple construction can drift. Reuse one. | qsl-analyze/src/model_check/request.rs:96-101; qsl-replay/src/model_check/identity.rs:68-71 |
| FND-005 | low | `pub mod request;` exposes the module path, while every other `model_check` submodule is private and re-exported with `pub use`. Follow the module's pattern (a private `mod request;` plus `pub use request::{...}`). | qsl-analyze/src/model_check/mod.rs:25 |

## Dispositions

| FND | outcome | sha |
|-----|---------|-----|
| FND-001 | fixed: SubjectIdentity holds the checked package's emitted package_id; test arm fails without it | 10ee83f10 |
| FND-002 | fixed: domain digests come from the package's model selections only; test arm with an unselected package | 10ee83f10 |
| FND-003 | fixed: repetition case replaced by the admitted reordering [b, a] | 10ee83f10 |
| FND-004 | fixed: qsl_replay::node_occurrence is the one triple construction, used by ClauseIdentity::resolve and author_clause | 10ee83f10 |
| FND-005 | fixed: private mod request plus pub use | 10ee83f10 |
