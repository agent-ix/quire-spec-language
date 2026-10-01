---
id: SR-946
title: "QSL-355 code review (with rust-review lane) of PR 556: delete revision and definition literals from native-linked-package/1"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@bd6ceb715d1d991272922ffe779a33e43b84007e; git diff 3260a231...bd6ceb71: src/package/{reading,view,wire}.rs, schemas/native-linked-package-1.schema.json, tests/package_construction_cases/{static_changes,vectors,features}.rs, tests/package_reading_cases/mod.rs, tests/it/package_construction.rs, tests/fixtures/native-package/*; tools/fixture-audit/*, tests/support/package_vector_setup.rs, tests/package_construction_cases/fixed.rs read as context; ~/dev/quire-contract-ir@c37e5d4, ~/dev/quire-contract-codegen@bda01f1, ~/dev/quire-observation@649da78 grepped read-only"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-019
    type: reviews
---
## Summary

Ticket: QSL-355. PR: quire-spec-language#556 at bd6ceb71.

Checks run independently of the coder's report:

- **No digest recompute.** Before this PR the only reader of
  `base_definition.digest` / `rules_definition.digest` was `select` in
  `src/package/reading.rs`, which compared the wire string with a string
  constant in `src/package/view.rs`. `tools/fixture-audit/rule_syntax.rs:50-64`
  does hash `state-semantics.md` and `profile.md`, but compares them with the
  external fixture's own `ruleContract.digest` / `baseProfile.definitionDigest`,
  never with the package constants or wire. quire-contract-ir and
  quire-contract-codegen contain no `native-linked-package`, `ir_revision`,
  `base_definition`, `rules_definition` or pin-literal references.
  quire-observation names only the format string
  (`src/lib.rs:13 NATIVE_LINKED_PACKAGE_FORMAT`). Deleting the digests and the
  whole `Definition` record is correct under the value test.
- **Fixture delta.** For minimal/controls/multiple `.package.json` and
  `.canonical.json`, the main-branch JSON with the three members removed is
  structurally equal to the new file. In `.package.json`, the only other change
  is `canonical_identity.digest`; domain/version/algorithm are unchanged.
- **Canonical identity is recomputed, not recorded.**
  `tests/package_construction_cases/fixed.rs:21-46` hashes the domain prefix
  plus the canonical bytes independently with Sha256 and compares the result
  with the `.sha256` file and with the producer's `canonical_identity()`.
  `identity.rs:39` recomputes it again. It is a real content identity and stays.
- **No other consumers.** `make ci` passed on bd6ceb71 (log head=bd6ceb71
  exit=0, clippy and all targets included). The only `STANDARD` left in src is
  `protocol_artifact/handoff/writer.rs:58`, a URI (`ix://agent-ix/quire-specification`)
  and not a revision. No example, bench, xtask or tool reads the deleted fields.
- **Rust lane.** The change is deletions only. No new panic surface, no
  integer conversions, no dead imports (clippy clean). The selection-order test
  still covers every remaining selector, and `UnknownProfile` keeps real
  producers (syntax/model/checking profile, canonical domain/version/algorithm).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | `header.canonical.json` and `unicode.canonical.json` are dead fixtures. No test, example or tool includes them (only `header.native` is used), and they were already stale before this PR: they lack the S-4b `authority`/`revision_namespace` source labels. The PR edits them by hand anyway. The fixture README still claims they are "retained as adverse empty-inventory data for TC-087" (TC-087 uses `header.native`), and it records header.native's SHA-256 `2bba5308…`, which nothing recomputes. Fix: delete both files and README lines 3-10. | tests/fixtures/native-package/header.canonical.json; tests/fixtures/native-package/unicode.canonical.json; tests/fixtures/native-package/README.md:3-10 |

## Verdict

The code deletion is correct and complete for the wire, reader, writer, schema
and the three live vector families. One medium finding: the two hand-edited
canonical files are dead and the README makes a false claim about them. They
should be deleted in this PR rather than maintained. Not mergeable until
FND-001 is fixed.

## Dispositions

Round 1, reviewed at bb0af37e896554e915c43236f8eb0de439aadd16.

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | a762f54e (files and README lines 3-10 deleted); bb0af37e also drops the README's commit-SHA provenance (2c6b9b8, 1c3aa50) |
