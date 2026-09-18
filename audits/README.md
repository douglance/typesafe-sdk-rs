# Audits

Question sets for auditing this codebase with `jev classify`.

```sh
jev classify --questions-file audits/test-strength.json \
             --items-file <(scripts/extract-tests.py) --json
```

An answer above 0.5 means a wrong implementation would still pass that test.
Treat it as a ranking, not a verdict: confirm by mutation before changing
anything, because the classifier reads the shape of the assertions and cannot
always tell identity from equality.

## Why these questions are worded this way

`test-strength.json` was chosen by measurement, not taste. Eleven tests whose
strength had already been settled by mutation — four that provably passed while
the behaviour was broken, seven that provably failed — were used as an
evaluation set, and four wordings were scored against it:

| prompt | correct | uncertain | separation |
| --- | --- | --- | --- |
| choice over specific/shallow/tautological | 10/11 | 10/11 | −0.17 |
| one noul, "would a broken implementation pass?" | 11/11 | 4/11 | +0.23 |
| the same, plus an example in the weak criterion | 8/11 | 4/11 | +0.12 |
| **the same, with the misleading clause removed** | **11/11** | **2/11** | **+0.31** |

Two lessons are worth keeping. Overlapping choice options spread probability
and produced confident-looking labels with no confidence behind them; one noul
asking a single falsifiable question separated the classes far better. And
adding a concrete example to the weak criterion made things worse, not better —
the example described the surface form of three strong tests and the model
anchored on it. Removing the offending clause beat explaining it.
