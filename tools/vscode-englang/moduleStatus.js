function moduleStatusDisplay(module) {
  return module?.status_label || module?.statusLabel || moduleStatusLabelForStatus(module?.status);
}

function moduleStatusDetailDisplay(module) {
  return module?.status_detail || module?.statusDetail || moduleStatusDetailForStatus(module?.status);
}

function moduleStatusLabelForStatus(status) {
  switch (status) {
    case "supported":
      return "Supported";
    case "supported_narrow":
      return "Supported narrow";
    case "native_preview":
      return "Native workflow support";
    case "planned":
      return "Planned";
    case "internal_planned":
      return "Internal planned";
    case "internal":
      return "Internal";
    default:
      return status || "-";
  }
}

function moduleStatusDetailForStatus(status) {
  switch (status) {
    case "supported":
      return "Public built-in surface supported by compiler/runtime.";
    case "supported_narrow":
      return "Supported for the listed syntax forms and review artifacts.";
    case "native_preview":
      return "Native runtime path is implemented for the listed workflow commands and artifacts; unsupported combinations report diagnostics.";
    case "planned":
      return "Documented target surface; not executable as a public module yet.";
    case "internal_planned":
      return "Internal design target, not a public stdlib contract.";
    case "internal":
      return "Internal compiler/runtime vocabulary, not a public stdlib contract.";
    default:
      return "-";
  }
}

function moduleBackingLabel(module) {
  switch (module?.backing) {
    case "compiler_runtime_builtin":
      return "Compiler/runtime";
    case "none":
      return "No executable backing";
    case "internal":
      return "Internal";
    default:
      return module?.backing ? String(module.backing).replaceAll("_", " ") : "-";
  }
}

module.exports = {
  moduleStatusDisplay,
  moduleStatusDetailDisplay,
  moduleBackingLabel
};
