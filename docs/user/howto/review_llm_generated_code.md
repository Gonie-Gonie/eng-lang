# Review LLM-Generated Code

Run LLM-generated EngLang code through the same path as human-written code:

```bat
eng.exe check path/to/candidate.eng
eng.exe run path/to/candidate.eng --save-artifacts
```

Reject code that removes units, hides input paths, skips schema promotion, or
produces a report without reviewable evidence.
