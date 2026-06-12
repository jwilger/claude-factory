//! Metrics types for tracking routing-table effectiveness.
//!
//! The kernel accumulates step outcomes (gate verdicts, completions) to enable
//! veto-rate and token-cost analysis per work type. This data justifies routing
//! table defaults and guides tuning decisions.

use crate::types::routing::WorkType;
use serde::{Deserialize, Serialize};

/// The outcome of a completed step, recorded for metrics.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StepOutcome {
    /// A gate reviewer approved the submission.
    Approved,
    /// A gate reviewer vetoed the submission (loop back).
    Vetoed,
    /// A non-gate step completed successfully.
    Completed,
}

/// Aggregated metrics for one work type.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WorkTypeMetrics {
    /// Number of gate approvals recorded.
    pub approvals: u32,
    /// Number of gate vetoes recorded (`veto_rate = vetoes / (approvals + vetoes)`).
    pub vetoes: u32,
    /// Number of non-gate step completions recorded.
    pub completions: u32,
    /// Total tokens reported across all steps for this work type.
    pub total_tokens: u64,
    /// Number of steps for which token data was available.
    pub token_sample_count: u32,
}

impl WorkTypeMetrics {
    /// Veto rate as a fraction in [0.0, 1.0]. Returns `None` if no gate
    /// outcomes have been recorded yet.
    #[must_use]
    pub fn veto_rate(&self) -> Option<f64> {
        let total = self.approvals + self.vetoes;
        if total == 0 {
            None
        } else {
            Some(f64::from(self.vetoes) / f64::from(total))
        }
    }

    /// Average tokens per step where token data was available. Returns `None`
    /// if no token samples have been recorded.
    #[must_use]
    #[expect(clippy::cast_precision_loss, reason = "u64→f64 precision loss is acceptable for an average token estimate; exact token counts are not required")]
    pub fn avg_tokens(&self) -> Option<f64> {
        if self.token_sample_count == 0 {
            None
        } else {
            // Precision loss from u64→f64 is acceptable for an average estimate.
            Some(self.total_tokens as f64 / f64::from(self.token_sample_count))
        }
    }

    /// Record an outcome, accumulating into the running totals.
    pub fn record(&mut self, outcome: StepOutcome, tokens: Option<u32>) {
        match outcome {
            StepOutcome::Approved => self.approvals = self.approvals.saturating_add(1),
            StepOutcome::Vetoed => self.vetoes = self.vetoes.saturating_add(1),
            StepOutcome::Completed => self.completions = self.completions.saturating_add(1),
        }
        if let Some(t) = tokens {
            self.total_tokens = self.total_tokens.saturating_add(u64::from(t));
            self.token_sample_count = self.token_sample_count.saturating_add(1);
        }
    }
}

/// A summary of all per-work-type metrics for display.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsSummary {
    pub entries: Vec<WorkTypeMetricEntry>,
}

/// One row in the metrics summary.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkTypeMetricEntry {
    pub work_type: WorkType,
    pub approvals: u32,
    pub vetoes: u32,
    pub completions: u32,
    pub veto_rate: Option<f64>,
    pub avg_tokens: Option<f64>,
}
