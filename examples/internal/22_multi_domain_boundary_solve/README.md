# Internal 22 Multi-Domain Boundary Solve

This internal fixture exercises a small multi-domain algebraic boundary solve
across Thermal, Fluid, and MechanicalNode connection sets.

Each domain contributes generated connection equations plus component-local
boundary equations. Together they form a square residual graph that the runtime
solves through the dense linear residual graph path. Report, review, result,
and IDE inspectors must expose:

- generated equations and reasons
- residual graph dependency metadata
- solved named variables
- normalized residuals and largest residuals
- explicit solver status and convergence status

This is still a constrained algebraic seed. It is not a production physical
multi-domain simulation engine, not a nonlinear coupled component solver, and
not a domain package ecosystem claim.
