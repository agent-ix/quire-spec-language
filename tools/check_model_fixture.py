#!/usr/bin/env python3
# SPDX-License-Identifier: AGPL-3.0-only
"""Reproduce the synthetic model with its existing, explicitly selected producer.

This checks package production/lock behavior, not typed-model/native binding.
No installation, network access, existing-repository edit or publication occurs.
"""
import hashlib
import json
from pathlib import Path
import subprocess
import sys
import tempfile

PIN = '3b75e01c652ba00bb07c352ff5467419401e792b'


def check_bytes(root):
    fixtures = root / 'tests/fixtures'
    provenance = json.loads((fixtures / 'model-output/provenance.json').read_bytes())
    assert provenance['fixtureVersion'] == 'agent-a-model-production/1-draft'
    assert provenance['producer']['revision'] == PIN
    for artifact in provenance['artifacts']:
        path = (fixtures / artifact['path']).resolve()
        assert path.is_relative_to(fixtures.resolve())
        actual = 'sha256:' + hashlib.sha256(path.read_bytes()).hexdigest()
        assert actual == artifact['digest'], artifact['path']
    print(f"passed: {len(provenance['artifacts'])} producer checkpoint byte digests; compiler not executed by this check")


def check(filament):
    root = Path(__file__).resolve().parents[1]
    check_bytes(root)
    actual = subprocess.check_output(['git', '-C', str(filament), 'rev-parse', 'HEAD'], text=True).strip()
    if actual != PIN:
        raise SystemExit(f'producer revision mismatch: expected {PIN}, got {actual}')
    subprocess.run(['git', '-C', str(filament), 'diff', '--quiet', PIN, '--'], check=True)
    compiler = filament / 'src/compiler/cli.mjs'
    source = root / 'tests/fixtures/model-source'
    expected = root / 'tests/fixtures/model-output'
    typespec = json.loads((filament / 'node_modules/@typespec/compiler/package.json').read_text())
    assert typespec['version'] == '1.15.0' and typespec['license'] == 'MIT'
    with tempfile.TemporaryDirectory(prefix='native-model-check-') as directory:
        temporary = Path(directory)
        base = ['node', str(compiler), 'compile', '--package', str(source), '--entrypoint', 'types/main.tsp']
        for attempt in ['fresh', 'locked']:
            output = temporary / (attempt + '.json')
            diagnostics = temporary / (attempt + '-diagnostics.json')
            arguments = base + ['--out', str(output), '--diagnostics', str(diagnostics)]
            if attempt == 'fresh':
                arguments += ['--write-lock', str(temporary / 'package-lock.json')]
            else:
                arguments += ['--lock', str(expected / 'package-lock.json')]
            subprocess.run(arguments, check=True)
            assert output.read_bytes() == (expected / 'semantic-ir.json').read_bytes()
            assert diagnostics.read_bytes() == (expected / 'diagnostics.json').read_bytes()
            assert json.loads(diagnostics.read_bytes()) == []
        assert (temporary / 'package-lock.json').read_bytes() == (expected / 'package-lock.json').read_bytes()
        stale = json.loads((expected / 'package-lock.json').read_bytes())
        stale['fingerprint'] = 'sha256:' + '0' * 64
        stale_path = temporary / 'stale-lock.json'
        stale_path.write_text(json.dumps(stale))
        stale_before = stale_path.read_bytes()
        bad_output = temporary / 'refused.json'
        bad_diagnostics = temporary / 'refused-diagnostics.json'
        refused = subprocess.run(base + ['--out', str(bad_output), '--diagnostics', str(bad_diagnostics), '--lock', str(stale_path)])
        assert refused.returncode == 1
        assert not bad_output.exists()
        assert stale_path.read_bytes() == stale_before
        codes = {entry['code'] for entry in json.loads(bad_diagnostics.read_bytes())}
        assert 'agent-ix.compiler.STALE_LOCK' in codes
    print('passed: actual IR/lock bytes reproduced fresh and with selected lock; stale lock refused without output or repair; no native binding/evaluation executed')


if __name__ == '__main__':
    if len(sys.argv) != 2:
        raise SystemExit('usage: check_model_fixture.py --bytes-only | <Filament-checkout-at-pinned-revision>')
    if sys.argv[1] == '--bytes-only':
        check_bytes(Path(__file__).resolve().parents[1])
    else:
        check(Path(sys.argv[1]).resolve())
