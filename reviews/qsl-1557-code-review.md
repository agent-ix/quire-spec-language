---
id: SR-20261103230157
title: "Code and Rust review of QSL-1557 PR #2220 clock-selections flag"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@d5b2e550570cc3f2a2369b8ef7ea96e8e61e8b72; PR #2220 diff: src/emit_checked_v2.rs, tests/it/checked_handoff.rs, README.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-517
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-1312
    type: reviews
---

## Summary

Ticket: QSL-1557. Reviewed PR #2220 at `d5b2e550570cc3f2a2369b8ef7ea96e8e61e8b72`. The change adds `--clock-selections <json-file>` to `emit_checked_v2`. `Request::parse` reads the file into `temporal_selections`, the same field the struct API uses. A new `Error::Clocks` variant reports malformed files with the refusal exit code. The duplicate-detecting object reader `PointsFile` is renamed `NamedEntries` and now serves both the points file and the clocks file. The Rust lane is folded into this file.

## Verdict

**PASS with two low findings.** The parse path is correct. The flag is single-use, so a repeated flag falls through to `Usage`. Source and package name are checked before the file is read. The file goes through the shared size ceiling. `ClockConfiguration`'s custom `Deserialize` carries `deny_unknown_fields` and a tagged `kind`, so the unknown-kind and extra-member refusals come from the wire type itself rather than from new code. `Clocks` is added to the `exit_code` match, which yields exit 20. `run` parses before emitting and writing, so a refusal writes nothing to stdout or `--out`. No new dependency, `unsafe`, panic path, or `unwrap` in production code.

## Assurance Context

No AssuranceProfile applies to `src/emit_checked_v2.rs` under `spec/`.

## Scope

- `src/emit_checked_v2.rs`: examined `USAGE`, `Error::Clocks`, `exit_code`, `Request::parse`, `NamedEntries`, `execution_points`, `read_clock_selections`, and `run`.
- `tests/it/checked_handoff.rs`: examined `command_line` and `the_clock_selections_flag_emits_the_requests_bytes_and_bad_files_refuse`, with the `recipe` and `request` helpers as context.
- `README.md`: examined the new "Clock selections" paragraph.
- `ClockConfiguration` in `src/protocol_artifact/v2/wire.rs`: context only. It shows that `deny_unknown_fields` is present.

## Checks

- `cargo test --features checked-handoff --test it -- checked_handoff::the_clock_selections_flag checked_handoff::bad_requests emit_checked_v2::` (dev, debug=0, via locked-build.sh): aborted. The reviewer removed the target directory while the build was still compiling, so the reviewer did not verify a test result.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The test cannot fail if the file order is lost. It builds the clocks file from a `serde_json::Map`, which sorts keys because `preserve_order` is not enabled. The only multi-entry recipe (`event_position`: `Due`, `Full`, `Partial`) is already in alphabetical order. A reader that sorted names, or used a `BTreeMap`, would still pass `assert_eq!(Request::parse(..), expected)`. That leaves the "in file order" property of FR-517 untested. Fix: write the `event_position` file text with its keys out of alphabetical order (for example `Partial`, `Due`, `Full`) and assert that `temporal_selections` comes back in that order. | tests/it/checked_handoff.rs:557-570 |
| FND-002 | low | The README examples mix two shapes. The first example is a whole file, `{"Due": {...}}`. The next two, `fixed_sample` and `timestamped_event`, are bare clock configurations with no name key. A reader who copies the second or third example as the file gets an `Error::Clocks` refusal. Fix: wrap each example under a clause name, or introduce the last two as "configurations such as". | README.md:316-321 |
