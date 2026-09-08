#!/usr/bin/env python3
# SPDX-License-Identifier: AGPL-3.0-only
"""Producer byte/region checks for the review packet, not a production matcher."""
import hashlib
import json
from pathlib import Path
import sys


def digest(raw):
    return 'sha256:' + hashlib.sha256(raw).hexdigest()


def audit(root):
    root = root.resolve()
    packet = json.loads((root / 'role-compositions.json').read_bytes())
    assert packet['fixtureVersion'] == 'agent-a-role-compositions/1-draft'
    refs = packet['artifactRefs']
    raws = {}
    for key, ref in refs.items():
        path = (root / packet['artifactLocators'][key]).resolve()
        assert path.is_relative_to(root)
        raw = path.read_bytes()
        assert ref['refVersion'] == 'ix.artifact-ref/2-draft'
        assert digest(raw) == ref['digest'], key
        assert digest(raw + b' ') != ref['digest'], key
        raws[key] = raw
    for key, source_key in [('property-current', 'current-source'), ('property-post', 'operation-source')]:
        payload = json.loads(raws[key])
        source = payload['source']
        assert source['artifact'] == refs[source_key]
        assert source['irRevisionMapping']['revision'] > 0
        raw = raws[source_key]
        for span_key, digest_key in [('clauseSpan', 'clauseDigest'), ('expressionSpan', 'expressionDigest')]:
            span = source[span_key]
            assert 0 <= span['startByte'] < span['endByte'] <= len(raw)
            assert digest(raw[span['startByte']:span['endByte']]) == source[digest_key]
            for boundary in ['start', 'end']:
                prefix = raw[:span[boundary + 'Byte']].decode('utf-8')
                assert span[boundary + 'Line'] == prefix.count('\n') + 1
                assert span[boundary + 'Column'] == len(prefix.rsplit('\n', 1)[-1]) + 1
        semantic = packet['semanticRefs'][key]
        assert semantic['artifact'] == refs[key]
        assert semantic['requiredFeatures'] == sorted(set(semantic['requiredFeatures']))
        assert semantic['semanticProfile']['definitionDigest'] == digest((root.parent / 'profile.md').read_bytes())
    ir, lock, manifest = (json.loads(raws[key]) for key in ['model', 'lock', 'manifest'])
    assert ir['contractVersion'] == '1.1.0'
    assert ir['package']['identity'] == manifest['package']['identity'] == lock['rootPackage']
    assert ir['package']['version'] == manifest['package']['version']
    # These observed producer fields bind exact supplied manifest/lock bytes.
    assert ir['package']['manifestDigest'] == digest(raws['manifest'])
    assert ir['package']['lockDigest'] == digest(raws['lock'])
    closure = json.loads(raws['closure'])
    assert closure['establishedFingerprint'] == lock['fingerprint']
    assert closure['establishedCanonicalization'] == lock['canonicalization']
    assert closure['packages'] == lock['packages']
    assert closure['nativeQualification']['state'] == 'unavailable'
    output = json.loads(raws['native-output'])
    assert output['status'] == 'parsed' and output['source']['digest'] == digest(raws['native-source'])
    assert output['source']['identity'] == refs['native-source']['identity']
    assert refs['native-source']['revision']['namespace'] == 'quire-spec.cli-source-label'
    assert output['source']['revision'] == refs['native-source']['revision']['value']
    run = json.loads(raws['run'])
    assert run['runIdentity'] == refs['run']['identity']
    assert run['observedExitCode'] == 0 and run['logicalOutcome'] == 'not-evaluated'
    assert run['source'] == refs['native-source']
    assert run['nativeArtifact'] == refs['native-output']
    assert run['scopeExample'] == refs['environment']
    assert run['arguments'][1:3] == [output['source']['identity'], output['source']['revision']]
    for case in packet['cases']:
        for role, selected in case['roles'].items():
            assert selected in list(refs.values()) + list(packet['semanticRefs'].values()), (case['id'], role)
    print(f'passed: {len(refs)} exact artifacts and changed-byte controls; four source regions/coordinates; model wrapper consistency; actual syntax output; no semantic matcher or evaluator executed')


if __name__ == '__main__':
    if len(sys.argv) != 2:
        raise SystemExit('usage: audit_role_compositions.py <specification-fixture-directory>')
    audit(Path(sys.argv[1]))
