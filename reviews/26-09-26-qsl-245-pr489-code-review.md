---
id: SR-732
title: "QSL-245 code and Rust review of PR 489 (blank-label and empty-path causes)"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@237f65714b313473649a155f87932778a790d819; qsl-foundation/src/source.rs; qsl-foundation/src/diagnostic.rs; qsl-foundation/src/lib.rs; qsl-cst/src/diagnostic.rs; qsl-cst/src/cst.rs; qsl-cst/tests/it/complete_cst.rs; qsl-cst/tests/it/complete_grammar.rs; qsl-replay/src/execute/tests.rs; src/main.rs; src/command/output.rs; src/command/output/types.rs; src/complete/edit.rs; src/complete/editor.rs; src/runtime/construction.rs; src/runtime/input.rs; tests/it/cli.rs; tests/it/complete_cst.rs; tests/it/complete_editor.rs; tests/it/runtime_inputs.rs; tests/it/standalone.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-001
    type: reviews
---
## Summary

Ticket: QSL-245. PR: quire-spec-language#489 at 237f6571. This is a code
review with the rust-review lane. It covers only `git diff origin/main...HEAD`,
which touches 21 Rust files.

Sound:

- `SourceReadCause::UnnamedSource` is split into `BlankLabel { label }` and
  `EmptyPath`.
- `SourceIdentity::first_blank_label` checks `SourceLabel::ALL` in the order
  authority, identity, revision_namespace, revision. It runs before the
  path check (qsl-foundation/src/source.rs:340-353). Blank means
  `str::trim().is_empty()`, which is Unicode `White_Space`.
- Neither cause has a region: `refuse(.., None, ..)` in foundation, and
  `region: None` on the complete reader's diagnostic.
- `HostCause::code()` maps `EditPredecessor`, `ForeignNode` and
  `RequestRevision` to `InvalidSourceMap`.
- The new `error_without_region` builds a `CompleteDiagnostic` with
  `region: None`, and all three sites use it.
- The cause and label reach four surfaces: the CLI refusal line, the run
  output `details`, `InputError.identity_cause` and the replay S1 refusal.
- Every new `match` is exhaustive and has no wildcard. No production
  `unwrap`, `expect` or `panic` was added.

Public API: several public items change.

- `SourceReadCause::UnnamedSource` and `HostCause::UnnamedSource` are removed.
- `HostCause::BlankLabel` now carries data.
- `InputError` gains a `pub` field.
- `SourceLabel` is new and re-exported.

A search of every checkout under ~/dev found consumers only inside QSL
checkouts, so no external matcher breaks.

Merge hazard with PR #490 (task/245-kernel-refusal-records): none measured.
`git merge-tree` of the two heads gives 0 conflicts. PR #490 changes
qsl-foundation/src/diagnostic.rs only at lines 815-1000 (`CATALOG_CATEGORIES`,
`kernel_refusal_record`). This PR changes the `Diagnostic` struct (lines
346-350), its accessors (lines 457-470), `error` (line 512) and
`source_refusal` (lines 561-603). PR #490 adds no `Diagnostic { .. }`
literal, which would otherwise have had to supply `identity_cause`. `spec/spec.md` and
`spec/tests.md` are changed by both PRs, in different rows.

Forbidden-path check: the diff touches no `quire-exact`,
`kernel_refusal_record`, FR-100..111, TC-450..469 or `.github/workflows` file.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The build still claims `quire.native.diagnostics/v1` revision `1-draft.7`, but it now emits `invalid_source_identity` with `blank-label` and `empty-path`. The catalog at QSpec 84ed5298 says: "A producer claiming revision `1-draft.7` or earlier ... emits `invalid_source_identity` with neither `blank-label` nor `empty-path`." Scenario: #489 merges before the revision bump, so main is a nonconforming producer until the bump lands. Holding the bump back is intentional here (coder A bumps it), so this is a merge-order constraint: land it with the bump or after it. It is not a code defect. | src/linking/composed/definition_source.rs:248; qsl-foundation/src/source.rs:340-353 |
| FND-002 | low | `SourceIdentity::reference` reports provenance errors that have nothing to do with labels as `BlankLabel { label: Authority }`: `NotSourceBytes`, `ReversedRegion`, `NoRegion` and `DuplicateOccurrence`. `Source::read_typed` then refuses with "source authority must not be blank". The path is unreachable today, because provenance refuses only empty labels and the digest domain is fixed. If a future provenance rule is added, the refusal would give a false cause instead of reporting an internal fault. Prefer an established-invariant or internal-fault refusal over a made-up `blank-label`. | qsl-foundation/src/source.rs:118-126, 366-374 |
| FND-003 | low | `Diagnostic::with_identity_cause` is a new `pub` builder with no caller and no test anywhere in the workspace. `main.rs` and `construction.rs` carry their causes on their own structs instead. Either wire it or remove it. | qsl-foundation/src/diagnostic.rs:465-470 |
| FND-004 | low | The catalog tags `blank-label` and `empty-path` are spelled out in two places: `SourceReadCause::identity_tag` and `HostCause::tag`. `HostCause::tag` could return `self.identity_cause().and_then(SourceReadCause::identity_tag)` for those two variants, which would give one source of truth. | qsl-cst/src/diagnostic.rs:199-203; qsl-foundation/src/source.rs:228-236 |
| FND-005 | low | The CLI refusal line adds `cause` and `label` by inserting string keys into a `serde_json::Value` (rust-review §10). The run output carries the same fact on a typed `Serialize` struct (`types::Diagnostic.cause/label`). That makes two schemas for one fact, and only the typed one has a field list the compiler checks. | src/main.rs:39-44; src/command/output/types.rs:114-120 |

## Verdict

Approve with findings. No finding blocks a merge. FND-001 is a merge-order
constraint: land #489 together with the catalog revision bump, or after it.
Mutation checks are in SR-733.

The full gates were not re-run, as the brief instructed. Only the targeted tests were run: at
237f6571, the 5 root `it` tests, 12 `qsl-foundation` `source::` unit tests
and 1 `qsl-replay` TC-444 test pass.
