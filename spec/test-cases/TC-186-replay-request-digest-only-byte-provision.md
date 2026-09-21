---
id: TC-186
title: "The replay request's byte provision is reachable only by digest, never by path"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-071
    type: verifies
---
# TC-186: The replay request's byte provision is reachable only by digest, never by path

## Description

Verify that the replay request type has no field, accessor or constructor
argument typed as a filesystem path, environment variable name, or search
location, so every recompilation input is reachable only by digest lookup
in the byte provision, and that a byte-provision entry naming a digest
domain outside the closed FR-201 set refuses at decode. This is the type-level
guarantee #243's executor depends on (QSpec FR-323-AC-5: "the executor
reads no recompilation input by path, environment variable or search
location"). A wrong implementation this test would catch: a request type
that carries an optional `source_root: Option<PathBuf>` "fallback" field
for local development, which would let an executor silently resolve an
input from the filesystem instead of refusing when the byte provision is
incomplete. Scope: FR-071-AC-2.

## Test Procedure

1. Inspect the replay request type's full public field/accessor surface
   (source, generated docs, or a reflection-based listing) for any member
   typed as a path, `OsString` treated as a path, or environment-variable
   name.
2. Attempt to construct a request whose byte provision entry declares a
   digest domain outside the closed FR-201 domain set.
3. Attempt to construct a request whose byte provision entry's digest does
   not match the RFC 8785/JCS bytes it is claimed to address (a
   digest/bytes mismatch).
4. Construct a well-formed request and confirm every recompilation input
   named by the `package` reference's `RawSourceRef` digests has a
   corresponding byte-provision entry keyed by that exact digest.

## Expected Results

- Step 1 finds no path-, environment-variable-, or search-location-typed
  member anywhere on the request type.
- Steps 2 and 3 each refuse construction with a structured, typed cause
  (`stale_dependency`/`digest-domain-mismatch` and
  `stale_dependency`/`byte-digest-mismatch` respectively).
- Step 4's lookup succeeds by digest alone, with no path involved at any
  point.
