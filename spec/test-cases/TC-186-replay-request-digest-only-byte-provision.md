---
id: TC-186
title: "The replay request's byte provision is reachable only by label or identity, never by path, is complete, and stays within the size bound"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-071
    type: verifies
---
# TC-186: The replay request's byte provision is reachable only by label or identity, never by path, is complete, and stays within the size bound

## Description

Verify that the replay request type has no field, accessor or constructor
argument typed as a filesystem path, environment variable name, or search
location, so every recompilation input is reachable only by lookup (source
labels, package identity) in the byte provision; that an entry keyed by a
digest domain outside the closed FR-201 set refuses at decode; that two
entries under one key refuse; that the
byte provision must be complete for every source the package reference
names; and that an oversized encoding refuses rather than decoding
partially. This is the type-level guarantee #243's executor depends on
(QSpec FR-323-AC-5: "the executor reads no recompilation input by path,
environment variable or search location"). A wrong implementation this
test would catch: a request type that carries an optional
`source_root: Option<PathBuf>` "fallback" field for local development,
which would let an executor silently resolve an input from the filesystem
instead of refusing when the byte provision is incomplete; or a
constructor that admits a request whose package reference names a source
with no matching byte-provision entry, silently deferring the gap to
whatever consumes the request later (#243's executor) instead of refusing
at construction. Scope: FR-071-AC-2, FR-071-AC-5, FR-071-AC-7,
FR-071-AC-9, FR-071-AC-10, FR-071-AC-13 (AC-6 and AC-11 are retired at decode).

## Test Procedure

1. Inspect the replay request type's full public field/accessor surface
   (source, generated docs, or a reflection-based listing) for any member
   typed as a path, `OsString` treated as a path, or environment-variable
   name.
2. Attempt to construct a request whose byte provision entry is keyed by a
   digest domain outside the closed FR-201 domain set, and one with two
   entries under one source label pair.
3. (Retired with FR-071-AC-6: supply a source whose bytes differ from the
   proving run's; the request constructs, and the edit is refused at
   execution by the `package_id` comparison, TC-444 step 3.)
4. Construct a well-formed request and confirm every recompilation input
   named by the `package` reference's sources has a
   corresponding byte-provision entry keyed by its labels.
5. Attempt to construct a request whose package reference names a
   source (authority, identity) with no matching byte-provision entry at all (omit
   one entry that step 4's well-formed request included).
6. Attempt to construct a request whose encoded size exceeds the reader's
   configured bound (pad the byte provision with additional well-formed
   entries until the bound is crossed), and inspect whatever value the
   constructor returns.

7. Construct a request whose package reference carries two `dependencies`
   entries and round-trip it; omit one entry's source from the byte
   provision; decode an entry with an empty identity and one whose
   `package_id` is in the `quire.source.bytes/v1` domain.
8. Decode a request whose semantic profile selection names
   `quire.profile.unknown/v1`, with a byte-provision entry whose bytes do
   not recompute to its key.
9. (Retired at decode with FR-071-AC-11; the same documents are now read by
   intake at recompile and refuse there, FR-056.) A domain package entry holding a
   repeated member name, then one longer than the request's
   `intake.input_bytes` (4). Map an
   allocation failure of 4096 requested bytes from intake's read of such an
   entry. Decode entries holding `18446744073709551616` at `/package/count`
   and `0.1000000000000000000001` at `/a~1b/0`.
10. Round-trip a request whose package reference and `dependencies` entry carry
    `refinement_internals`, one with the member absent, and ones with the
    values `{}`, `null` and `7`.

## Expected Results

- Step 1 finds no path-, environment-variable-, or search-location-typed
  member anywhere on the request type.
- Step 2 refuses construction (`stale_dependency`/`digest-domain-mismatch` for the domain; `invalid_package`/`conflicting-definition` for the repeat).
  Step 3 is retired.
- Step 4's lookup succeeds by labels alone, with no path involved at any
  point.
- Step 5 refuses construction with `missing_import`/`missing-selection`, naming the requested source labels and where the package reference names it; no request is returned that carries an
  incomplete byte provision for a later consumer to discover.
- Step 6 refuses with a bound-exceeded cause; the constructor returns no
  request value at all, in particular no request holding a truncated
  prefix of the padded entries.
- Step 7: the round trip preserves both entries in order; the omitted
  source refuses construction as step 5 does; the empty identity and the
  source-domain `package_id` each refuse at decode.
- Step 8 refuses `unknown_profile`/`unsupported-selection`, not
  a digest mismatch, keeping the selection
  (`quire.profile.unknown/v1`) and its role
  (`replay.semantic_profile_selections`).
- Step 9 (at recompile, `ReplayRefusal::Recompile`): the repeated-name entry refuses
  `invalid_model_binding`/`malformed-declaration`; the entry over `intake.input_bytes`
  refuses `resource_exhausted`/`intake-limit-exceeded`, naming that limit,
  its bound (4) and its size; the allocation failure refuses
  `resource_exhausted`/`allocation-failed` carrying 4096; the two number
  entries refuse `noncanonical_wire`, `inexact-integer` at
  `/package/count` and `inexact-number` at `/a~1b/0`, each keeping its
  `document_pointer`.
- Step 10: each member round-trips exactly, absent stays absent, and no value is refused at decode.
