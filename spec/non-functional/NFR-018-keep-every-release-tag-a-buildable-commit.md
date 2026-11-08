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

When a release tag `vX.Y.Z` is checked, the `make release-check
TAG=vX.Y.Z` gate shall build a clean clone of the tag's commit with locked
dependencies; and when a release tag is created, the `make tag-release
TAG=vX.Y.Z` target shall create it only after NFR-017's full clean-clone gate
passes at that commit.

## Scope

A tag is what an outside user checks out to get a known version. Cargo
resolves the workspace's git dependencies from the revisions `Cargo.lock`
records, so a tag stays buildable only while those revisions stay reachable
in their repositories. A history rewrite in a dependency repository can make
an old tag unbuildable without any change in QSL. The gate finds that.

- `make release-check TAG=<tag>` resolves the tag to its commit and runs
  NFR-017's clean-clone gate there with steps 1 to 4 and 7: the clone, the
  empty Cargo home, the locked build, the source check and the cleanup. It
  does not run step 5, because the examples a tag's README documents change
  between releases. A tag that does not exist refuses, exit 2, before
  cloning.
- The current release tag is the newest tag `make tag-release` created. The
  tags made before this gate existed (`v0.1.0`, `v0.2.0`) are retired and
  are not checked. Until `make tag-release` has created a tag, AC-1 is run
  at the tag NFR-018-AC-3 creates in its scratch repository.
- `make tag-release TAG=<tag>` runs `make clean-clone-check` at `HEAD`,
  creates the annotated tag at `HEAD` only when it passes, and otherwise
  exits non-zero and creates no tag. Publishing the tag and its GitHub
  Release follows.

## Measurement and Evaluation

| Metric | Target | Threshold | Method |
| --- | --- | --- | --- |
| Failed steps of `make release-check` at the current release tag | 0 | 0 | Test |
| Tags `make tag-release` creates at a commit whose clean-clone gate fails | 0 | 0 | Test |

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| NFR-018-AC-1 | `make release-check` at the current release tag passes: the clean clone at the tag's commit builds with `--locked` and every package source passes NFR-017's source check. | Test (TC-1399) |
| NFR-018-AC-2 | `make release-check` at a scratch tag on a commit whose `Cargo.lock` records a git revision absent from that dependency's repository fails, naming the dependency and the revision; at a tag name that does not exist it refuses, exit 2, and clones nothing. | Test (TC-1399) |
| NFR-018-AC-3 | In a scratch repository, `make tag-release TAG=v9.9.9` at a commit whose `Cargo.lock` lacks a declared dependency exits non-zero and `git tag -l v9.9.9` is empty; at a commit where the gate passes it creates the annotated tag `v9.9.9` at `HEAD`. | Test (TC-1399) |

## Verification

TC-1399 runs the gate at the current release tag and at the scratch tags.

## Dependencies

- [NFR-017](NFR-017-build-a-clean-clone-with-locked-dependencies-and-run-the-documented-examples.md): the clean-clone gate.

## References

- QSL-12: the Layer 5 outside-user path.
