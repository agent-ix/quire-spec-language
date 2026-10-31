---
id: SR-1926001
title: "code review (with rust-review) of PR #1926 (QSL-1261): delete DigestRecord, ByteDigest, DigestDomain and dead outcome/witness machinery"
type: SpecReview
analysis: code-review
date: 2026-10-31
scope: "agent-ix/quire-spec-language; PR #1926 git diff origin/main...0ff58a1d4 (qsl-foundation, qsl-replay/src/{lib,outcome,witness}.rs, tools/arch-lint, Makefile, xtask tests, Cargo manifests)"
review_set: subset
---
# SR-1926001: code review of PR #1926 (QSL-1261)

## Summary

Ticket: QSL-1261. One code-review pass with the rust-review lane folded in, over `git diff origin/main...0ff58a1d4` only.

Examined, clean:

- `OutcomeDocument` manual `Serialize` (qsl-replay/src/outcome.rs:1103-1116). The removed derive's field order was format, operation, last_stage, category (`serialize_with = "category_str"`, i.e. `category.as_str()`), items, diagnostics, artifacts, result, with no `skip_serializing_if`. The manual impl writes the same eight names in the same order, `category` as the same string, `Option` fields as `null`, and `artifacts` as `&[(); 0]`, which `serde_json::to_vec` (the only writer, outcome.rs:1444) emits as `[]`. The byte-literal test at outcome.rs:1710 pins `..."diagnostics":[],"artifacts":[],"result":null}`. The quire-outcome/1 wire is unchanged.
- Test helper `pn` (qsl-replay/src/lib.rs:437-449) is byte-for-byte the deleted `parameter_name` + `percent_encode` (same escape set `% = ; | < >`, controls and 0x7f, `%XX` upper hex), with `unwrap_or_default` replaced by `expect`. Production `parameter_node` percent-decodes and calls `node_key_from_text`, so the round-trip test at witness.rs:1616 still binds the encoder to the decoder.
- No live behavior removed: on origin/main `with_artifacts`/`OutcomeArtifact::generated` had no caller, `parameter_name` had only test callers (lib.rs:440, witness.rs:719, 1649), and the Witness `Debug` digest was a diagnostic field only. `sha2` stays in qsl-eval (sampler RNG) and `[profile.dev.package.sha2]` still applies to it.
- arch-lint `canonical-encoder` removal is complete: no remaining mode, Makefile target, CI_CHECKS entry or xtask expectation.

Cross-repo note (not a defect of this diff): quire-contract-codegen origin/main imports `qsl_replay::{ByteDigest, DigestRecord, DigestDomain}` (src/kani/identity.rs:217, src/replay/{composite_builder,frame,function,state_clause}.rs) through a `branch = "main"` git dependency. The ticket sequences the CG deletion after this lands.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - | correct-requirement-no-evidence |

## Verdict

**Clean.** The wire is preserved and the deleted code was dead. Not built: the verdict rests on reading the diff and the byte-literal test at outcome.rs:1710.
