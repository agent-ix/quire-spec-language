---
id: TC-915
title: "Native CI preserves feature lane paths and complete Cargo argv"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/NFR-013
    type: verifies
---
# TC-915: Native CI preserves feature lane paths and complete Cargo argv

## Description

Check NFR-013-AC-1, NFR-013-AC-2 and NFR-013-AC-3 at the actual Make-to-Cargo
process boundary. `xtask/src/ci_feature_lane_tests.rs` copies the repository's
Makefile into an isolated fixture and compiles the Rust recorder
`ci_feature_lane_tests/cargo_double.rs` under the owned target. The recorder
reports the received `CARGO_TARGET_DIR` and full Cargo argument vector; it does
not compile a workspace or qualify compiler feature behavior.

## Test Procedure

1. Run `every_feature_recipe_uses_its_child_and_preserves_complete_argv` with
   an unset root, empty root, relative/absolute roots and literal path cases:
   spaces, double quotes, backticks, `${FEATURE_LANE_PATH_VALUE}`,
   `$(FEATURE_LANE_PATH_VALUE)`,
   `$FEATURE_LANE_PATH_VALUE`, single quotes, backslash, semicolon and glob
   characters. Compare all seven default and two all-feature calls to the
   independent complete argv and child-path expectations (AC-1/2/3).
2. Run `explicit_roots_and_lane_overrides_are_one_quoted_argument` for
   command-line root precedence and both explicit lane overrides, including
   the same metacharacter classes. Pass assignments as direct Rust process
   arguments. Supply the desired raw `CARGO_TARGET_DIR` bytes unchanged and
   compare every received child path and complete argv: one-dollar roots
   remain one dollar and independently supplied doubled-dollar roots remain
   two, including before braces or parentheses. For explicit
   `CI_DEFAULT_TARGET_DIR` and `CI_ALL_TARGET_DIR` overrides, double each
   desired dollar for recursive GNU Make assignment decoding and independently
   compare the decoded path bytes. Environment roots retain their raw bytes
   (AC-2/3).
3. Run `repeated_switches_in_both_directions_keep_each_lane_directory` twice
   in each direction with one caller root; compare every actual command's
   directory and complete argv (AC-1/3).
4. Run `aggregate_reaches_default_then_all_features_without_changing_recipe_order`
   against `ci`, marking unrelated prerequisites old with `-o` only inside
   this focused fixture. Compare the ordered nine actual calls. This checks
   feature-lane reachability, not full aggregate execution (AC-1/3).
5. Run `clean_and_core_tooling_retain_their_own_target_and_features` with
   the ordinary caller root containing a space. Compare complete argv for the
   clean build, writer check, parse, string-edge, route-lint and checked-input
   calls, including their existing root/clean-child policy (AC-3). This is
   supplementary recipe preservation; parent clean-path metacharacter controls
   and actual compiler qualification remain separate.
6. Run `routing_oracle_rejects_safe_path_and_command_mutations`. Establish
   equality for a controlled quote/dollar root. Independently mutate exactly
   one site in the fixture copy: remove the default child suffix, remove a
   literal quote through Make's safe substitution while retaining quoted
   environment transport, or add `--all-features` to the default test command.
   For each, require the actual recording to differ from the independent
   complete oracle, restore the exact original Makefile bytes and require
   equality again (AC-1/2/3). Never execute the unsafe historical shell
   interpolation or use real secret environment values.
7. As fixture hygiene, after the first test's synchronous subprocess calls
   complete, require the compiled helper to exist, drop its fixture owner and
   require its temporary directory to be absent. Preserve the retained owned
   warm Cargo artifacts; the check removes only its owned temporary helper.

## Expected Results

All unmodified recordings equal the independent path/argv expectations. Each
safe mutation is rejected and each exact restoration recovers equality. Both
switch directions retain distinct feature children. Shell metacharacters remain
literal path data; the controlled dollar canary is not expanded. The helper's
temporary directory is removed after child completion and owner drop.

These results establish the routing criteria and fixture hygiene only. Real
Cargo compilation and real CLI default/all-feature extraction controls remain
separate evidence. The full ticket acceptance is retained: "Running `make ci`
twice in succession from a warm target dir produces the same result both times,
and no lane can execute an artifact built under different features." This TC
does not waive or substitute for the two unchanged-source warm aggregate runs.
