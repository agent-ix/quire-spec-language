---
id: NFR-018
org: agent-ix
title: "Keep every release tag a buildable commit"
type: NFR
quality_attribute: reliability
relationships:
  - target: ix://agent-ix/quire-spec-language/US-037
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/NFR-017
    type: depends_on
---
# NFR-018: Keep every release tag a buildable commit

## Statement

When a release tag `vX.Y.Z` is created or checked, the `make release-check
TAG=vX.Y.Z` gate shall run NFR-017's clean-clone gate at the tag's commit,
and the repository shall create a release tag only at a commit where that
gate passed.

## Scope

A tag is what an outside user checks out to get a known version. Cargo
resolves the workspace's git dependencies from the revisions `Cargo.lock`
records, so a tag stays buildable only while those revisions stay reachable
in their repositories. A history rewrite in a dependency repository can make
an old tag unbuildable without any change in QSL. The gate finds that.

- `make release-check TAG=<tag>` resolves the tag to its commit and runs
  `make clean-clone-check REV=<commit>`. A tag that does not exist refuses,
  exit 2, before cloning.
- The release procedure runs `make clean-clone-check` at the commit to be
  tagged, creates the tag only when it passes, and then publishes the tag
  with its GitHub Release.

## Measurement and Evaluation

| Metric | Target | Threshold | Method |
| --- | --- | --- | --- |
| Failed steps of `make release-check` at the newest release tag | 0 | 0 | Test |

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| NFR-018-AC-1 | `make release-check` at the repository's newest `v*` tag passes: the clean clone at the tag's commit builds with `--locked` and runs the README examples as NFR-017-AC-2 states for that commit's README. | Test (TC-1399) |
| NFR-018-AC-2 | `make release-check` at a scratch tag on a commit whose `Cargo.lock` records a git revision absent from that dependency's repository fails, naming the dependency and the revision; at a tag name that does not exist it refuses, exit 2, and clones nothing. | Test (TC-1399) |

## Verification

TC-1399 runs the gate at the newest tag and at the scratch tags.

## Dependencies

- [NFR-017](NFR-017-build-a-clean-clone-with-locked-dependencies-and-run-the-documented-examples.md): the clean-clone gate.

## References

- QSL-12: the Layer 5 outside-user path.
