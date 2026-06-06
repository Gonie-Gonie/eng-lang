#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct QuantityCompletion {
    pub quantity_kind: &'static str,
    pub canonical_unit: &'static str,
    pub dimension: &'static str,
    pub description: &'static str,
}

pub const QUANTITY_COMPLETIONS: &[QuantityCompletion] = &[
    QuantityCompletion {
        quantity_kind: "AbsoluteTemperature",
        canonical_unit: "K",
        dimension: "Temperature",
        description: "Affine absolute thermodynamic temperature.",
    },
    QuantityCompletion {
        quantity_kind: "TemperatureDelta",
        canonical_unit: "K",
        dimension: "Temperature",
        description: "Temperature interval or difference.",
    },
    QuantityCompletion {
        quantity_kind: "Length",
        canonical_unit: "m",
        dimension: "Length",
        description: "Linear distance.",
    },
    QuantityCompletion {
        quantity_kind: "Conductance",
        canonical_unit: "W/K",
        dimension: "Power/Temperature",
        description: "Thermal conductance.",
    },
    QuantityCompletion {
        quantity_kind: "SpecificHeat",
        canonical_unit: "J/kg/K",
        dimension: "Energy/Mass/Temperature",
        description: "Specific heat capacity.",
    },
    QuantityCompletion {
        quantity_kind: "HeatRate",
        canonical_unit: "W",
        dimension: "Power",
        description: "Thermal power or heat flow rate.",
    },
    QuantityCompletion {
        quantity_kind: "ElectricPower",
        canonical_unit: "W",
        dimension: "Power",
        description: "Electrical power.",
    },
    QuantityCompletion {
        quantity_kind: "MechanicalPower",
        canonical_unit: "W",
        dimension: "Power",
        description: "Mechanical shaft or fluid power.",
    },
    QuantityCompletion {
        quantity_kind: "Energy",
        canonical_unit: "J",
        dimension: "Energy",
        description: "Energy, heat, or work quantity.",
    },
    QuantityCompletion {
        quantity_kind: "MassFlowRate",
        canonical_unit: "kg/s",
        dimension: "Mass/Time",
        description: "Mass flow rate.",
    },
    QuantityCompletion {
        quantity_kind: "Ratio",
        canonical_unit: "1",
        dimension: "Dimensionless",
        description: "Dimensionless ratio.",
    },
    QuantityCompletion {
        quantity_kind: "ReynoldsNumber",
        canonical_unit: "1",
        dimension: "Dimensionless",
        description: "Dimensionless Reynolds number.",
    },
];

pub fn all_quantity_completions() -> &'static [QuantityCompletion] {
    QUANTITY_COMPLETIONS
}

pub fn completion_count() -> usize {
    QUANTITY_COMPLETIONS.len()
}

pub fn candidates_for_unit(unit: &str) -> Vec<QuantityCompletion> {
    match normalize_unit(unit).as_str() {
        "m" | "cm" | "mm" => completions_for(&["Length"]),
        "k" => completions_for(&["AbsoluteTemperature", "TemperatureDelta"]),
        "degc" => completions_for(&["AbsoluteTemperature"]),
        "w/k" => completions_for(&["Conductance"]),
        "j/kg/k" => completions_for(&["SpecificHeat"]),
        "w" | "kw" => completions_for(&["HeatRate", "ElectricPower", "MechanicalPower"]),
        "j" | "wh" | "kwh" | "mj" => completions_for(&["Energy"]),
        "kg/s" => completions_for(&["MassFlowRate"]),
        "1" => completions_for(&["Ratio", "ReynoldsNumber"]),
        _ => Vec::new(),
    }
}

pub fn infer_quantity_from_name_and_unit(name: &str, unit: &str) -> Option<QuantityCompletion> {
    let lowered_name = name.to_ascii_lowercase();
    let candidates = candidates_for_unit(unit);

    if candidates.len() <= 1 {
        return candidates.first().copied();
    }

    if lowered_name.starts_with('q')
        || lowered_name.contains("heat")
        || lowered_name.contains("cool")
    {
        return find_completion("HeatRate");
    }

    if lowered_name.starts_with("p_") || lowered_name.contains("fan") {
        return find_completion("ElectricPower");
    }

    if lowered_name.contains("shaft") || lowered_name.contains("mech") {
        return find_completion("MechanicalPower");
    }

    None
}

pub fn first_unit_in_expression(expression: &str) -> Option<String> {
    let words: Vec<String> = expression
        .split_whitespace()
        .map(|word| trim_expression_punctuation(word).to_owned())
        .collect();

    for pair in words.windows(2) {
        let [number, unit] = pair else {
            continue;
        };
        if is_number_literal(number) && !candidates_for_unit(unit).is_empty() {
            return Some(unit.to_owned());
        }
    }

    if expression.contains("J/kg/K") {
        return Some("J/kg/K".to_owned());
    }
    if expression.contains("W/K") {
        return Some("W/K".to_owned());
    }
    if expression.contains("kg/s") {
        return Some("kg/s".to_owned());
    }

    None
}

pub fn is_number_literal(value: &str) -> bool {
    let mut seen_digit = false;
    let mut seen_dot = false;

    for character in value.chars() {
        if character.is_ascii_digit() {
            seen_digit = true;
        } else if character == '.' && !seen_dot {
            seen_dot = true;
        } else {
            return false;
        }
    }

    seen_digit
}

pub fn normalize_unit(unit: &str) -> String {
    trim_expression_punctuation(unit).to_ascii_lowercase()
}

pub fn completion_labels(completions: &[QuantityCompletion]) -> String {
    completions
        .iter()
        .map(|completion| completion.quantity_kind)
        .collect::<Vec<_>>()
        .join(", ")
}

fn completions_for(names: &[&str]) -> Vec<QuantityCompletion> {
    names
        .iter()
        .filter_map(|name| find_completion(name))
        .collect()
}

fn find_completion(name: &str) -> Option<QuantityCompletion> {
    QUANTITY_COMPLETIONS
        .iter()
        .find(|completion| completion.quantity_kind == name)
        .copied()
}

fn trim_expression_punctuation(value: &str) -> &str {
    value.trim_matches(|character: char| {
        matches!(
            character,
            ',' | ';' | ')' | '(' | ']' | '[' | '{' | '}' | '"' | '\''
        )
    })
}
