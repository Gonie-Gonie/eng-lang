use super::{solve_fixed_step_ode, FixedStepMethod, SolverFailure, SolverInput, SolverResult};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FirstOrderThermalModel {
    pub heat_capacity_j_per_k: f64,
    pub conductance_w_per_k: f64,
    pub internal_heat_w: f64,
}

impl FirstOrderThermalModel {
    pub fn new(
        heat_capacity_j_per_k: f64,
        conductance_w_per_k: f64,
        internal_heat_w: f64,
    ) -> Result<Self, SolverFailure> {
        if !heat_capacity_j_per_k.is_finite() || heat_capacity_j_per_k <= 0.0 {
            return Err(SolverFailure::new(
                "E-SOLVER-THERMAL-CAPACITY-INVALID",
                "first-order thermal solver requires positive finite heat capacity",
            ));
        }
        if !conductance_w_per_k.is_finite() || conductance_w_per_k < 0.0 {
            return Err(SolverFailure::new(
                "E-SOLVER-THERMAL-CONDUCTANCE-INVALID",
                "first-order thermal solver requires non-negative finite conductance",
            ));
        }
        if !internal_heat_w.is_finite() {
            return Err(SolverFailure::new(
                "E-SOLVER-THERMAL-HEAT-INVALID",
                "first-order thermal solver requires finite internal heat input",
            ));
        }
        Ok(Self {
            heat_capacity_j_per_k,
            conductance_w_per_k,
            internal_heat_w,
        })
    }
}

pub fn solve_first_order_thermal<F>(
    method: FixedStepMethod,
    input: &SolverInput,
    model: FirstOrderThermalModel,
    mut outdoor_temperature_k_at: F,
) -> Result<SolverResult, SolverFailure>
where
    F: FnMut(f64) -> Result<f64, SolverFailure>,
{
    if input.state_layout.len() != 1 {
        return Err(SolverFailure::new(
            "E-SIM-SYSTEM-SHAPE-UNSUPPORTED",
            "first-order thermal solver requires exactly one state variable",
        ));
    }

    solve_fixed_step_ode(method, input, |sample| {
        let temperature_k = sample.state[0];
        let outdoor_temperature_k = outdoor_temperature_k_at(sample.time_s)?;
        if !outdoor_temperature_k.is_finite() {
            return Err(SolverFailure::new(
                "E-SOLVER-THERMAL-INPUT-INVALID",
                "first-order thermal solver requires finite outdoor temperature input",
            ));
        }
        let derivative_k_per_s = (model.conductance_w_per_k
            * (outdoor_temperature_k - temperature_k)
            + model.internal_heat_w)
            / model.heat_capacity_j_per_k;
        Ok(vec![derivative_k_per_s])
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::solver::{
        InputLayout, LayoutEntry, OutputLayout, ParameterLayout, SimulationPlan, SolverOptions,
        SolverPlan, SolverScalar, StateLayout, TimeGrid,
    };

    #[test]
    fn solves_first_order_thermal_model_through_fixed_step_api() {
        let method = FixedStepMethod::ExplicitEuler;
        let input = SolverInput {
            plan: SolverPlan::new(
                "RoomThermal",
                SimulationPlan {
                    states: vec!["T_zone".to_owned()],
                    inputs: vec!["T_out".to_owned(), "Q_internal".to_owned()],
                    parameters: vec!["C".to_owned(), "UA".to_owned()],
                    outputs: vec!["T_zone".to_owned()],
                },
                SolverOptions::fixed_step(method.method_name(""), 1.0),
            ),
            time_grid: TimeGrid::fixed_step(2.0, 1.0).unwrap(),
            state_layout: StateLayout::new(vec![LayoutEntry::new(
                0,
                "T_zone",
                "AbsoluteTemperature",
                "K",
                "degC",
            )]),
            input_layout: InputLayout::new(vec![
                LayoutEntry::new(0, "T_out", "AbsoluteTemperature", "K", "degC"),
                LayoutEntry::new(1, "Q_internal", "HeatRate", "W", "W"),
            ]),
            parameter_layout: ParameterLayout::new(vec![
                LayoutEntry::new(0, "C", "HeatCapacity", "J/K", "J/K"),
                LayoutEntry::new(1, "UA", "Conductance", "W/K", "W/K"),
            ]),
            output_layout: OutputLayout::new(vec![LayoutEntry::new(
                0,
                "T_zone",
                "AbsoluteTemperature",
                "K",
                "degC",
            )]),
            initial_state: vec![300.0],
            inputs: vec![
                SolverScalar::new("T_out", "AbsoluteTemperature", "K", 290.0),
                SolverScalar::new("Q_internal", "HeatRate", "W", 0.0),
            ],
            parameters: vec![
                SolverScalar::new("C", "HeatCapacity", "J/K", 10.0),
                SolverScalar::new("UA", "Conductance", "W/K", 2.0),
            ],
        };
        let model = FirstOrderThermalModel::new(10.0, 2.0, 0.0).unwrap();

        let result = solve_first_order_thermal(method, &input, model, |_| Ok(290.0)).unwrap();

        assert_eq!(
            result.output.state_trajectories[0].values,
            vec![300.0, 298.0, 296.4]
        );
        assert_eq!(
            result.diagnostics.convergence_status,
            "fixed_step_completed"
        );
    }
}
