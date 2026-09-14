---
id: Task-037
title: "Publish the compiled-protocol v2 handoff addresses"
type: Task
status: done
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-050
    type: references
  - target: ix://agent-ix/quire-spec-language/TC-138
    type: verifies
---
## Scope

Complete issue #78's public Rust handoff surface by exporting the five
consumer-facing `/2` member filenames and mutation-corpus format beside the
already published directory path. Test the constants against the committed
inventory and decoded manifest. Do not regenerate the corpus, change a tag or
release, or edit any TL-owned repository.

## Subtasks

- [x] Add the TC-138 control before the missing constants.
- [x] Export the producer-owned names and replace local handoff literals.
- [x] Run the required serial local Rust gates with `target-codex-backends`.
- [x] Complete self `/rust-review` and `/gap-analysis` and resolve their findings.
- [x] Reconcile issue #78, Plan-009 and the Test Matrix for the reviewable one-ticket PR.

## Delivery

The test first failed at the missing public imports, then passed against the
committed inventory and decoded manifest. Formatting, strict Clippy and the full
test suites pass under both no-default and all-feature configurations; the
minimal build, two fixture audits and parse/format smoke checks also pass. SR-422
records the Rust review and SR-423 records the final targeted gap analysis.
Issue closure and the dependent qprotocol pin follow the admin merge.
