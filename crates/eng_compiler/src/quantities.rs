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
        quantity_kind: "Area",
        canonical_unit: "m2",
        dimension: "Area",
        description: "Surface area.",
    },
    QuantityCompletion {
        quantity_kind: "Volume",
        canonical_unit: "m3",
        dimension: "Volume",
        description: "Spatial volume.",
    },
    QuantityCompletion {
        quantity_kind: "Conductance",
        canonical_unit: "W/K",
        dimension: "Power/Temperature",
        description: "Thermal conductance.",
    },
    QuantityCompletion {
        quantity_kind: "HeatCapacity",
        canonical_unit: "J/K",
        dimension: "Energy/Temperature",
        description: "Thermal capacitance or lumped heat capacity.",
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
        quantity_kind: "Duration",
        canonical_unit: "s",
        dimension: "Time",
        description: "Elapsed time duration.",
    },
    QuantityCompletion {
        quantity_kind: "Irradiance",
        canonical_unit: "W/m2",
        dimension: "Power/Area",
        description: "Radiant power incident per unit area.",
    },
    QuantityCompletion {
        quantity_kind: "ThermalTransmittance",
        canonical_unit: "W/m2/K",
        dimension: "Power/Area/Temperature",
        description: "Heat transfer rate per unit area and temperature difference.",
    },
    QuantityCompletion {
        quantity_kind: "PeopleDensity",
        canonical_unit: "person/m2",
        dimension: "Count/Area",
        description: "Occupant count per unit floor area.",
    },
    QuantityCompletion {
        quantity_kind: "Pressure",
        canonical_unit: "Pa",
        dimension: "Pressure",
        description: "Static or differential pressure.",
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
        quantity_kind: "DimensionlessNumber",
        canonical_unit: "1",
        dimension: "Dimensionless",
        description: "Plain dimensionless numeric value.",
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
        "m2" | "m^2" => completions_for(&["Area"]),
        "m3" | "m^3" => completions_for(&["Volume"]),
        "k" => completions_for(&["AbsoluteTemperature", "TemperatureDelta"]),
        "degc" => completions_for(&["AbsoluteTemperature"]),
        "w/k" => completions_for(&["Conductance"]),
        "j/k" | "kj/k" => completions_for(&["HeatCapacity"]),
        "j/kg/k" => completions_for(&["SpecificHeat"]),
        "w" | "kw" => completions_for(&["HeatRate", "ElectricPower", "MechanicalPower"]),
        "j" | "wh" | "kwh" | "mj" => completions_for(&["Energy"]),
        "s" | "min" | "h" | "hr" | "hour" | "hours" => completions_for(&["Duration"]),
        "w/m2" | "w/m^2" => completions_for(&["Irradiance"]),
        "w/m2/k" | "w/m^2/k" | "w/(m2*k)" | "w/(m^2*k)" => {
            completions_for(&["ThermalTransmittance"])
        }
        "person/m2" | "people/m2" => completions_for(&["PeopleDensity"]),
        "pa" | "kpa" => completions_for(&["Pressure"]),
        "kg/s" => completions_for(&["MassFlowRate"]),
        "%" => completions_for(&["Ratio"]),
        "1" => completions_for(&["Ratio", "DimensionlessNumber", "ReynoldsNumber"]),
        _ => Vec::new(),
    }
}

pub fn infer_quantity_from_name_and_unit(name: &str, unit: &str) -> Option<QuantityCompletion> {
    let lowered_name = name.to_ascii_lowercase();
    let normalized_unit = normalize_unit(unit);
    let candidates = candidates_for_unit(unit);
    let candidate = |quantity_kind: &str| {
        candidates
            .iter()
            .find(|completion| completion.quantity_kind == quantity_kind)
            .copied()
    };

    if candidates.len() <= 1 {
        return candidates.first().copied();
    }

    if normalized_unit == "k"
        && (lowered_name.starts_with("dt")
            || lowered_name.starts_with("d_t")
            || lowered_name.contains("delta")
            || lowered_name.contains("difference"))
    {
        return candidate("TemperatureDelta");
    }

    if normalized_unit == "k" && (lowered_name.starts_with('t') || lowered_name.contains("temp")) {
        return candidate("AbsoluteTemperature");
    }

    if lowered_name.starts_with('q')
        || lowered_name.contains("heat")
        || lowered_name.contains("cool")
    {
        if let Some(completion) = candidate("HeatRate") {
            return Some(completion);
        }
    }

    if lowered_name.starts_with("p_") || lowered_name.contains("fan") {
        if let Some(completion) = candidate("ElectricPower") {
            return Some(completion);
        }
    }

    if lowered_name.contains("shaft") || lowered_name.contains("mech") {
        if let Some(completion) = candidate("MechanicalPower") {
            return Some(completion);
        }
    }

    if normalized_unit == "1" {
        if lowered_name == "re" || lowered_name.contains("reynolds") {
            return candidate("ReynoldsNumber");
        }
        if lowered_name == "eta"
            || lowered_name == "cop"
            || lowered_name.ends_with("_cop")
            || lowered_name.contains("ratio")
            || lowered_name.contains("efficiency")
            || lowered_name.contains("fraction")
        {
            return candidate("Ratio");
        }
        return candidate("DimensionlessNumber");
    }

    None
}

pub fn parse_numeric_literal(expression: &str) -> Option<(f64, Option<String>)> {
    let mut parts = expression.split_whitespace();
    let value_or_attached_unit = parts.next()?;
    let spaced_unit = parts.next();
    if parts.next().is_some() {
        return None;
    }

    if let Some(value) = parse_numeric_value(value_or_attached_unit) {
        return Some((value, spaced_unit.map(str::to_owned)));
    }
    if spaced_unit.is_some() {
        return None;
    }

    let value = parse_numeric_value(value_or_attached_unit.strip_suffix('%')?)?;
    Some((value, Some("%".to_owned())))
}

pub fn first_unit_in_expression(expression: &str) -> Option<String> {
    let raw_words = expression.split_whitespace().collect::<Vec<_>>();
    for pair in raw_words.windows(2) {
        let [number, unit] = pair else {
            continue;
        };
        if parse_numeric_value(trim_expression_punctuation(number)).is_some() {
            if let Some(unit) = registered_unit_from_word(unit) {
                return Some(unit);
            }
        }
    }

    let normalized = expression
        .chars()
        .map(|character| match character {
            '(' | ')' | ',' | ';' | '[' | ']' | '{' | '}' | '"' | '\'' | '=' => ' ',
            other => other,
        })
        .collect::<String>();
    let words: Vec<String> = normalized
        .split_whitespace()
        .map(|word| trim_expression_punctuation(word).to_owned())
        .collect();

    for word in &words {
        if let Some((_value, Some(unit))) = parse_numeric_literal(word) {
            if !candidates_for_unit(&unit).is_empty() {
                return Some(unit);
            }
        }
    }

    for pair in words.windows(2) {
        let [number, unit] = pair else {
            continue;
        };
        if parse_numeric_literal(number).is_some_and(|(_value, unit)| unit.is_none())
            && !candidates_for_unit(unit).is_empty()
        {
            return Some(unit.to_owned());
        }
    }

    if expression.contains("J/kg/K") {
        return Some("J/kg/K".to_owned());
    }
    if expression.contains("kJ/K") {
        return Some("kJ/K".to_owned());
    }
    if expression.contains("J/K") {
        return Some("J/K".to_owned());
    }
    if expression.contains("W/K") {
        return Some("W/K".to_owned());
    }
    if expression.contains("W/m^2") {
        return Some("W/m^2".to_owned());
    }
    if expression.contains("W/m2") {
        return Some("W/m2".to_owned());
    }
    if expression.contains("person/m2") {
        return Some("person/m2".to_owned());
    }
    if expression.contains("people/m2") {
        return Some("people/m2".to_owned());
    }
    if expression.contains("kPa") {
        return Some("kPa".to_owned());
    }
    if expression.contains("Pa") {
        return Some("Pa".to_owned());
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
    let normalized = trim_unit_punctuation(unit).to_ascii_lowercase();
    match normalized.as_str() {
        "°c" | "℃" => "degc".to_owned(),
        _ => normalized,
    }
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

fn parse_numeric_value(value: &str) -> Option<f64> {
    if !value.bytes().any(|byte| byte.is_ascii_digit()) {
        return None;
    }
    value.parse::<f64>().ok()
}

fn registered_unit_from_word(word: &str) -> Option<String> {
    let mut candidate = word.trim_matches(|character: char| {
        matches!(character, ',' | ';' | ']' | '[' | '{' | '}' | '"' | '\'')
    });
    loop {
        if !candidates_for_unit(candidate).is_empty() {
            return Some(candidate.to_owned());
        }
        if let Some(stripped) = candidate.strip_suffix(')') {
            candidate = stripped;
            continue;
        }
        if let Some(stripped) = candidate.strip_prefix('(') {
            candidate = stripped;
            continue;
        }
        return None;
    }
}

fn trim_unit_punctuation(value: &str) -> &str {
    let mut trimmed = value.trim_matches(|character: char| {
        matches!(character, ',' | ';' | ']' | '[' | '{' | '}' | '"' | '\'')
    });
    while parentheses_wrap_entire_value(trimmed) {
        trimmed = &trimmed[1..trimmed.len() - 1];
    }
    trimmed
}

fn parentheses_wrap_entire_value(value: &str) -> bool {
    if !value.starts_with('(') || !value.ends_with(')') {
        return false;
    }
    let mut depth = 0usize;
    for (index, character) in value.char_indices() {
        match character {
            '(' => depth += 1,
            ')' => {
                let Some(next_depth) = depth.checked_sub(1) else {
                    return false;
                };
                depth = next_depth;
                if depth == 0 && index + character.len_utf8() < value.len() {
                    return false;
                }
            }
            _ => {}
        }
    }
    depth == 0
}

fn trim_expression_punctuation(value: &str) -> &str {
    value.trim_matches(|character: char| {
        matches!(
            character,
            ',' | ';' | ')' | '(' | ']' | '[' | '{' | '}' | '"' | '\''
        )
    })
}
