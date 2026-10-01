---
id: SR-958
title: "QSL-358 slice 3 code review (with rust-review lane and test-oracle check) of PR 567, the structural EnumDeclaration and the definition refusal vocabulary moved into quire-semantic-value"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@f0166db0e28e7cd401619ed93d6ea261cd895ae7; diff bcc946cd...f0166db0; quire-semantic-value/src/{enumeration,definition,lib}.rs; qsl-semantics/src/value/{enumeration,definition,mod}.rs; qsl-semantics/src/check/{assemble,check,identity,mod}.rs; qsl-semantics/src/check/lowering/tests/rows.rs; qsl-semantics/tests/it/{complete_value_lock,ieee_profiles,integer_division}.rs; qsl-bench/src/check.rs; qsl-eval/tests/it/{collection_algebra,equality_matrix}.rs; qsl-package/src/emit/tests.rs; tests/it/text_enum_identity.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/ADR-011
    type: reviews
---
## Summary

Ticket: QSL-358 (slice 3). PR: quire-spec-language#567 at f0166db0, diff
`bcc946cd...f0166db0` (slice 2's head is the base). Gate: `make ci` exited 0 at
f0166db0 (`head=f0166db0... exit=0`, including
`cargo build -p quire-semantic-value --target thumbv7em-none-eabi` and the two
new SV unit tests). Not re-run.

The coder's claims, checked against the code:

- **Enum admission order is unchanged.** Before: unsorted, owner, stale key.
  After: `EnumDeclaration::new` (non-canonical, then unsorted), owner, stale
  key. Both `EnumDeclarationPreimage` constructors already refuse an empty,
  repeated or non-identifier member list, so the new `NonCanonicalPreimage`
  branch cannot fire from `admit`. The observable order is the same.
- **Member admission order is unchanged.** Foreign declaration, undeclared
  case, then stale key, as before. `EnumDeclaration::member` builds the same
  `EnumValue` fields (declaration key, member key, ordered, position, case)
  that `EnumValue::admitted` got.
- **No goldens or fixtures changed.** The diff touches no golden, snapshot or
  fixture file.
- **One construction path.** `EnumValue::admitted` is deleted. A repo-wide grep
  for `EnumValue {` and `EnumValue::` finds one constructor:
  `EnumDeclaration::member` (quire-semantic-value/src/enumeration.rs:91). The
  other hits are the unrelated syntax `ExprKind::EnumValue`. But see FND-001:
  that path checks the case, and takes the member key on trust.
- **The definition vocabulary is #558's.** #558's merge commit d1c5fa47 is an
  ancestor of the branch. The lines removed from `qsl-semantics`'
  `definition.rs` and the new SV `definition.rs` differ only by one blank line,
  the new doc line, and `fn` changed to `pub fn` on `invalid_package`.
- **SV stays a leaf.** SV `definition.rs` uses only `core` and `thiserror`
  (already a no-std dependency). SV `enumeration.rs` adds `alloc`
  `BTreeSet`/`String`/`Vec` and `quire_exact::is_identifier`. SV's Cargo.toml is
  unchanged. No qsl-foundation, std or serde_json.
- **No QSpec data copied.** `SelectionRefusalCode::ALL` is the typed code set;
  the lock reader still checks the lock's `selection_refusal_codes` against it
  (qsl-semantics/src/value/definition.rs:470-480), as in #558. No digest, lock
  row or catalog moved.
- **No re-exports.** `qsl_semantics::value` drops the four definition types,
  and nothing re-exports `EnumDeclaration`. No caller uses an old path.
- **`invalid_package` is not a bypass.** `PackageRefusal`'s `code` and `cause`
  fields were already `pub` (tests build it as a struct literal). Making the
  helper `pub` lets callers build a refusal *value*. It admits nothing.

Rust review lane: no `unsafe`, no panics added, no `unwrap` outside tests, no
`std` in SV. The two SV unit tests have real oracles: `["b","a","a"]` is both
unsorted and repeated, so it pins non-canonical before unsorted. The undeclared
case is checked by its cause.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | `EnumDeclaration::member(case, member)` is the only `EnumValue` constructor, and it is public in the leaf that RT will call. It checks `case` against the declaration, but takes the member key `member` without checking it. That is by design: SV mints no `NodeKey` (ADR-011 SV bullet, T-12 rule (b)), and RT's copy has the same signature. But the docs still say the key was verified. `EnumValue::variant` says the bytes were "verified against that exact preimage at `qsl-semantics`' `AdmittedEnumDeclaration::admit_member`". `EnumMemberIndex::record` says two structurally identical members always share one `VariantId`. Neither is true for a value built through SV directly, which is RT's path. A caller that passes the wrong member key gets an `EnumValue` whose `=` (compares member keys) and ordering (compares positions) disagree, and an index entry under the wrong `VariantId`. Say on `member` (and on `new` for the declaration key) that the key is the caller's admitted key, taken as given. Reword `variant` and `record` to say what SV guarantees: the case is declared, the position and flag come from the declaration, and the key is whatever the caller admitted. The `variant` doc also has a broken wrap ("-- so this needs no" on its own line). | quire-semantic-value/src/enumeration.rs:78-97,143-152,172-178 |
| FND-002 | low | The member-list check (nonempty, distinct, identifiers) is now written three times: in SV `EnumDeclaration::new` and in `EnumDeclarationPreimage::from_json` and `::new`. `AdmittedEnumDeclaration::admit` clones the members, so every admitted declaration holds two copies, and callers still read `preimage().is_ordered()` (check/mod.rs:906) where `declaration()` now answers. Expose the check from SV once and call it from both preimage constructors, so the rules cannot drift. Read ordering and members from `declaration()`. | quire-semantic-value/src/enumeration.rs:40-63; qsl-semantics/src/value/enumeration.rs:89-130,266; qsl-semantics/src/check/mod.rs:906 |

## Verdict

Changes requested (one medium). The move is correct. Refusal order, goldens,
the single construction path, the leaf boundary, #558 parity and the no
re-export rule all check out. FND-001 is a documentation fix: the trust
boundary on the member key has to be stated where RT will read it. FND-002 is a
cleanup. Mergeable after #565 and #566, once FND-001 is fixed.
