use std::collections::{HashMap, HashSet};

use crate::solver::algorithms::fixed_point::{solve_fixed_point, FixedPointOptions};
use crate::solver::algorithms::linear::solve_dense_linear_system;
use crate::solver::algorithms::nonlinear::{solve_newton, NewtonOptions};
use crate::solver::assembly::EquationAssembly;
use crate::solver::{
    OutputLayout, ResidualGraph, ResidualScale, SimulationPlan, SolverDiagnostics, SolverFailure,
    SolverInput, SolverOptions, SolverOutput, SolverPlan, SolverResult, SolverScalar, StateLayout,
    StateTrajectory, TimeGrid,
};

#[derive(Clone, Debug, Default, PartialEq)]
pub struct DynamicComponentOptions {
    pub algebraic: FixedPointOptions,
    pub algebraic_residual_scales: Option<Vec<f64>>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DynamicComponentStepDiagnostic {
    pub step_index: usize,
    pub time_s: f64,
    pub algebraic_iteration_count: usize,
    pub residual_norm: f64,
    pub residual_values: Vec<f64>,
    pub normalized_residual_values: Vec<f64>,
    pub convergence_status: String,
    pub failure: Option<SolverFailure>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DynamicComponentResult {
    pub solver_result: SolverResult,
    pub algebraic_layout: StateLayout,
    pub algebraic_trajectories: Vec<StateTrajectory>,
    pub step_diagnostics: Vec<DynamicComponentStepDiagnostic>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DynamicComponentAssemblySolveInput {
    pub duration_s: f64,
    pub timestep_s: f64,
    pub initial_state: Vec<f64>,
    pub initial_algebraic: Vec<f64>,
    pub inputs: Vec<SolverScalar>,
    pub parameters: Vec<SolverScalar>,
}

pub struct AlgebraicStepInput<'a> {
    pub time_s: f64,
    pub step_index: usize,
    pub state: &'a [f64],
    pub algebraic: &'a [f64],
    pub inputs: &'a [SolverScalar],
    pub parameters: &'a [SolverScalar],
}

pub struct DynamicStepInput<'a> {
    pub time_s: f64,
    pub step_index: usize,
    pub state: &'a [f64],
    pub algebraic: &'a [f64],
    pub inputs: &'a [SolverScalar],
    pub parameters: &'a [SolverScalar],
}

#[derive(Clone, Debug, PartialEq)]
pub struct ResidualGraphRhsEvaluator {
    equations: Vec<ResidualRhsEquation>,
    state_count: usize,
    algebraic_count: usize,
    input_count: usize,
    parameter_count: usize,
    state_names: Vec<String>,
    algebraic_names: Vec<String>,
    input_names: Vec<String>,
    parameter_names: Vec<String>,
}

#[derive(Clone, Debug, PartialEq)]
struct ResidualRhsEquation {
    residual_name: String,
    derivative_variable: String,
    derivative_coefficient: f64,
    rhs_value: f64,
    terms: Vec<ResidualRhsTerm>,
}

#[derive(Clone, Debug, PartialEq)]
struct ResidualRhsTerm {
    role: ResidualRhsRole,
    local_index: usize,
    coefficient: f64,
}

#[derive(Clone, Debug, PartialEq)]
struct ResidualGraphAlgebraicLinearEvaluator {
    equations: Vec<ResidualAlgebraicEquation>,
    state_count: usize,
    algebraic_count: usize,
    input_count: usize,
    parameter_count: usize,
    state_names: Vec<String>,
    algebraic_names: Vec<String>,
    input_names: Vec<String>,
    parameter_names: Vec<String>,
}

#[derive(Clone, Debug, PartialEq)]
struct ResidualAlgebraicEquation {
    residual_name: String,
    rhs_value: f64,
    scale_value: f64,
    terms: Vec<ResidualRhsTerm>,
}

#[derive(Clone, Debug, PartialEq)]
struct AlgebraicStepSolveResult {
    values: Vec<f64>,
    iteration_count: usize,
    residual_norm: f64,
    residual_values: Vec<f64>,
    normalized_residual_values: Vec<f64>,
    convergence_status: String,
    failure: Option<SolverFailure>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ResidualRhsRole {
    State,
    Algebraic,
    Input,
    Parameter,
    Time,
}

#[derive(Clone, Debug, Default)]
struct ResidualRhsRoleCounts {
    state: usize,
    algebraic: usize,
    input: usize,
    parameter: usize,
}

#[derive(Clone, Debug, Default)]
struct ResidualRhsRoleNames {
    state: Vec<String>,
    algebraic: Vec<String>,
    input: Vec<String>,
    parameter: Vec<String>,
}

impl ResidualGraphRhsEvaluator {
    pub fn new(graph: &ResidualGraph) -> Result<Self, SolverFailure> {
        let mut counts = ResidualRhsRoleCounts::default();
        let mut names = ResidualRhsRoleNames::default();
        let mut variable_slots = HashMap::new();
        let mut derivative_variables = HashMap::new();

        for variable in &graph.variables {
            if is_derivative_role(&variable.role) {
                if derivative_variables
                    .insert(variable.index, variable.name.clone())
                    .is_some()
                {
                    return Err(SolverFailure::new(
                        "E-DYNAMIC-COMPONENT-RHS-VARIABLE",
                        format!("duplicate derivative variable index {}", variable.index),
                    ));
                }
                continue;
            }

            let role = residual_rhs_role(&variable.role).ok_or_else(|| {
                SolverFailure::new(
                    "E-DYNAMIC-COMPONENT-RHS-VARIABLE",
                    format!(
                        "residual RHS graph variable `{}` has unsupported role `{}`",
                        variable.name, variable.role
                    ),
                )
            })?;
            let local_index = counts.allocate(role);
            names.record(role, local_index, variable.name.clone());
            if variable_slots
                .insert(variable.index, (role, local_index))
                .is_some()
            {
                return Err(SolverFailure::new(
                    "E-DYNAMIC-COMPONENT-RHS-VARIABLE",
                    format!("duplicate residual RHS variable index {}", variable.index),
                ));
            }
        }

        if counts.state == 0 {
            return Err(SolverFailure::new(
                "E-DYNAMIC-COMPONENT-RHS-SHAPE",
                "residual RHS graph requires at least one state variable",
            ));
        }
        if derivative_variables.len() != counts.state {
            return Err(SolverFailure::new(
                "E-DYNAMIC-COMPONENT-RHS-SHAPE",
                "residual RHS graph requires one derivative variable per state",
            ));
        }

        let mut equations_by_derivative = Vec::new();
        for residual in &graph.residuals {
            if !residual.rhs_value.is_finite() {
                return Err(SolverFailure::new(
                    "E-DYNAMIC-COMPONENT-RHS-VALUE",
                    format!("residual `{}` has a non-finite RHS value", residual.name),
                ));
            }
            let derivative_terms = residual
                .terms
                .iter()
                .filter(|term| derivative_variables.contains_key(&term.variable_index))
                .collect::<Vec<_>>();
            if derivative_terms.len() != 1 {
                return Err(SolverFailure::new(
                    "E-DYNAMIC-COMPONENT-RHS-SHAPE",
                    format!(
                        "residual `{}` must contain exactly one derivative variable",
                        residual.name
                    ),
                ));
            }
            let derivative_term = derivative_terms[0];
            if !derivative_term.coefficient.is_finite() || derivative_term.coefficient == 0.0 {
                return Err(SolverFailure::new(
                    "E-DYNAMIC-COMPONENT-RHS-VALUE",
                    format!(
                        "residual `{}` derivative coefficient must be a finite non-zero value",
                        residual.name
                    ),
                ));
            }

            let mut terms = Vec::new();
            for term in &residual.terms {
                if !term.coefficient.is_finite() {
                    return Err(SolverFailure::new(
                        "E-DYNAMIC-COMPONENT-RHS-VALUE",
                        format!(
                            "residual `{}` term for `{}` has a non-finite coefficient",
                            residual.name, term.variable
                        ),
                    ));
                }
                if term.variable_index == derivative_term.variable_index {
                    continue;
                }
                let (role, local_index) =
                    variable_slots.get(&term.variable_index).ok_or_else(|| {
                        SolverFailure::new(
                            "E-DYNAMIC-COMPONENT-RHS-VARIABLE",
                            format!(
                                "residual `{}` references unsupported RHS variable `{}`",
                                residual.name, term.variable
                            ),
                        )
                    })?;
                terms.push(ResidualRhsTerm {
                    role: *role,
                    local_index: *local_index,
                    coefficient: term.coefficient,
                });
            }

            equations_by_derivative.push((
                derivative_term.variable_index,
                ResidualRhsEquation {
                    residual_name: residual.name.clone(),
                    derivative_variable: derivative_variables
                        .get(&derivative_term.variable_index)
                        .cloned()
                        .unwrap_or_else(|| term_variable_name(derivative_term.variable_index)),
                    derivative_coefficient: derivative_term.coefficient,
                    rhs_value: residual.rhs_value,
                    terms,
                },
            ));
        }

        equations_by_derivative.sort_by_key(|(variable_index, _)| *variable_index);
        if equations_by_derivative
            .windows(2)
            .any(|pair| pair[0].0 == pair[1].0)
        {
            return Err(SolverFailure::new(
                "E-DYNAMIC-COMPONENT-RHS-SHAPE",
                "residual RHS graph contains multiple residuals for the same derivative variable",
            ));
        }
        if equations_by_derivative.len() != counts.state {
            return Err(SolverFailure::new(
                "E-DYNAMIC-COMPONENT-RHS-SHAPE",
                "residual RHS graph requires one derivative residual per state",
            ));
        }

        Ok(Self {
            equations: equations_by_derivative
                .into_iter()
                .map(|(_, equation)| equation)
                .collect(),
            state_count: counts.state,
            algebraic_count: counts.algebraic,
            input_count: counts.input,
            parameter_count: counts.parameter,
            state_names: names.state,
            algebraic_names: names.algebraic,
            input_names: names.input,
            parameter_names: names.parameter,
        })
    }

    pub fn evaluate(&self, sample: &DynamicStepInput<'_>) -> Result<Vec<f64>, SolverFailure> {
        if sample.state.len() != self.state_count
            || sample.algebraic.len() != self.algebraic_count
            || sample.inputs.len() != self.input_count
            || sample.parameters.len() != self.parameter_count
        {
            return Err(SolverFailure::new(
                "E-DYNAMIC-COMPONENT-RHS-LAYOUT",
                "residual RHS sample layout does not match residual graph variables",
            ));
        }
        if !sample.time_s.is_finite() {
            return Err(SolverFailure::new(
                "E-DYNAMIC-COMPONENT-RHS-VALUE",
                "residual RHS sample time must be finite",
            ));
        }
        ensure_finite_values(
            "E-DYNAMIC-COMPONENT-RHS-VALUE",
            "residual RHS state",
            sample.state,
        )?;
        ensure_finite_values(
            "E-DYNAMIC-COMPONENT-RHS-VALUE",
            "residual RHS algebraic",
            sample.algebraic,
        )?;
        ensure_finite_scalars(
            "E-DYNAMIC-COMPONENT-RHS-VALUE",
            "residual RHS input",
            sample.inputs,
        )?;
        ensure_finite_scalars(
            "E-DYNAMIC-COMPONENT-RHS-VALUE",
            "residual RHS parameter",
            sample.parameters,
        )?;

        let mut derivatives = Vec::with_capacity(self.equations.len());
        for equation in &self.equations {
            let mut remaining = equation.rhs_value;
            for term in &equation.terms {
                let value = match term.role {
                    ResidualRhsRole::State => sample.state[term.local_index],
                    ResidualRhsRole::Algebraic => sample.algebraic[term.local_index],
                    ResidualRhsRole::Input => sample.inputs[term.local_index].value,
                    ResidualRhsRole::Parameter => sample.parameters[term.local_index].value,
                    ResidualRhsRole::Time => sample.time_s,
                };
                remaining -= term.coefficient * value;
                if !remaining.is_finite() {
                    return Err(SolverFailure::new(
                        "E-DYNAMIC-COMPONENT-RHS-VALUE",
                        format!(
                            "residual `{}` produced a non-finite RHS accumulator",
                            equation.residual_name
                        ),
                    ));
                }
            }

            let derivative = remaining / equation.derivative_coefficient;
            if !derivative.is_finite() {
                return Err(SolverFailure::new(
                    "E-DYNAMIC-COMPONENT-RHS-VALUE",
                    format!(
                        "residual `{}` produced a non-finite derivative for `{}`",
                        equation.residual_name, equation.derivative_variable
                    ),
                ));
            }
            derivatives.push(derivative);
        }

        Ok(derivatives)
    }
}

impl ResidualGraphAlgebraicLinearEvaluator {
    fn new(graph: &ResidualGraph) -> Result<Self, SolverFailure> {
        let mut counts = ResidualRhsRoleCounts::default();
        let mut names = ResidualRhsRoleNames::default();
        let mut variable_slots = HashMap::new();
        let mut derivative_variables = HashSet::new();

        for variable in &graph.variables {
            if is_derivative_role(&variable.role) {
                if !derivative_variables.insert(variable.index) {
                    return Err(SolverFailure::new(
                        "E-DYNAMIC-COMPONENT-ALGEBRAIC-VARIABLE",
                        format!("duplicate derivative variable index {}", variable.index),
                    ));
                }
                continue;
            }

            let role = residual_rhs_role(&variable.role).ok_or_else(|| {
                SolverFailure::new(
                    "E-DYNAMIC-COMPONENT-ALGEBRAIC-VARIABLE",
                    format!(
                        "residual algebraic graph variable `{}` has unsupported role `{}`",
                        variable.name, variable.role
                    ),
                )
            })?;
            let local_index = counts.allocate(role);
            names.record(role, local_index, variable.name.clone());
            if variable_slots
                .insert(variable.index, (role, local_index))
                .is_some()
            {
                return Err(SolverFailure::new(
                    "E-DYNAMIC-COMPONENT-ALGEBRAIC-VARIABLE",
                    format!(
                        "duplicate residual algebraic variable index {}",
                        variable.index
                    ),
                ));
            }
        }

        if counts.algebraic == 0 {
            return Err(SolverFailure::new(
                "E-DYNAMIC-COMPONENT-ALGEBRAIC-SHAPE",
                "residual graph semi-implicit solve requires at least one algebraic variable",
            ));
        }

        let mut equations = Vec::new();
        for residual in &graph.residuals {
            let derivative_term_count = residual
                .terms
                .iter()
                .filter(|term| derivative_variables.contains(&term.variable_index))
                .count();
            if derivative_term_count > 1 {
                return Err(SolverFailure::new(
                    "E-DYNAMIC-COMPONENT-ALGEBRAIC-SHAPE",
                    format!(
                        "residual `{}` contains multiple derivative variables",
                        residual.name
                    ),
                ));
            }
            if derivative_term_count == 1 {
                continue;
            }
            if !residual.rhs_value.is_finite() {
                return Err(SolverFailure::new(
                    "E-DYNAMIC-COMPONENT-ALGEBRAIC-VALUE",
                    format!("residual `{}` has a non-finite RHS value", residual.name),
                ));
            }

            let mut terms = Vec::new();
            for term in &residual.terms {
                if !term.coefficient.is_finite() {
                    return Err(SolverFailure::new(
                        "E-DYNAMIC-COMPONENT-ALGEBRAIC-VALUE",
                        format!(
                            "residual `{}` term for `{}` has a non-finite coefficient",
                            residual.name, term.variable
                        ),
                    ));
                }
                let (role, local_index) =
                    variable_slots.get(&term.variable_index).ok_or_else(|| {
                        SolverFailure::new(
                            "E-DYNAMIC-COMPONENT-ALGEBRAIC-VARIABLE",
                            format!(
                                "residual `{}` references unsupported algebraic variable `{}`",
                                residual.name, term.variable
                            ),
                        )
                    })?;
                terms.push(ResidualRhsTerm {
                    role: *role,
                    local_index: *local_index,
                    coefficient: term.coefficient,
                });
            }
            equations.push(ResidualAlgebraicEquation {
                residual_name: residual.name.clone(),
                rhs_value: residual.rhs_value,
                scale_value: residual.scale.value,
                terms,
            });
        }

        if equations.len() != counts.algebraic {
            return Err(SolverFailure::new(
                "E-DYNAMIC-COMPONENT-ALGEBRAIC-SHAPE",
                format!(
                    "residual graph semi-implicit solve requires one algebraic residual per algebraic variable, got {} residual(s) for {} variable(s)",
                    equations.len(),
                    counts.algebraic
                ),
            ));
        }

        Ok(Self {
            equations,
            state_count: counts.state,
            algebraic_count: counts.algebraic,
            input_count: counts.input,
            parameter_count: counts.parameter,
            state_names: names.state,
            algebraic_names: names.algebraic,
            input_names: names.input,
            parameter_names: names.parameter,
        })
    }

    fn solve(
        &self,
        sample: &AlgebraicStepInput<'_>,
        tolerance: f64,
    ) -> Result<AlgebraicStepSolveResult, SolverFailure> {
        if sample.state.len() != self.state_count
            || sample.algebraic.len() != self.algebraic_count
            || sample.inputs.len() != self.input_count
            || sample.parameters.len() != self.parameter_count
        {
            return Err(SolverFailure::new(
                "E-DYNAMIC-COMPONENT-ALGEBRAIC-LAYOUT",
                "residual algebraic sample layout does not match residual graph variables",
            ));
        }
        if !sample.time_s.is_finite() {
            return Err(SolverFailure::new(
                "E-DYNAMIC-COMPONENT-ALGEBRAIC-VALUE",
                "residual algebraic sample time must be finite",
            ));
        }
        ensure_finite_values(
            "E-DYNAMIC-COMPONENT-ALGEBRAIC-VALUE",
            "residual algebraic state",
            sample.state,
        )?;
        ensure_finite_values(
            "E-DYNAMIC-COMPONENT-ALGEBRAIC-VALUE",
            "residual algebraic guess",
            sample.algebraic,
        )?;
        ensure_finite_scalars(
            "E-DYNAMIC-COMPONENT-ALGEBRAIC-VALUE",
            "residual algebraic input",
            sample.inputs,
        )?;
        ensure_finite_scalars(
            "E-DYNAMIC-COMPONENT-ALGEBRAIC-VALUE",
            "residual algebraic parameter",
            sample.parameters,
        )?;

        let mut matrix = vec![vec![0.0; self.algebraic_count]; self.equations.len()];
        let mut rhs = self
            .equations
            .iter()
            .map(|equation| equation.rhs_value)
            .collect::<Vec<_>>();
        for (row_index, equation) in self.equations.iter().enumerate() {
            for term in &equation.terms {
                match term.role {
                    ResidualRhsRole::Algebraic => {
                        matrix[row_index][term.local_index] += term.coefficient;
                    }
                    ResidualRhsRole::State => {
                        rhs[row_index] -= term.coefficient * sample.state[term.local_index];
                    }
                    ResidualRhsRole::Input => {
                        rhs[row_index] -= term.coefficient * sample.inputs[term.local_index].value;
                    }
                    ResidualRhsRole::Parameter => {
                        rhs[row_index] -=
                            term.coefficient * sample.parameters[term.local_index].value;
                    }
                    ResidualRhsRole::Time => {
                        rhs[row_index] -= term.coefficient * sample.time_s;
                    }
                }
                if !rhs[row_index].is_finite()
                    || matrix[row_index].iter().any(|value| !value.is_finite())
                {
                    return Err(SolverFailure::new(
                        "E-DYNAMIC-COMPONENT-ALGEBRAIC-VALUE",
                        format!(
                            "residual `{}` produced a non-finite algebraic linear system",
                            equation.residual_name
                        ),
                    ));
                }
            }
        }

        match solve_dense_linear_system(&matrix, &rhs, tolerance) {
            Ok(solution) => {
                let failure = if solution.status == "converged" {
                    None
                } else {
                    Some(SolverFailure::new(
                        "E-DYNAMIC-COMPONENT-ALGEBRAIC-RESIDUAL",
                        format!(
                            "linear algebraic solve residual norm {} exceeded tolerance {}",
                            solution.residual_norm, tolerance
                        ),
                    ))
                };
                Ok(AlgebraicStepSolveResult {
                    values: solution.values,
                    iteration_count: 1,
                    residual_norm: solution.residual_norm,
                    residual_values: solution.residual_values.clone(),
                    normalized_residual_values: normalize_residual_values(
                        &solution.residual_values,
                        &self.residual_scales(),
                    ),
                    convergence_status: if failure.is_none() {
                        "linear_algebraic_converged".to_owned()
                    } else {
                        "linear_algebraic_residual_above_tolerance".to_owned()
                    },
                    failure,
                })
            }
            Err(failure) => Ok(AlgebraicStepSolveResult {
                values: sample.algebraic.to_vec(),
                iteration_count: 1,
                residual_norm: 0.0,
                residual_values: Vec::new(),
                normalized_residual_values: Vec::new(),
                convergence_status: "linear_algebraic_solve_failed".to_owned(),
                failure: Some(failure),
            }),
        }
    }

    fn residual_scales(&self) -> Vec<f64> {
        self.equations
            .iter()
            .map(|equation| equation.scale_value)
            .collect()
    }
}

impl ResidualRhsRoleCounts {
    fn allocate(&mut self, role: ResidualRhsRole) -> usize {
        match role {
            ResidualRhsRole::State => {
                let index = self.state;
                self.state += 1;
                index
            }
            ResidualRhsRole::Algebraic => {
                let index = self.algebraic;
                self.algebraic += 1;
                index
            }
            ResidualRhsRole::Input => {
                let index = self.input;
                self.input += 1;
                index
            }
            ResidualRhsRole::Parameter => {
                let index = self.parameter;
                self.parameter += 1;
                index
            }
            ResidualRhsRole::Time => 0,
        }
    }
}

impl ResidualRhsRoleNames {
    fn record(&mut self, role: ResidualRhsRole, local_index: usize, name: String) {
        let names = match role {
            ResidualRhsRole::State => &mut self.state,
            ResidualRhsRole::Algebraic => &mut self.algebraic,
            ResidualRhsRole::Input => &mut self.input,
            ResidualRhsRole::Parameter => &mut self.parameter,
            ResidualRhsRole::Time => return,
        };
        debug_assert_eq!(names.len(), local_index);
        names.push(name);
    }
}

fn is_derivative_role(role: &str) -> bool {
    matches!(role, "derivative" | "state_derivative" | "xdot")
}

fn residual_rhs_role(role: &str) -> Option<ResidualRhsRole> {
    match role {
        "state" => Some(ResidualRhsRole::State),
        "algebraic" => Some(ResidualRhsRole::Algebraic),
        "input" => Some(ResidualRhsRole::Input),
        "parameter" => Some(ResidualRhsRole::Parameter),
        "time" => Some(ResidualRhsRole::Time),
        _ => None,
    }
}

fn term_variable_name(variable_index: usize) -> String {
    format!("variable#{variable_index}")
}

pub fn solve_explicit_euler_with_algebraic<A, R>(
    input: &SolverInput,
    algebraic_layout: StateLayout,
    initial_algebraic: Vec<f64>,
    options: DynamicComponentOptions,
    mut algebraic_update: A,
    rhs: R,
) -> Result<DynamicComponentResult, SolverFailure>
where
    A: FnMut(AlgebraicStepInput<'_>) -> Result<Vec<f64>, SolverFailure>,
    R: FnMut(DynamicStepInput<'_>) -> Result<Vec<f64>, SolverFailure>,
{
    let fixed_point_options = options.algebraic.clone();
    let algebraic_residual_scales = options
        .algebraic_residual_scales
        .clone()
        .unwrap_or_else(|| layout_residual_scales(&algebraic_layout));
    solve_explicit_euler_with_algebraic_solver(
        input,
        algebraic_layout,
        initial_algebraic,
        options,
        |sample| {
            let fixed_point = solve_fixed_point(sample.algebraic, &fixed_point_options, |guess| {
                algebraic_update(AlgebraicStepInput {
                    time_s: sample.time_s,
                    step_index: sample.step_index,
                    state: sample.state,
                    algebraic: guess,
                    inputs: sample.inputs,
                    parameters: sample.parameters,
                })
            })?;
            Ok(AlgebraicStepSolveResult {
                values: fixed_point.values,
                iteration_count: fixed_point.iteration_count,
                residual_norm: fixed_point.residual_history.last().copied().unwrap_or(0.0),
                residual_values: fixed_point
                    .residual_value_history
                    .last()
                    .cloned()
                    .unwrap_or_default(),
                normalized_residual_values: normalize_residual_values(
                    fixed_point
                        .residual_value_history
                        .last()
                        .map(Vec::as_slice)
                        .unwrap_or(&[]),
                    &algebraic_residual_scales,
                ),
                convergence_status: fixed_point.convergence_status,
                failure: fixed_point.failure,
            })
        },
        rhs,
    )
}

pub fn solve_explicit_euler_with_newton_algebraic<A, R>(
    input: &SolverInput,
    algebraic_layout: StateLayout,
    initial_algebraic: Vec<f64>,
    options: DynamicComponentOptions,
    algebraic_residual: A,
    rhs: R,
) -> Result<DynamicComponentResult, SolverFailure>
where
    A: FnMut(AlgebraicStepInput<'_>) -> Result<Vec<f64>, SolverFailure>,
    R: FnMut(DynamicStepInput<'_>) -> Result<Vec<f64>, SolverFailure>,
{
    let static_inputs = input.inputs.clone();
    solve_explicit_euler_with_newton_algebraic_and_input_sampler(
        input,
        algebraic_layout,
        initial_algebraic,
        options,
        algebraic_residual,
        rhs,
        move |_| Ok(static_inputs.clone()),
    )
}

pub fn solve_explicit_euler_with_newton_algebraic_and_input_sampler<A, R, I>(
    input: &SolverInput,
    algebraic_layout: StateLayout,
    initial_algebraic: Vec<f64>,
    options: DynamicComponentOptions,
    mut algebraic_residual: A,
    rhs: R,
    input_values_at: I,
) -> Result<DynamicComponentResult, SolverFailure>
where
    A: FnMut(AlgebraicStepInput<'_>) -> Result<Vec<f64>, SolverFailure>,
    R: FnMut(DynamicStepInput<'_>) -> Result<Vec<f64>, SolverFailure>,
    I: FnMut(f64) -> Result<Vec<SolverScalar>, SolverFailure>,
{
    let mut newton_options = NewtonOptions {
        tolerance: options.algebraic.tolerance,
        max_iterations: options.algebraic.max_iterations,
        ..NewtonOptions::default()
    };
    newton_options.variable_scales = layout_residual_scales(&algebraic_layout);
    newton_options.variable_scale_policy = "algebraic_layout_quantity_unit".to_owned();
    let has_source_residual_scales = options.algebraic_residual_scales.is_some();
    let algebraic_residual_scales = options
        .algebraic_residual_scales
        .clone()
        .unwrap_or_else(|| layout_residual_scales(&algebraic_layout));
    solve_explicit_euler_with_algebraic_solver_with_input_sampler(
        input,
        algebraic_layout,
        initial_algebraic,
        options,
        |sample| {
            let newton = solve_newton(sample.algebraic, &newton_options, |guess| {
                algebraic_residual(AlgebraicStepInput {
                    time_s: sample.time_s,
                    step_index: sample.step_index,
                    state: sample.state,
                    algebraic: guess,
                    inputs: sample.inputs,
                    parameters: sample.parameters,
                })
            })?;
            let solver_residual_values = newton
                .residual_value_history
                .last()
                .cloned()
                .unwrap_or_default();
            let (residual_values, normalized_residual_values) = if has_source_residual_scales {
                (
                    restore_raw_residual_values(
                        &solver_residual_values,
                        &algebraic_residual_scales,
                    ),
                    solver_residual_values,
                )
            } else {
                (
                    solver_residual_values.clone(),
                    normalize_residual_values(&solver_residual_values, &algebraic_residual_scales),
                )
            };
            Ok(AlgebraicStepSolveResult {
                values: newton.values,
                iteration_count: newton.iteration_count,
                residual_norm: newton.residual_history.last().copied().unwrap_or(0.0),
                normalized_residual_values,
                residual_values,
                convergence_status: newton.convergence_status,
                failure: newton.failure,
            })
        },
        rhs,
        input_values_at,
    )
}

fn solve_explicit_euler_with_algebraic_solver<S, R>(
    input: &SolverInput,
    algebraic_layout: StateLayout,
    initial_algebraic: Vec<f64>,
    options: DynamicComponentOptions,
    algebraic_solve: S,
    rhs: R,
) -> Result<DynamicComponentResult, SolverFailure>
where
    S: FnMut(AlgebraicStepInput<'_>) -> Result<AlgebraicStepSolveResult, SolverFailure>,
    R: FnMut(DynamicStepInput<'_>) -> Result<Vec<f64>, SolverFailure>,
{
    let static_inputs = input.inputs.clone();
    solve_explicit_euler_with_algebraic_solver_with_input_sampler(
        input,
        algebraic_layout,
        initial_algebraic,
        options,
        algebraic_solve,
        rhs,
        move |_| Ok(static_inputs.clone()),
    )
}

fn solve_explicit_euler_with_algebraic_solver_with_input_sampler<S, R, I>(
    input: &SolverInput,
    algebraic_layout: StateLayout,
    initial_algebraic: Vec<f64>,
    options: DynamicComponentOptions,
    mut algebraic_solve: S,
    mut rhs: R,
    mut input_values_at: I,
) -> Result<DynamicComponentResult, SolverFailure>
where
    S: FnMut(AlgebraicStepInput<'_>) -> Result<AlgebraicStepSolveResult, SolverFailure>,
    R: FnMut(DynamicStepInput<'_>) -> Result<Vec<f64>, SolverFailure>,
    I: FnMut(f64) -> Result<Vec<SolverScalar>, SolverFailure>,
{
    if input.state_layout.is_empty() {
        return Err(SolverFailure::new(
            "E-DYNAMIC-COMPONENT-STATE-SHAPE",
            "dynamic component solver requires at least one state variable",
        ));
    }
    if input.initial_state.len() != input.state_layout.len() {
        return Err(SolverFailure::new(
            "E-DYNAMIC-COMPONENT-STATE-LAYOUT",
            "initial state vector length does not match the state layout",
        ));
    }
    input.validate_layouts()?;
    if initial_algebraic.len() != algebraic_layout.len() {
        return Err(SolverFailure::new(
            "E-DYNAMIC-COMPONENT-ALGEBRAIC-LAYOUT",
            "initial algebraic vector length does not match the algebraic layout",
        ));
    }
    ensure_finite_values(
        "E-DYNAMIC-COMPONENT-ALGEBRAIC-VALUE",
        "initial algebraic",
        &initial_algebraic,
    )?;

    let mut state = input.initial_state.clone();
    let mut algebraic = initial_algebraic;
    let mut state_values_by_state =
        vec![Vec::with_capacity(input.time_grid.step_count + 1); state.len()];
    let mut algebraic_values_by_variable =
        vec![Vec::with_capacity(input.time_grid.step_count + 1); algebraic.len()];
    for (index, value) in state.iter().copied().enumerate() {
        state_values_by_state[index].push(value);
    }

    let mut step_diagnostics = Vec::with_capacity(input.time_grid.step_count + 1);
    let mut total_iterations = 0usize;

    for step_index in 0..=input.time_grid.step_count {
        let time_s = input.time_grid.step_time_s(step_index);
        let sampled_inputs = input_values_at(time_s)?;
        validate_dynamic_component_input_sample(input, &sampled_inputs)?;
        let (
            algebraic_iteration_count,
            residual_norm,
            residual_values,
            normalized_residual_values,
            convergence_status,
            failure,
        ) = if algebraic.is_empty() {
            (
                0,
                0.0,
                Vec::new(),
                Vec::new(),
                "algebraic_not_required".to_owned(),
                None,
            )
        } else {
            let solve = algebraic_solve(AlgebraicStepInput {
                time_s,
                step_index,
                state: &state,
                algebraic: &algebraic,
                inputs: &sampled_inputs,
                parameters: &input.parameters,
            })?;
            if solve.values.len() != algebraic.len() {
                return Err(SolverFailure::new(
                        "E-DYNAMIC-COMPONENT-ALGEBRAIC-LAYOUT",
                        "dynamic component algebraic solve vector length does not match the algebraic layout",
                    ));
            }
            ensure_finite_values(
                "E-DYNAMIC-COMPONENT-ALGEBRAIC-VALUE",
                "dynamic component algebraic solve",
                &solve.values,
            )?;
            total_iterations += solve.iteration_count;
            algebraic = solve.values;
            (
                solve.iteration_count,
                solve.residual_norm,
                solve.residual_values,
                solve.normalized_residual_values,
                solve.convergence_status,
                solve.failure,
            )
        };

        for (index, value) in algebraic.iter().copied().enumerate() {
            algebraic_values_by_variable[index].push(value);
        }
        step_diagnostics.push(DynamicComponentStepDiagnostic {
            step_index,
            time_s,
            algebraic_iteration_count,
            residual_norm,
            residual_values,
            normalized_residual_values,
            convergence_status,
            failure: failure.clone(),
        });
        if let Some(failure) = failure {
            return Ok(dynamic_component_result(
                input,
                algebraic_layout,
                state_values_by_state,
                algebraic_values_by_variable,
                step_diagnostics,
                SolverDiagnostics {
                    status: "failed".to_owned(),
                    convergence_status: "algebraic_solve_failed".to_owned(),
                    failure: Some(failure),
                    iteration_count: total_iterations,
                    tolerance: options.algebraic.tolerance,
                    max_iterations: options.algebraic.max_iterations,
                },
            ));
        }

        if step_index == input.time_grid.step_count {
            break;
        }

        let dt = input.time_grid.step_dt_s(step_index + 1);
        let derivative = rhs(DynamicStepInput {
            time_s,
            step_index,
            state: &state,
            algebraic: &algebraic,
            inputs: &sampled_inputs,
            parameters: &input.parameters,
        })?;
        if derivative.len() != state.len() {
            return Err(SolverFailure::new(
                "E-DYNAMIC-COMPONENT-RHS-LAYOUT",
                "dynamic component RHS vector length does not match the state layout",
            ));
        }
        ensure_finite_values(
            "E-DYNAMIC-COMPONENT-RHS-VALUE",
            "dynamic component RHS",
            &derivative,
        )?;
        for (state_value, derivative_value) in state.iter_mut().zip(derivative) {
            *state_value += derivative_value * dt;
        }
        ensure_finite_values(
            "E-DYNAMIC-COMPONENT-STATE-VALUE",
            "dynamic component state",
            &state,
        )?;
        for (index, value) in state.iter().copied().enumerate() {
            state_values_by_state[index].push(value);
        }
    }

    Ok(dynamic_component_result(
        input,
        algebraic_layout,
        state_values_by_state,
        algebraic_values_by_variable,
        step_diagnostics,
        SolverDiagnostics {
            status: "computed".to_owned(),
            convergence_status: "dynamic_component_fixed_step_completed".to_owned(),
            failure: None,
            iteration_count: total_iterations,
            tolerance: options.algebraic.tolerance,
            max_iterations: options.algebraic.max_iterations,
        },
    ))
}

fn validate_dynamic_component_input_sample(
    input: &SolverInput,
    sampled_inputs: &[SolverScalar],
) -> Result<(), SolverFailure> {
    if sampled_inputs.len() != input.input_layout.len() {
        return Err(SolverFailure::new(
            "E-DYNAMIC-COMPONENT-INPUT-LAYOUT",
            "dynamic component sampled input count does not match the input layout",
        ));
    }
    for (entry, sampled) in input.input_layout.entries.iter().zip(sampled_inputs.iter()) {
        if entry.name != sampled.name
            || entry.quantity_kind != sampled.quantity_kind
            || entry.canonical_unit != sampled.canonical_unit
        {
            return Err(SolverFailure::new(
                "E-DYNAMIC-COMPONENT-INPUT-LAYOUT",
                format!(
                    "dynamic component sampled input `{}` does not match layout entry `{}`",
                    sampled.name, entry.name
                ),
            ));
        }
        if !sampled.value.is_finite() {
            return Err(SolverFailure::new(
                "E-DYNAMIC-COMPONENT-INPUT-VALUE",
                format!(
                    "dynamic component sampled input `{}` must be finite",
                    sampled.name
                ),
            ));
        }
    }
    Ok(())
}

pub fn solve_residual_graph_explicit_euler(
    input: &SolverInput,
    graph: &ResidualGraph,
    options: DynamicComponentOptions,
) -> Result<DynamicComponentResult, SolverFailure> {
    let evaluator = ResidualGraphRhsEvaluator::new(graph)?;
    if evaluator.algebraic_count != 0 {
        return Err(SolverFailure::new(
            "E-DYNAMIC-COMPONENT-RHS-SHAPE",
            "residual graph explicit-Euler solve requires an algebraic-free dynamic graph",
        ));
    }
    validate_residual_graph_solver_layout(input, &StateLayout::default(), &evaluator)?;
    solve_explicit_euler_with_algebraic(
        input,
        StateLayout::default(),
        Vec::new(),
        options,
        |_| Ok(Vec::new()),
        |sample| evaluator.evaluate(&sample),
    )
}

pub fn solve_residual_graph_semi_implicit_euler(
    input: &SolverInput,
    graph: &ResidualGraph,
    algebraic_layout: StateLayout,
    initial_algebraic: Vec<f64>,
    options: DynamicComponentOptions,
) -> Result<DynamicComponentResult, SolverFailure> {
    let derivative_graph = residual_graph_with_derivative_residuals(graph)?;
    let rhs_evaluator = ResidualGraphRhsEvaluator::new(&derivative_graph)?;
    validate_residual_graph_solver_layout(input, &algebraic_layout, &rhs_evaluator)?;

    solve_residual_graph_semi_implicit_euler_with_rhs(
        input,
        graph,
        algebraic_layout,
        initial_algebraic,
        options,
        |sample| rhs_evaluator.evaluate(&sample),
    )
}

pub fn solve_residual_graph_semi_implicit_euler_with_rhs<R>(
    input: &SolverInput,
    graph: &ResidualGraph,
    algebraic_layout: StateLayout,
    initial_algebraic: Vec<f64>,
    options: DynamicComponentOptions,
    rhs: R,
) -> Result<DynamicComponentResult, SolverFailure>
where
    R: FnMut(DynamicStepInput<'_>) -> Result<Vec<f64>, SolverFailure>,
{
    let static_inputs = input.inputs.clone();
    solve_residual_graph_semi_implicit_euler_with_rhs_and_input_sampler(
        input,
        graph,
        algebraic_layout,
        initial_algebraic,
        options,
        move |_| Ok(static_inputs.clone()),
        rhs,
    )
}

pub fn solve_residual_graph_semi_implicit_euler_with_rhs_and_input_sampler<R, I>(
    input: &SolverInput,
    graph: &ResidualGraph,
    algebraic_layout: StateLayout,
    initial_algebraic: Vec<f64>,
    options: DynamicComponentOptions,
    input_values_at: I,
    rhs: R,
) -> Result<DynamicComponentResult, SolverFailure>
where
    R: FnMut(DynamicStepInput<'_>) -> Result<Vec<f64>, SolverFailure>,
    I: FnMut(f64) -> Result<Vec<SolverScalar>, SolverFailure>,
{
    let algebraic_evaluator = ResidualGraphAlgebraicLinearEvaluator::new(graph)?;
    validate_residual_graph_algebraic_layout(input, &algebraic_layout, &algebraic_evaluator)?;

    let tolerance = options.algebraic.tolerance;
    solve_explicit_euler_with_algebraic_solver_with_input_sampler(
        input,
        algebraic_layout,
        initial_algebraic,
        options,
        |sample| algebraic_evaluator.solve(&sample, tolerance),
        rhs,
        input_values_at,
    )
}

pub fn solve_residual_graph_semi_implicit_euler_with_input_sampler<I>(
    input: &SolverInput,
    graph: &ResidualGraph,
    algebraic_layout: StateLayout,
    initial_algebraic: Vec<f64>,
    options: DynamicComponentOptions,
    input_values_at: I,
) -> Result<DynamicComponentResult, SolverFailure>
where
    I: FnMut(f64) -> Result<Vec<SolverScalar>, SolverFailure>,
{
    let derivative_graph = residual_graph_with_derivative_residuals(graph)?;
    let rhs_evaluator = ResidualGraphRhsEvaluator::new(&derivative_graph)?;
    validate_residual_graph_solver_layout(input, &algebraic_layout, &rhs_evaluator)?;

    solve_residual_graph_semi_implicit_euler_with_rhs_and_input_sampler(
        input,
        graph,
        algebraic_layout,
        initial_algebraic,
        options,
        input_values_at,
        |sample| rhs_evaluator.evaluate(&sample),
    )
}

pub fn solve_dynamic_component_assembly(
    assembly: &EquationAssembly,
    solve_input: DynamicComponentAssemblySolveInput,
    options: DynamicComponentOptions,
) -> Result<DynamicComponentResult, SolverFailure> {
    let split = assembly.dynamic_component_split()?;
    let graph = ResidualGraph::from_dynamic_component_assembly(assembly)?;
    let method = if split.algebraic_layout.is_empty() {
        "dynamic_component_assembly_explicit_euler"
    } else {
        "dynamic_component_assembly_semi_implicit_euler"
    };
    let solver_input = SolverInput {
        plan: SolverPlan::new(
            assembly.name.clone(),
            SimulationPlan {
                inputs: layout_names(&split.input_layout.entries),
                outputs: layout_names(&split.state_layout.entries),
                states: layout_names(&split.state_layout.entries),
                parameters: layout_names(&split.parameter_layout.entries),
            },
            SolverOptions::fixed_step(method, solve_input.timestep_s),
        ),
        time_grid: TimeGrid::fixed_step(solve_input.duration_s, solve_input.timestep_s)?,
        state_layout: split.state_layout.clone(),
        input_layout: split.input_layout.clone(),
        parameter_layout: split.parameter_layout.clone(),
        output_layout: OutputLayout {
            entries: split.state_layout.entries.clone(),
        },
        initial_state: solve_input.initial_state,
        inputs: solve_input.inputs,
        parameters: solve_input.parameters,
    };

    if split.algebraic_layout.is_empty() {
        if !solve_input.initial_algebraic.is_empty() {
            return Err(SolverFailure::new(
                "E-DYNAMIC-COMPONENT-ASSEMBLY-LAYOUT",
                "dynamic component assembly explicit solve received algebraic initial values but the assembly has no algebraic layout",
            ));
        }
        solve_residual_graph_explicit_euler(&solver_input, &graph, options)
    } else {
        solve_residual_graph_semi_implicit_euler(
            &solver_input,
            &graph,
            split.algebraic_layout,
            solve_input.initial_algebraic,
            options,
        )
    }
}

pub fn solve_dynamic_component_assembly_with_input_sampler<I>(
    assembly: &EquationAssembly,
    solve_input: DynamicComponentAssemblySolveInput,
    options: DynamicComponentOptions,
    input_values_at: I,
) -> Result<DynamicComponentResult, SolverFailure>
where
    I: FnMut(f64) -> Result<Vec<SolverScalar>, SolverFailure>,
{
    let split = assembly.dynamic_component_split()?;
    let graph = ResidualGraph::from_dynamic_component_assembly(assembly)?;
    let solver_input = SolverInput {
        plan: SolverPlan::new(
            assembly.name.clone(),
            SimulationPlan {
                inputs: layout_names(&split.input_layout.entries),
                outputs: layout_names(&split.state_layout.entries),
                states: layout_names(&split.state_layout.entries),
                parameters: layout_names(&split.parameter_layout.entries),
            },
            SolverOptions::fixed_step(
                "dynamic_component_assembly_semi_implicit_euler",
                solve_input.timestep_s,
            ),
        ),
        time_grid: TimeGrid::fixed_step(solve_input.duration_s, solve_input.timestep_s)?,
        state_layout: split.state_layout.clone(),
        input_layout: split.input_layout.clone(),
        parameter_layout: split.parameter_layout.clone(),
        output_layout: OutputLayout {
            entries: split.state_layout.entries.clone(),
        },
        initial_state: solve_input.initial_state,
        inputs: solve_input.inputs,
        parameters: solve_input.parameters,
    };

    if split.algebraic_layout.is_empty() {
        return Err(SolverFailure::new(
            "E-DYNAMIC-COMPONENT-ASSEMBLY-LAYOUT",
            "dynamic component assembly input-sampled solve requires a semi-implicit algebraic layout",
        ));
    }
    solve_residual_graph_semi_implicit_euler_with_input_sampler(
        &solver_input,
        &graph,
        split.algebraic_layout,
        solve_input.initial_algebraic,
        options,
        input_values_at,
    )
}

pub fn solve_dynamic_component_assembly_with_rhs_and_input_sampler<R, I>(
    assembly: &EquationAssembly,
    solve_input: DynamicComponentAssemblySolveInput,
    options: DynamicComponentOptions,
    input_values_at: I,
    rhs: R,
) -> Result<DynamicComponentResult, SolverFailure>
where
    R: FnMut(DynamicStepInput<'_>) -> Result<Vec<f64>, SolverFailure>,
    I: FnMut(f64) -> Result<Vec<SolverScalar>, SolverFailure>,
{
    let split = assembly.dynamic_component_split()?;
    let graph = ResidualGraph::from_dynamic_component_assembly(assembly)?;
    let solver_input = SolverInput {
        plan: SolverPlan::new(
            assembly.name.clone(),
            SimulationPlan {
                inputs: layout_names(&split.input_layout.entries),
                outputs: layout_names(&split.state_layout.entries),
                states: layout_names(&split.state_layout.entries),
                parameters: layout_names(&split.parameter_layout.entries),
            },
            SolverOptions::fixed_step(
                "dynamic_component_assembly_semi_implicit_euler",
                solve_input.timestep_s,
            ),
        ),
        time_grid: TimeGrid::fixed_step(solve_input.duration_s, solve_input.timestep_s)?,
        state_layout: split.state_layout.clone(),
        input_layout: split.input_layout.clone(),
        parameter_layout: split.parameter_layout.clone(),
        output_layout: OutputLayout {
            entries: split.state_layout.entries.clone(),
        },
        initial_state: solve_input.initial_state,
        inputs: solve_input.inputs,
        parameters: solve_input.parameters,
    };

    if split.algebraic_layout.is_empty() {
        return Err(SolverFailure::new(
            "E-DYNAMIC-COMPONENT-ASSEMBLY-LAYOUT",
            "dynamic component assembly custom-RHS solve requires a semi-implicit algebraic layout",
        ));
    }
    solve_residual_graph_semi_implicit_euler_with_rhs_and_input_sampler(
        &solver_input,
        &graph,
        split.algebraic_layout,
        solve_input.initial_algebraic,
        options,
        input_values_at,
        rhs,
    )
}

fn normalize_residual_values(values: &[f64], scales: &[f64]) -> Vec<f64> {
    values
        .iter()
        .enumerate()
        .map(|(index, value)| {
            scales
                .get(index)
                .copied()
                .filter(|scale| scale.is_finite() && *scale > 0.0)
                .map(|scale| *value / scale)
                .unwrap_or(*value)
        })
        .collect()
}

fn restore_raw_residual_values(values: &[f64], scales: &[f64]) -> Vec<f64> {
    values
        .iter()
        .enumerate()
        .map(|(index, value)| {
            scales
                .get(index)
                .copied()
                .filter(|scale| scale.is_finite() && *scale > 0.0)
                .map(|scale| *value * scale)
                .unwrap_or(*value)
        })
        .collect()
}

fn layout_residual_scales(layout: &StateLayout) -> Vec<f64> {
    layout
        .entries
        .iter()
        .map(|entry| {
            ResidualScale::from_quantity_unit(&entry.quantity_kind, &entry.canonical_unit).value
        })
        .collect()
}
fn layout_names(entries: &[crate::solver::LayoutEntry]) -> Vec<String> {
    entries.iter().map(|entry| entry.name.clone()).collect()
}

fn validate_residual_graph_algebraic_layout(
    input: &SolverInput,
    algebraic_layout: &StateLayout,
    evaluator: &ResidualGraphAlgebraicLinearEvaluator,
) -> Result<(), SolverFailure> {
    validate_residual_graph_layout_entries(
        "state",
        &input.state_layout.entries,
        &evaluator.state_names,
        evaluator.state_count,
    )
    .map_err(|failure| {
        SolverFailure::new("E-DYNAMIC-COMPONENT-ALGEBRAIC-LAYOUT", failure.message)
    })?;
    validate_residual_graph_layout_entries(
        "algebraic",
        &algebraic_layout.entries,
        &evaluator.algebraic_names,
        evaluator.algebraic_count,
    )
    .map_err(|failure| {
        SolverFailure::new("E-DYNAMIC-COMPONENT-ALGEBRAIC-LAYOUT", failure.message)
    })?;
    validate_residual_graph_layout_entries(
        "input",
        &input.input_layout.entries,
        &evaluator.input_names,
        evaluator.input_count,
    )
    .map_err(|failure| {
        SolverFailure::new("E-DYNAMIC-COMPONENT-ALGEBRAIC-LAYOUT", failure.message)
    })?;
    validate_residual_graph_layout_entries(
        "parameter",
        &input.parameter_layout.entries,
        &evaluator.parameter_names,
        evaluator.parameter_count,
    )
    .map_err(|failure| {
        SolverFailure::new("E-DYNAMIC-COMPONENT-ALGEBRAIC-LAYOUT", failure.message)
    })?;
    Ok(())
}

fn residual_graph_with_derivative_residuals(
    graph: &ResidualGraph,
) -> Result<ResidualGraph, SolverFailure> {
    let derivative_variables = graph
        .variables
        .iter()
        .filter(|variable| is_derivative_role(&variable.role))
        .map(|variable| variable.index)
        .collect::<HashSet<_>>();
    let mut derivative_graph = graph.clone();
    derivative_graph.residuals = graph
        .residuals
        .iter()
        .filter_map(|residual| {
            let derivative_term_count = residual
                .terms
                .iter()
                .filter(|term| derivative_variables.contains(&term.variable_index))
                .count();
            if derivative_term_count > 1 {
                return Some(Err(SolverFailure::new(
                    "E-DYNAMIC-COMPONENT-RHS-SHAPE",
                    format!(
                        "residual `{}` contains multiple derivative variables",
                        residual.name
                    ),
                )));
            }
            if derivative_term_count == 1 {
                Some(Ok(residual.clone()))
            } else {
                None
            }
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(derivative_graph)
}

fn validate_residual_graph_solver_layout(
    input: &SolverInput,
    algebraic_layout: &StateLayout,
    evaluator: &ResidualGraphRhsEvaluator,
) -> Result<(), SolverFailure> {
    validate_residual_graph_layout_entries(
        "state",
        &input.state_layout.entries,
        &evaluator.state_names,
        evaluator.state_count,
    )?;
    validate_residual_graph_layout_entries(
        "algebraic",
        &algebraic_layout.entries,
        &evaluator.algebraic_names,
        evaluator.algebraic_count,
    )?;
    validate_residual_graph_layout_entries(
        "input",
        &input.input_layout.entries,
        &evaluator.input_names,
        evaluator.input_count,
    )?;
    validate_residual_graph_layout_entries(
        "parameter",
        &input.parameter_layout.entries,
        &evaluator.parameter_names,
        evaluator.parameter_count,
    )?;
    Ok(())
}

fn validate_residual_graph_layout_entries(
    role: &str,
    entries: &[crate::solver::LayoutEntry],
    expected_names: &[String],
    expected_count: usize,
) -> Result<(), SolverFailure> {
    if entries.len() != expected_count {
        return Err(SolverFailure::new(
            "E-DYNAMIC-COMPONENT-RHS-LAYOUT",
            format!("solver {role} layout count does not match residual graph {role} variables"),
        ));
    }
    for (entry, expected_name) in entries.iter().zip(expected_names.iter()) {
        if entry.name != *expected_name {
            return Err(SolverFailure::new(
                "E-DYNAMIC-COMPONENT-RHS-LAYOUT",
                format!(
                    "solver {role} layout entry `{}` does not match residual graph variable `{}`",
                    entry.name, expected_name
                ),
            ));
        }
    }
    Ok(())
}

fn dynamic_component_result(
    input: &SolverInput,
    algebraic_layout: StateLayout,
    state_values_by_state: Vec<Vec<f64>>,
    algebraic_values_by_variable: Vec<Vec<f64>>,
    step_diagnostics: Vec<DynamicComponentStepDiagnostic>,
    diagnostics: SolverDiagnostics,
) -> DynamicComponentResult {
    let state_trajectories = trajectories_from_layout(&input.state_layout, state_values_by_state);
    let algebraic_trajectories =
        trajectories_from_layout(&algebraic_layout, algebraic_values_by_variable);
    DynamicComponentResult {
        solver_result: SolverResult {
            plan: input.plan.clone(),
            time_grid: input.time_grid.clone(),
            state_layout: input.state_layout.clone(),
            output_layout: input.output_layout.clone(),
            output: SolverOutput {
                state_trajectories,
                algebraic_trajectories: algebraic_trajectories.clone(),
            },
            diagnostics,
        },
        algebraic_layout,
        algebraic_trajectories,
        step_diagnostics,
    }
}

fn trajectories_from_layout(
    layout: &StateLayout,
    values_by_variable: Vec<Vec<f64>>,
) -> Vec<StateTrajectory> {
    layout
        .entries
        .iter()
        .zip(values_by_variable)
        .map(|(entry, values)| StateTrajectory {
            name: entry.name.clone(),
            quantity_kind: entry.quantity_kind.clone(),
            canonical_unit: entry.canonical_unit.clone(),
            values,
        })
        .collect()
}

fn ensure_finite_values(code: &str, label: &str, values: &[f64]) -> Result<(), SolverFailure> {
    if values.iter().all(|value| value.is_finite()) {
        Ok(())
    } else {
        Err(SolverFailure::new(
            code,
            format!("{label} vector contains a non-finite value"),
        ))
    }
}

fn ensure_finite_scalars(
    code: &str,
    label: &str,
    values: &[SolverScalar],
) -> Result<(), SolverFailure> {
    if values.iter().all(|value| value.value.is_finite()) {
        Ok(())
    } else {
        Err(SolverFailure::new(
            code,
            format!("{label} vector contains a non-finite value"),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::solver::assembly::{EquationAssembly, GeneratedEquation, UnknownVariable};
    use crate::solver::{
        InputLayout, LayoutEntry, OutputLayout, ParameterLayout, ResidualEquation,
        ResidualExpression, ResidualScale, ResidualSource, ResidualTerm, ResidualUnit,
        ResidualVariableRef, SimulationPlan, SolverOptions, SolverPlan, TimeGrid,
    };

    #[test]
    fn solves_dynamic_component_two_state_without_algebraic_node() {
        let input = SolverInput {
            plan: SolverPlan::new(
                "ComponentGraph",
                SimulationPlan::default(),
                SolverOptions::fixed_step("dynamic_component_explicit_euler", 1.0),
            ),
            time_grid: TimeGrid::fixed_step(2.0, 1.0).unwrap(),
            state_layout: StateLayout::new(vec![
                LayoutEntry::new(0, "x", "Dimensionless", "1", "1"),
                LayoutEntry::new(1, "y", "Dimensionless", "1", "1"),
            ]),
            input_layout: InputLayout::default(),
            parameter_layout: ParameterLayout::default(),
            output_layout: OutputLayout::default(),
            initial_state: vec![0.0, 10.0],
            inputs: Vec::new(),
            parameters: Vec::new(),
        };
        let mut algebraic_update_called = false;

        let result = solve_explicit_euler_with_algebraic(
            &input,
            StateLayout::default(),
            Vec::new(),
            DynamicComponentOptions::default(),
            |_| {
                algebraic_update_called = true;
                Ok(Vec::new())
            },
            |sample| {
                assert!(sample.algebraic.is_empty());
                Ok(vec![1.0, -2.0])
            },
        )
        .unwrap();

        assert!(!algebraic_update_called);
        assert_eq!(result.solver_result.diagnostics.status, "computed");
        assert_eq!(result.solver_result.diagnostics.iteration_count, 0);
        assert_eq!(
            result.solver_result.output.state_trajectories[0].values,
            vec![0.0, 1.0, 2.0]
        );
        assert_eq!(
            result.solver_result.output.state_trajectories[1].values,
            vec![10.0, 8.0, 6.0]
        );
        assert!(result.algebraic_trajectories.is_empty());
        assert!(result
            .solver_result
            .output
            .algebraic_trajectories
            .is_empty());
        assert_eq!(result.step_diagnostics.len(), 3);
        assert!(result.step_diagnostics.iter().all(|diagnostic| {
            diagnostic.algebraic_iteration_count == 0
                && diagnostic.residual_norm == 0.0
                && diagnostic.convergence_status == "algebraic_not_required"
                && diagnostic.failure.is_none()
        }));
    }

    #[test]
    fn dynamic_component_uses_partial_final_step() {
        let input = SolverInput {
            plan: SolverPlan::new(
                "ComponentGraph",
                SimulationPlan::default(),
                SolverOptions::fixed_step("dynamic_component_explicit_euler", 1.0),
            ),
            time_grid: TimeGrid::fixed_step(2.5, 1.0).unwrap(),
            state_layout: StateLayout::new(vec![LayoutEntry::new(
                0,
                "x",
                "Dimensionless",
                "1",
                "1",
            )]),
            input_layout: InputLayout::default(),
            parameter_layout: ParameterLayout::default(),
            output_layout: OutputLayout::default(),
            initial_state: vec![0.0],
            inputs: Vec::new(),
            parameters: Vec::new(),
        };

        let result = solve_explicit_euler_with_algebraic(
            &input,
            StateLayout::default(),
            Vec::new(),
            DynamicComponentOptions::default(),
            |_| Ok(Vec::new()),
            |_sample| Ok(vec![2.0]),
        )
        .unwrap();

        assert_eq!(
            result.solver_result.output.state_trajectories[0].values,
            vec![0.0, 2.0, 4.0, 5.0]
        );
        assert_eq!(result.step_diagnostics.len(), 4);
        assert_eq!(result.step_diagnostics[3].time_s, 2.5);
    }

    #[test]
    fn dynamic_component_rejects_nonfinite_values() {
        let input = SolverInput {
            plan: SolverPlan::new(
                "ComponentGraph",
                SimulationPlan::default(),
                SolverOptions::fixed_step("dynamic_component_explicit_euler", 1.0),
            ),
            time_grid: TimeGrid::fixed_step(1.0, 1.0).unwrap(),
            state_layout: StateLayout::new(vec![LayoutEntry::new(
                0,
                "x",
                "Dimensionless",
                "1",
                "1",
            )]),
            input_layout: InputLayout::default(),
            parameter_layout: ParameterLayout::default(),
            output_layout: OutputLayout::default(),
            initial_state: vec![0.0],
            inputs: Vec::new(),
            parameters: Vec::new(),
        };
        let algebraic_layout =
            StateLayout::new(vec![LayoutEntry::new(0, "z", "Dimensionless", "1", "1")]);

        let failure = solve_explicit_euler_with_algebraic(
            &input,
            algebraic_layout.clone(),
            vec![f64::NAN],
            DynamicComponentOptions::default(),
            |_| Ok(vec![0.0]),
            |_sample| Ok(vec![0.0]),
        )
        .unwrap_err();
        assert_eq!(failure.code, "E-DYNAMIC-COMPONENT-ALGEBRAIC-VALUE");

        let failure = solve_explicit_euler_with_algebraic(
            &input,
            StateLayout::default(),
            Vec::new(),
            DynamicComponentOptions::default(),
            |_| Ok(Vec::new()),
            |_sample| Ok(vec![f64::INFINITY]),
        )
        .unwrap_err();
        assert_eq!(failure.code, "E-DYNAMIC-COMPONENT-RHS-VALUE");

        let failure = solve_explicit_euler_with_algebraic(
            &input,
            algebraic_layout,
            vec![0.0],
            DynamicComponentOptions::default(),
            |_| Ok(vec![f64::INFINITY]),
            |_sample| Ok(vec![0.0]),
        )
        .unwrap_err();
        assert_eq!(failure.code, "E-FIXED-POINT-VALUE");
    }

    #[test]
    fn residual_graph_rhs_evaluator_lowers_derivative_residuals() {
        let graph = residual_rhs_graph();
        let evaluator = ResidualGraphRhsEvaluator::new(&graph).unwrap();
        let state = vec![1.0, 2.0];
        let inputs = vec![SolverScalar::new("u", "Dimensionless", "1", 3.0)];
        let parameters = Vec::new();

        let derivative = evaluator
            .evaluate(&DynamicStepInput {
                time_s: 0.0,
                step_index: 0,
                state: &state,
                algebraic: &[],
                inputs: &inputs,
                parameters: &parameters,
            })
            .unwrap();

        assert_eq!(derivative, vec![4.0, -2.0]);
    }

    #[test]
    fn residual_graph_rhs_evaluator_rejects_invalid_dynamic_graphs() {
        let mut graph = residual_rhs_graph();
        graph.residuals.pop();

        let failure = ResidualGraphRhsEvaluator::new(&graph).unwrap_err();

        assert_eq!(failure.code, "E-DYNAMIC-COMPONENT-RHS-SHAPE");
    }

    #[test]
    fn dynamic_component_solver_uses_residual_graph_rhs() {
        let graph = residual_rhs_graph();
        let evaluator = ResidualGraphRhsEvaluator::new(&graph).unwrap();
        let input = SolverInput {
            plan: SolverPlan::new(
                "ComponentGraph",
                SimulationPlan::default(),
                SolverOptions::fixed_step("dynamic_component_explicit_euler", 1.0),
            ),
            time_grid: TimeGrid::fixed_step(1.0, 1.0).unwrap(),
            state_layout: StateLayout::new(vec![
                LayoutEntry::new(0, "x", "Dimensionless", "1", "1"),
                LayoutEntry::new(1, "y", "Dimensionless", "1", "1"),
            ]),
            input_layout: InputLayout {
                entries: vec![LayoutEntry::new(0, "u", "Dimensionless", "1", "1")],
            },
            parameter_layout: ParameterLayout::default(),
            output_layout: OutputLayout::default(),
            initial_state: vec![1.0, 2.0],
            inputs: vec![SolverScalar::new("u", "Dimensionless", "1", 3.0)],
            parameters: Vec::new(),
        };

        let result = solve_explicit_euler_with_algebraic(
            &input,
            StateLayout::default(),
            Vec::new(),
            DynamicComponentOptions::default(),
            |_| Ok(Vec::new()),
            |sample| evaluator.evaluate(&sample),
        )
        .unwrap();

        assert_eq!(
            result.solver_result.output.state_trajectories[0].values,
            vec![1.0, 5.0]
        );
        assert_eq!(
            result.solver_result.output.state_trajectories[1].values,
            vec![2.0, 0.0]
        );
        assert_eq!(
            result
                .step_diagnostics
                .iter()
                .map(|diagnostic| diagnostic.convergence_status.as_str())
                .collect::<Vec<_>>(),
            vec!["algebraic_not_required", "algebraic_not_required"]
        );
    }

    #[test]
    fn residual_graph_explicit_euler_entrypoint_solves_algebraic_free_graph() {
        let graph = residual_rhs_graph();
        let input = SolverInput {
            plan: SolverPlan::new(
                "ComponentGraph",
                SimulationPlan::default(),
                SolverOptions::fixed_step("dynamic_component_residual_graph_explicit_euler", 1.0),
            ),
            time_grid: TimeGrid::fixed_step(1.0, 1.0).unwrap(),
            state_layout: StateLayout::new(vec![
                LayoutEntry::new(0, "x", "Dimensionless", "1", "1"),
                LayoutEntry::new(1, "y", "Dimensionless", "1", "1"),
            ]),
            input_layout: InputLayout {
                entries: vec![LayoutEntry::new(0, "u", "Dimensionless", "1", "1")],
            },
            parameter_layout: ParameterLayout::default(),
            output_layout: OutputLayout::default(),
            initial_state: vec![1.0, 2.0],
            inputs: vec![SolverScalar::new("u", "Dimensionless", "1", 3.0)],
            parameters: Vec::new(),
        };

        let result =
            solve_residual_graph_explicit_euler(&input, &graph, DynamicComponentOptions::default())
                .unwrap();

        assert_eq!(
            result.solver_result.diagnostics.convergence_status.as_str(),
            "dynamic_component_fixed_step_completed"
        );
        assert_eq!(
            result.solver_result.output.state_trajectories[0].values,
            vec![1.0, 5.0]
        );
        assert_eq!(
            result.solver_result.output.state_trajectories[1].values,
            vec![2.0, 0.0]
        );
        assert!(result.algebraic_trajectories.is_empty());
    }

    #[test]
    fn residual_graph_explicit_euler_entrypoint_uses_parameters() {
        let graph = parameterized_residual_rhs_graph();
        let input = SolverInput {
            plan: SolverPlan::new(
                "ComponentGraph",
                SimulationPlan::default(),
                SolverOptions::fixed_step("dynamic_component_residual_graph_explicit_euler", 1.0),
            ),
            time_grid: TimeGrid::fixed_step(1.0, 1.0).unwrap(),
            state_layout: StateLayout::new(vec![LayoutEntry::new(
                0,
                "x",
                "Dimensionless",
                "1",
                "1",
            )]),
            input_layout: InputLayout {
                entries: vec![LayoutEntry::new(0, "u", "Dimensionless", "1", "1")],
            },
            parameter_layout: ParameterLayout {
                entries: vec![LayoutEntry::new(0, "k", "Dimensionless", "1", "1")],
            },
            output_layout: OutputLayout::default(),
            initial_state: vec![1.0],
            inputs: vec![SolverScalar::new("u", "Dimensionless", "1", 3.0)],
            parameters: vec![SolverScalar::new("k", "Dimensionless", "1", 2.0)],
        };

        let result =
            solve_residual_graph_explicit_euler(&input, &graph, DynamicComponentOptions::default())
                .unwrap();

        assert_eq!(
            result.solver_result.output.state_trajectories[0].values,
            vec![1.0, 6.0]
        );
    }

    #[test]
    fn residual_graph_explicit_euler_entrypoint_rejects_layout_mismatch() {
        let graph = residual_rhs_graph();
        let input = SolverInput {
            plan: SolverPlan::new(
                "ComponentGraph",
                SimulationPlan::default(),
                SolverOptions::fixed_step("dynamic_component_residual_graph_explicit_euler", 1.0),
            ),
            time_grid: TimeGrid::fixed_step(1.0, 1.0).unwrap(),
            state_layout: StateLayout::new(vec![LayoutEntry::new(
                0,
                "x",
                "Dimensionless",
                "1",
                "1",
            )]),
            input_layout: InputLayout {
                entries: vec![LayoutEntry::new(0, "u", "Dimensionless", "1", "1")],
            },
            parameter_layout: ParameterLayout::default(),
            output_layout: OutputLayout::default(),
            initial_state: vec![1.0],
            inputs: vec![SolverScalar::new("u", "Dimensionless", "1", 3.0)],
            parameters: Vec::new(),
        };

        let failure =
            solve_residual_graph_explicit_euler(&input, &graph, DynamicComponentOptions::default())
                .unwrap_err();

        assert_eq!(failure.code, "E-DYNAMIC-COMPONENT-RHS-LAYOUT");
    }

    #[test]
    fn residual_graph_explicit_euler_entrypoint_rejects_layout_name_mismatch() {
        let graph = parameterized_residual_rhs_graph();
        let input = SolverInput {
            plan: SolverPlan::new(
                "ComponentGraph",
                SimulationPlan::default(),
                SolverOptions::fixed_step("dynamic_component_residual_graph_explicit_euler", 1.0),
            ),
            time_grid: TimeGrid::fixed_step(1.0, 1.0).unwrap(),
            state_layout: StateLayout::new(vec![LayoutEntry::new(
                0,
                "x",
                "Dimensionless",
                "1",
                "1",
            )]),
            input_layout: InputLayout {
                entries: vec![LayoutEntry::new(0, "u", "Dimensionless", "1", "1")],
            },
            parameter_layout: ParameterLayout {
                entries: vec![LayoutEntry::new(0, "wrong_k", "Dimensionless", "1", "1")],
            },
            output_layout: OutputLayout::default(),
            initial_state: vec![1.0],
            inputs: vec![SolverScalar::new("u", "Dimensionless", "1", 3.0)],
            parameters: vec![SolverScalar::new("wrong_k", "Dimensionless", "1", 2.0)],
        };

        let failure =
            solve_residual_graph_explicit_euler(&input, &graph, DynamicComponentOptions::default())
                .unwrap_err();

        assert_eq!(failure.code, "E-DYNAMIC-COMPONENT-RHS-LAYOUT");
        assert!(failure.message.contains("wrong_k"));
        assert!(failure.message.contains("k"));
    }

    #[test]
    fn residual_graph_explicit_euler_entrypoint_rejects_algebraic_graph() {
        let graph = semi_implicit_residual_graph();
        let input = SolverInput {
            plan: SolverPlan::new(
                "ComponentGraph",
                SimulationPlan::default(),
                SolverOptions::fixed_step("dynamic_component_residual_graph_explicit_euler", 1.0),
            ),
            time_grid: TimeGrid::fixed_step(1.0, 1.0).unwrap(),
            state_layout: StateLayout::new(vec![LayoutEntry::new(
                0,
                "x",
                "Dimensionless",
                "1",
                "1",
            )]),
            input_layout: InputLayout {
                entries: vec![LayoutEntry::new(0, "u", "Dimensionless", "1", "1")],
            },
            parameter_layout: ParameterLayout::default(),
            output_layout: OutputLayout::default(),
            initial_state: vec![1.0],
            inputs: vec![SolverScalar::new("u", "Dimensionless", "1", 3.0)],
            parameters: Vec::new(),
        };

        let failure =
            solve_residual_graph_explicit_euler(&input, &graph, DynamicComponentOptions::default())
                .unwrap_err();

        assert_eq!(failure.code, "E-DYNAMIC-COMPONENT-RHS-SHAPE");
    }

    #[test]
    fn residual_graph_semi_implicit_entrypoint_solves_linear_algebraic_residuals() {
        let graph = semi_implicit_residual_graph();
        let input = SolverInput {
            plan: SolverPlan::new(
                "ComponentGraph",
                SimulationPlan::default(),
                SolverOptions::fixed_step("dynamic_component_residual_graph_semi_implicit", 1.0),
            ),
            time_grid: TimeGrid::fixed_step(1.0, 1.0).unwrap(),
            state_layout: StateLayout::new(vec![LayoutEntry::new(
                0,
                "x",
                "Dimensionless",
                "1",
                "1",
            )]),
            input_layout: InputLayout {
                entries: vec![LayoutEntry::new(0, "u", "Dimensionless", "1", "1")],
            },
            parameter_layout: ParameterLayout::default(),
            output_layout: OutputLayout::default(),
            initial_state: vec![1.0],
            inputs: vec![SolverScalar::new("u", "Dimensionless", "1", 3.0)],
            parameters: Vec::new(),
        };
        let algebraic_layout =
            StateLayout::new(vec![LayoutEntry::new(0, "z", "Dimensionless", "1", "1")]);

        let result = solve_residual_graph_semi_implicit_euler(
            &input,
            &graph,
            algebraic_layout,
            vec![0.0],
            DynamicComponentOptions::default(),
        )
        .unwrap();

        assert_eq!(result.solver_result.diagnostics.status, "computed");
        assert_eq!(result.solver_result.diagnostics.iteration_count, 2);
        assert_eq!(
            result.solver_result.output.state_trajectories[0].values,
            vec![1.0, 3.0]
        );
        assert_eq!(result.algebraic_trajectories[0].values, vec![2.0, 0.0]);
        assert_eq!(
            result.solver_result.output.algebraic_trajectories[0].values,
            vec![2.0, 0.0]
        );
        assert!(result.step_diagnostics.iter().all(|diagnostic| {
            diagnostic.convergence_status == "linear_algebraic_converged"
                && diagnostic.algebraic_iteration_count == 1
                && diagnostic.residual_values.len() == 1
                && diagnostic.normalized_residual_values.len() == 1
                && diagnostic.failure.is_none()
        }));
    }

    #[test]
    fn residual_graph_semi_implicit_entrypoint_samples_inputs_per_step() {
        let graph = semi_implicit_residual_graph();
        let input = SolverInput {
            plan: SolverPlan::new(
                "ComponentGraph",
                SimulationPlan::default(),
                SolverOptions::fixed_step("dynamic_component_residual_graph_semi_implicit", 1.0),
            ),
            time_grid: TimeGrid::fixed_step(2.0, 1.0).unwrap(),
            state_layout: StateLayout::new(vec![LayoutEntry::new(
                0,
                "x",
                "Dimensionless",
                "1",
                "1",
            )]),
            input_layout: InputLayout {
                entries: vec![LayoutEntry::new(0, "u", "Dimensionless", "1", "1")],
            },
            parameter_layout: ParameterLayout::default(),
            output_layout: OutputLayout::default(),
            initial_state: vec![1.0],
            inputs: vec![SolverScalar::new("u", "Dimensionless", "1", 3.0)],
            parameters: Vec::new(),
        };
        let algebraic_layout =
            StateLayout::new(vec![LayoutEntry::new(0, "z", "Dimensionless", "1", "1")]);

        let result = solve_residual_graph_semi_implicit_euler_with_input_sampler(
            &input,
            &graph,
            algebraic_layout,
            vec![0.0],
            DynamicComponentOptions::default(),
            |time_s| {
                Ok(vec![SolverScalar::new(
                    "u",
                    "Dimensionless",
                    "1",
                    3.0 + 2.0 * time_s,
                )])
            },
        )
        .unwrap();

        assert_eq!(
            result.solver_result.output.state_trajectories[0].values,
            vec![1.0, 3.0, 5.0]
        );
        assert_eq!(result.algebraic_trajectories[0].values, vec![2.0, 2.0, 2.0]);
    }

    #[test]
    fn residual_graph_semi_implicit_entrypoint_uses_parameters() {
        let graph = parameterized_semi_implicit_residual_graph();
        let input = SolverInput {
            plan: SolverPlan::new(
                "ComponentGraph",
                SimulationPlan::default(),
                SolverOptions::fixed_step("dynamic_component_residual_graph_semi_implicit", 1.0),
            ),
            time_grid: TimeGrid::fixed_step(1.0, 1.0).unwrap(),
            state_layout: StateLayout::new(vec![LayoutEntry::new(
                0,
                "x",
                "Dimensionless",
                "1",
                "1",
            )]),
            input_layout: InputLayout {
                entries: vec![LayoutEntry::new(0, "u", "Dimensionless", "1", "1")],
            },
            parameter_layout: ParameterLayout {
                entries: vec![LayoutEntry::new(0, "k", "Dimensionless", "1", "1")],
            },
            output_layout: OutputLayout::default(),
            initial_state: vec![1.0],
            inputs: vec![SolverScalar::new("u", "Dimensionless", "1", 5.0)],
            parameters: vec![SolverScalar::new("k", "Dimensionless", "1", 2.0)],
        };
        let algebraic_layout =
            StateLayout::new(vec![LayoutEntry::new(0, "z", "Dimensionless", "1", "1")]);

        let result = solve_residual_graph_semi_implicit_euler(
            &input,
            &graph,
            algebraic_layout,
            vec![0.0],
            DynamicComponentOptions::default(),
        )
        .unwrap();

        assert_eq!(
            result.solver_result.output.state_trajectories[0].values,
            vec![1.0, 3.0]
        );
        assert_eq!(result.algebraic_trajectories[0].values, vec![2.0, 0.0]);
    }

    #[test]
    fn residual_graph_semi_implicit_entrypoint_reports_linear_algebraic_failure() {
        let mut graph = semi_implicit_residual_graph();
        graph.residuals[1] = residual("z_balance", &[(0, "x", 1.0)], 0.0);
        let input = SolverInput {
            plan: SolverPlan::new(
                "ComponentGraph",
                SimulationPlan::default(),
                SolverOptions::fixed_step("dynamic_component_residual_graph_semi_implicit", 1.0),
            ),
            time_grid: TimeGrid::fixed_step(1.0, 1.0).unwrap(),
            state_layout: StateLayout::new(vec![LayoutEntry::new(
                0,
                "x",
                "Dimensionless",
                "1",
                "1",
            )]),
            input_layout: InputLayout {
                entries: vec![LayoutEntry::new(0, "u", "Dimensionless", "1", "1")],
            },
            parameter_layout: ParameterLayout::default(),
            output_layout: OutputLayout::default(),
            initial_state: vec![1.0],
            inputs: vec![SolverScalar::new("u", "Dimensionless", "1", 3.0)],
            parameters: Vec::new(),
        };
        let algebraic_layout =
            StateLayout::new(vec![LayoutEntry::new(0, "z", "Dimensionless", "1", "1")]);

        let result = solve_residual_graph_semi_implicit_euler(
            &input,
            &graph,
            algebraic_layout,
            vec![0.0],
            DynamicComponentOptions::default(),
        )
        .unwrap();

        assert_eq!(result.solver_result.diagnostics.status, "failed");
        assert_eq!(
            result.solver_result.diagnostics.convergence_status,
            "algebraic_solve_failed"
        );
        assert_eq!(
            result
                .solver_result
                .diagnostics
                .failure
                .as_ref()
                .map(|failure| failure.code.as_str()),
            Some("E-LINEAR-SINGULAR")
        );
        assert_eq!(
            result.step_diagnostics[0].convergence_status,
            "linear_algebraic_solve_failed"
        );
        assert_eq!(
            result.solver_result.output.state_trajectories[0].values,
            vec![1.0]
        );
        assert_eq!(result.algebraic_trajectories[0].values, vec![0.0]);
    }

    #[test]
    fn residual_graph_semi_implicit_entrypoint_rejects_algebraic_layout_name_mismatch() {
        let graph = parameterized_semi_implicit_residual_graph();
        let input = SolverInput {
            plan: SolverPlan::new(
                "ComponentGraph",
                SimulationPlan::default(),
                SolverOptions::fixed_step("dynamic_component_residual_graph_semi_implicit", 1.0),
            ),
            time_grid: TimeGrid::fixed_step(1.0, 1.0).unwrap(),
            state_layout: StateLayout::new(vec![LayoutEntry::new(
                0,
                "x",
                "Dimensionless",
                "1",
                "1",
            )]),
            input_layout: InputLayout {
                entries: vec![LayoutEntry::new(0, "u", "Dimensionless", "1", "1")],
            },
            parameter_layout: ParameterLayout {
                entries: vec![LayoutEntry::new(0, "k", "Dimensionless", "1", "1")],
            },
            output_layout: OutputLayout::default(),
            initial_state: vec![1.0],
            inputs: vec![SolverScalar::new("u", "Dimensionless", "1", 5.0)],
            parameters: vec![SolverScalar::new("k", "Dimensionless", "1", 2.0)],
        };
        let algebraic_layout = StateLayout::new(vec![LayoutEntry::new(
            0,
            "wrong_z",
            "Dimensionless",
            "1",
            "1",
        )]);

        let failure = solve_residual_graph_semi_implicit_euler(
            &input,
            &graph,
            algebraic_layout,
            vec![0.0],
            DynamicComponentOptions::default(),
        )
        .unwrap_err();

        assert_eq!(failure.code, "E-DYNAMIC-COMPONENT-RHS-LAYOUT");
        assert!(failure.message.contains("wrong_z"));
        assert!(failure.message.contains("z"));
    }

    #[test]
    fn solves_dynamic_component_with_algebraic_node() {
        let input = SolverInput {
            plan: SolverPlan::new(
                "ComponentGraph",
                SimulationPlan::default(),
                SolverOptions::fixed_step("dynamic_component_explicit_euler", 1.0),
            ),
            time_grid: TimeGrid::fixed_step(2.0, 1.0).unwrap(),
            state_layout: StateLayout::new(vec![LayoutEntry::new(
                0,
                "x",
                "Dimensionless",
                "1",
                "1",
            )]),
            input_layout: InputLayout::default(),
            parameter_layout: ParameterLayout::default(),
            output_layout: OutputLayout::default(),
            initial_state: vec![0.0],
            inputs: Vec::new(),
            parameters: Vec::new(),
        };
        let algebraic_layout =
            StateLayout::new(vec![LayoutEntry::new(0, "z", "Dimensionless", "1", "1")]);

        let result = solve_explicit_euler_with_algebraic(
            &input,
            algebraic_layout,
            vec![0.0],
            DynamicComponentOptions::default(),
            |sample| Ok(vec![0.5 * sample.state[0] + 1.0]),
            |sample| Ok(vec![sample.algebraic[0]]),
        )
        .unwrap();

        assert_eq!(result.solver_result.diagnostics.status, "computed");
        assert_eq!(
            result.solver_result.diagnostics.convergence_status,
            "dynamic_component_fixed_step_completed"
        );
        assert_eq!(
            result.solver_result.output.state_trajectories[0].values,
            vec![0.0, 1.0, 2.5]
        );
        assert_eq!(
            result.algebraic_trajectories[0].values,
            vec![1.0, 1.5, 2.25]
        );
        assert_eq!(
            result.solver_result.output.algebraic_trajectories[0].values,
            vec![1.0, 1.5, 2.25]
        );
        assert_eq!(result.step_diagnostics.len(), 3);
        assert!(result
            .step_diagnostics
            .iter()
            .all(|diagnostic| diagnostic.failure.is_none()
                && diagnostic.convergence_status == "fixed_point_converged"));
    }

    #[test]
    fn reports_dynamic_component_algebraic_nonconvergence() {
        let input = SolverInput {
            plan: SolverPlan::new(
                "ComponentGraph",
                SimulationPlan::default(),
                SolverOptions::fixed_step("dynamic_component_explicit_euler", 1.0),
            ),
            time_grid: TimeGrid::fixed_step(2.0, 1.0).unwrap(),
            state_layout: StateLayout::new(vec![LayoutEntry::new(
                0,
                "x",
                "Dimensionless",
                "1",
                "1",
            )]),
            input_layout: InputLayout::default(),
            parameter_layout: ParameterLayout::default(),
            output_layout: OutputLayout::default(),
            initial_state: vec![0.0],
            inputs: Vec::new(),
            parameters: Vec::new(),
        };
        let algebraic_layout =
            StateLayout::new(vec![LayoutEntry::new(0, "z", "Dimensionless", "1", "1")]);
        let options = DynamicComponentOptions {
            algebraic: FixedPointOptions {
                tolerance: 1e-12,
                max_iterations: 3,
                relaxation: 1.0,
            },
            ..DynamicComponentOptions::default()
        };

        let result = solve_explicit_euler_with_algebraic(
            &input,
            algebraic_layout,
            vec![0.0],
            options,
            |sample| Ok(vec![sample.algebraic[0] + 1.0]),
            |_sample| Ok(vec![0.0]),
        )
        .unwrap();

        assert_eq!(result.solver_result.diagnostics.status, "failed");
        assert_eq!(
            result.solver_result.diagnostics.convergence_status,
            "algebraic_solve_failed"
        );
        assert_eq!(
            result
                .solver_result
                .diagnostics
                .failure
                .as_ref()
                .map(|failure| failure.code.as_str()),
            Some("E-FIXED-POINT-NONCONVERGENCE")
        );
        assert_eq!(result.step_diagnostics.len(), 1);
        assert_eq!(
            result.step_diagnostics[0].convergence_status,
            "fixed_point_not_converged"
        );
        assert_eq!(
            result.solver_result.output.state_trajectories[0].values,
            vec![0.0]
        );
        assert_eq!(result.algebraic_trajectories[0].values, vec![3.0]);
        assert_eq!(
            result.solver_result.output.algebraic_trajectories[0].values,
            vec![3.0]
        );
    }

    #[test]
    fn dynamic_component_assembly_entrypoint_solves_semi_implicit_residual_graph() {
        let assembly = dynamic_component_assembly_fixture();

        let result = solve_dynamic_component_assembly(
            &assembly,
            DynamicComponentAssemblySolveInput {
                duration_s: 1.0,
                timestep_s: 1.0,
                initial_state: vec![1.0],
                initial_algebraic: vec![0.0],
                inputs: vec![SolverScalar::new("u", "Dimensionless", "1", 5.0)],
                parameters: vec![SolverScalar::new("k", "Dimensionless", "1", 2.0)],
            },
            DynamicComponentOptions::default(),
        )
        .unwrap();

        assert_eq!(result.solver_result.diagnostics.status, "computed");
        assert_eq!(
            result.solver_result.plan.options.method,
            "dynamic_component_assembly_semi_implicit_euler"
        );
        assert_eq!(
            result.solver_result.output.state_trajectories[0].values,
            vec![1.0, 3.0]
        );
        assert_eq!(result.algebraic_trajectories[0].values, vec![2.0, 0.0]);
        assert!(result
            .step_diagnostics
            .iter()
            .all(|diagnostic| diagnostic.convergence_status == "linear_algebraic_converged"));
    }

    fn residual_rhs_graph() -> ResidualGraph {
        ResidualGraph {
            name: "component.rhs".to_owned(),
            variables: vec![
                variable(0, "x", "state"),
                variable(1, "y", "state"),
                variable(2, "u", "input"),
                variable(3, "der_x", "state_derivative"),
                variable(4, "der_y", "state_derivative"),
            ],
            residuals: vec![
                residual(
                    "x_rhs",
                    &[(3, "der_x", 1.0), (0, "x", -1.0), (2, "u", -1.0)],
                    0.0,
                ),
                residual("y_rhs", &[(4, "der_y", 1.0), (1, "y", 1.0)], 0.0),
            ],
            parameters: Vec::new(),
            dependencies: Vec::new(),
        }
    }

    fn parameterized_residual_rhs_graph() -> ResidualGraph {
        ResidualGraph {
            name: "component.parameterized_rhs".to_owned(),
            variables: vec![
                variable(0, "x", "state"),
                variable(1, "u", "input"),
                variable(2, "k", "parameter"),
                variable(3, "der_x", "state_derivative"),
            ],
            residuals: vec![residual(
                "x_rhs",
                &[(3, "der_x", 1.0), (1, "u", -1.0), (2, "k", -1.0)],
                0.0,
            )],
            parameters: Vec::new(),
            dependencies: Vec::new(),
        }
    }

    fn semi_implicit_residual_graph() -> ResidualGraph {
        ResidualGraph {
            name: "component.semi_implicit".to_owned(),
            variables: vec![
                variable(0, "x", "state"),
                variable(1, "z", "algebraic"),
                variable(2, "u", "input"),
                variable(3, "der_x", "state_derivative"),
            ],
            residuals: vec![
                residual("x_rhs", &[(3, "der_x", 1.0), (1, "z", -1.0)], 0.0),
                residual(
                    "z_balance",
                    &[(1, "z", 1.0), (0, "x", 1.0), (2, "u", -1.0)],
                    0.0,
                ),
            ],
            parameters: Vec::new(),
            dependencies: Vec::new(),
        }
    }

    fn parameterized_semi_implicit_residual_graph() -> ResidualGraph {
        ResidualGraph {
            name: "component.parameterized_semi_implicit".to_owned(),
            variables: vec![
                variable(0, "x", "state"),
                variable(1, "z", "algebraic"),
                variable(2, "u", "input"),
                variable(3, "k", "parameter"),
                variable(4, "der_x", "state_derivative"),
            ],
            residuals: vec![
                residual("x_rhs", &[(4, "der_x", 1.0), (1, "z", -1.0)], 0.0),
                residual(
                    "z_balance",
                    &[(1, "z", 1.0), (0, "x", 1.0), (3, "k", 1.0), (2, "u", -1.0)],
                    0.0,
                ),
            ],
            parameters: Vec::new(),
            dependencies: Vec::new(),
        }
    }

    fn dynamic_component_assembly_fixture() -> EquationAssembly {
        let x = assembly_variable("x", "state");
        let z = assembly_variable("z", "algebraic");
        let u = assembly_variable("u", "input");
        let k = assembly_variable("k", "parameter");
        EquationAssembly {
            name: "component_graph".to_owned(),
            generated_equations: vec![
                GeneratedEquation {
                    name: "x_rhs".to_owned(),
                    kind: "dynamic_rhs".to_owned(),
                    domain: "Test".to_owned(),
                    expression: "der(x) eq z".to_owned(),
                    residual: "der_x - z".to_owned(),
                    rhs_value: None,
                    dependencies: vec!["der_x".to_owned(), "z".to_owned()],
                    source: "test".to_owned(),
                    reason: "test dynamic assembly derivative residual".to_owned(),
                    source_line: Some(1),
                    status: "generated".to_owned(),
                },
                GeneratedEquation {
                    name: "z_balance".to_owned(),
                    kind: "dynamic_algebraic".to_owned(),
                    domain: "Test".to_owned(),
                    expression: "z + x + k eq u".to_owned(),
                    residual: "z + x + k - u".to_owned(),
                    rhs_value: None,
                    dependencies: vec![
                        "z".to_owned(),
                        "x".to_owned(),
                        "k".to_owned(),
                        "u".to_owned(),
                    ],
                    source: "test".to_owned(),
                    reason: "test dynamic assembly algebraic residual".to_owned(),
                    source_line: Some(2),
                    status: "generated".to_owned(),
                },
            ],
            unknowns: vec![x.clone(), z.clone()],
            states: vec![x],
            algebraic_variables: vec![z],
            inputs: vec![u],
            parameters: vec![k],
            ..EquationAssembly::default()
        }
    }

    fn assembly_variable(name: &str, role: &str) -> UnknownVariable {
        UnknownVariable {
            name: name.to_owned(),
            role: role.to_owned(),
            quantity_kind: "Dimensionless".to_owned(),
            unit: "1".to_owned(),
            source: format!("Test.{name}"),
            status: "classified".to_owned(),
            value: None,
        }
    }

    fn variable(index: usize, name: &str, role: &str) -> ResidualVariableRef {
        ResidualVariableRef {
            index,
            name: name.to_owned(),
            role: role.to_owned(),
            unit: "1".to_owned(),
        }
    }

    fn residual(name: &str, terms: &[(usize, &str, f64)], rhs_value: f64) -> ResidualEquation {
        ResidualEquation {
            name: name.to_owned(),
            expression: ResidualExpression::manual(name),
            rhs_value,
            unit: ResidualUnit {
                unit: "1".to_owned(),
                quantity_kind: "Dimensionless".to_owned(),
            },
            scale: ResidualScale::default(),
            source: ResidualSource::default(),
            variable_indices: terms.iter().map(|(index, _, _)| *index).collect(),
            terms: terms
                .iter()
                .map(|(index, variable, coefficient)| ResidualTerm {
                    variable_index: *index,
                    variable: (*variable).to_owned(),
                    coefficient: *coefficient,
                })
                .collect(),
        }
    }
}
