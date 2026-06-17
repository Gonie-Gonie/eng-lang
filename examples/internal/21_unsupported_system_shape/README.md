# Unsupported System Shape

This internal example exercises the solver boundary for a simulated system that
does not match a supported runtime solver shape. It should not produce a
fabricated trajectory; runtime artifacts should record
`skipped_unsupported_shape` with a failure reason.
