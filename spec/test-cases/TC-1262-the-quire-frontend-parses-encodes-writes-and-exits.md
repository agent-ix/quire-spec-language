---
id: TC-1262
title: "The quire frontend parses, encodes, writes and exits for parse, format, run and compile"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-287
    type: verifies
---
# TC-1262: The quire frontend parses, encodes, writes and exits for parse, format, run and compile

## Description

Verify the `quire` binary's frontend behavior for `parse`, `format`, `run` and `compile` over the real binary: usage and arity refusal before I/O, option spelling for the intake limits, invalid invocation, OS encoding, stream write failure, broken pipe, and the bytes written to each stream.

Scope: FR-287-AC-3, FR-287-AC-4, FR-287-AC-5, FR-287-AC-6, FR-287-AC-7, FR-287-AC-8, FR-287-AC-9.

## Test Procedure

1. Run each verb with a missing and with an extra operand, a nonexistent request or source path, a blank label with a missing file, an unknown option and a malformed option value; record exit code, stdout and stderr, and whether the path was opened.
2. Run `run` and `compile` over requests one byte over `request_bytes` and `dependent_bytes`; rerun each with `--request-bytes` or `--dependent-bytes` set to the size.
3. Run `parse` with a non-UTF-8 command or label operand, and over a valid source at a non-UTF-8 Unix path.
4. Run each verb with stdout bound to `/dev/full`.
5. Run each verb with the stdout reader closed before the write.
6. Run `compile` and `run` over valid requests, and a refused request; compare stdout and stderr bytes with `command`'s outcome, from a relative and an absolute request path in two working directories.

Tag the tests `#[trace("TC-1262", "<AC id>")]`.

## Expected Results

- Step 1: exit 20, usage on stderr, empty stdout, the path never opened; the blank-label call refuses `invalid_source_identity`/`blank-label`; every invalid invocation exits 20 without panic and the next valid call succeeds.
- Step 2: each over-limit request refuses naming the limit, its value and the option; the raised run succeeds.
- Step 3: the non-UTF-8 operand exits 20 before the file is opened, no panic; the non-UTF-8 path parses.
- Step 4: exit 30 for every verb.
- Step 5: no stderr output and the outcome's FR-285 exit code.
- Step 6: stdout is exactly the package bytes or result document, stderr exactly the refusal envelope, no extra newline, and the two working directories give equal bytes and exit codes.
