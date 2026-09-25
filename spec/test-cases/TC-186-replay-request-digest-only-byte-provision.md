---
id: TC-186
title: "The replay request's byte provision is reachable only by digest, never by path, is complete, and stays within the size bound"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-071
    type: verifies
---
# TC-186: The replay request's byte provision is reachable only by digest, never by path, is complete, and stays within the size bound

## Description

Verify that the replay request type has no field, accessor or constructor
argument typed as a filesystem path, environment variable name, or search
location, so every recompilation input is reachable only by digest lookup
in the byte provision; that a byte-provision entry naming a digest domain
outside the closed FR-201 set refuses at decode; that a byte-provision
entry's bytes must themselves hash to their own declared digest; that the
byte provision must be complete for every digest the package reference
names; and that an oversized encoding refuses rather than decoding
partially. This is the type-level guarantee #243's executor depends on
(QSpec FR-323-AC-5: "the executor reads no recompilation input by path,
environment variable or search location"). A wrong implementation this
test would catch: a request type that carries an optional
`source_root: Option<PathBuf>` "fallback" field for local development,
which would let an executor silently resolve an input from the filesystem
instead of refusing when the byte provision is incomplete; or a
constructor that admits a request whose package reference names a digest
with no matching byte-provision entry, silently deferring the gap to
whatever consumes the request later (#243's executor) instead of refusing
at construction. Scope: FR-071-AC-2, FR-071-AC-5, FR-071-AC-6, FR-071-AC-7,
FR-071-AC-9.

## Test Procedure

1. Inspect the replay request type's full public field/accessor surface
   (source, generated docs, or a reflection-based listing) for any member
   typed as a path, `OsString` treated as a path, or environment-variable
   name.
2. Attempt to construct a request whose byte provision entry declares a
   digest domain outside the closed FR-201 domain set.
3. Attempt to construct a request whose byte provision entry's digest does
   not match the RFC 8785/JCS bytes it is claimed to address (a
   digest/bytes mismatch) — this is this requirement's own decode-time
   integrity check (FR-071-AC-6), distinct from #243's execution-time
   recompiled-`package_id` check.
4. Construct a well-formed request and confirm every recompilation input
   named by the `package` reference's `RawSourceRef` digests has a
   corresponding byte-provision entry keyed by that exact digest.
5. Attempt to construct a request whose package reference names a
   `RawSourceRef` digest with no matching byte-provision entry at all (omit
   one entry that step 4's well-formed request included).
6. Attempt to construct a request whose encoded size exceeds the reader's
   configured bound (pad the byte provision with additional well-formed
   entries until the bound is crossed), and inspect whatever value the
   constructor returns.

7. Construct a request whose package reference carries two `dependencies`
   entries and round-trip it; omit one entry's source from the byte
   provision; decode an entry with an empty identity, one with an empty version, and one whose
   `package_id` is in the `quire.source.bytes/v1` domain.

## Expected Results

- Step 1 finds no path-, environment-variable-, or search-location-typed
  member anywhere on the request type.
- Steps 2 and 3 each refuse construction with a structured, typed cause
  (`stale_dependency`/`digest-domain-mismatch` and
  `stale_dependency`/`byte-digest-mismatch` respectively).
- Step 4's lookup succeeds by digest alone, with no path involved at any
  point.
- Step 5 refuses construction; no request is returned that carries an
  incomplete byte provision for a later consumer to discover.
- Step 6 refuses with a bound-exceeded cause; the constructor returns no
  request value at all, in particular no request holding a truncated
  prefix of the padded entries.
- Step 7: the round trip preserves both entries in order; the omitted
  source refuses construction as step 5 does; the empty identity, the empty
  version and the source-domain `package_id` each refuse at decode.
