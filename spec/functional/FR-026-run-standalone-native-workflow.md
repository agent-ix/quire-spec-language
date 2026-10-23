---
id: FR-026
title: "Read a native-v1 request and its selected files"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-002
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/FR-027
    type: references
  - target: "ix://agent-ix/quire-specification/FR-301"
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-100
    type: references
---
## Description

When an author invokes quire-spec run with a native-run/1 request file, or
quire-spec compile with a native-compile/1 request file, the command shall
read the request and each file it selects under this requirement's bounded
intake.

The command routes the program by that edition with FR-027's edition reader.
Only a `1-draft` program reaches a runner or a compiler: `run` calls a named
function or runs a clause through the spine
([FR-100](FR-100-run-a-named-function-through-the-spine.md),
[FR-312](FR-312-read-a-clause-run-request-and-run-it.md)), and `compile`
writes `quire.checked-package/v2` bytes
([FR-027](FR-027-export-compiled-native-package.md)). Every other declared
edition, `0-draft` included, refuses `unknown_edition` (FR-027, FR-100).
This requirement owns what the two requests share: the envelope, the
source selections and their labels, path resolution, the intake limits, the
digest checks, command-line arity and the intake refusal envelope. FR-100,
FR-312 and FR-027 own each request's members, outcome and exit statuses.

## Inputs

A JSON object with format and request fields. Each source selection names
its file, the two source labels of [FR-001](FR-001-read-exact-source.md)
(`authority` and `identity`), its SHA-256 digest and its formal
document/revision. Both labels are required members. A request whose source
identity lacks one, such as a request written with only `identity`, refuses
at the request stage with the native code `invalid-request` and exits 20;
the catalog keeps that code as a retained host code with its broad meaning,
malformed command (`quire.native.diagnostics/v1`, "Other retained
host/source codes").
Relative paths resolve against the request's directory; absolute paths remain
valid local file operands. Source display paths retain the authored file string.
Parent-relative components are also admitted local operands; the command does
not establish a tenant filesystem boundary.

## Outputs

Intake failures write a JSON command-error envelope (`native-run-result/1`)
on stderr, with the request digest when read, the typed stage and code and
the relevant original source details, and write nothing to stdout. On
FR-301's six-code contract, malformed requests, invalid command usage and
I/O failures exit 20, and output failures exit 30. Broken pipes end quietly
with the computed status. Command error spellings belong to the native code
catalog. Resource causes distinguish the `request_bytes` and
`dependent_bytes` limits.

## Behavior

The command shall decode the envelope and select its format before request fields.
The command shall admit, at decode, the members FR-100 and FR-027 define for a
`1-draft` program.
The command shall reject unknown/duplicate fields and positional record arrays.
The command shall open each file once and cap reads before parsing or hashing.
The command shall read the request under the caller's `request_bytes` limit
(default 1 MiB) and the dependent files it names under the caller's
`dependent_bytes` limit on their total size (default 8 MiB). The CLI raises
them with `--request-bytes` and `--dependent-bytes`; neither has a ceiling.
The command shall verify each selected file's digest before consuming it.
The command shall begin each invocation with fresh budgets.
The CLI shall parse its command and operands once before I/O and report usage
for the selected command when operands are missing or excessive.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-026-AC-3 | A `1-draft` run request with a stale source digest, a malformed request, an unknown format and an unavailable file each refuse at their actual stage with their catalogued code and the available source details, write nothing to stdout and exit 20; each emitted command-error envelope conforms to the native result schema. | Test (TC-104) |
| FR-026-AC-4 | A `1-draft` run request one byte over `request_bytes`, and dependent files one byte over `dependent_bytes`, each refuse naming the limit, its configured value and its CLI option; the same requests with each limit raised to their size run; a fresh default request then succeeds. | Test (TC-104) |
| FR-026-AC-5 | Command-specific arity errors precede I/O and exit 20, and relative, absolute and parent-relative file operands in a request stored outside the working directory resolve independently of the working directory. | Test (TC-104) |
| FR-026-AC-6 | A `1-draft` run request whose program source names `authority` `agent-ix` and `identity` `p` runs, and its `spine-run-result/1` `source` renders those two labels and the source digest, and no other identity member. The same request without the program source's `authority`, and again without its `identity`, refuses at the request stage with `invalid-request` and exits 20. A present but blank `authority` refuses with `invalid_source_identity`, cause `blank-label`, field `label` `authority` (FR-001). | Test (TC-430) |

## Dependencies

- [FR-001](FR-001-read-exact-source.md): the two source labels.
- [FR-027](FR-027-export-compiled-native-package.md): the edition reader and
  the native-compile/1 members.
- [FR-100](FR-100-run-a-named-function-through-the-spine.md): the
  native-run/1 members of a `1-draft` program.
