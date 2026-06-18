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
        validate_time_grid(&self.time_grid)?;
        validate_solver_options(&self.plan.options, &self.time_grid)?;
        if self.initial_state.len() != self.state_layout.len() {
            return Err(SolverFailure::new(
                "E-SOLVER-STATE-LAYOUT-MISMATCH",
                "initial state vector length does not match the state layout",
            ));
        }
        validate_state_values(&self.state_layout, &self.initial_state)?;
        validate_scalar_layout("input", &self.input_layout.entries, &self.inputs)?;
        validate_scalar_layout(
            "parameter",
            &self.parameter_layout.entries,
            &self.parameters,
        )?;
        validate_output_layout(
            &self.plan.simulation.outputs,
            &self.state_layout,
            &self.output_layout,
        )?;
        Ok(())
    }
}

fn validate_time_grid(time_grid: &TimeGrid) -> Result<(), SolverFailure> {
    if !time_grid.duration_s.is_finite() || time_grid.duration_s <= 0.0 {
        return Err(SolverFailure::new(
            "E-SIM-DURATION-INVALID",
            "simulation time grid duration must be a positive finite number of seconds",
        ));
    }
    if !time_grid.timestep_s.is_finite() || time_grid.timestep_s <= 0.0 {
        return Err(SolverFailure::new(
            "E-SIM-TIMESTEP-INVALID",
            "simulation time grid timestep must be a positive finite number of seconds",
        ));
    }
    if time_grid.step_count == 0 {
        return Err(SolverFailure::new(
            "E-SOLVER-TIMEGRID-INVALID",
            "simulation time grid must contain at least one step",
        ));
    }
    if time_grid.unit.trim().is_empty() {
        return Err(SolverFailure::new(
            "E-SOLVER-TIMEGRID-INVALID",
            "simulation time grid unit must be present",
        ));
    }
    Ok(())
}

fn validate_solver_options(
    options: &SolverOptions,
    time_grid: &TimeGrid,
) -> Result<(), SolverFailure> {
    if options.method.trim().is_empty() {
        return Err(SolverFailure::new(
            "E-SOLVER-METHOD-INVALID",
            "solver method must be present",
        ));
    }
    if !options.timestep_s.is_finite() || options.timestep_s <= 0.0 {
        return Err(SolverFailure::new(
            "E-SIM-TIMESTEP-INVALID",
            "solver option timestep must be a positive finite number of seconds",
        ));
    }
    let timestep_tolerance = options
        .timestep_s
        .abs()
        .max(time_grid.timestep_s.abs())
        .max(1.0)
        * f64::EPSILON
        * 16.0;
    if (options.timestep_s - time_grid.timestep_s).abs() > timestep_tolerance {
        return Err(SolverFailure::new(
            "E-SOLVER-TIMESTEP-MISMATCH",
            "solver option timestep does not match the simulation time grid timestep",
        ));
    }
    if !options.tolerance.is_finite() || options.tolerance <= 0.0 {
        return Err(SolverFailure::new(
            "E-SOLVER-TOLERANCE-INVALID",
            "solver tolerance must be a positive finite number",
        ));
    }
    if options.max_iterations == 0 {
        return Err(SolverFailure::new(
            "E-SOLVER-MAX-ITERATIONS-INVALID",
            "solver max_iterations must be greater than zero",
        ));
    }
    Ok(())
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

fn validate_state_values(state_layout: &StateLayout, values: &[f64]) -> Result<(), SolverFailure> {
    for (entry, value) in state_layout.entries.iter().zip(values.iter().copied()) {
        if !value.is_finite() {
            return Err(SolverFailure::new(
                "E-SOLVER-STATE-VALUE-INVALID",
                format!("initial state `{}` must be finite", entry.name),
            ));
        }
    }
    Ok(())
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
        if !value.value.is_finite() {
            return Err(SolverFailure::new(
                format!("E-SOLVER-{}-VALUE-INVALID", role.to_ascii_uppercase()),
                format!("{role} scalar `{}` must be finite", value.name),
            ));
        }
    }
    Ok(())
}

fn validate_output_layout(
    declared_outputs: &[String],
    state_layout: &StateLayout,
    output_layout: &OutputLayout,
) -> Result<(), SolverFailure> {
    if output_layout.is_empty() {
        return Ok(());
    }
    if !declared_outputs.is_empty() && declared_outputs.len() != output_layout.len() {
        return Err(SolverFailure::new(
            "E-SOLVER-OUTPUT-LAYOUT-MISMATCH",
            "declared solver outputs do not match the output layout",
        ));
    }
    for output_name in declared_outputs {
        if output_layout.get(output_name).is_none() {
            return Err(SolverFailure::new(
                "E-SOLVER-OUTPUT-LAYOUT-MISMATCH",
                format!("declared solver output `{output_name}` is missing from the output layout"),
            ));
        }
    }
    for output in &output_layout.entries {
        let Some(state) = state_layout.get(&output.name) else {
            return Err(SolverFailure::new(
                "E-SOLVER-OUTPUT-LAYOUT-MISMATCH",
                format!(
                    "output `{}` does not resolve to a state layout entry",
                    output.name
                ),
            ));
        };
        if output.quantity_kind != state.quantity_kind
            || output.canonical_unit != state.canonical_unit
        {
            return Err(SolverFailure::new(
                "E-SOLVER-OUTPUT-LAYOUT-MISMATCH",
                format!(
                    "output `{}` quantity/unit metadata does not match the state layout",
                    output.name
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

    pub fn step_dt_s(&self, step: usize) -> f64 {
        if step == 0 {
            return 0.0;
        }
        self.step_time_s(step) - self.step_time_s(step - 1)
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

        let time_grid = TimeGrid::fixed_step(2.5, 1.0).unwrap();
        assert_eq!(time_grid.step_count, 3);
        assert_eq!(time_grid.step_time_s(3), 2.5);
        assert_eq!(time_grid.step_dt_s(1), 1.0);
        assert_eq!(time_grid.step_dt_s(2), 1.0);
        assert_eq!(time_grid.step_dt_s(3), 0.5);
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

        let mut state_value_invalid = input.clone();
        state_value_invalid.initial_state[0] = f64::NAN;
        assert_eq!(
            state_value_invalid.validate_layouts().unwrap_err().code,
            "E-SOLVER-STATE-VALUE-INVALID"
        );

        let mut input_count_mismatch = input.clone();
        input_count_mismatch.inputs.clear();
        assert_eq!(
            input_count_mismatch.validate_layouts().unwrap_err().code,
            "E-SOLVER-INPUT-LAYOUT-MISMATCH"
        );

        let mut input_value_invalid = input.clone();
        input_value_invalid.inputs[0].value = f64::INFINITY;
        assert_eq!(
            input_value_invalid.validate_layouts().unwrap_err().code,
            "E-SOLVER-INPUT-VALUE-INVALID"
        );

        let mut output_plan_mismatch = input.clone();
        output_plan_mismatch.plan.simulation.outputs = vec!["T_missing".to_owned()];
        assert_eq!(
            output_plan_mismatch.validate_layouts().unwrap_err().code,
            "E-SOLVER-OUTPUT-LAYOUT-MISMATCH"
        );

        let mut output_state_mismatch = input.clone();
        output_state_mismatch.output_layout = OutputLayout::new(vec![LayoutEntry::new(
            0,
            "T_missing",
            "AbsoluteTemperature",
            "K",
            "degC",
        )]);
        assert_eq!(
            output_state_mismatch.validate_layouts().unwrap_err().code,
            "E-SOLVER-OUTPUT-LAYOUT-MISMATCH"
        );

        let mut output_quantity_mismatch = input.clone();
        output_quantity_mismatch.output_layout =
            OutputLayout::new(vec![LayoutEntry::new(0, "T_zone", "HeatRate", "W", "W")]);
        assert_eq!(
            output_quantity_mismatch
                .validate_layouts()
                .unwrap_err()
                .code,
            "E-SOLVER-OUTPUT-LAYOUT-MISMATCH"
        );

        let mut empty_output_layout = input.clone();
        empty_output_layout.output_layout = OutputLayout::default();
        empty_output_layout.validate_layouts().unwrap();

        let mut invalid_method = input.clone();
        invalid_method.plan.options.method = "  ".to_owned();
        assert_eq!(
            invalid_method.validate_layouts().unwrap_err().code,
            "E-SOLVER-METHOD-INVALID"
        );

        let mut invalid_option_timestep = input.clone();
        invalid_option_timestep.plan.options.timestep_s = f64::NAN;
        assert_eq!(
            invalid_option_timestep.validate_layouts().unwrap_err().code,
            "E-SIM-TIMESTEP-INVALID"
        );

        let mut mismatched_option_timestep = input.clone();
        mismatched_option_timestep.plan.options.timestep_s = 30.0;
        assert_eq!(
            mismatched_option_timestep
                .validate_layouts()
                .unwrap_err()
                .code,
            "E-SOLVER-TIMESTEP-MISMATCH"
        );

        let mut invalid_tolerance = input.clone();
        invalid_tolerance.plan.options.tolerance = 0.0;
        assert_eq!(
            invalid_tolerance.validate_layouts().unwrap_err().code,
            "E-SOLVER-TOLERANCE-INVALID"
        );

        let mut invalid_max_iterations = input.clone();
        invalid_max_iterations.plan.options.max_iterations = 0;
        assert_eq!(
            invalid_max_iterations.validate_layouts().unwrap_err().code,
            "E-SOLVER-MAX-ITERATIONS-INVALID"
        );

        let mut invalid_time_grid = input.clone();
        invalid_time_grid.time_grid.step_count = 0;
        assert_eq!(
            invalid_time_grid.validate_layouts().unwrap_err().code,
            "E-SOLVER-TIMEGRID-INVALID"
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

        let mut parameter_value_invalid = parameter_scalar_mismatch;
        parameter_value_invalid.parameters[0].canonical_unit = "W/K".to_owned();
        parameter_value_invalid.parameters[0].value = f64::NEG_INFINITY;
        assert_eq!(
            parameter_value_invalid.validate_layouts().unwrap_err().code,
            "E-SOLVER-PARAMETER-VALUE-INVALID"
        );
    }
}
