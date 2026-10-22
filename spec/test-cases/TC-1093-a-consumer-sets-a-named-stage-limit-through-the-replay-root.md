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
Scope: FR-399-AC-1 to FR-399-AC-6.

## Test Procedure

1. From an integration test that names only the `qsl_replay` root, generate a
   valid unit padded with comments to just over 1 MiB and compile it through
   `compile_admitted` with default limits.
2. Compile it with `StageLimits::default().with_named("s1.input_bytes", n)` and
   default `ReplayLimits`.
3. Compile it with the same limits and `ReplayLimits::with_input_bytes(n)`.
4. Generate a unit of enum declarations over 1 MiB. Compile it at the
   defaults; with `s1.input_bytes` and the reader limit raised; with
   `s1.tokens` also raised; and with `s1.nodes` also raised.
5. Call `with_named` with an unknown name, the empty name,
   `replay.input_bytes` and `format.output_bytes`.
6. Compile a comment-padded source just under 1 MiB at the defaults. Set one
   name twice and compile with the result.

## Expected Results

1. Refused as a stage limit naming the `s1.input_bytes` ceiling.
2. Refused as above the reader limit.
3. Compiled, with its enum listed.
4. Refused at the `s1.input_bytes` ceiling; then at the `s1.tokens` ceiling;
   then at the `s1.nodes` ceiling; then compiled.
5. Each is a `StageLimitNameRefusal` carrying the name.
6. The source compiles; the later bound governs.
