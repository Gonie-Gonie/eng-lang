use super::diagnostics::SolverFailure;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ResidualInput {
    pub x: Vec<f64>,
    pub xdot: Option<Vec<f64>>,
    pub z: Vec<f64>,
    pub u: Vec<f64>,
    pub p: Vec<f64>,
    pub t: f64,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ResidualOutput {
    pub residuals: Vec<f64>,
    pub named_residuals: Vec<NamedResidualValue>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct NamedResidualValue {
    pub name: String,
    pub value: f64,
    pub normalized_value: f64,
}

pub trait ResidualEvaluator {
    fn evaluate(&self, input: &ResidualInput) -> Result<ResidualOutput, SolverFailure>;
}

pub struct ClosureResidualEvaluator<F>
where
    F: Fn(&ResidualInput) -> Result<ResidualOutput, SolverFailure>,
{
    evaluator: F,
}

impl<F> ClosureResidualEvaluator<F>
where
    F: Fn(&ResidualInput) -> Result<ResidualOutput, SolverFailure>,
{
    pub fn new(evaluator: F) -> Self {
        Self { evaluator }
    }
}

impl<F> ResidualEvaluator for ClosureResidualEvaluator<F>
where
    F: Fn(&ResidualInput) -> Result<ResidualOutput, SolverFailure>,
{
    fn evaluate(&self, input: &ResidualInput) -> Result<ResidualOutput, SolverFailure> {
        (self.evaluator)(input)
    }
}
