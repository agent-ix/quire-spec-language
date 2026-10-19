# SPDX-License-Identifier: AGPL-3.0-or-later
# Copyright (C) 2026 Agent-IX
"""QSL's documentation figures, rendered by the docs-figures skill.

Regenerate from the repository root with the agent-ix/dev-tools skill:

    python3 -m pip install fonttools
    python3 <dev-tools>/skills/docs-figures/scripts/render.py docs/images/figures.py

Add --check to confirm the committed SVGs are current. The first-rule source is
the ConfigVersion example unit and the capability counts are the complete-V1
inventory, both read from this repository.
"""

import re
from pathlib import Path

from docs_figures import FONTS, SMALL, TITLE, Figure  # noqa: F401

STYLE = "ix-docs"

ROOT = Path(__file__).resolve().parents[2]

# One role per kind of thing, fixed across the Quire repositories.
ROLES = [
    ("code", "authored source", "box"),
    ("logic", "compiler stage", "box"),
    ("model", "checked package / IR", "box"),
    ("evidence", "outcome", "box"),
    ("ink", "input", "box"),
]


# ---- repository data ---------------------------------------------------------


def config_version_unit():
    """The ConfigVersion `1-draft` unit, one statement per line, digest elided."""
    source = (ROOT / "examples/config-version/spine_unit.rs").read_text(encoding="utf-8")
    body = source.split("format!(\n", 1)[1].split('",\n    )', 1)[0]
    text = body.replace("\\\n", "").replace('\\"', '"').replace("{{", "{").replace("}}", "}")
    text = text.replace("{PACKAGE_IDENTITY:?}", '"example/config-version"')
    text = text.replace("{PACKAGE_VERSION:?}", '"1.0.0"').replace("{model_digest_hex}", "…")
    lines = []
    for raw in text.split("\\n"):
        raw = " ".join(raw.split()).lstrip('"')
        if not raw or raw.startswith("//") or raw.startswith("function"):
            continue
        if raw.endswith("}") and "{" in raw:
            head, rest = raw.split("{", 1)
            lines += [head.strip() + " {", "  " + rest[:-1].strip(), "}"]
        else:
            lines.append(raw)
    return lines


def capability_families():
    """The complete-V1 capability families and their sizes, in inventory order."""
    source = (ROOT / "qsl-semantics/src/library/bundle.rs").read_text(encoding="utf-8")
    table = source.split("const COMPLETE_CAPABILITY_FAMILIES", 1)[1].split("];", 1)[0]
    return [(name, int(n)) for name, n in re.findall(r'\("([A-Z]+)",\s*(\d+)\)', table)]


KEYWORDS = {
    "language", "edition", "profile", "model", "version", "digest", "invariant",
    "using", "on", "at", "post", "pre", "implies", "not", "present", "deref",
    "value", "reaches",
}


def spans(line):
    """Colour one source line: keywords as stages, strings as authored text."""
    out = []
    for token in re.findall(r'"[^"]*"|\w+|\s+|[^\w\s"]+', line):
        if token.startswith('"'):
            out.append((token, "code"))
        elif token in KEYWORDS:
            out.append((token, "logic"))
        else:
            out.append((token, "ink"))
    return out


# ---- figures -----------------------------------------------------------------


def hero():
    """README hero: one rule, four snapshots, four different answers."""
    cases = [
        ("parent 1, child 2", "holds", "exit 0", "logic"),
        ("parent 3, child 2", "violated", "exit 10", "evidence"),
        ("parent not in data", "refused", "exit 20", "evidence"),
        ("population incomplete", "incomplete", "exit 22", "evidence"),
    ]
    f = Figure(
        "hero",
        980,
        336,
        "A rule written in QSL, either in a .native file or in a fenced block inside a Markdown "
        "spec, is compiled against a domain model into a checked package, then evaluated against "
        "data. The result is one of: holds, violated, refused, unsupported or incomplete.",
    )
    f.box(0, 40, 170, 56, "code", "rule in .native", "ParentOrder")
    f.box(0, 116, 170, 56, "code", "rule in Markdown", "```ix:native fence")
    f.box(0, 210, 170, 56, "ink", "domain model", "config-version")
    f.box(214, 116, 140, 56, "logic", "quire-spec", "parse · check")
    f.box(398, 116, 150, 56, "model", "checked package", "exact identity")
    f.line([(170, 68), (190, 68), (190, 144), (211, 144)], "code")
    f.line([(170, 144), (211, 144)], "code")
    f.line([(170, 238), (190, 238), (190, 160), (211, 160)], "ink")
    f.line([(354, 144), (395, 144)], "logic")
    top, step = 4, 80
    f.line([(548, 144), (572, 144)], "model", arrow=False)
    f.line([(572, top + 28), (572, top + 3 * step + 28)], "model", arrow=False)
    for i, (data, outcome, code, role) in enumerate(cases):
        y = top + i * step
        f.line([(572, y + 28), (597, y + 28)], "model")
        f.box(600, y, 170, 56, "ink", "evaluate on", data)
        f.line([(770, y + 28), (807, y + 28)], role, weight=2.4)
        f.box(810, y, 170, 56, role, outcome, code)
    f.legend(0, 330, [ROLES[0], ROLES[1], ROLES[2], ("evidence", "no truth value", "box"), ROLES[4]])
    return f


def first_rule():
    """An annotated ConfigVersion unit."""
    lines = config_version_unit()
    f = Figure(
        "first-rule",
        980,
        330,
        "An annotated QSL unit: (1) the header names the language and edition, (2) the profile "
        "says which part of the language the rules use, (3) the model line imports a domain "
        "package by exact identity and digest, (4) invariants must hold for every object, "
        "(5) a post rule compares a value after an operation with its value before.",
    )
    first = lambda prefix: next(i for i, s in enumerate(lines) if s.startswith(prefix))  # noqa: E731
    notes = [
        (first("language"), "edition"),
        (first("profile"), "allowed features"),
        (first("model"), "model by digest"),
        (first("invariant"), "every object"),
        (first("post"), "after vs. before"),
    ]
    panel = f.code(
        0,
        4,
        780,
        [spans(s) for s in lines],
        title="config-version.native",
        callouts=[(i, n + 1, "logic") for n, (i, _) in enumerate(notes)],
    )
    for (x, y), (_, note) in zip(panel["anchors"], notes):
        f.line([(x + 6, y), (x + 22, y)], "faint", 1, arrow=False)
        f.note(x + 28, y + 4, note, "ink")
    f.height = 4 + panel["height"] + 4
    return f


def outcomes():
    """Every way a check ends, grouped by whether it says anything about the rule."""
    answers = [("success", "exit 0", "the rule holds"), ("violation · undefined", "exit 10", "the rule is broken")]
    reasons = [
        ("refused", "exit 20", "input rejected"),
        ("unsupported", "exit 21", "profile can't check it"),
        ("incomplete · inconclusive", "exit 22", "data or budget ran out"),
        ("internal failure", "exit 30", "a bug in QSL"),
    ]
    f = Figure(
        "outcomes",
        980,
        344,
        "Every QSL check ends in exactly one outcome with its own exit code: success 0; violation "
        "or undefined 10; refused 20; unsupported 21; incomplete or inconclusive 22; internal "
        "failure 30. Only success and violation are answers about the rule; the others say why "
        "no answer was given.",
    )
    f.box(0, 150, 150, 56, "logic", "a check", "runs")
    f.text(240, 14, "AN ANSWER ABOUT THE RULE · TRUTH VALUE IN THE OUTPUT", "muted", "mono", SMALL)
    f.text(240, 140, "NO ANSWER, AND WHY · NO TRUTH VALUE TO MISREAD", "muted", "mono", SMALL)
    rows = [(answers, 24, "logic"), (reasons, 150, "evidence")]
    for group, top, role in rows:
        for i, (name, code, why) in enumerate(group):
            y = top + i * 48
            f.box(240, y, 230, 40, role, name)
            f.box(500, y, 90, 40, role, code)
            f.text(620, y + 25, why, "muted", "mono", SMALL + 1)
            f.line([(470, y + 20), (497, y + 20)], role, arrow=False)
    f.line([(150, 178), (190, 178)], "logic", arrow=False)
    ys = [24 + 20, 24 + 48 + 20] + [150 + i * 48 + 20 for i in range(4)]
    f.line([(190, ys[0]), (190, ys[-1])], "logic", arrow=False)
    for y in ys:
        f.line([(190, y), (237, y)], "logic")
    return f


def pipeline():
    """The compiler stages, the three ways a checked package is used, and replay."""
    f = Figure(
        "pipeline",
        980,
        426,
        "QSL compiles source through a lossless syntax tree and parsed forms into a checked graph, "
        "resolving names against domain models and dependency packages, then links a checked "
        "package. The package goes three ways: to the reference evaluator, to native model "
        "checking, or as checked-package/v2 bytes to Contract IR and proof backends. A "
        "counterexample from any engine is replayed through the same compiler and evaluator to "
        "confirm it.",
    )
    row = 116
    f.box(0, 10, 140, 50, "code", "Markdown fence", "extracted bytes")
    f.box(0, row, 140, 56, "code", "source", "bytes + identity")
    f.box(180, row, 140, 56, "logic", "syntax tree", "lossless")
    f.box(360, row, 140, 56, "logic", "parsed forms", "unchecked")
    f.box(540, row, 160, 56, "logic", "checked graph", "typed · defined")
    f.box(500, 10, 120, 50, "ink", "domain model", "by digest")
    f.box(640, 10, 140, 50, "ink", "dependencies", "checked packages")
    f.box(780, row, 200, 56, "model", "checked package", "identity + source map")
    f.line([(70, 60), (70, row - 3)], "code")
    for a, b in ((140, 177), (320, 357), (500, 537), (700, 777)):
        f.line([(a, row + 28), (b, row + 28)], "logic")
    f.line([(560, 60), (560, row - 3)], "ink")
    f.line([(690, 60), (690, row - 3)], "ink")

    low = 256
    f.line([(880, row + 56), (880, 214)], "model", arrow=False)
    f.line([(140, 214), (880, 214)], "model", arrow=False)
    f.box(40, low, 200, 56, "logic", "reference evaluation", "five outcomes")
    f.box(300, low, 200, 56, "logic", "model checking", "temporal · protocol")
    f.box(560, low, 200, 56, "model", "Contract IR", "checked-package/v2")
    f.box(800, low, 180, 56, "model", "proof backends", "code · tests · proofs")
    for x in (140, 400, 660):
        f.line([(x, 214), (x, low - 3)], "model")
    f.note(668, 236, "bytes", "model")
    f.line([(760, low + 28), (797, low + 28)], "model")

    f.box(300, 366, 200, 56, "evidence", "replay", "recompiled from source")
    f.line([(400, low + 56), (400, 363)], "evidence")
    f.line([(890, low + 56), (890, 394), (503, 394)], "evidence")
    f.note(700, 386, "counterexample", "evidence")
    f.line([(300, 394), (140, 394), (140, low + 59)], "evidence")
    f.note(150, 386, "re-run here", "evidence")
    return f


def capabilities():
    """The complete-V1 inventory by family."""
    names = {
        "SRC": "source", "TYPE": "types", "EXPR": "expressions", "TEMP": "temporal",
        "PROT": "protocol", "RUN": "runtime", "BACK": "backend", "EVID": "evidence", "TOOL": "tooling",
    }
    families = capability_families()
    total = sum(n for _, n in families)
    f = Figure(
        "capabilities",
        980,
        250,
        "The complete 1-draft language is a closed list of capabilities in nine families: source, "
        "types, expressions, temporal, protocol, runtime, backend, evidence and tooling.",
    )
    f.text(0, 16, f"{total} CAPABILITIES IN {len(families)} FAMILIES", "muted", "mono", SMALL)
    f.columns(0, 30, 980, 216, [n for _, n in families], labels=[names[k] for k, _ in families], role="logic")
    return f


REPOS = [
    ("ORCHESTRATION", [("quire-verification", True), ("quire-driver", True), ("quire-integration", True)]),
    ("BACKENDS", [("quire-contract-codegen", False), ("quire-contract-runtime", False),
                  ("quire-analyze", False), ("quire-protocol", True), ("quire-mltl", False)]),
    ("LANGUAGE", [("quire-specification", True), ("quire-spec-language", False), ("quire-contract-ir", False)]),
    ("FOUNDATIONS", [("quire-exact", False), ("quire-semantic-value", False), ("quire-walk", False),
                     ("quire-canonical", False), ("quire-verification-contracts", False),
                     ("quire-observation", False)]),
]


def ecosystem(here="quire-spec-language"):
    """The Quire repositories in bands; each band builds on the ones below."""
    f = Figure(
        "ecosystem",
        980,
        364,
        "The Quire repositories in four bands. Foundations: quire-exact, quire-semantic-value, "
        "quire-walk, quire-canonical, quire-verification-contracts, quire-observation. Language: "
        "quire-specification, quire-spec-language, quire-contract-ir. Backends: "
        "quire-contract-codegen, quire-contract-runtime, quire-analyze, quire-protocol, "
        "quire-mltl. Orchestration: quire-verification, quire-driver, quire-integration.",
    )
    y = 0
    gap = 10
    for band, repos in REPOS:
        f.text(0, y + 12, band, "muted", "mono", SMALL)
        widths = [FONTS["title"].width(name, TITLE) + 24 for name, _ in repos]
        spare = (980 - sum(widths) - gap * (len(repos) - 1)) / len(repos)
        x = 0
        for (name, private), w in zip(repos, widths):
            w = round(w + spare)
            role = "logic" if name == here else "ink"
            f.box(x, y + 20, w, 50, role, name, "private" if private else "public", dashed=private)
            x += w + gap
        y += 88
    f.legend(0, 360, [("logic", "this repository", "box"), ("ink", "private", "dash")])
    return f


LAYERS = [
    ("COMMAND LINE", [("qsl-replay", "replay counterexamples"), ("qsl-analyze", "model checking"),
                      ("quire-spec", "the binary")]),
    ("EXECUTE", [("qsl-eval", "reference evaluator"), ("qsl-route", "backend routing")]),
    ("PACKAGE", [("qsl-package", "checked-package/v2")]),
    ("CHECK", [("qsl-semantics", "names · types · definedness")]),
    ("PARSE", [("qsl-forms", "parsed forms"), ("qsl-cst", "syntax tree · formatter"),
               ("qsl-source", "Markdown intake (optional)")]),
    ("FOUNDATION", [("qsl-foundation", "identity · diagnostics")]),
    ("SHARED", [("quire-exact", "exact values"), ("quire-semantic-value", "semantic values"),
                ("quire-walk", "heap-stack walker"), ("quire-canonical", "canonical JSON")]),
]


def layers():
    """The workspace crates as layers; each depends only on rows below it."""
    f = Figure(
        "layers",
        980,
        420,
        "The QSL workspace in layers, bottom to top: the shared no_std libraries quire-exact, "
        "quire-semantic-value and quire-walk; qsl-foundation; qsl-cst; qsl-forms; qsl-semantics; "
        "qsl-package; then qsl-eval and qsl-route; and at the top qsl-replay, qsl-analyze and the "
        "quire-spec command line.",
    )
    left, step = 130, 60
    for i, (band, crates) in enumerate(LAYERS):
        y = i * step
        f.text(0, y + 30, band, "muted", "mono", SMALL)
        w = (980 - left - 12 * (len(crates) - 1)) / len(crates)
        for j, (name, what) in enumerate(crates):
            x = left + j * (w + 12)
            role = "ink" if band == "SHARED" else "model" if band == "PACKAGE" else "logic"
            f.box(round(x), y, round(w), 50, role, name, what, dashed=name == "qsl-source")
    return f


FIGURES = [hero, first_rule, outcomes, pipeline, capabilities, ecosystem, layers]
