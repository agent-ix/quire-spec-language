---
id: TC-142
title: "Publish and read control temporal activation mappings"
type: TC
relationships:
  - { target: ix://agent-ix/quire-spec-language/FR-054, type: verifies }
---
# TC-142: Publish and read control temporal activation mappings

## Description

Verify canonical compiled-protocol `/3` mappings between eligible linked controls
and their exact selected temporal declarations without changing `/1` or `/2`.

## Test Procedure

Produce a `/3` artifact with independently selected authored mappings, then
strictly read it against an independent expected population. Mutate each mapping
axis by removal, duplication, reordering, cross-control substitution,
cross-temporal substitution, foreign/out-of-range handle and an ineligible
control. Offer `/1` and `/2` documents to the `/3` reader and `/3` to each prior
reader. Attempt to substitute matching profile/clock, names, source positions,
timestamps and observations for one expected mapping.

## Expected Results

Only the exact canonical `/3` document admits. Every independent mutation and
inference attempt returns its typed refusal without a partial view; all prior
versioned bytes and readers remain unchanged and cross-version reads refuse.
