---
id: US-027
title: "Compile, check, run and replay deeply nested specifications under limits I can raise"
type: US
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-255
    type: exercises
  - target: ix://agent-ix/quire-spec-language/FR-256
    type: exercises
  - target: ix://agent-ix/quire-spec-language/FR-257
    type: exercises
  - target: ix://agent-ix/quire-spec-language/FR-258
    type: exercises
  - target: ix://agent-ix/quire-spec-language/FR-259
    type: exercises
  - target: ix://agent-ix/quire-spec-language/FR-260
    type: exercises
  - target: ix://agent-ix/quire-spec-language/FR-261
    type: exercises
  - target: ix://agent-ix/quire-spec-language/FR-262
    type: exercises
  - target: ix://agent-ix/quire-spec-language/FR-263
    type: exercises
  - target: ix://agent-ix/quire-spec-language/FR-264
    type: exercises
  - target: ix://agent-ix/quire-spec-language/FR-356
    type: exercises
  - target: ix://agent-ix/quire-spec-language/ADR-030
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/StR-001
    type: traces_to
---
# US-027: Compile, check, run and replay deeply nested specifications under limits I can raise

## Story

**As a** QSL caller (a specification author using the driver CLI `quire`, a
library caller, or a proof pipeline replaying a proved package)
**I want** expressions, types, values and JSON documents of any nesting depth
to compile, check, emit, evaluate and replay, bounded only by resource limits
that measure what the input costs: bytes, nodes and work
**So that** a long sum, a long `else if` chain, a long run of nested `let`s or
a long recursive list value works whenever it fits my limits, and when it
does not fit, the outcome tells me which limit it reached, its value, and the
setting that raises it, so I can run it again.

## Context

QSL expressions are binary trees, so a chain of `n` terms is about `n` deep.
ADR-030 decides that depth is never a limit in QSL: every walk runs over an
explicit heap stack or over a representation whose depth its schema fixes,
and every stage is bounded by caller-configurable resource limits with
published defaults. The defaults are sized for realistic specifications. A
deeper input states the limits it needs.

## Acceptance Examples (Illustrative)

### US-027-EX-1: A long chain within the defaults compiles

- **Given** a function whose body is a sum of 1,000 terms, which fits the
  default limits.
- **When** the caller compiles and runs it.
- **Then** it checks, emits and evaluates, and no outcome mentions depth.

### US-027-EX-2: A deeper chain names the limit to raise

- **Given** a function whose body is a 100,000-term `else if` chain.
- **When** the caller compiles it at the default limits.
- **Then** the compile stops with `stage_limit_exceeded`, naming the limit
  it reached, its configured value, the count it would have reached, and the
  setting, for example `s1.tokens`.
- **When** the caller raises that setting with the driver CLI's `--limit s1.tokens=<n>`, and
  any further limit the next outcome names, the same way.
- **Then** the compile succeeds, on a thread whose stack is far smaller than
  one frame per level would need.

### US-027-EX-3: Replay carries the limits it needs

- **Given** a proving run that compiled a 100,000-deep expression under
  raised limits and found a counterexample whose value is a 100,000-long
  recursive list.
- **When** the pipeline replays it with those limits in the request's
  `stage_limits`.
- **Then** the replay reaches the proving run's verdict.

### US-027-EX-4: A deep document is judged on its content

- **Given** a semantic-IR package document or an observation document nested
  100,000 deep, within its byte limit.
- **When** QSL admits it.
- **Then** it is admitted or refused on its content, and never because of
  its depth.

## Priority and Risk (Informative)

Priority: High. The owner ruled that deep nesting must be supported. Today a
129-term sum is refused by fixed depth constants long before any size limit.

## Traceability (Informative)

- [FR-255](../functional/FR-255-name-the-setting-that-raises-a-reached-limit.md)
- [FR-256](../functional/FR-256-parse-source-at-any-nesting-depth.md)
- [FR-257](../functional/FR-257-build-forms-at-any-depth.md)
- [FR-258](../functional/FR-258-check-and-lower-expressions-at-any-depth.md)
- [FR-259](../functional/FR-259-encode-identities-and-read-json-through-quire-canonical-at-any-depth.md)
- [FR-260](../functional/FR-260-admit-semantic-ir-documents-at-any-depth.md)
- [FR-261](../functional/FR-261-read-other-untrusted-json-at-any-depth.md)
- [FR-262](../functional/FR-262-evaluate-values-and-calls-at-any-depth.md)
- [FR-263](../functional/FR-263-replay-at-any-depth-under-the-request-limits.md)
- [FR-264](../functional/FR-264-emit-and-read-v2-packages-at-schema-fixed-depth.md)
- [FR-356](../functional/FR-356-walk-nested-structures-through-one-iterative-walker-toolkit.md)
- [ADR-030](../decisions/ADR-030-arbitrary-nesting-depth-no-fixed-caps.md)
