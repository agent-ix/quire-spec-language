---
id: TC-1093
title: "A consumer sets a named stage limit through the replay root"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-399
    type: verifies
---
# TC-1093: A consumer sets a named stage limit through the replay root

## Description

Verify a consumer naming only the `qsl_replay` root raises `s1.input_bytes`
by name and compiles a source over the default, and that the name is checked.
Scope: FR-399-AC-1 to FR-399-AC-4.

## Test Procedure

1. From an integration test that names only the `qsl_replay` root, generate a
   valid unit padded with comments to just over 1 MiB and compile it through
   `compile_admitted` with default limits.
2. Compile it with `StageLimits::default().with_named("s1.input_bytes", n)` and
   default `ReplayLimits`, then also with `ReplayLimits::with_input_bytes(n)`.
3. Call `with_named` with an unknown name, the empty name,
   `replay.input_bytes` and `format.output_bytes`.
4. Read a default `StageLimits`; set one name twice.

## Expected Results

1. The compile is refused as a stage limit.
2. Refused as above the reader limit; then compiled, with its enum listed.
3. Each is a `StageLimitNameRefusal` carrying the name.
4. The default holds no entry; the later bound governs.
