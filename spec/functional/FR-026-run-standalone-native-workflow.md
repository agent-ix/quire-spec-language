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
  - target: ix://agent-ix/quire-spec-language/FR-285
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-287
    type: references
---
## Description

When a caller hands QSL's library operation `command` (FR-287) a native-run/1
request file or a native-compile/1 request file, `command` shall read the
request and each file it selects under this requirement's bounded intake.

`command` routes the program by the edition its header declares, with
FR-027's edition reader. Only a `1-draft` program reaches a runner or a
compiler: `run` calls a named function or runs a clause through the spine
([FR-100](FR-100-run-a-named-function-through-the-spine.md),
[FR-312](FR-312-read-a-clause-run-request-and-run-it.md)), and `compile`
returns `quire.checked-package/v2` bytes
([FR-027](FR-027-export-compiled-native-package.md)). Every other declared
edition, `0-draft` included, refuses `unknown_edition` (FR-027, FR-100).
This requirement owns what the two requests share: the envelope, the
source selections and their labels, path resolution, the intake limits and
the intake refusal envelope. FR-100, FR-312 and FR-027 own
each request's members, outcome and exit statuses. The driver owns the
request-file operand, the option spelling that sets each limit, usage and
arity errors, and writing to the process streams (FR-287).

The intake limits are the two byte limits `request_bytes` and
`dependent_bytes` (Behavior). No intake limit counts files.

## Inputs

A JSON object with format and request fields. Each source selection names
its file, the two source labels of [FR-001](FR-001-read-exact-source.md)
(`authority` and `identity`) and its formal
document/revision. Both labels are required members. A request whose source
identity lacks one, such as a request written with only `identity`, refuses
at the request stage with the native code `invalid-request`, exit code 20;
the catalog keeps that code as a retained host code with its broad meaning,
malformed command (`quire.native.diagnostics/v1`, "Other retained
host/source codes").
Relative paths resolve against the request's directory; absolute paths remain
valid local file operands. Source display paths retain the authored file string.
Parent-relative components are also admitted local operands; the command does
not establish a tenant filesystem boundary.

## Outputs

`command` returns a typed outcome and writes nothing to the process streams
(FR-287); the driver renders it and exits with its FR-285 exit code. An
intake failure's outcome carries no result document and no package bytes. It
carries a command-error envelope in format `native-run-result/2` (FR-267)
with the typed stage and code and the relevant
original source details. On FR-301's six-code contract, the outcome's exit
code is 20 for malformed requests and I/O failures. Writing the outcome to a
stream, and a stream that fails or closes, are the driver's (FR-287,
Overlap). Command error spellings belong to the native code catalog. A
resource cause names the byte limit reached, `request_bytes` or
`dependent_bytes`.

## Behavior

The command shall decode the envelope and select its format before request fields.
The command shall admit, at decode, the members FR-100 and FR-027 define for a
`1-draft` program.
The command shall reject unknown/duplicate fields and positional record arrays.
The command shall open each file once and cap reads before parsing or hashing.
The command shall read the request under the caller's `request_bytes` limit
(default 1 MiB) and the dependent files it names under the caller's
`dependent_bytes` limit on their total size (default 8 MiB). The caller sets
both in the limits it gives `command`; neither has a ceiling. A refusal at
either names its setting (`request_bytes` or `dependent_bytes`) and its
configured value.
The command shall count every dependent file toward `dependent_bytes` by its
size, whatever its role (model, each program unit, library, snapshot, invocation or
package), and shall apply no limit on the number of files.
The command shall begin each invocation with fresh budgets.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-026-AC-3 | Through `command` in process, a `1-draft` run request with a malformed request, an unknown format and an unavailable file each refuse at their actual stage with their catalogued code and the available source details, with no result document and exit code 20; each command-error envelope conforms to the native result schema. | Test (TC-104) |
| FR-026-AC-4 | Through `command` in process, a `1-draft` run request one byte over `request_bytes`, and dependent files one byte over `dependent_bytes`, each refuse naming the setting and its configured value; the same requests with each limit raised to their size run; a fresh default request then succeeds. | Test (TC-104) |
| FR-026-AC-5 | `command` given one `1-draft` request file by its path from two different process working directories returns equal outcomes, the request's relative, absolute and parent-relative file paths resolving against the request's directory and as written; no call writes a byte to stdout or stderr. | Test (TC-104) |
| FR-026-AC-6 | A `1-draft` run request whose program source names `authority` `agent-ix` and `identity` `p` runs, and its `spine-run-result/1` `source` renders those two labels and no other identity member. The same request without the program source's `authority`, and again without its `identity`, refuses at the request stage with `invalid-request`, exit code 20. A present but blank `authority` refuses with `invalid_source_identity`, cause `blank-label`, field `label` `authority` (FR-001). | Test (TC-430) |

## Dependencies

- [FR-001](FR-001-read-exact-source.md): the two source labels.
- [FR-027](FR-027-export-compiled-native-package.md): the edition reader and
  the native-compile/1 members.
- [FR-100](FR-100-run-a-named-function-through-the-spine.md): the
  native-run/1 members of a `1-draft` program.
- [FR-267](FR-267-write-run-results-as-native-run-result-2.md): the
  `native-run-result/2` command-error envelope.
- [FR-287](FR-287-reach-qsl-through-the-driver-cli.md): the driver, which
  owns operands, option spellings, usage errors and stream writes.
