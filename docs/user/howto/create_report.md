# Create A Report

Use a report block when a result needs a human-readable artifact:

```eng partial
report {
    summarize Q_coil by [mean, max, p95]
    show E_coil
    plot Q_coil over Time
}
```

Treat report.html as presentation and review.json as evidence. Review both
before accepting a result.
