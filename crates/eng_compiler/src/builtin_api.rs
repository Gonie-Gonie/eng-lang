use std::collections::BTreeSet;
use std::sync::OnceLock;

use crate::module_registry::{
    bundled_module_registry, ModuleFunctionParameter, ModuleFunctionSignature,
};
use crate::semantic::DIMENSIONLESS_MATH_FUNCTIONS;
use crate::stats::is_percentile_statistic;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BuiltinFunctionSignature {
    pub owner: String,
    pub status: String,
    pub status_label: String,
    pub documentation: String,
    pub name: String,
    pub label: String,
    pub parameters: Vec<ModuleFunctionParameter>,
    pub return_type: String,
    pub return_display_unit: Option<String>,
}

static BUILTIN_FUNCTION_SIGNATURES: OnceLock<Vec<BuiltinFunctionSignature>> = OnceLock::new();

pub fn builtin_function_signatures() -> &'static [BuiltinFunctionSignature] {
    BUILTIN_FUNCTION_SIGNATURES
        .get_or_init(build_builtin_function_signatures)
        .as_slice()
}

pub fn builtin_function_signatures_for_name(name: &str) -> Vec<BuiltinFunctionSignature> {
    if is_percentile_statistic(name) {
        return builtin_function_signatures()
            .iter()
            .find(|signature| signature.name == "pNN")
            .cloned()
            .map(|mut signature| {
                signature.name = name.to_owned();
                signature.label = signature.label.replacen("pNN", name, 1);
                signature
            })
            .into_iter()
            .collect();
    }
    if name == "pNN" {
        return Vec::new();
    }
    builtin_function_signatures()
        .iter()
        .filter(|signature| signature.name == name)
        .cloned()
        .collect()
}

fn build_builtin_function_signatures() -> Vec<BuiltinFunctionSignature> {
    let registry = bundled_module_registry()
        .expect("bundled module registry must contain valid API contracts");
    let mut signatures = Vec::new();
    for module in registry
        .modules
        .into_iter()
        .filter(|module| module_has_executable_contract(&module.status))
    {
        let status_label = module.status_label().to_owned();
        for signature in module.function_signatures() {
            signatures.push(from_module_signature(
                &module.name,
                &module.status,
                &status_label,
                &module.purpose,
                signature,
            ));
        }
    }

    for name in DIMENSIONLESS_MATH_FUNCTIONS {
        signatures.push(dimensionless_math_signature(name));
    }
    signatures.push(percentile_signature_template());
    signatures.sort_by(|left, right| {
        left.name
            .cmp(&right.name)
            .then_with(|| left.label.cmp(&right.label))
    });

    let mut labels = BTreeSet::new();
    signatures.retain(|signature| labels.insert(signature.label.clone()));
    signatures
}

fn module_has_executable_contract(status: &str) -> bool {
    matches!(
        status,
        "supported" | "supported_narrow" | "native_preview" | "internal_planned" | "internal"
    )
}

fn from_module_signature(
    owner: &str,
    status: &str,
    status_label: &str,
    documentation: &str,
    signature: ModuleFunctionSignature,
) -> BuiltinFunctionSignature {
    BuiltinFunctionSignature {
        owner: owner.to_owned(),
        status: status.to_owned(),
        status_label: status_label.to_owned(),
        documentation: documentation.to_owned(),
        name: signature.name,
        label: signature.label,
        parameters: signature.parameters,
        return_type: signature.return_type,
        return_display_unit: signature.return_display_unit,
    }
}

fn dimensionless_math_signature(name: &str) -> BuiltinFunctionSignature {
    BuiltinFunctionSignature {
        owner: "EngLang core".to_owned(),
        status: "supported_narrow".to_owned(),
        status_label: "Supported narrow".to_owned(),
        documentation:
            "Dimensionless scalar math for solver, component, and scalar arithmetic expressions."
                .to_owned(),
        name: name.to_owned(),
        label: format!("{name}(value: Number) -> Number"),
        parameters: vec![ModuleFunctionParameter {
            name: "value".to_owned(),
            label: "value: Number".to_owned(),
            type_name: "Number".to_owned(),
            optional: false,
        }],
        return_type: "Number".to_owned(),
        return_display_unit: None,
    }
}

fn percentile_signature_template() -> BuiltinFunctionSignature {
    BuiltinFunctionSignature {
        owner: "eng.stats".to_owned(),
        status: "native_preview".to_owned(),
        status_label: "Native workflow support".to_owned(),
        documentation:
            "Nearest-rank percentile for p1 through p100, including leading-zero spellings."
                .to_owned(),
        name: "pNN".to_owned(),
        label: "pNN(series: TimeSeries | Uncertain, axis?: TimeAxis) -> Quantity".to_owned(),
        parameters: vec![
            ModuleFunctionParameter {
                name: "series".to_owned(),
                label: "series: TimeSeries | Uncertain".to_owned(),
                type_name: "TimeSeries | Uncertain".to_owned(),
                optional: false,
            },
            ModuleFunctionParameter {
                name: "axis".to_owned(),
                label: "axis?: TimeAxis".to_owned(),
                type_name: "TimeAxis".to_owned(),
                optional: true,
            },
        ],
        return_type: "Quantity".to_owned(),
        return_display_unit: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_covers_native_callable_api_families() {
        for name in [
            "file",
            "date",
            "url",
            "integrate",
            "mean",
            "duration_above",
            "rmse",
            "measured",
            "propagate",
            "train_test_split",
            "model_card",
            "delay",
            "predictor",
            "sqrt",
        ] {
            assert!(
                !builtin_function_signatures_for_name(name).is_empty(),
                "missing compiler-owned signature for {name}"
            );
        }
    }

    #[test]
    fn catalog_preserves_overloads_optional_parameters_and_units() {
        let measured = builtin_function_signatures_for_name("measured");
        assert_eq!(measured.len(), 2);
        assert!(measured
            .iter()
            .any(|signature| signature.label.contains("relative_error")));

        let normal = builtin_function_signatures_for_name("normal");
        assert!(normal[0]
            .parameters
            .iter()
            .any(|parameter| parameter.name == "samples" && parameter.optional));

        let uniform = builtin_function_signatures_for_name("uniform");
        assert_eq!(uniform.len(), 2);
        assert_eq!(uniform[0].owner, "eng.sampling");
        assert_eq!(uniform[0].return_type, "SampleDistribution");
        assert!(uniform
            .iter()
            .any(|signature| signature.owner == "eng.uncertainty"));

        let duration = builtin_function_signatures_for_name("duration_above");
        assert_eq!(duration[0].return_display_unit.as_deref(), Some("s"));
    }

    #[test]
    fn catalog_materializes_only_valid_percentile_names() {
        let percentile = builtin_function_signatures_for_name("p05");
        assert_eq!(percentile.len(), 1);
        assert!(percentile[0].label.starts_with("p05("));
        assert!(builtin_function_signatures_for_name("p0").is_empty());
        assert!(builtin_function_signatures_for_name("p101").is_empty());
        assert!(builtin_function_signatures_for_name("pNN").is_empty());
    }
}
