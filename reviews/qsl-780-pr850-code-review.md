---
id: SR-9135
title: "Code review of quire-spec-language PR #850: header protocol_profile survives the replay lock overwrite"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@4ac5e4283db82aeedcc0c5343c2970997f415257; PR #850: qsl-replay/src/spine/lifecycle.rs, qsl-semantics/src/check/lowering.rs, qsl-replay/tests/it/compile_protocol_profile.rs, qsl-replay/tests/it/main.rs"
review_set: subset
---
# Code review of quire-spec-language PR #850

## Summary

Ticket: QSL-780. Code review with the rust-review lane folded in. Static review of `git diff origin/main...HEAD`; no build run.

The cause is confirmed. The assembler sets `protocol_profile` from the unit header (`qsl-semantics/src/check/assemble.rs:2673-2680`). Then `Resolution` in `qsl-replay/src/spine/lifecycle.rs` overwrote `declarations.lock_evidence` with `frame.lock`, and that value only ever carries a text profile. The fix keeps the header's selection by calling `with_header_selection_of`, which uses `clone_from`. That means the header always wins, including when the header selects nothing.

## Verdict

**PASS (no blocking code-review findings).**

- No injection path: `LockEvidence` has private fields, derives `Default`, has no serde derive and no public setter for `protocol_profile`. On main, `with_protocol_profile` was called only from `assemble.rs`, so making it `pub(crate)` broke no caller.
- `with_header_selection_of` takes the value laws from the caller and the protocol profile from the assembled evidence. A caller cannot build evidence that carries a protocol profile, and if one somehow did, `clone_from` would overwrite it.
- The overwrite happens in one place only. Every public path (`check`, `compile_package` / `execute.rs`, `call_site`, `outcome`, `spine::call`, `spine::clause`, `src/command.rs`) reaches it through `check_unit`. Library frames apply their own header in the same way.
- No new `unwrap`, `expect` or panic in non-test code.

Recorded, not blocking:

- `with_header_selection_of` is `pub`. A qsl-semantics caller holding another unit's assembled evidence could carry that unit's selection over. The public replay path cannot reach this. Low.
- The test name `a_changed_profile_binding_changes_the_selection_or_refuses` claims more than it checks: the selection never changes in it, it only refuses. Low.
- AC-1's parenthetical "the same DefinitionLock that supplies TextProfile" cannot be met literally. `CatalogRole` has no protocol role, and under ADR-012 §2 a clause profile resolves in its own clause family's catalog (`profile::protocol_profile`). The substance of the AC is met: the catalog is authoritative, there is no fallback and no injection. Low.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |
