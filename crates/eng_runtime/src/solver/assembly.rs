use std::collections::HashSet;

use super::{
    diagnostics::SolverFailure,
    plan::{InputLayout, LayoutEntry, ParameterLayout, StateLayout},
};

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ComponentInstance {
    pub name: String,
    pub component_type: String,
    pub ports: Vec<PortInstance>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct PortInstance {
    pub name: String,
    pub component: String,
    pub domain: String,
    pub medium: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ConnectionEdge {
    pub from: String,
    pub to: String,
    pub source_line: usize,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ConnectionSet {
    pub name: String,
    pub domain: String,
    pub ports: Vec<String>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct GeneratedEquation {
    pub name: String,
    pub kind: String,
    pub domain: String,
    pub expression: String,
    pub residual: String,
    pub rhs_value: Option<f64>,
    pub dependencies: Vec<String>,
    pub source: String,
    pub reason: String,
    pub source_line: Option<usize>,
    pub status: String,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ComponentEquation {
    pub name: String,
    pub expression: String,
    pub component: String,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct UnknownVariable {
    pub name: String,
    pub role: String,
    pub quantity_kind: String,
    pub unit: String,
    pub source: String,
    pub status: String,
    pub value: Option<f64>,
}

pub type StateVariable = UnknownVariable;
pub type AlgebraicVariable = UnknownVariable;
pub type InputVariable = UnknownVariable;
pub type ParameterVariable = UnknownVariable;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct EquationAssembly {
    pub name: String,
    pub components: Vec<ComponentInstance>,
    pub ports: Vec<PortInstance>,
    pub connections: Vec<ConnectionEdge>,
    pub connection_sets: Vec<ConnectionSet>,
    pub generated_equations: Vec<GeneratedEquation>,
    pub component_equations: Vec<ComponentEquation>,
    pub unknowns: Vec<UnknownVariable>,
    pub states: Vec<StateVariable>,
    pub algebraic_variables: Vec<AlgebraicVariable>,
    pub inputs: Vec<InputVariable>,
    pub parameters: Vec<ParameterVariable>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct DynamicComponentAssemblySplit {
    pub assembly: String,
    pub state_layout: StateLayout,
    pub algebraic_layout: StateLayout,
    pub input_layout: InputLayout,
    pub parameter_layout: ParameterLayout,
    pub equation_count: usize,
    pub unknown_count: usize,
}

impl EquationAssembly {
    pub fn equation_count(&self) -> usize {
        self.generated_equations.len() + self.component_equations.len()
    }

    pub fn unknown_count(&self) -> usize {
        self.unknowns.len()
    }

    pub fn dynamic_component_split(&self) -> Result<DynamicComponentAssemblySplit, SolverFailure> {
        if self.states.is_empty() {
            return Err(SolverFailure::new(
                "E-DYNAMIC-COMPONENT-SPLIT-SHAPE",
                "dynamic component assembly split requires at least one state variable",
            ));
        }
        validate_role_list("state", &self.states)?;
        validate_role_list("algebraic", &self.algebraic_variables)?;
        validate_role_list("input", &self.inputs)?;
        validate_role_list("parameter", &self.parameters)?;
        validate_unique_split_names(self)?;
        validate_unknown_split_consistency(self)?;

        Ok(DynamicComponentAssemblySplit {
            assembly: self.name.clone(),
            state_layout: StateLayout::new(layout_entries(&self.states)),
            algebraic_layout: StateLayout::new(layout_entries(&self.algebraic_variables)),
            input_layout: InputLayout::new(layout_entries(&self.inputs)),
            parameter_layout: ParameterLayout::new(layout_entries(&self.parameters)),
            equation_count: self.equation_count(),
            unknown_count: self.unknown_count(),
        })
    }
}

fn validate_role_list(
    expected_role: &str,
    variables: &[UnknownVariable],
) -> Result<(), SolverFailure> {
    if let Some(variable) = variables
        .iter()
        .find(|variable| variable.role != expected_role)
    {
        return Err(SolverFailure::new(
            "E-DYNAMIC-COMPONENT-SPLIT-ROLE",
            format!(
                "dynamic component split expected `{}` to have role `{expected_role}`, got `{}`",
                variable.name, variable.role
            ),
        ));
    }
    Ok(())
}

fn validate_unique_split_names(assembly: &EquationAssembly) -> Result<(), SolverFailure> {
    let mut names = HashSet::new();
    for variable in assembly
        .states
        .iter()
        .chain(assembly.algebraic_variables.iter())
        .chain(assembly.inputs.iter())
        .chain(assembly.parameters.iter())
    {
        if !names.insert(variable.name.as_str()) {
            return Err(SolverFailure::new(
                "E-DYNAMIC-COMPONENT-SPLIT-DUPLICATE",
                format!(
                    "dynamic component split contains duplicate variable `{}`",
                    variable.name
                ),
            ));
        }
    }
    Ok(())
}

fn validate_unknown_split_consistency(assembly: &EquationAssembly) -> Result<(), SolverFailure> {
    let expected_unknowns = assembly
        .states
        .iter()
        .chain(assembly.algebraic_variables.iter())
        .map(|variable| variable.name.as_str())
        .collect::<HashSet<_>>();
    let actual_unknowns = assembly
        .unknowns
        .iter()
        .map(|variable| variable.name.as_str())
        .collect::<HashSet<_>>();

    for unknown in &assembly.unknowns {
        if unknown.role != "state" && unknown.role != "algebraic" {
            return Err(SolverFailure::new(
                "E-DYNAMIC-COMPONENT-SPLIT-ROLE",
                format!(
                    "dynamic component split unknown `{}` has unsupported role `{}`",
                    unknown.name, unknown.role
                ),
            ));
        }
        if !expected_unknowns.contains(unknown.name.as_str()) {
            return Err(SolverFailure::new(
                "E-DYNAMIC-COMPONENT-SPLIT-SHAPE",
                format!(
                    "dynamic component split unknown `{}` is missing from state/algebraic lists",
                    unknown.name
                ),
            ));
        }
    }

    for expected in expected_unknowns {
        if !actual_unknowns.contains(expected) {
            return Err(SolverFailure::new(
                "E-DYNAMIC-COMPONENT-SPLIT-SHAPE",
                format!("dynamic component split variable `{expected}` is missing from unknowns"),
            ));
        }
    }

    Ok(())
}

fn layout_entries(variables: &[UnknownVariable]) -> Vec<LayoutEntry> {
    variables
        .iter()
        .enumerate()
        .map(|(index, variable)| {
            LayoutEntry::new(
                index,
                variable.name.clone(),
                variable.quantity_kind.clone(),
                variable.unit.clone(),
                variable.unit.clone(),
            )
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dynamic_component_split_builds_solver_layouts() {
        let assembly = EquationAssembly {
            name: "component_graph".to_owned(),
            unknowns: vec![variable("x", "state"), variable("z", "algebraic")],
            states: vec![variable("x", "state")],
            algebraic_variables: vec![variable("z", "algebraic")],
            inputs: vec![variable("u", "input")],
            parameters: vec![variable("p", "parameter")],
            generated_equations: vec![GeneratedEquation::default(), GeneratedEquation::default()],
            ..EquationAssembly::default()
        };

        let split = assembly.dynamic_component_split().unwrap();

        assert_eq!(split.assembly, "component_graph");
        assert_eq!(split.state_layout.entries[0].name, "x");
        assert_eq!(split.algebraic_layout.entries[0].name, "z");
        assert_eq!(split.input_layout.entries[0].name, "u");
        assert_eq!(split.parameter_layout.entries[0].name, "p");
        assert_eq!(split.equation_count, 2);
        assert_eq!(split.unknown_count, 2);
    }

    #[test]
    fn dynamic_component_split_rejects_invalid_roles_and_shapes() {
        let mut assembly = EquationAssembly {
            name: "component_graph".to_owned(),
            unknowns: vec![variable("x", "state")],
            states: vec![variable("x", "algebraic")],
            ..EquationAssembly::default()
        };

        let failure = assembly.dynamic_component_split().unwrap_err();
        assert_eq!(failure.code, "E-DYNAMIC-COMPONENT-SPLIT-ROLE");

        assembly.states = vec![variable("x", "state")];
        assembly.inputs = vec![variable("x", "input")];
        let failure = assembly.dynamic_component_split().unwrap_err();
        assert_eq!(failure.code, "E-DYNAMIC-COMPONENT-SPLIT-DUPLICATE");

        assembly.inputs.clear();
        assembly.unknowns.clear();
        let failure = assembly.dynamic_component_split().unwrap_err();
        assert_eq!(failure.code, "E-DYNAMIC-COMPONENT-SPLIT-SHAPE");
    }

    fn variable(name: &str, role: &str) -> UnknownVariable {
        UnknownVariable {
            name: name.to_owned(),
            role: role.to_owned(),
            quantity_kind: "Dimensionless".to_owned(),
            unit: "1".to_owned(),
            source: "test".to_owned(),
            status: "active".to_owned(),
            value: None,
        }
    }
}
