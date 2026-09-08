#!/usr/bin/env python3
# SPDX-License-Identifier: AGPL-3.0-only
"""Check draft FS03 expression syntax only, without interpreting type hypotheses."""
import hashlib
import json
from pathlib import Path
import subprocess
import sys
import tempfile


def check(spec):
    fixture = json.loads((spec / 'fixtures/typing-cases.json').read_bytes())
    assert fixture['fixtureVersion'] == 'agent-a-typing-cases/1-draft'
    assert fixture['modelBinding']['state'] == 'unavailable'
    for path, expected in [
        ('state-semantics.md', fixture['ruleContract']['digest']),
        ('profile.md', fixture['baseProfile']['definitionDigest']),
    ]:
        assert 'sha256:' + hashlib.sha256((spec / path).read_bytes()).hexdigest() == expected
    compiler = Path(__file__).resolve().parents[1] / 'target/debug/quire-spec'
    if not compiler.is_file():
        raise SystemExit('build the local CLI first: cargo build --locked --target-dir target')
    ids = [case['id'] for case in fixture['cases']]
    assert len(ids) == len(set(ids))
    parsed = unsupported = 0
    with tempfile.TemporaryDirectory(prefix='fs03-syntax-') as directory:
        for index, case in enumerate(fixture['cases']):
            assert case['anchor'] in ('post', 'current')
            anchor = 'post Case on M::Node::step' if case['anchor'] == 'post' else 'invariant Case on M::Node at current'
            source = (
                'language "ix:native" edition "0-draft";\n'
                'profile "state-finite/0-draft";\n'
                'model M = "example/rule-tests" version "0.0.0-fixture" digest "unresolved-model-package";\n'
                + anchor + ' { ' + case['expression'] + ' }\n'
            )
            path = Path(directory) / f'{index}.native'
            path.write_text(source, encoding='utf-8')
            result = subprocess.run(
                [str(compiler), 'parse', 'fs03:' + case['id'], 'fixture:1', str(path)],
                capture_output=True, text=True, check=False,
            )
            expected = case['expected']['syntax']
            if expected == 'parsed':
                assert result.returncode == 0, (case['id'], result.stderr)
                assert json.loads(result.stdout)['status'] == 'parsed'
                parsed += 1
            else:
                assert expected == 'unsupported'
                assert result.returncode == 1 and 'unsupported_construct' in result.stderr, (case['id'], result.stderr)
                unsupported += 1
    print(f'passed syntax only: {parsed} parsed expressions; {unsupported} unsupported refusal; case IDs and rule/profile digests checked; no typechecker or evaluator executed')


if __name__ == '__main__':
    if len(sys.argv) != 2:
        raise SystemExit('usage: check_rule_syntax.py <specification/proposals/state-core>')
    check(Path(sys.argv[1]).resolve())
