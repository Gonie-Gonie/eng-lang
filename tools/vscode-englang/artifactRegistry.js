const fs = require("fs");
const path = require("path");

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
    label: "Report Source Data",
    description: "build/result/report_spec.json",
    relativePath: ["build", "result", "report_spec.json"]
  },
  {
    id: "outputManifest",
    label: "Generated Output List",
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
    label: "Process Results",
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

function lastRunArtifactAvailability(artifact, root) {
  const artifactPath = lastRunArtifactPath(artifact, root);
  const exists = Boolean(artifactPath && fs.existsSync(artifactPath));
  return {
    exists,
    path: artifactPath,
    status: exists ? "available" : "missing"
  };
}

function lastRunArtifactPath(artifact, root) {
  if (!root || !Array.isArray(artifact?.relativePath)) {
    return undefined;
  }
  return path.join(root, ...artifact.relativePath);
}

function lastRunArtifactDisplay(artifact, root) {
  const availability = lastRunArtifactAvailability(artifact, root);
  if (artifact.id !== "processResults" || !root) {
    return {
      label: artifact.label,
      detail: lastRunArtifactAvailabilityDetail(availability, root)
    };
  }
  const processCount = readProcessCount(availability.path);
  if (processCount === 0) {
    return {
      label: "Process Results (0 external processes)",
      detail: "No external process executions were recorded in the latest run."
    };
  }
  if (processCount > 0) {
    return {
      label: `External Process Results (${processCount})`,
      detail: `${processCount} external process execution${processCount === 1 ? "" : "s"} recorded.`
    };
  }
  return {
    label: artifact.label,
    detail: lastRunArtifactAvailabilityDetail(availability, root)
  };
}

function lastRunArtifactAvailabilityDetail(availability, root) {
  if (!root) {
    return "Open an EngLang workspace to inspect artifact availability.";
  }
  return availability.exists
    ? "Available from the latest saved-artifacts run."
    : "Missing from the latest saved-artifacts run. Run the current file first.";
}

function readProcessCount(artifactPath) {
  if (!artifactPath || !fs.existsSync(artifactPath)) {
    return undefined;
  }
  try {
    const processResults = JSON.parse(fs.readFileSync(artifactPath, "utf8"));
    const count = Number(processResults?.process_count);
    return Number.isFinite(count) && count >= 0 ? count : undefined;
  } catch {
    return undefined;
  }
}

module.exports = {
  LAST_RUN_ARTIFACTS,
  lastRunArtifactAvailability,
  lastRunArtifactDisplay,
  lastRunArtifactPath,
  readProcessCount
};
