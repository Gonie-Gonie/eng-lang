# Run An External Command

Use run command only when the boundary is intentional:

```eng partial
result = run command "cmd"
with {
    args = ["/C", "echo", "eng-process-ok"]
    env = { ENG_MODE = "review" }
    timeout = 10 s
    retry = 1
}
```

The review artifact should record the command boundary, arguments, status, and
related outputs. Use `allow_failure = true` only when a failed or timed-out
process is expected data. For policy details, read
docs/reference/language/side_effect_policy.md.
