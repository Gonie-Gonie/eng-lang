use super::diagnostics::SolverFailure;

#[derive(Clone, Debug, PartialEq)]
pub struct SolverPlan {
    pub system: String,
    pub simulation: SimulationPlan,
    pub options: SolverOptions,
}

impl SolverPlan {
    pub fn new(
        system: impl Into<String>,
        simulation: SimulationPlan,
        options: SolverOptions,
    ) -> Self {
        Self {
            system: system.into(),
            simulation,
            options,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct SimulationPlan {
    pub inputs: Vec<String>,
    pub outputs: Vec<String>,
    pub states: Vec<String>,
    pub parameters: Vec<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SolverOptions {
    pub method: String,
    pub timestep_s: f64,
    pub tolerance: f64,
    pub max_iterations: usize,
}

impl SolverOptions {
    pub fn fixed_step(method: impl Into<String>, timestep_s: f64) -> Self {
        Self {
            method: method.into(),
            timestep_s,
            tolerance: 1e-9,
            max_iterations: 1,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct SolverInput {
    pub plan: SolverPlan,
    pub time_grid: TimeGrid,
    pub state_layout: StateLayout,
    pub input_layout: InputLayout,
    pub parameter_layout: ParameterLayout,
    pub output_layout: OutputLayout,
    pub initial_state: Vec<f64>,
    pub inputs: Vec<SolverScalar>,
    pub parameters: Vec<SolverScalar>,
}

impl SolverInput {
    pub fn validate_layouts(&self) -> Result<(), SolverFailure> {
        if self.initial_state.len() != self.state_layout.len() {
            return Err(SolverFailure::new(
                "E-SOLVER-STATE-LAYOUT-MISMATCH",
                "initial state vector length does not match the state layout",
            ));
        }
        validate_scalar_layout("input", &self.input_layout.entries, &self.inputs)?;
        validate_scalar_layout(
            "parameter",
            &self.parameter_layout.entries,
            &self.parameters,
        )?;
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct SolverScalar {
    pub name: String,
    pub quantity_kind: String,
    pub canonical_unit: String,
    pub value: f64,
}

impl SolverScalar {
    pub fn new(
        name: impl Into<String>,
        quantity_kind: impl Into<String>,
        canonical_unit: impl Into<String>,
        value: f64,
    ) -> Self {
        Self {
            name: name.into(),
            quantity_kind: quantity_kind.into(),
            canonical_unit: canonical_unit.into(),
            value,
        }
    }
}

fn validate_scalar_layout(
    role: &str,
    layout: &[LayoutEntry],
    values: &[SolverScalar],
) -> Result<(), SolverFailure> {
    if layout.len() != values.len() {
        return Err(SolverFailure::new(
            format!("E-SOLVER-{}-LAYOUT-MISMATCH", role.to_ascii_uppercase()),
            format!("{role} value count does not match the {role} layout"),
        ));
    }
    for (entry, value) in layout.iter().zip(values.iter()) {
        if entry.name != value.name
            || entry.quantity_kind != value.quantity_kind
            || entry.canonical_unit != value.canonical_unit
        {
            return Err(SolverFailure::new(
                format!("E-SOLVER-{}-SCALAR-MISMATCH", role.to_ascii_uppercase()),
                format!(
                    "{role} scalar `{}` does not match layout entry `{}`",
                    value.name, entry.name
                ),
            ));
        }
    }
    Ok(())
}

#[derive(Clone, Debug, PartialEq)]
pub struct TimeGrid {
    pub start_s: f64,
    pub duration_s: f64,
    pub timestep_s: f64,
    pub step_count: usize,
    pub unit: String,
}

impl TimeGrid {
    pub fn fixed_step(duration_s: f64, timestep_s: f64) -> Result<Self, SolverFailure> {
        if !duration_s.is_finite() || duration_s <= 0.0 {
            return Err(SolverFailure::new(
                "E-SIM-DURATION-INVALID",
                "simulation duration must be a positive finite number of seconds",
            ));
        }
        if !timestep_s.is_finite() || timestep_s <= 0.0 {
            return Err(SolverFailure::new(
                "E-SIM-TIMESTEP-INVALID",
                "solver timestep must be a positive finite number of seconds",
            ));
        }
        Ok(Self {
            start_s: 0.0,
            duration_s,
            timestep_s,
            step_count: (duration_s / timestep_s).ceil() as usize,
            unit: "s".to_owned(),
        })
    }

    pub fn step_time_s(&self, step: usize) -> f64 {
        (step as f64 * self.timestep_s).min(self.duration_s)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct LayoutEntry {
    pub index: usize,
    pub name: String,
    pub quantity_kind: String,
    pub canonical_unit: String,
    pub display_unit: String,
}

impl LayoutEntry {
    pub fn new(
        index: usize,
        name: impl Into<String>,
        quantity_kind: impl Into<String>,
        canonical_unit: impl Into<String>,
        display_unit: impl Into<String>,
    ) -> Self {
        Self {
            index,
            name: name.into(),
            quantity_kind: quantity_kind.into(),
            canonical_unit: canonical_unit.into(),
            display_unit: display_unit.into(),
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct StateLayout {
    pub entries: Vec<LayoutEntry>,
}

impl StateLayout {
    pub fn new(entries: Vec<LayoutEntry>) -> Self {
        Self { entries }
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn get(&self, name: &str) -> Option<&LayoutEntry> {
        self.entries.iter().find(|entry| entry.name == name)
    }

    pub fn index_of(&self, name: &str) -> Option<usize> {
        self.get(name).map(|entry| entry.index)
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct InputLayout {
    pub entries: Vec<LayoutEntry>,
}

impl InputLayout {
    pub fn new(entries: Vec<LayoutEntry>) -> Self {
        Self { entries }
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn get(&self, name: &str) -> Option<&LayoutEntry> {
        self.entries.iter().find(|entry| entry.name == name)
    }

    pub fn index_of(&self, name: &str) -> Option<usize> {
        self.get(name).map(|entry| entry.index)
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ParameterLayout {
    pub entries: Vec<LayoutEntry>,
}

impl ParameterLayout {
    pub fn new(entries: Vec<LayoutEntry>) -> Self {
        Self { entries }
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn get(&self, name: &str) -> Option<&LayoutEntry> {
        self.entries.iter().find(|entry| entry.name == name)
    }

    pub fn index_of(&self, name: &str) -> Option<usize> {
        self.get(name).map(|entry| entry.index)
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct OutputLayout {
    pub entries: Vec<LayoutEntry>,
}

impl OutputLayout {
    pub fn new(entries: Vec<LayoutEntry>) -> Self {
        Self { entries }
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn get(&self, name: &str) -> Option<&LayoutEntry> {
        self.entries.iter().find(|entry| entry.name == name)
    }

    pub fn index_of(&self, name: &str) -> Option<usize> {
        self.get(name).map(|entry| entry.index)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layouts_preserve_named_indices_and_units() {
        let states = StateLayout::new(vec![
            LayoutEntry::new(0, "T_air", "AbsoluteTemperature", "K", "degC"),
            LayoutEntry::new(1, "T_wall", "AbsoluteTemperature", "K", "degC"),
        ]);
        let inputs = InputLayout::new(vec![LayoutEntry::new(
            0,
            "T_out",
            "AbsoluteTemperature",
            "K",
            "degC",
        )]);
        let parameters =
            ParameterLayout::new(vec![LayoutEntry::new(0, "UA", "Conductance", "W/K", "W/K")]);
        let outputs = OutputLayout::new(vec![
            LayoutEntry::new(0, "T_air", "AbsoluteTemperature", "K", "degC"),
            LayoutEntry::new(1, "T_wall", "AbsoluteTemperature", "K", "degC"),
        ]);

        assert_eq!(states.len(), 2);
        assert_eq!(states.index_of("T_wall"), Some(1));
        assert_eq!(states.get("T_air").unwrap().display_unit, "degC");
        assert_eq!(inputs.len(), 1);
        assert_eq!(inputs.index_of("T_out"), Some(0));
        assert_eq!(parameters.len(), 1);
        assert_eq!(parameters.get("UA").unwrap().canonical_unit, "W/K");
        assert_eq!(outputs.len(), 2);
        assert_eq!(outputs.index_of("missing"), None);
        assert!(InputLayout::default().is_empty());
        assert!(ParameterLayout::default().is_empty());
        assert!(OutputLayout::default().is_empty());
    }

    #[test]
    fn solver_input_validates_numeric_layout_contract() {
        let input = SolverInput {
            plan: SolverPlan::new(
                "RoomThermal",
                SimulationPlan {
                    states: vec!["T_zone".to_owned()],
                    inputs: vec!["T_out".to_owned()],
                    parameters: vec!["UA".to_owned()],
                    outputs: vec!["T_zone".to_owned()],
                },
                SolverOptions::fixed_step("explicit_euler_fixed_step", 60.0),
            ),
            time_grid: TimeGrid::fixed_step(120.0, 60.0).unwrap(),
            state_layout: StateLayout::new(vec![LayoutEntry::new(
                0,
                "T_zone",
                "AbsoluteTemperature",
                "K",
                "degC",
            )]),
            input_layout: InputLayout::new(vec![LayoutEntry::new(
                0,
                "T_out",
                "AbsoluteTemperature",
                "K",
                "degC",
            )]),
            parameter_layout: ParameterLayout::new(vec![LayoutEntry::new(
                0,
                "UA",
                "Conductance",
                "W/K",
                "W/K",
            )]),
            output_layout: OutputLayout::new(vec![LayoutEntry::new(
                0,
                "T_zone",
                "AbsoluteTemperature",
                "K",
                "degC",
            )]),
            initial_state: vec![295.15],
            inputs: vec![SolverScalar::new(
                "T_out",
                "AbsoluteTemperature",
                "K",
                283.15,
            )],
            parameters: vec![SolverScalar::new("UA", "Conductance", "W/K", 160.0)],
        };

        input.validate_layouts().unwrap();

        let mut state_mismatch = input.clone();
        state_mismatch.initial_state.push(300.0);
        assert_eq!(
            state_mismatch.validate_layouts().unwrap_err().code,
            "E-SOLVER-STATE-LAYOUT-MISMATCH"
        );

        let mut input_count_mismatch = input.clone();
        input_count_mismatch.inputs.clear();
        assert_eq!(
            input_count_mismatch.validate_layouts().unwrap_err().code,
            "E-SOLVER-INPUT-LAYOUT-MISMATCH"
        );

        let mut parameter_scalar_mismatch = input;
        parameter_scalar_mismatch.parameters[0].canonical_unit = "kW/K".to_owned();
        assert_eq!(
            parameter_scalar_mismatch
                .validate_layouts()
                .unwrap_err()
                .code,
            "E-SOLVER-PARAMETER-SCALAR-MISMATCH"
        );
    }
}
