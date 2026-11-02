# Historical 0-draft model fixtures

`model-source.json` is an upstream rule-model input read by the retained
`model_source` frontend. `model-artifact.json` is its admitted artifact as
written before the package producer existed; the frontend and `NativeModel`
tests assert the admitted bytes still equal it.

`header.native` is the shared 0-draft unit header. The formatter identity
corpus composes the historical package vectors (`minimal`, `controls`,
`multiple`) from it and requires each to keep its observed original refusal
through the `1-draft` front end.
