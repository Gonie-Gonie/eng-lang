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
    pub expression: String,
    pub source: String,
    pub reason: String,
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

impl EquationAssembly {
    pub fn equation_count(&self) -> usize {
        self.generated_equations.len() + self.component_equations.len()
    }

    pub fn unknown_count(&self) -> usize {
        self.unknowns.len()
    }
}
