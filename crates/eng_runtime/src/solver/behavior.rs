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

    pub fn relationship_artifact(&self) -> DelayRelationshipArtifact {
        self.buffer.relationship_artifact("ready")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn delay_buffer_rejects_out_of_order_samples() {
        let mut buffer = linear_hold_buffer(1.0);
        buffer.record(1.0, 10.0).unwrap();
        let failure = buffer.record(0.5, 12.0).unwrap_err();

        assert_eq!(failure.code, "E-DELAY-TIME-ORDER");
    }
}
