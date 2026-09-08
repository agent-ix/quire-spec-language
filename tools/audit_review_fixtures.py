#!/usr/bin/env python3
# SPDX-License-Identifier: AGPL-3.0-only
"""Producer review check only; not a model binder or production wire reader.

Pass the specification fixture directory, or --self-test for standalone negative
controls. This tool does not interpret predicates or claimed expected outcomes.
"""
import copy
import hashlib
import json
from pathlib import Path
import sys


def digest(raw):
    return 'sha256:' + hashlib.sha256(raw).hexdigest()


def unique_pairs(pairs):
    result = {}
    for key, value in pairs:
        if key in result:
            raise ValueError('duplicate_key')
        result[key] = value
    return result


def decode(raw):
    return json.loads(raw, object_pairs_hook=unique_pairs)


class Registry:
    def __init__(self):
        self.bindings = {}

    def accept(self, ref, raw):
        if digest(raw) != ref['digest']:
            raise ValueError('digest_mismatch')
        payload = decode(raw)
        if payload['fixtureVersion'] != ref['profile']:
            raise ValueError('profile_mismatch')
        identity = payload.get('identity', payload.get('id', payload.get('invocation')))
        if identity != ref['identity'] or payload['revision'] != ref['revision']:
            raise ValueError('identity_mismatch')
        key = (ref['authority'], ref['identity'], ref['revision']['namespace'], ref['revision']['value'])
        binding = (ref['profile'], ref['digest'])
        if key in self.bindings and self.bindings[key] != binding:
            raise ValueError('identity_content_conflict')
        self.bindings[key] = binding
        return payload


def controls(ref, raw, field, changed):
    registry = Registry()
    registry.accept(ref, raw)
    payload = decode(raw)
    payload[field] = changed
    modified = (json.dumps(payload, indent=2) + '\n').encode()
    assert modified != raw
    expect_rejection(lambda: registry.accept(ref, modified), 'digest_mismatch')
    new_ref = copy.deepcopy(ref)
    new_ref['digest'] = digest(modified)
    expect_rejection(lambda: registry.accept(new_ref, modified), 'identity_content_conflict')


def expect_rejection(action, reason):
    try:
        action()
    except ValueError as error:
        assert str(error) == reason, (reason, str(error))
    else:
        raise AssertionError('mutation accepted: ' + reason)


def self_test():
    for field, original, modified in [('objects', [1], [2]), ('frame', {'allowed': []}, {'allowed': ['x']}), ('result', True, False)]:
        payload = {'identity': 'test:' + field, 'revision': {'namespace': 'test', 'value': '1'}, 'fixtureVersion': 'test/1', field: original}
        raw = json.dumps(payload).encode()
        ref = {'authority': 'test', 'identity': payload['identity'], 'revision': payload['revision'], 'profile': payload['fixtureVersion'], 'digest': digest(raw)}
        controls(ref, raw, field, modified)
    expect_rejection(lambda: decode(b'{"x":1,"x":2}'), 'duplicate_key')
    print('passed: stale bytes and changed-content identity reuse refused for snapshot/frame/invocation; duplicate keys refused')


def audit(root):
    root = root.resolve()
    manifest = decode((root / 'invocation-cases.json').read_bytes())
    assert manifest['fixtureVersion'] == 'agent-a-invocation-cases/1-draft'
    registry = Registry()
    inspected = {}

    def resolve(entry):
        path = (root / entry['path']).resolve()
        assert path.is_relative_to(root), 'foreign fixture path'
        raw = path.read_bytes()
        ref = entry['artifactRef']
        payload = registry.accept(ref, raw)
        inspected[path] = (ref, raw)
        return payload

    for case in manifest['cases']:
        invocation = resolve(case['input'])
        for role in ['pre', 'post']:
            if invocation[role].get('availability') != 'unavailable':
                resolve(invocation[role])
        resolve(invocation['frameBinding'])
    for frame in manifest['frameBindings']:
        resolve(frame)
    exercised = set()
    for ref, raw in inspected.values():
        profile = ref['profile']
        if profile in exercised:
            continue
        if profile == 'agent-a-snapshot/1-draft':
            controls(ref, raw, 'objects', [])
        elif profile == 'agent-a-frame-binding/1-draft':
            controls(ref, raw, 'frame', {'allowedChangedFields': ['changed'], 'allowedCreatedTypes': [], 'allowedDeletedTypes': []})
        elif profile == 'agent-a-invocation/1-draft':
            controls(ref, raw, 'result', not decode(raw)['result'])
        else:
            raise ValueError('unknown_fixture_profile')
        exercised.add(profile)
    assert len(exercised) == 3
    print(f'passed: {len(inspected)} exact artifact files, {len(manifest["cases"])} invocation cases; 6 content/digest negative controls; no evaluator executed')


if __name__ == '__main__':
    if sys.argv[1:] == ['--self-test']:
        self_test()
    elif len(sys.argv) == 2:
        audit(Path(sys.argv[1]))
    else:
        raise SystemExit('usage: audit_review_fixtures.py <fixture-directory>|--self-test')
