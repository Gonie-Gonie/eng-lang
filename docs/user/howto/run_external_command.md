# Run An External Command

Use run command only when the boundary is intentional:

```eng partial
result = run command "cmd"
with {
    args = ["/C", "echo", "eng-process-ok"]
}
```

The review artifact should record the command boundary, arguments, status, and
related outputs. For policy details, read docs/reference/language/side_effect_policy.md.
