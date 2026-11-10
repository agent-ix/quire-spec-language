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
2. Run `run` over request and dependent files one byte over `request_bytes` and `dependent_bytes`; rerun with each option set to the required size. Run `compile` over a request file one byte over its fixed `request_bytes` ceiling and a source one byte over its fixed `dependent_bytes` ceiling; confirm each refusal names the setting and maximum and offers no option hint.
3. Run `parse` with a non-UTF-8 command or label operand, and over a valid source at a non-UTF-8 Unix path.
4. Run each verb in machine mode with stdout bound to `/dev/full`, and repeat for `format`'s supported formatted-text output.
5. Run each verb in machine mode over valid and refused requests with the stdout reader closed before the primary write, and repeat for `format`'s supported formatted-text output. An unsupported text rendering with no primary write is outside this pipe-write check.
6. Run `compile` and `run` in machine mode over valid requests and a refused request; compare stdout with the producer-owned FR-286 outcome bytes, from a relative and an absolute request path in two working directories. Check that the CLI neither substitutes the native library products nor writes a refusal envelope in place of the document.

Tag the tests `#[trace("TC-1262", "<AC id>")]`.

## Expected Results

- Step 1: usage errors exit 20, with usage on stderr, empty stdout and the path never opened; the blank-label call instead returns the machine outcome with `invalid_source_identity`/`blank-label` and the exact label, before opening the missing file. Every invalid invocation exits 20 without panic and the next valid call succeeds.
- Step 2: each over-limit run refuses naming the limit, its value and the option; the raised run succeeds. Each over-limit compile refuses naming its fixed setting and maximum with no option hint.
- Step 3: the non-UTF-8 operand exits 20 before the file is opened, no panic; the non-UTF-8 path parses.
- Step 4: exit 30 for every verb.
- Step 5: no stderr output and the outcome's FR-285 exit code.
- Step 6: stdout is exactly the producer-owned `quire-outcome/1` document for success and refusal, with no CLI-added or removed member. No package bytes, native result document or refusal envelope replace it on either stream. The two working directories give equal machine document bytes and exit codes.
