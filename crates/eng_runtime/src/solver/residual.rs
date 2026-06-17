#[derive(Clone, Debug, Default, PartialEq)]
pub struct ResidualGraph {
    pub name: String,
    pub residuals: Vec<ResidualEquation>,
    pub variables: Vec<ResidualVariableRef>,
    pub dependencies: Vec<(String, String)>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ResidualEquation {
    pub name: String,
    pub expression: ResidualExpression,
    pub unit: ResidualUnit,
    pub scale: ResidualScale,
    pub source: ResidualSource,
    pub variable_indices: Vec<usize>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ResidualVariableRef {
    pub index: usize,
    pub name: String,
    pub role: String,
    pub unit: String,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ResidualExpression {
    pub text: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ResidualScale {
    pub value: f64,
    pub policy: String,
}

impl Default for ResidualScale {
    fn default() -> Self {
        Self {
            value: 1.0,
            policy: "unit_nominal_default".to_owned(),
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ResidualUnit {
    pub unit: String,
    pub quantity_kind: String,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ResidualSource {
    pub line: Option<usize>,
    pub generated_reason: Option<String>,
}
