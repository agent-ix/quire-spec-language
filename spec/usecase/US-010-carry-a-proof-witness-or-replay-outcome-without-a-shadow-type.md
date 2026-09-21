---
id: US-010
title: "Carry a proof result, witness or replay outcome without a shadow type"
type: US
relationships:
  - target: ix://agent-ix/quire-spec-language/StR-001
    type: traces_to
---
# US-010: Carry a proof result, witness or replay outcome without a shadow type

## Story

**As a** QSL contributor building the function-application Kani/replay
exemplar (#217) or widening the replay facade for a new semantic family
(#186's state `forall`)
**I want** one typed proof-result, counterexample/witness and replay
request/result envelope that every family and every backend consumer reuses
unchanged
**So that** I never have to invent my own ad hoc struct to carry a proof
verdict or a witness across the QSL/backend boundary, and a caller can never
mistake a partially-decoded or re-keyed envelope for a trustworthy one.

## Context

ADR-013 O-24 to O-27 and ADR-011 §2.1 E8/E9 decide that a proof result, a
counterexample/witness and a replay request/result each has exactly one
canonical QSL-owned type, living in the layer-6 `replay` module's public
API, and that this is the *only* surface the CG replay adapter reaches
(ADR-011 FB-05). Before this ticket, no such type exists: AD-016 arrow 7
still names a bare `&str` function-selection parameter, the AD-016
Replay-ownership row still lists five separately-stored `Witness` fields
that this record's amendment (QC-13) collapses to one stored field plus four
derived ones, and #217's Kani exemplar and #186's state-`forall` payload
both have nowhere to plug in without either inventing their own witness type
or reaching into QSL internals CG is not allowed to touch.

This ticket (#231) builds the four envelope types and their round trips. It
does not invoke a backend, does not implement the Kani harness, and does not
execute a replay: `CheckedPackage::call` and the layer-6 `replay` facade's
recompile-and-select behavior belong to #243 (TK-01) and #217. The typed
replay *request* this story asks for is the input #243's executor consumes;
building the executor itself is explicitly out of scope here.

## Acceptance Examples (Illustrative)

### US-010-EX-1: A proof result never loses its category

- **Given** an FR-331 terminal record for a Kani run that ended
  `Counterexample`.
- **When** the record is read into the proof-result envelope.
- **Then** the envelope reports the `violation` category, never `success`,
  `unsupported` or any other category, and a `tested` backend result is
  never silently promoted to `proved`.

### US-010-EX-2: A witness round-trips without becoming a string

- **Given** a witness whose transcript names a concrete counterexample.
- **When** the witness envelope is serialized and read back.
- **Then** every derived fact (`harness_symbol`, `concrete_values`, `decode`)
  is recomputed from the same stored transcript, not carried as a second,
  possibly-disagreeing copy, and nothing about the round trip depends on
  parsing rendered text.

### US-010-EX-3: A stale package fails closed, not open

"Stale package identity" splits into two checks, each owned by a different
ticket, and this ticket owns only the first:

- **Given** a replay request whose byte-provision entry's stored bytes do
  not hash to that entry's own declared digest.
- **When** this ticket's own decoder constructs the request.
- **Then** construction refuses with a structured
  `stale_dependency`/`byte-digest-mismatch` cause, before any recompilation
  is attempted, and never returns a partial or best-effort request
  (FR-071-AC-6).

- **Given** a replay request naming a `package_id` that no longer matches
  the source the byte provision recompiles to.
- **When** #243's executor (a later ticket) recompiles that source and
  compares the result's `package_id` against the request's declared one.
- **Then** the executor refuses with a structured
  `DependencyIdentityMismatch`/`stale_dependency` cause; this ticket builds
  the request type the executor reads to do that comparison, but performs
  no recompilation and makes no `package_id` comparison itself.

### US-010-EX-4: A new family plugs in without touching the shared envelope

- **Given** #186's state-`forall` payload, which needs to carry a
  family-specific witness detail no other family has.
- **When** #186 adds that payload.
- **Then** it does so as a typed consumer of the common witness envelope's
  extension point, and the common envelope's own type, constructors and
  round-trip contract are unchanged by #186's change.

## Priority and Risk (Informative)

Priority: High — this is the widest unblock currently available on the
#205 spine (ARCH-G2 gate #216, the replay facade #243, and the Kani
exemplar #217 all wait on it). Risk if unmet: each downstream ticket invents
its own witness/replay shape, producing exactly the "two authoritative
producer paths" outcome ADR-011's §4 failure rule forbids, and a witness or
replay result becomes trustworthy only by convention rather than by the type
system.

## Traceability (Informative)

- [FR-069](../functional/FR-069-implement-typed-proof-result-envelope.md) —
  typed proof-result envelope.
- [FR-070](../functional/FR-070-implement-typed-counterexample-witness-envelope.md) —
  typed counterexample/witness envelope.
- [FR-071](../functional/FR-071-implement-typed-replay-request.md) — typed
  replay request.
- [FR-072](../functional/FR-072-implement-typed-replay-result.md) — typed
  replay result and record carrier.
