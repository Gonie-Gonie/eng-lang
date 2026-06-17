use super::SolverFailure;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DelayInterpolationPolicy {
    Linear,
    PreviousSample,
}

impl DelayInterpolationPolicy {
    fn as_str(&self) -> &'static str {
        match self {
            Self::Linear => "linear",
            Self::PreviousSample => "previous_sample",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DelayInitialHistoryPolicy {
    HoldInitial,
    ErrorBeforeHistory,
}

impl DelayInitialHistoryPolicy {
    fn as_str(&self) -> &'static str {
        match self {
            Self::HoldInitial => "hold_initial",
            Self::ErrorBeforeHistory => "error_before_history",
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct DelayRelationshipArtifact {
    pub signal_name: String,
    pub quantity_kind: String,
    pub canonical_unit: String,
    pub delay_s: f64,
    pub interpolation_policy: String,
    pub initial_history_policy: String,
    pub sample_count: usize,
    pub status: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DelayEvaluation {
    pub value: f64,
    pub source_time_s: f64,
    pub status: String,
    pub relationship: DelayRelationshipArtifact,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DelayRhsEvaluation {
    pub derivatives: Vec<f64>,
    pub delay: DelayEvaluation,
    pub status: String,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct DelaySample {
    time_s: f64,
    value: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DelayBuffer {
    signal_name: String,
    quantity_kind: String,
    canonical_unit: String,
    delay_s: f64,
    interpolation_policy: DelayInterpolationPolicy,
    initial_history_policy: DelayInitialHistoryPolicy,
    samples: Vec<DelaySample>,
}

impl DelayBuffer {
    pub fn new(
        signal_name: impl Into<String>,
        quantity_kind: impl Into<String>,
        canonical_unit: impl Into<String>,
        delay_s: f64,
        interpolation_policy: DelayInterpolationPolicy,
        initial_history_policy: DelayInitialHistoryPolicy,
    ) -> Result<Self, SolverFailure> {
        if !delay_s.is_finite() || delay_s < 0.0 {
            return Err(SolverFailure::new(
                "E-DELAY-DURATION",
                "delay duration must be a finite non-negative number of seconds",
            ));
        }
        Ok(Self {
            signal_name: signal_name.into(),
            quantity_kind: quantity_kind.into(),
            canonical_unit: canonical_unit.into(),
            delay_s,
            interpolation_policy,
            initial_history_policy,
            samples: Vec::new(),
        })
    }

    pub fn record(&mut self, time_s: f64, value: f64) -> Result<(), SolverFailure> {
        if !time_s.is_finite() || !value.is_finite() {
            return Err(SolverFailure::new(
                "E-DELAY-SAMPLE-FINITE",
                "delay samples require finite time and value",
            ));
        }
        if let Some(last) = self.samples.last_mut() {
            if time_s < last.time_s {
                return Err(SolverFailure::new(
                    "E-DELAY-TIME-ORDER",
                    "delay samples must be recorded in nondecreasing time order",
                ));
            }
            if time_s == last.time_s {
                last.value = value;
                return Ok(());
            }
        }
        self.samples.push(DelaySample { time_s, value });
        Ok(())
    }

    pub fn evaluate(&self, time_s: f64) -> Result<DelayEvaluation, SolverFailure> {
        if !time_s.is_finite() {
            return Err(SolverFailure::new(
                "E-DELAY-EVALUATION-TIME",
                "delay evaluation time must be finite",
            ));
        }
        if self.samples.is_empty() {
            return Err(SolverFailure::new(
                "E-DELAY-HISTORY-EMPTY",
                "delay buffer has no recorded history",
            ));
        }

        let target_time_s = time_s - self.delay_s;
        let first = self.samples[0];
        if target_time_s <= first.time_s {
            return match self.initial_history_policy {
                DelayInitialHistoryPolicy::HoldInitial => {
                    Ok(self.evaluation(first.value, first.time_s, "initial_history"))
                }
                DelayInitialHistoryPolicy::ErrorBeforeHistory => Err(SolverFailure::new(
                    "E-DELAY-HISTORY-UNDERFLOW",
                    "delay evaluation requested time before recorded history",
                )),
            };
        }

        let last = *self.samples.last().unwrap();
        if target_time_s > last.time_s {
            return Err(SolverFailure::new(
                "E-DELAY-HISTORY-MISSING",
                "delay evaluation requested time after recorded history",
            ));
        }
        if target_time_s == last.time_s {
            return Ok(self.evaluation(last.value, last.time_s, "sample"));
        }

        for window in self.samples.windows(2) {
            let left = window[0];
            let right = window[1];
            if target_time_s < left.time_s || target_time_s > right.time_s {
                continue;
            }
            let value = match self.interpolation_policy {
                DelayInterpolationPolicy::Linear => {
                    let span = right.time_s - left.time_s;
                    if span <= f64::EPSILON {
                        left.value
                    } else {
                        let ratio = (target_time_s - left.time_s) / span;
                        left.value + ratio * (right.value - left.value)
                    }
                }
                DelayInterpolationPolicy::PreviousSample => left.value,
            };
            return Ok(self.evaluation(value, target_time_s, "interpolated"));
        }

        Err(SolverFailure::new(
            "E-DELAY-HISTORY-MISSING",
            "delay buffer could not bracket the requested history time",
        ))
    }

    pub fn relationship_artifact(&self, status: impl Into<String>) -> DelayRelationshipArtifact {
        DelayRelationshipArtifact {
            signal_name: self.signal_name.clone(),
            quantity_kind: self.quantity_kind.clone(),
            canonical_unit: self.canonical_unit.clone(),
            delay_s: self.delay_s,
            interpolation_policy: self.interpolation_policy.as_str().to_owned(),
            initial_history_policy: self.initial_history_policy.as_str().to_owned(),
            sample_count: self.samples.len(),
            status: status.into(),
        }
    }

    fn evaluation(
        &self,
        value: f64,
        source_time_s: f64,
        status: impl Into<String>,
    ) -> DelayEvaluation {
        let status = status.into();
        DelayEvaluation {
            value,
            source_time_s,
            relationship: self.relationship_artifact(status.clone()),
            status,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct DelayBehaviorNode {
    buffer: DelayBuffer,
}

impl DelayBehaviorNode {
    pub fn new(buffer: DelayBuffer) -> Self {
        Self { buffer }
    }

    pub fn evaluate(
        &mut self,
        time_s: f64,
        current_value: f64,
    ) -> Result<DelayEvaluation, SolverFailure> {
        self.buffer.record(time_s, current_value)?;
        self.buffer.evaluate(time_s)
    }

    pub fn evaluate_rhs<F>(
        &mut self,
        time_s: f64,
        current_value: f64,
        rhs: F,
    ) -> Result<DelayRhsEvaluation, SolverFailure>
    where
        F: FnOnce(&DelayEvaluation) -> Result<Vec<f64>, SolverFailure>,
    {
        let delay = self.evaluate(time_s, current_value)?;
        let derivatives = rhs(&delay)?;
        if derivatives.iter().any(|value| !value.is_finite()) {
            return Err(SolverFailure::new(
                "E-DELAY-RHS-FINITE",
                "delay RHS evaluation returned a non-finite derivative",
            ));
        }
        Ok(DelayRhsEvaluation {
            status: delay.status.clone(),
            delay,
            derivatives,
        })
    }

    pub fn relationship_artifact(&self) -> DelayRelationshipArtifact {
        self.buffer.relationship_artifact("ready")
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct BehaviorSignalContract {
    pub name: String,
    pub quantity_kind: String,
    pub canonical_unit: String,
    pub valid_min: Option<f64>,
    pub valid_max: Option<f64>,
}

impl BehaviorSignalContract {
    pub fn new(
        name: impl Into<String>,
        quantity_kind: impl Into<String>,
        canonical_unit: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            quantity_kind: quantity_kind.into(),
            canonical_unit: canonical_unit.into(),
            valid_min: None,
            valid_max: None,
        }
    }

    pub fn with_valid_range(
        mut self,
        valid_min: Option<f64>,
        valid_max: Option<f64>,
    ) -> Result<Self, SolverFailure> {
        if valid_min.is_some_and(|value| !value.is_finite())
            || valid_max.is_some_and(|value| !value.is_finite())
            || matches!((valid_min, valid_max), (Some(min), Some(max)) if min > max)
        {
            return Err(SolverFailure::new(
                "E-BEHAVIOR-RANGE",
                "behavior signal valid range must be finite and ordered",
            ));
        }
        self.valid_min = valid_min;
        self.valid_max = valid_max;
        Ok(self)
    }

    fn range_warning(&self, role: &str, value: f64) -> Option<BehaviorWarning> {
        if self.valid_min.is_some_and(|min| value < min)
            || self.valid_max.is_some_and(|max| value > max)
        {
            Some(BehaviorWarning {
                signal: self.name.clone(),
                code: "W-BEHAVIOR-RANGE".to_owned(),
                message: format!(
                    "{role} `{}` value {} is outside the declared valid range",
                    self.name, value
                ),
            })
        } else {
            None
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PredictorDifferentiability {
    Differentiable,
    NonDifferentiable,
    Unknown,
}

impl PredictorDifferentiability {
    fn as_str(&self) -> &'static str {
        match self {
            Self::Differentiable => "differentiable",
            Self::NonDifferentiable => "non_differentiable",
            Self::Unknown => "unknown",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PredictorJacobianPolicy {
    Supplied,
    FiniteDifferenceAllowed,
    NoJacobian,
}

impl PredictorJacobianPolicy {
    fn as_str(&self) -> &'static str {
        match self {
            Self::Supplied => "supplied",
            Self::FiniteDifferenceAllowed => "finite_difference_allowed",
            Self::NoJacobian => "no_jacobian",
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct PredictorSolverPolicy {
    pub explicit_call_only: bool,
    pub finite_difference_allowed: bool,
    pub jacobian_policy: PredictorJacobianPolicy,
}

impl Default for PredictorSolverPolicy {
    fn default() -> Self {
        Self {
            explicit_call_only: true,
            finite_difference_allowed: false,
            jacobian_policy: PredictorJacobianPolicy::NoJacobian,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct PredictorContract {
    pub name: String,
    pub inputs: Vec<BehaviorSignalContract>,
    pub outputs: Vec<BehaviorSignalContract>,
    pub model_hash: String,
    pub differentiability: PredictorDifferentiability,
    pub solver_policy: PredictorSolverPolicy,
}

impl PredictorContract {
    pub fn new(
        name: impl Into<String>,
        inputs: Vec<BehaviorSignalContract>,
        outputs: Vec<BehaviorSignalContract>,
        model_hash: impl Into<String>,
        differentiability: PredictorDifferentiability,
        solver_policy: PredictorSolverPolicy,
    ) -> Result<Self, SolverFailure> {
        if inputs.is_empty() || outputs.is_empty() {
            return Err(SolverFailure::new(
                "E-PREDICTOR-CONTRACT-SHAPE",
                "predictor contract requires at least one input and one output",
            ));
        }
        let model_hash = model_hash.into();
        if model_hash.trim().is_empty() {
            return Err(SolverFailure::new(
                "E-PREDICTOR-MODEL-HASH",
                "predictor contract requires a provenance/model hash",
            ));
        }
        Ok(Self {
            name: name.into(),
            inputs,
            outputs,
            model_hash,
            differentiability,
            solver_policy,
        })
    }

    pub fn artifact(&self, status: impl Into<String>) -> PredictorContractArtifact {
        PredictorContractArtifact {
            name: self.name.clone(),
            input_count: self.inputs.len(),
            output_count: self.outputs.len(),
            model_hash: self.model_hash.clone(),
            differentiability: self.differentiability.as_str().to_owned(),
            explicit_call_only: self.solver_policy.explicit_call_only,
            finite_difference_allowed: self.solver_policy.finite_difference_allowed,
            jacobian_policy: self.solver_policy.jacobian_policy.as_str().to_owned(),
            status: status.into(),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct PredictorContractArtifact {
    pub name: String,
    pub input_count: usize,
    pub output_count: usize,
    pub model_hash: String,
    pub differentiability: String,
    pub explicit_call_only: bool,
    pub finite_difference_allowed: bool,
    pub jacobian_policy: String,
    pub status: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct BehaviorWarning {
    pub signal: String,
    pub code: String,
    pub message: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PredictorEvaluation {
    pub outputs: Vec<f64>,
    pub warnings: Vec<BehaviorWarning>,
    pub status: String,
    pub contract: PredictorContractArtifact,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PredictorRhsEvaluation {
    pub derivatives: Vec<f64>,
    pub predictor: PredictorEvaluation,
    pub status: String,
}

pub struct PredictorBehaviorNode<F>
where
    F: Fn(&[f64]) -> Result<Vec<f64>, SolverFailure>,
{
    contract: PredictorContract,
    evaluator: F,
}

impl<F> PredictorBehaviorNode<F>
where
    F: Fn(&[f64]) -> Result<Vec<f64>, SolverFailure>,
{
    pub fn new(contract: PredictorContract, evaluator: F) -> Self {
        Self {
            contract,
            evaluator,
        }
    }

    pub fn evaluate(&self, inputs: &[f64]) -> Result<PredictorEvaluation, SolverFailure> {
        if inputs.len() != self.contract.inputs.len() {
            return Err(SolverFailure::new(
                "E-PREDICTOR-INPUT-LAYOUT",
                "predictor input vector length does not match the contract",
            ));
        }
        if inputs.iter().any(|value| !value.is_finite()) {
            return Err(SolverFailure::new(
                "E-PREDICTOR-INPUT-FINITE",
                "predictor inputs must be finite",
            ));
        }

        let mut warnings = self
            .contract
            .inputs
            .iter()
            .zip(inputs.iter().copied())
            .filter_map(|(contract, value)| contract.range_warning("input", value))
            .collect::<Vec<_>>();

        let outputs = (self.evaluator)(inputs)?;
        if outputs.len() != self.contract.outputs.len() {
            return Err(SolverFailure::new(
                "E-PREDICTOR-OUTPUT-LAYOUT",
                "predictor output vector length does not match the contract",
            ));
        }
        if outputs.iter().any(|value| !value.is_finite()) {
            return Err(SolverFailure::new(
                "E-PREDICTOR-OUTPUT-FINITE",
                "predictor outputs must be finite",
            ));
        }
        warnings.extend(
            self.contract
                .outputs
                .iter()
                .zip(outputs.iter().copied())
                .filter_map(|(contract, value)| contract.range_warning("output", value)),
        );

        let status = if warnings.is_empty() {
            "ok"
        } else {
            "range_warning"
        };
        Ok(PredictorEvaluation {
            outputs,
            warnings,
            status: status.to_owned(),
            contract: self.contract.artifact(status),
        })
    }

    pub fn contract_artifact(&self) -> PredictorContractArtifact {
        self.contract.artifact("ready")
    }

    pub fn evaluate_rhs<R>(
        &self,
        inputs: &[f64],
        rhs: R,
    ) -> Result<PredictorRhsEvaluation, SolverFailure>
    where
        R: FnOnce(&PredictorEvaluation) -> Result<Vec<f64>, SolverFailure>,
    {
        let predictor = self.evaluate(inputs)?;
        let derivatives = rhs(&predictor)?;
        if derivatives.iter().any(|value| !value.is_finite()) {
            return Err(SolverFailure::new(
                "E-PREDICTOR-RHS-FINITE",
                "predictor RHS evaluation returned a non-finite derivative",
            ));
        }
        Ok(PredictorRhsEvaluation {
            status: predictor.status.clone(),
            predictor,
            derivatives,
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ExternalBehaviorKind {
    Function,
    Process,
}

impl ExternalBehaviorKind {
    fn as_str(&self) -> &'static str {
        match self {
            Self::Function => "function",
            Self::Process => "process",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ExternalBehaviorDeterminism {
    Deterministic,
    NonDeterministic,
    Unknown,
}

impl ExternalBehaviorDeterminism {
    fn as_str(&self) -> &'static str {
        match self {
            Self::Deterministic => "deterministic",
            Self::NonDeterministic => "non_deterministic",
            Self::Unknown => "unknown",
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ExternalBehaviorProfilePolicy {
    pub safe_allowed: bool,
    pub repro_allowed: bool,
}

impl Default for ExternalBehaviorProfilePolicy {
    fn default() -> Self {
        Self {
            safe_allowed: false,
            repro_allowed: true,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BehaviorExecutionProfile {
    Safe,
    Normal,
    Repro,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ExternalBehaviorContract {
    pub name: String,
    pub kind: ExternalBehaviorKind,
    pub inputs: Vec<BehaviorSignalContract>,
    pub outputs: Vec<BehaviorSignalContract>,
    pub provenance_hash: String,
    pub determinism: ExternalBehaviorDeterminism,
    pub profile_policy: ExternalBehaviorProfilePolicy,
}

impl ExternalBehaviorContract {
    pub fn new(
        name: impl Into<String>,
        kind: ExternalBehaviorKind,
        inputs: Vec<BehaviorSignalContract>,
        outputs: Vec<BehaviorSignalContract>,
        provenance_hash: impl Into<String>,
        determinism: ExternalBehaviorDeterminism,
        profile_policy: ExternalBehaviorProfilePolicy,
    ) -> Result<Self, SolverFailure> {
        if inputs.is_empty() || outputs.is_empty() {
            return Err(SolverFailure::new(
                "E-EXTERNAL-BEHAVIOR-CONTRACT-SHAPE",
                "external behavior contract requires at least one input and one output",
            ));
        }
        let provenance_hash = provenance_hash.into();
        if provenance_hash.trim().is_empty() {
            return Err(SolverFailure::new(
                "E-EXTERNAL-BEHAVIOR-PROVENANCE",
                "external behavior contract requires deterministic provenance metadata",
            ));
        }
        Ok(Self {
            name: name.into(),
            kind,
            inputs,
            outputs,
            provenance_hash,
            determinism,
            profile_policy,
        })
    }

    pub fn artifact(&self, status: impl Into<String>) -> ExternalBehaviorArtifact {
        ExternalBehaviorArtifact {
            name: self.name.clone(),
            kind: self.kind.as_str().to_owned(),
            input_count: self.inputs.len(),
            output_count: self.outputs.len(),
            provenance_hash: self.provenance_hash.clone(),
            determinism: self.determinism.as_str().to_owned(),
            safe_allowed: self.profile_policy.safe_allowed,
            repro_allowed: self.profile_policy.repro_allowed,
            status: status.into(),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ExternalBehaviorArtifact {
    pub name: String,
    pub kind: String,
    pub input_count: usize,
    pub output_count: usize,
    pub provenance_hash: String,
    pub determinism: String,
    pub safe_allowed: bool,
    pub repro_allowed: bool,
    pub status: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ExternalBehaviorEvaluation {
    pub outputs: Vec<f64>,
    pub warnings: Vec<BehaviorWarning>,
    pub status: String,
    pub contract: ExternalBehaviorArtifact,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ExternalBehaviorRhsEvaluation {
    pub derivatives: Vec<f64>,
    pub external: ExternalBehaviorEvaluation,
    pub status: String,
}

pub struct ExternalBehaviorNode<F>
where
    F: Fn(&[f64]) -> Result<Vec<f64>, SolverFailure>,
{
    contract: ExternalBehaviorContract,
    evaluator: F,
}

impl<F> ExternalBehaviorNode<F>
where
    F: Fn(&[f64]) -> Result<Vec<f64>, SolverFailure>,
{
    pub fn new(contract: ExternalBehaviorContract, evaluator: F) -> Self {
        Self {
            contract,
            evaluator,
        }
    }

    pub fn evaluate(
        &self,
        profile: BehaviorExecutionProfile,
        inputs: &[f64],
    ) -> Result<ExternalBehaviorEvaluation, SolverFailure> {
        self.validate_profile(profile)?;
        if inputs.len() != self.contract.inputs.len() {
            return Err(SolverFailure::new(
                "E-EXTERNAL-BEHAVIOR-INPUT-LAYOUT",
                "external behavior input vector length does not match the contract",
            ));
        }
        if inputs.iter().any(|value| !value.is_finite()) {
            return Err(SolverFailure::new(
                "E-EXTERNAL-BEHAVIOR-INPUT-FINITE",
                "external behavior inputs must be finite",
            ));
        }

        let mut warnings = self
            .contract
            .inputs
            .iter()
            .zip(inputs.iter().copied())
            .filter_map(|(contract, value)| contract.range_warning("input", value))
            .collect::<Vec<_>>();
        let outputs = (self.evaluator)(inputs).map_err(|failure| {
            SolverFailure::new(
                "E-EXTERNAL-BEHAVIOR-FAILURE",
                format!(
                    "external behavior `{}` failed with {}: {}",
                    self.contract.name, failure.code, failure.message
                ),
            )
        })?;
        if outputs.len() != self.contract.outputs.len() {
            return Err(SolverFailure::new(
                "E-EXTERNAL-BEHAVIOR-OUTPUT-LAYOUT",
                "external behavior output vector length does not match the contract",
            ));
        }
        if outputs.iter().any(|value| !value.is_finite()) {
            return Err(SolverFailure::new(
                "E-EXTERNAL-BEHAVIOR-OUTPUT-FINITE",
                "external behavior outputs must be finite",
            ));
        }
        warnings.extend(
            self.contract
                .outputs
                .iter()
                .zip(outputs.iter().copied())
                .filter_map(|(contract, value)| contract.range_warning("output", value)),
        );
        let status = if warnings.is_empty() {
            "ok"
        } else {
            "range_warning"
        };
        Ok(ExternalBehaviorEvaluation {
            outputs,
            warnings,
            status: status.to_owned(),
            contract: self.contract.artifact(status),
        })
    }

    pub fn contract_artifact(&self) -> ExternalBehaviorArtifact {
        self.contract.artifact("ready")
    }

    pub fn evaluate_rhs<R>(
        &self,
        profile: BehaviorExecutionProfile,
        inputs: &[f64],
        rhs: R,
    ) -> Result<ExternalBehaviorRhsEvaluation, SolverFailure>
    where
        R: FnOnce(&ExternalBehaviorEvaluation) -> Result<Vec<f64>, SolverFailure>,
    {
        let external = self.evaluate(profile, inputs)?;
        let derivatives = rhs(&external)?;
        if derivatives.iter().any(|value| !value.is_finite()) {
            return Err(SolverFailure::new(
                "E-EXTERNAL-BEHAVIOR-RHS-FINITE",
                "external behavior RHS evaluation returned a non-finite derivative",
            ));
        }
        Ok(ExternalBehaviorRhsEvaluation {
            status: external.status.clone(),
            external,
            derivatives,
        })
    }

    fn validate_profile(&self, profile: BehaviorExecutionProfile) -> Result<(), SolverFailure> {
        match profile {
            BehaviorExecutionProfile::Safe if !self.contract.profile_policy.safe_allowed => {
                Err(SolverFailure::new(
                    "E-EXTERNAL-BEHAVIOR-PROFILE",
                    "external behavior is not allowed in the safe execution profile",
                ))
            }
            BehaviorExecutionProfile::Repro => {
                if !self.contract.profile_policy.repro_allowed
                    || self.contract.determinism != ExternalBehaviorDeterminism::Deterministic
                {
                    Err(SolverFailure::new(
                        "E-EXTERNAL-BEHAVIOR-PROFILE",
                        "external behavior must be deterministic and repro-allowed in the repro execution profile",
                    ))
                } else {
                    Ok(())
                }
            }
            _ => Ok(()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::solver::{
        solve_fixed_step_ode, FixedStepMethod, InputLayout, LayoutEntry, OutputLayout,
        ParameterLayout, SimulationPlan, SolverInput, SolverOptions, SolverPlan, TimeGrid,
    };

    fn linear_hold_buffer(delay_s: f64) -> DelayBuffer {
        DelayBuffer::new(
            "temperature",
            "AbsoluteTemperature",
            "K",
            delay_s,
            DelayInterpolationPolicy::Linear,
            DelayInitialHistoryPolicy::HoldInitial,
        )
        .unwrap()
    }

    #[test]
    fn delay_buffer_uses_linear_interpolation() {
        let mut buffer = linear_hold_buffer(10.0);
        buffer.record(0.0, 10.0).unwrap();
        buffer.record(10.0, 20.0).unwrap();

        let delayed = buffer.evaluate(15.0).unwrap();

        assert_eq!(delayed.status, "interpolated");
        assert!((delayed.value - 15.0).abs() < 1e-9);
        assert!((delayed.source_time_s - 5.0).abs() < 1e-9);
        assert_eq!(delayed.relationship.delay_s, 10.0);
        assert_eq!(delayed.relationship.sample_count, 2);
    }

    #[test]
    fn delay_buffer_applies_initial_history_policy() {
        let mut buffer = linear_hold_buffer(5.0);
        buffer.record(0.0, 10.0).unwrap();

        let delayed = buffer.evaluate(2.0).unwrap();

        assert_eq!(delayed.status, "initial_history");
        assert_eq!(delayed.value, 10.0);
        assert_eq!(delayed.source_time_s, 0.0);
    }

    #[test]
    fn delay_buffer_reports_history_underflow_when_configured() {
        let mut buffer = DelayBuffer::new(
            "flow",
            "MassFlowRate",
            "kg/s",
            5.0,
            DelayInterpolationPolicy::PreviousSample,
            DelayInitialHistoryPolicy::ErrorBeforeHistory,
        )
        .unwrap();
        buffer.record(0.0, 1.0).unwrap();

        let failure = buffer.evaluate(2.0).unwrap_err();

        assert_eq!(failure.code, "E-DELAY-HISTORY-UNDERFLOW");
    }

    #[test]
    fn delay_behavior_node_records_current_value_before_evaluation() {
        let buffer = linear_hold_buffer(1.0);
        let mut node = DelayBehaviorNode::new(buffer);

        let first = node.evaluate(0.0, 10.0).unwrap();
        let second = node.evaluate(1.0, 20.0).unwrap();
        let third = node.evaluate(1.5, 30.0).unwrap();

        assert_eq!(first.status, "initial_history");
        assert_eq!(first.value, 10.0);
        assert_eq!(second.value, 10.0);
        assert!((third.value - 15.0).abs() < 1e-9);
    }

    #[test]
    fn delay_behavior_node_can_drive_fixed_step_rhs() {
        let input = SolverInput {
            plan: SolverPlan::new(
                "delayed_rhs",
                SimulationPlan {
                    states: vec!["x".to_owned()],
                    inputs: Vec::new(),
                    outputs: vec!["x".to_owned()],
                    parameters: Vec::new(),
                },
                SolverOptions::fixed_step("explicit_euler", 1.0),
            ),
            time_grid: TimeGrid::fixed_step(2.0, 1.0).unwrap(),
            state_layout: crate::solver::StateLayout::new(vec![LayoutEntry::new(
                0,
                "x",
                "Dimensionless",
                "1",
                "1",
            )]),
            input_layout: InputLayout::default(),
            parameter_layout: ParameterLayout::default(),
            output_layout: OutputLayout::default(),
            initial_state: vec![1.0],
            inputs: Vec::new(),
            parameters: Vec::new(),
        };
        let buffer = DelayBuffer::new(
            "x",
            "Dimensionless",
            "1",
            1.0,
            DelayInterpolationPolicy::PreviousSample,
            DelayInitialHistoryPolicy::HoldInitial,
        )
        .unwrap();
        let mut node = DelayBehaviorNode::new(buffer);
        let mut delay_statuses = Vec::new();

        let result = solve_fixed_step_ode(FixedStepMethod::ExplicitEuler, &input, |sample| {
            let evaluation = node.evaluate_rhs(sample.time_s, sample.state[0], |delay| {
                Ok(vec![-delay.value])
            })?;
            delay_statuses.push(evaluation.status);
            Ok(evaluation.derivatives)
        })
        .unwrap();

        assert_eq!(
            result.output.state_trajectories[0].values,
            vec![1.0, 0.0, -1.0]
        );
        assert_eq!(delay_statuses, vec!["initial_history", "initial_history"]);
    }

    #[test]
    fn delay_behavior_node_rejects_nonfinite_rhs_derivatives() {
        let buffer = linear_hold_buffer(1.0);
        let mut node = DelayBehaviorNode::new(buffer);

        let failure = node
            .evaluate_rhs(0.0, 1.0, |_delay| Ok(vec![f64::INFINITY]))
            .unwrap_err();

        assert_eq!(failure.code, "E-DELAY-RHS-FINITE");
    }

    #[test]
    fn delay_buffer_rejects_out_of_order_samples() {
        let mut buffer = linear_hold_buffer(1.0);
        buffer.record(1.0, 10.0).unwrap();
        let failure = buffer.record(0.5, 12.0).unwrap_err();

        assert_eq!(failure.code, "E-DELAY-TIME-ORDER");
    }

    #[test]
    fn predictor_behavior_node_evaluates_with_contract_artifact() {
        let contract = PredictorContract::new(
            "cooling_load_predictor",
            vec![
                BehaviorSignalContract::new("T_out", "AbsoluteTemperature", "K")
                    .with_valid_range(Some(250.0), Some(330.0))
                    .unwrap(),
                BehaviorSignalContract::new("occupancy", "Count", "1"),
            ],
            vec![BehaviorSignalContract::new("load", "Power", "W")],
            "sha256:abc123",
            PredictorDifferentiability::Unknown,
            PredictorSolverPolicy {
                explicit_call_only: true,
                finite_difference_allowed: true,
                jacobian_policy: PredictorJacobianPolicy::FiniteDifferenceAllowed,
            },
        )
        .unwrap();
        let node =
            PredictorBehaviorNode::new(contract, |inputs| Ok(vec![100.0 + inputs[1] * 25.0]));

        let evaluation = node.evaluate(&[300.0, 4.0]).unwrap();

        assert_eq!(evaluation.status, "ok");
        assert_eq!(evaluation.outputs, vec![200.0]);
        assert_eq!(evaluation.contract.name, "cooling_load_predictor");
        assert_eq!(evaluation.contract.model_hash, "sha256:abc123");
        assert!(evaluation.contract.finite_difference_allowed);
        assert_eq!(
            evaluation.contract.jacobian_policy,
            "finite_difference_allowed"
        );
    }

    #[test]
    fn predictor_behavior_node_can_drive_fixed_step_rhs() {
        let input = SolverInput {
            plan: SolverPlan::new(
                "predictor_rhs",
                SimulationPlan {
                    states: vec!["x".to_owned()],
                    inputs: Vec::new(),
                    outputs: vec!["x".to_owned()],
                    parameters: Vec::new(),
                },
                SolverOptions::fixed_step("explicit_euler", 1.0),
            ),
            time_grid: TimeGrid::fixed_step(2.0, 1.0).unwrap(),
            state_layout: crate::solver::StateLayout::new(vec![LayoutEntry::new(
                0,
                "x",
                "Dimensionless",
                "1",
                "1",
            )]),
            input_layout: InputLayout::default(),
            parameter_layout: ParameterLayout::default(),
            output_layout: OutputLayout::default(),
            initial_state: vec![1.0],
            inputs: Vec::new(),
            parameters: Vec::new(),
        };
        let contract = PredictorContract::new(
            "state_feedback_predictor",
            vec![BehaviorSignalContract::new("x", "Dimensionless", "1")],
            vec![BehaviorSignalContract::new(
                "x_feedback",
                "Dimensionless",
                "1",
            )],
            "sha256:predictor-rhs",
            PredictorDifferentiability::Differentiable,
            PredictorSolverPolicy {
                explicit_call_only: true,
                finite_difference_allowed: true,
                jacobian_policy: PredictorJacobianPolicy::FiniteDifferenceAllowed,
            },
        )
        .unwrap();
        let node = PredictorBehaviorNode::new(contract, |inputs| Ok(vec![inputs[0]]));
        let mut predictor_statuses = Vec::new();

        let result = solve_fixed_step_ode(FixedStepMethod::ExplicitEuler, &input, |sample| {
            let evaluation = node.evaluate_rhs(&[sample.state[0]], |predictor| {
                Ok(vec![-predictor.outputs[0]])
            })?;
            predictor_statuses.push(evaluation.status);
            Ok(evaluation.derivatives)
        })
        .unwrap();

        assert_eq!(
            result.output.state_trajectories[0].values,
            vec![1.0, 0.0, 0.0]
        );
        assert_eq!(predictor_statuses, vec!["ok", "ok"]);
    }

    #[test]
    fn predictor_behavior_node_reports_range_warnings() {
        let contract = PredictorContract::new(
            "range_checked_predictor",
            vec![BehaviorSignalContract::new("x", "Dimensionless", "1")
                .with_valid_range(Some(0.0), Some(1.0))
                .unwrap()],
            vec![BehaviorSignalContract::new("y", "Dimensionless", "1")
                .with_valid_range(Some(0.0), Some(2.0))
                .unwrap()],
            "sha256:def456",
            PredictorDifferentiability::Differentiable,
            PredictorSolverPolicy {
                explicit_call_only: true,
                finite_difference_allowed: false,
                jacobian_policy: PredictorJacobianPolicy::Supplied,
            },
        )
        .unwrap();
        let node = PredictorBehaviorNode::new(contract, |inputs| Ok(vec![inputs[0] * 4.0]));

        let evaluation = node.evaluate(&[2.0]).unwrap();

        assert_eq!(evaluation.status, "range_warning");
        assert_eq!(evaluation.warnings.len(), 2);
        assert_eq!(evaluation.warnings[0].code, "W-BEHAVIOR-RANGE");
        assert_eq!(evaluation.contract.differentiability, "differentiable");
        assert_eq!(evaluation.contract.jacobian_policy, "supplied");
    }

    #[test]
    fn predictor_behavior_node_propagates_layout_failures() {
        let contract = PredictorContract::new(
            "bad_shape_predictor",
            vec![BehaviorSignalContract::new("x", "Dimensionless", "1")],
            vec![BehaviorSignalContract::new("y", "Dimensionless", "1")],
            "sha256:badshape",
            PredictorDifferentiability::NonDifferentiable,
            PredictorSolverPolicy::default(),
        )
        .unwrap();
        let node = PredictorBehaviorNode::new(contract, |_inputs| Ok(vec![1.0, 2.0]));

        let failure = node.evaluate(&[1.0]).unwrap_err();

        assert_eq!(failure.code, "E-PREDICTOR-OUTPUT-LAYOUT");
    }

    #[test]
    fn predictor_behavior_node_rejects_nonfinite_rhs_derivatives() {
        let contract = PredictorContract::new(
            "bad_rhs_predictor",
            vec![BehaviorSignalContract::new("x", "Dimensionless", "1")],
            vec![BehaviorSignalContract::new("y", "Dimensionless", "1")],
            "sha256:bad-rhs",
            PredictorDifferentiability::Unknown,
            PredictorSolverPolicy::default(),
        )
        .unwrap();
        let node = PredictorBehaviorNode::new(contract, |_inputs| Ok(vec![1.0]));

        let failure = node
            .evaluate_rhs(&[1.0], |_predictor| Ok(vec![f64::INFINITY]))
            .unwrap_err();

        assert_eq!(failure.code, "E-PREDICTOR-RHS-FINITE");
    }

    #[test]
    fn external_behavior_node_evaluates_function_contract() {
        let contract = ExternalBehaviorContract::new(
            "legacy_heat_loss",
            ExternalBehaviorKind::Function,
            vec![BehaviorSignalContract::new(
                "temperature",
                "AbsoluteTemperature",
                "K",
            )],
            vec![BehaviorSignalContract::new("loss", "HeatRate", "W")],
            "sha256:external123",
            ExternalBehaviorDeterminism::Deterministic,
            ExternalBehaviorProfilePolicy {
                safe_allowed: true,
                repro_allowed: true,
            },
        )
        .unwrap();
        let node = ExternalBehaviorNode::new(contract, |inputs| Ok(vec![inputs[0] * 2.0]));

        let evaluation = node
            .evaluate(BehaviorExecutionProfile::Repro, &[300.0])
            .unwrap();

        assert_eq!(evaluation.status, "ok");
        assert_eq!(evaluation.outputs, vec![600.0]);
        assert_eq!(evaluation.contract.kind, "function");
        assert_eq!(evaluation.contract.provenance_hash, "sha256:external123");
        assert!(evaluation.contract.repro_allowed);
    }

    #[test]
    fn external_behavior_node_can_drive_fixed_step_rhs() {
        let input = SolverInput {
            plan: SolverPlan::new(
                "external_rhs",
                SimulationPlan {
                    states: vec!["x".to_owned()],
                    inputs: Vec::new(),
                    outputs: vec!["x".to_owned()],
                    parameters: Vec::new(),
                },
                SolverOptions::fixed_step("explicit_euler", 1.0),
            ),
            time_grid: TimeGrid::fixed_step(2.0, 1.0).unwrap(),
            state_layout: crate::solver::StateLayout::new(vec![LayoutEntry::new(
                0,
                "x",
                "Dimensionless",
                "1",
                "1",
            )]),
            input_layout: InputLayout::default(),
            parameter_layout: ParameterLayout::default(),
            output_layout: OutputLayout::default(),
            initial_state: vec![1.0],
            inputs: Vec::new(),
            parameters: Vec::new(),
        };
        let contract = ExternalBehaviorContract::new(
            "state_feedback_adapter",
            ExternalBehaviorKind::Function,
            vec![BehaviorSignalContract::new("x", "Dimensionless", "1")],
            vec![BehaviorSignalContract::new(
                "x_feedback",
                "Dimensionless",
                "1",
            )],
            "sha256:external-rhs",
            ExternalBehaviorDeterminism::Deterministic,
            ExternalBehaviorProfilePolicy {
                safe_allowed: true,
                repro_allowed: true,
            },
        )
        .unwrap();
        let node = ExternalBehaviorNode::new(contract, |inputs| Ok(vec![inputs[0]]));
        let mut external_statuses = Vec::new();

        let result = solve_fixed_step_ode(FixedStepMethod::ExplicitEuler, &input, |sample| {
            let evaluation = node.evaluate_rhs(
                BehaviorExecutionProfile::Repro,
                &[sample.state[0]],
                |external| Ok(vec![-external.outputs[0]]),
            )?;
            external_statuses.push(evaluation.status);
            Ok(evaluation.derivatives)
        })
        .unwrap();

        assert_eq!(
            result.output.state_trajectories[0].values,
            vec![1.0, 0.0, 0.0]
        );
        assert_eq!(external_statuses, vec!["ok", "ok"]);
    }

    #[test]
    fn external_behavior_node_rejects_safe_profile_when_disallowed() {
        let contract = ExternalBehaviorContract::new(
            "process_adapter",
            ExternalBehaviorKind::Process,
            vec![BehaviorSignalContract::new("x", "Dimensionless", "1")],
            vec![BehaviorSignalContract::new("y", "Dimensionless", "1")],
            "sha256:process123",
            ExternalBehaviorDeterminism::Deterministic,
            ExternalBehaviorProfilePolicy::default(),
        )
        .unwrap();
        let node = ExternalBehaviorNode::new(contract, |inputs| Ok(inputs.to_vec()));

        let failure = node
            .evaluate(BehaviorExecutionProfile::Safe, &[1.0])
            .unwrap_err();

        assert_eq!(failure.code, "E-EXTERNAL-BEHAVIOR-PROFILE");
    }

    #[test]
    fn external_behavior_node_rejects_nonfinite_rhs_derivatives() {
        let contract = ExternalBehaviorContract::new(
            "bad_rhs_adapter",
            ExternalBehaviorKind::Function,
            vec![BehaviorSignalContract::new("x", "Dimensionless", "1")],
            vec![BehaviorSignalContract::new("y", "Dimensionless", "1")],
            "sha256:bad-external-rhs",
            ExternalBehaviorDeterminism::Deterministic,
            ExternalBehaviorProfilePolicy {
                safe_allowed: true,
                repro_allowed: true,
            },
        )
        .unwrap();
        let node = ExternalBehaviorNode::new(contract, |_inputs| Ok(vec![1.0]));

        let failure = node
            .evaluate_rhs(BehaviorExecutionProfile::Repro, &[1.0], |_external| {
                Ok(vec![f64::INFINITY])
            })
            .unwrap_err();

        assert_eq!(failure.code, "E-EXTERNAL-BEHAVIOR-RHS-FINITE");
    }

    #[test]
    fn external_behavior_node_rejects_nondeterministic_repro_profile() {
        let contract = ExternalBehaviorContract::new(
            "weather_service",
            ExternalBehaviorKind::Process,
            vec![BehaviorSignalContract::new("x", "Dimensionless", "1")],
            vec![BehaviorSignalContract::new("y", "Dimensionless", "1")],
            "sha256:weather123",
            ExternalBehaviorDeterminism::NonDeterministic,
            ExternalBehaviorProfilePolicy {
                safe_allowed: false,
                repro_allowed: true,
            },
        )
        .unwrap();
        let node = ExternalBehaviorNode::new(contract, |inputs| Ok(inputs.to_vec()));

        let failure = node
            .evaluate(BehaviorExecutionProfile::Repro, &[1.0])
            .unwrap_err();

        assert_eq!(failure.code, "E-EXTERNAL-BEHAVIOR-PROFILE");
    }

    #[test]
    fn external_behavior_node_wraps_adapter_failures() {
        let contract = ExternalBehaviorContract::new(
            "failing_adapter",
            ExternalBehaviorKind::Function,
            vec![BehaviorSignalContract::new("x", "Dimensionless", "1")],
            vec![BehaviorSignalContract::new("y", "Dimensionless", "1")],
            "sha256:failing",
            ExternalBehaviorDeterminism::Deterministic,
            ExternalBehaviorProfilePolicy {
                safe_allowed: true,
                repro_allowed: true,
            },
        )
        .unwrap();
        let node = ExternalBehaviorNode::new(contract, |_inputs| {
            Err(SolverFailure::new("E-ADAPTER-BOOM", "adapter failed"))
        });

        let failure = node
            .evaluate(BehaviorExecutionProfile::Normal, &[1.0])
            .unwrap_err();

        assert_eq!(failure.code, "E-EXTERNAL-BEHAVIOR-FAILURE");
        assert!(failure.message.contains("E-ADAPTER-BOOM"));
    }
}
