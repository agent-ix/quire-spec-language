---
id: SR-9082
title: "Code review of quire-spec-language PR #831: dangling relationship ends refuse missing-name"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@05ff783f167720c91bd1af90646106561ce74638; PR #831 diff vs impl/business-model-meanings: qsl-semantics/src/model/normalize.rs, qsl-semantics/src/model/refusal.rs, qsl-semantics/tests/it/model_relationship.rs, qsl-replay/src/spine/lifecycle/tests.rs"
review_set: subset
---
# Code review of quire-spec-language PR #831

## Summary

Ticket: QSL-497. Code review with the Rust review lane folded in.

Checked:

- `check_node`'s Relationship arm in `normalize.rs`. Each end is checked on its own. An end that no record declares refuses `MissingDeclaration` with the new `UnknownRelationshipEnd` cause, naming only that end. An end that names a declaration of another meaning still refuses `InvalidModelBinding`/malformed at the same members as before.
- The widened gate (`role.is_some() || declared_at.is_some()`). The reader builds connections (`read_connection`) with `declared_at: None` and no roles on either end. It builds business relationships (`read_relationship`) with `declared_at: Some`. So no connection enters this arm. `systems::classify` runs only from `AdmittedPackage::admit`, never from `normalize`, so a dangling business end is not refused twice in the normalize path. The tests assert exact refusal counts of 1 and 2.
- Consumers of `UnknownRelationshipEnd`. `as_str` gives `missing-name`. `catalog_code` gives `missing_declaration`. `catalog_fields` gives `None`, the same as `UnknownPopulationMemberType`. The fixture and the two exhaustive test tables are updated. Every other consumer matches named variants or uses a wildcard that correctly gives `None`.
- Reds. In `qsl497a-red.log` the three TC-230 tests fail on the cause (`InvalidModelBinding` where `MissingDeclaration` was expected), which is a behavioural failure. TC-231 and TC-232 fail at admit because an older fixture used `model.*` identities. They are not reds for the behaviour under test.
- No existing test relied on the old malformed cause for an undeclared end. The full crate run in the gate log has no failures.
- Gate `qsl497a-gate.log`: fmt rc=0, clippy `--all-features -D warnings` rc=0, crate tests and root `cargo test` all `ok`, the xtask lib tests pass, and `make check-spec-validation` rc=0.

## Verdict

No blocking finding. One medium finding is recorded.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The TC-230 fixtures build `declared_at: None` with a source role, a shape the reader never produces. The widened gate's new half, `declared_at.is_some()` with no source role, has no test. Recorded, not blocking. | qsl-semantics/tests/it/model_relationship.rs:49-57; qsl-semantics/src/model/normalize.rs:2430 |
