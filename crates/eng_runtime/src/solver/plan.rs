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
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct InputLayout {
    pub entries: Vec<LayoutEntry>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ParameterLayout {
    pub entries: Vec<LayoutEntry>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct OutputLayout {
    pub entries: Vec<LayoutEntry>,
}
