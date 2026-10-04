---
id: SR-1271
title: "Code review of quire-spec-language PR #623: intake builds and drops its JSON tree iteratively (QSL-381 B6)"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@1a257626bbd0b7111971eea2c7ee378193ea6154; PR #623 diff against origin/main: Cargo.lock, qsl-semantics/Cargo.toml, qsl-semantics/src/model/intake.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-260
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-356
    type: reviews
---
# Code review of quire-spec-language PR #623

## Summary

Ticket: QSL-381 (slice B6). PR: quire-spec-language#623, one commit
1a257626b on origin/main. The Rust lane (rust-review) is folded into this
file.

What the PR does:

- `json_of` and `value_of` become one generic `build` over an explicit heap
  stack (`Vec<Frame>`), with a `scalar` callback that may fail, `array` and
  `object` closers, and a `discard` callback the error path hands every
  finished subtree to.
- `value_of` discards through `quire_canonical::drop_value`, which is an
  iterative pop-and-extend loop at b4bb97a5 (`src/value.rs:151-160`).
- `PackageDocument` gets a `Drop` that takes `tree` and hands it to
  `drop_value`.
- `quire-canonical` moves from 59fe4f06 to b4bb97a5 (branch `main`, so the
  lockfile is the only pin) and gains its `serde_json` feature, which
  exports `drop_value`.

Out of scope by ruling: `too_deep` and `IntakeLimit::NestingDepth` stay until
AGE-2224. They are not raised here.

Checked and clean:

- `build`'s state machine. A scalar root returns at once. A container root
  returns when its last frame pops (`finished` is set, `open` is empty). The
  `None => continue` arm at intake.rs:822 is unreachable on every path.
- The error arm (intake.rs:796-807): at that point `finished` is `None` and
  `next` is `None`. Every finished subtree sits in some frame's children and
  goes to `discard`. No finished subtree is left for native drop.
- Member order and duplicates. `object` receives members in document order.
  The shared reader refuses duplicate member names
  (`Malformed::DuplicateName`, read.rs:801), so `Map`'s collect never replaces
  and drops a deep value.
- `json_of` passes `drop` as `discard`, but its error type is `Infallible`,
  so that arm never runs.
- Focused tests: `cargo test -p qsl-semantics --lib -- tc_730`, 4 passed at
  1a257626b.

Recursive drops still on the paths the PR claims to fix: one, on
`PackageDocument::parse`'s digest-error path (FND-001). The `bundle: Json`
field still drops natively, but the PR documents that
(intake.rs:276-278: the semantic-IR crate's `Json` drop), so it is not a
finding.

## Verdict

Not mergeable yet. There are four findings: one high and three medium. The
builder and the `Drop` are correct on the main path. But `build` is a
hand-written walk where FR-356 requires the walker toolkit. One error path
still drops the `serde_json` tree natively. One deep test cannot fail. And
the builder's discard path has no test.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | `PackageDocument::parse` builds `tree` and then takes the digest. When `quire_canonical::sha256` fails (allocation or limit), `?` returns early and the local `tree: Value` drops through `Value`'s native, once-per-level drop. That contradicts the doc comment's claim that "the `serde_json` view builds and drops without recursion" (intake.rs:277-278). Today `too_deep` bounds this at depth 200, but it is the only native drop of the tree left in intake, and it becomes a real overflow when AGE-2224 removes the cap. Fix: take the digest before `value_of` (both read only `document`), or on the error arm call `quire_canonical::drop_value(tree)` before returning `digest_refusal(error)`. | qsl-semantics/src/model/intake.rs:333-343 |
| FND-002 | medium | `tc_730_a_parsed_package_document_drops_on_a_small_stack` cannot fail. It parses a document `MAX_DEPTH - 1` (199) deep, and a native recursive drop of 199 levels fits easily in 512 KiB. It also drops `bundle: Json` natively at the same depth in the same test and passes. A probe with `impl Drop for PackageDocument` deleted passed it too (see the Verdict note). Fix: build the `PackageDocument` directly in the test module, with `tree: value_of(..)` of a 100,000-deep document, `bundle: Json::Null` and any digest. Drop it on the 512 KiB thread, so the test fails when the `Drop` impl is removed. | qsl-semantics/src/model/intake.rs:6447-6466 |
| FND-003 | medium | Nothing tests `build`'s error and discard path (intake.rs:796-807). No test makes `scalar` fail after a deep subtree has finished. If `discard` were replaced by plain `drop`, or the frame loop removed, every test would still pass. A real document cannot drive it either. A probe found that every number the shared reader accepts converts in `value_of` (subnormals, `2e-324`, `18446744073709551616`, `1.7976931348623157e308`). Every lexeme that would fail (`1e309`, `1e400`, `1.7976931348623159e308`, a 401-digit integer) is refused first by the reader as `NumberOutOfRange`. Fix: test `build` directly on a 512 KiB thread. Use `value_of`'s closers and `discard = quire_canonical::drop_value`, with a `scalar` that returns `Err` on the string leaf in `format!("[{}0{},\"stop\"]", "[".repeat(100_000), "]".repeat(100_000))`, and assert that the `Err` comes back. | qsl-semantics/src/model/intake.rs:763-835 |
| FND-004 | high | `build` hand-writes an explicit-stack walk (`Vec<Frame>` with `Items` and `Members` cursors) over the reader tree. FR-356 Behavior 2 says "A top-down or mutually recursive walk SHALL run on the walker toolkit". Its use case is that an assessor reads "one Kani-verified traversal instead of a hand-written stack in each walk". `qsl-semantics` already depends on `quire-walk` and uses it in `type_form.rs`, `claims.rs`, `checked_dispatch.rs` and other files. Fix: implement `build` as a `quire_walk::Walk` with `Node = NodeRef<'d>`. `enter` makes leaves through `scalar` (a `ControlFlow::Break(error)` on failure) and pushes a container's children. `Frame` records the container kind, its child count and, for objects, the member names. `exit` pops that many finished values from the walk's own results stack and pushes `array`/`object` of them. On `Break`, the caller hands every value left on the results stack to `discard`. | qsl-semantics/src/model/intake.rs:763-835 |

Probes for FND-002 and FND-003 ran in a throwaway detached worktree at
1a257626b, with focused `cargo test -p qsl-semantics --lib`. With
`impl Drop for PackageDocument` deleted, all four `tc_730` tests still
passed, including `tc_730_a_parsed_package_document_drops_on_a_small_stack`.
A second probe ran 13 boundary lexemes through `quire_canonical::read` and
`value_of`. None that the reader accepts fails `value_of`.

## Dispositions

Round 1, reviewed at b38a14d6c21d3c4851fafe08e3b7a76ebaa79b1d (fix commit
b38a14d6c on 1a257626b; `git diff 1a257626b b38a14d6c`). The round adds no new
finding. Verified from the code: focused `cargo test -p qsl-semantics --lib --
model::intake` gave 87 passed, 0 failed, and `cargo clippy -p qsl-semantics
--all-targets -- -D warnings` was clean.

- FND-001: `parse` takes the digest before `value_of`. On the one later early
  return (a `json_of` fault), it hands `tree` to `drop_value`. No early return
  is left that drops the tree natively.
- FND-002: the drop test now builds a `PackageDocument` directly with a
  100,000-deep `tree` and `bundle: Json::Null`. It would overflow if the
  `Drop` impl were removed.
- FND-003: a new test runs `build` with a `scalar` that fails on the `"stop"`
  leaf after a 100,000-deep finished sibling. It asserts
  `Err(BuildFault::Scalar("stop"))` on a 512 KiB thread.
- FND-004: `build` is now a `quire_walk::Walk` (`Builder`). `enter` pushes a
  container's children in document order, and the toolkit enters them in
  push order (walker.rs). `exit` closes a container from the last `count`
  results. On `Break`, the caller hands the leftover results to `discard`.

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | b38a14d6c |
| FND-002 | fixed | b38a14d6c |
| FND-003 | fixed | b38a14d6c |
| FND-004 | fixed | b38a14d6c |
