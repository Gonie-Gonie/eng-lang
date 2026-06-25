# Save Artifacts

Use explicit write statements and overwrite policy:

```eng partial
write json "outputs/energy.json", E_coil
with {
    overwrite = true
}
```

Generated files should appear in the output manifest or review artifact so a
reviewer can distinguish intended outputs from incidental files.
