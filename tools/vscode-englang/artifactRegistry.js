const LAST_RUN_ARTIFACTS = [
  {
    id: "report",
    label: "Report HTML",
    description: "build/result/report.html",
    relativePath: ["build", "result", "report.html"],
    external: true
  },
  {
    id: "review",
    label: "Review Data",
    description: "build/result/review.json",
    relativePath: ["build", "result", "review.json"]
  },
  {
    id: "result",
    label: "Result Data",
    description: "build/result/result.engres",
    relativePath: ["build", "result", "result.engres"]
  },
  {
    id: "reportSpec",
    label: "Report Data",
    description: "build/result/report_spec.json",
    relativePath: ["build", "result", "report_spec.json"]
  },
  {
    id: "outputManifest",
    label: "Output List",
    description: "build/result/output_manifest.json",
    relativePath: ["build", "result", "output_manifest.json"]
  },
  {
    id: "runLog",
    label: "Run Log",
    description: "build/result/run_log.json",
    relativePath: ["build", "result", "run_log.json"]
  },
  {
    id: "staticRunPlan",
    label: "Static Run Graph",
    description: "build/result/static_run_plan.json",
    relativePath: ["build", "result", "static_run_plan.json"]
  },
  {
    id: "runPlan",
    label: "Run Graph",
    description: "build/result/run_plan.json",
    relativePath: ["build", "result", "run_plan.json"]
  },
  {
    id: "runLock",
    label: "Run Reproducibility Lock",
    description: "build/result/run_lock.json",
    relativePath: ["build", "result", "run_lock.json"]
  },
  {
    id: "processResults",
    label: "External Process Results",
    description: "build/result/process_results.json",
    relativePath: ["build", "result", "process_results.json"]
  },
  {
    id: "cacheManifest",
    label: "Cache Records",
    description: "build/result/cache_manifest.json",
    relativePath: ["build", "result", "cache_manifest.json"]
  },
  {
    id: "testResults",
    label: "Test Results",
    description: "build/result/test_results.json",
    relativePath: ["build", "result", "test_results.json"]
  },
  {
    id: "plotSpec",
    label: "Plot Data",
    description: "build/result/plots/plot_spec.json",
    relativePath: ["build", "result", "plots", "plot_spec.json"]
  },
  {
    id: "plotManifest",
    label: "Plot Output List",
    description: "build/result/plots/plot_manifest.json",
    relativePath: ["build", "result", "plots", "plot_manifest.json"]
  },
  {
    id: "plotSvg",
    label: "Plot SVG",
    description: "build/result/plots/timeseries.svg",
    relativePath: ["build", "result", "plots", "timeseries.svg"],
    external: true
  }
];

module.exports = {
  LAST_RUN_ARTIFACTS
};
