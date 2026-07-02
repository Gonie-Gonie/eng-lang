const EXECUTION_PROFILES = [
  {
    id: "normal",
    description: "Default workflow execution",
    detail: "Runs declared effects and writes the standard review artifacts."
  },
  {
    id: "safe",
    description: "Reject side effects",
    detail: "Fails workflows with explicit write, export, process, file, or DB mutation effects."
  },
  {
    id: "repro",
    description: "Require reproducibility metadata",
    detail: "Records environment dependencies and rejects unseeded sampling or unpinned network/cache reads."
  }
];

module.exports = {
  EXECUTION_PROFILES
};
