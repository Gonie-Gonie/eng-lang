use crate::quantities::{first_unit_in_expression, normalize_unit};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct UnitInfo {
    pub symbol: &'static str,
    pub canonical_unit: &'static str,
    pub quantity_hint: &'static str,
    pub dimension: &'static str,
    pub scale_to_canonical: &'static str,
    pub affine_offset: Option<&'static str>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UnitDerivation {
    pub name: String,
    pub expression: Option<String>,
    pub source_unit: Option<String>,
    pub display_unit: String,
    pub canonical_unit: String,
    pub quantity_kind: String,
    pub steps: Vec<String>,
    pub line: usize,
}

pub const UNIT_INFOS: &[UnitInfo] = &[
    UnitInfo {
        symbol: "m",
        canonical_unit: "m",
        quantity_hint: "Length",
        dimension: "Length",
        scale_to_canonical: "1",
        affine_offset: None,
    },
    UnitInfo {
        symbol: "cm",
        canonical_unit: "m",
        quantity_hint: "Length",
        dimension: "Length",
        scale_to_canonical: "0.01",
        affine_offset: None,
    },
    UnitInfo {
        symbol: "mm",
        canonical_unit: "m",
        quantity_hint: "Length",
        dimension: "Length",
        scale_to_canonical: "0.001",
        affine_offset: None,
    },
    UnitInfo {
        symbol: "m2",
        canonical_unit: "m2",
        quantity_hint: "Area",
        dimension: "Area",
        scale_to_canonical: "1",
        affine_offset: None,
    },
    UnitInfo {
        symbol: "m^2",
        canonical_unit: "m2",
        quantity_hint: "Area",
        dimension: "Area",
        scale_to_canonical: "1",
        affine_offset: None,
    },
    UnitInfo {
        symbol: "m3",
        canonical_unit: "m3",
        quantity_hint: "Volume",
        dimension: "Volume",
        scale_to_canonical: "1",
        affine_offset: None,
    },
    UnitInfo {
        symbol: "m^3",
        canonical_unit: "m3",
        quantity_hint: "Volume",
        dimension: "Volume",
        scale_to_canonical: "1",
        affine_offset: None,
    },
    UnitInfo {
        symbol: "1",
        canonical_unit: "1",
        quantity_hint: "DimensionlessNumber",
        dimension: "Dimensionless",
        scale_to_canonical: "1",
        affine_offset: None,
    },
    UnitInfo {
        symbol: "%",
        canonical_unit: "1",
        quantity_hint: "Ratio",
        dimension: "Dimensionless",
        scale_to_canonical: "0.01",
        affine_offset: None,
    },
    UnitInfo {
        symbol: "K",
        canonical_unit: "K",
        quantity_hint: "TemperatureDelta",
        dimension: "Temperature",
        scale_to_canonical: "1",
        affine_offset: None,
    },
    UnitInfo {
        symbol: "degC",
        canonical_unit: "K",
        quantity_hint: "AbsoluteTemperature",
        dimension: "Temperature",
        scale_to_canonical: "1",
        affine_offset: Some("273.15"),
    },
    UnitInfo {
        symbol: "°C",
        canonical_unit: "K",
        quantity_hint: "AbsoluteTemperature",
        dimension: "Temperature",
        scale_to_canonical: "1",
        affine_offset: Some("273.15"),
    },
    UnitInfo {
        symbol: "W",
        canonical_unit: "W",
        quantity_hint: "Power",
        dimension: "Power",
        scale_to_canonical: "1",
        affine_offset: None,
    },
    UnitInfo {
        symbol: "kW",
        canonical_unit: "W",
        quantity_hint: "Power",
        dimension: "Power",
        scale_to_canonical: "1000",
        affine_offset: None,
    },
    UnitInfo {
        symbol: "J",
        canonical_unit: "J",
        quantity_hint: "Energy",
        dimension: "Energy",
        scale_to_canonical: "1",
        affine_offset: None,
    },
    UnitInfo {
        symbol: "Wh",
        canonical_unit: "J",
        quantity_hint: "Energy",
        dimension: "Energy",
        scale_to_canonical: "3600",
        affine_offset: None,
    },
    UnitInfo {
        symbol: "kWh",
        canonical_unit: "J",
        quantity_hint: "Energy",
        dimension: "Energy",
        scale_to_canonical: "3600000",
        affine_offset: None,
    },
    UnitInfo {
        symbol: "MJ",
        canonical_unit: "J",
        quantity_hint: "Energy",
        dimension: "Energy",
        scale_to_canonical: "1000000",
        affine_offset: None,
    },
    UnitInfo {
        symbol: "W/K",
        canonical_unit: "W/K",
        quantity_hint: "Conductance",
        dimension: "Power/Temperature",
        scale_to_canonical: "1",
        affine_offset: None,
    },
    UnitInfo {
        symbol: "J/kg/K",
        canonical_unit: "J/kg/K",
        quantity_hint: "SpecificHeat",
        dimension: "Energy/Mass/Temperature",
        scale_to_canonical: "1",
        affine_offset: None,
    },
    UnitInfo {
        symbol: "J/K",
        canonical_unit: "J/K",
        quantity_hint: "HeatCapacity",
        dimension: "Energy/Temperature",
        scale_to_canonical: "1",
        affine_offset: None,
    },
    UnitInfo {
        symbol: "kJ/K",
        canonical_unit: "J/K",
        quantity_hint: "HeatCapacity",
        dimension: "Energy/Temperature",
        scale_to_canonical: "1000",
        affine_offset: None,
    },
    UnitInfo {
        symbol: "Pa",
        canonical_unit: "Pa",
        quantity_hint: "Pressure",
        dimension: "Pressure",
        scale_to_canonical: "1",
        affine_offset: None,
    },
    UnitInfo {
        symbol: "kPa",
        canonical_unit: "Pa",
        quantity_hint: "Pressure",
        dimension: "Pressure",
        scale_to_canonical: "1000",
        affine_offset: None,
    },
    UnitInfo {
        symbol: "s",
        canonical_unit: "s",
        quantity_hint: "Duration",
        dimension: "Time",
        scale_to_canonical: "1",
        affine_offset: None,
    },
    UnitInfo {
        symbol: "min",
        canonical_unit: "s",
        quantity_hint: "Duration",
        dimension: "Time",
        scale_to_canonical: "60",
        affine_offset: None,
    },
    UnitInfo {
        symbol: "h",
        canonical_unit: "s",
        quantity_hint: "Duration",
        dimension: "Time",
        scale_to_canonical: "3600",
        affine_offset: None,
    },
    UnitInfo {
        symbol: "kg/s",
        canonical_unit: "kg/s",
        quantity_hint: "MassFlowRate",
        dimension: "Mass/Time",
        scale_to_canonical: "1",
        affine_offset: None,
    },
    UnitInfo {
        symbol: "W/m2",
        canonical_unit: "W/m2",
        quantity_hint: "Irradiance",
        dimension: "Power/Area",
        scale_to_canonical: "1",
        affine_offset: None,
    },
    UnitInfo {
        symbol: "W/m^2",
        canonical_unit: "W/m2",
        quantity_hint: "Irradiance",
        dimension: "Power/Area",
        scale_to_canonical: "1",
        affine_offset: None,
    },
    UnitInfo {
        symbol: "person/m2",
        canonical_unit: "person/m2",
        quantity_hint: "PeopleDensity",
        dimension: "Count/Area",
        scale_to_canonical: "1",
        affine_offset: None,
    },
    UnitInfo {
        symbol: "people/m2",
        canonical_unit: "person/m2",
        quantity_hint: "PeopleDensity",
        dimension: "Count/Area",
        scale_to_canonical: "1",
        affine_offset: None,
    },
];

pub fn all_unit_infos() -> &'static [UnitInfo] {
    UNIT_INFOS
}

pub fn unit_info_count() -> usize {
    UNIT_INFOS.len()
}

pub fn unit_info_for_symbol(symbol: &str) -> Option<UnitInfo> {
    if let Some(info) = UNIT_INFOS
        .iter()
        .find(|unit| unit.symbol.eq_ignore_ascii_case(symbol))
        .copied()
    {
        return Some(info);
    }

    let normalized_symbol = normalize_unit(symbol);
    UNIT_INFOS
        .iter()
        .find(|unit| normalize_unit(unit.symbol) == normalized_symbol)
        .copied()
}

pub fn unit_derivation(
    name: &str,
    expression: Option<&str>,
    quantity_kind: &str,
    display_unit: &str,
    canonical_unit: &str,
    line: usize,
) -> UnitDerivation {
    let source_unit = expression.and_then(first_unit_in_expression);
    let mut steps = Vec::new();

    if let Some(unit) = source_unit.as_deref() {
        if let Some(info) = unit_info_for_symbol(unit) {
            steps.push(format!(
                "{} -> {} using scale {}",
                info.symbol, info.canonical_unit, info.scale_to_canonical
            ));
            if let Some(offset) = info.affine_offset {
                steps.push(format!("affine offset {}", offset));
            }
        } else {
            steps.push(format!("{unit} has no registered conversion rule"));
        }
    } else {
        steps.push("no source unit literal found in current expression".to_owned());
    }

    if display_unit != canonical_unit {
        steps.push(format!(
            "display unit {} is preserved while canonical unit is {}",
            display_unit, canonical_unit
        ));
    }

    UnitDerivation {
        name: name.to_owned(),
        expression: expression.map(str::to_owned),
        source_unit,
        display_unit: display_unit.to_owned(),
        canonical_unit: canonical_unit.to_owned(),
        quantity_kind: quantity_kind.to_owned(),
        steps,
        line,
    }
}
