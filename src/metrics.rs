use crate::model::SimulationResult;

#[derive(Debug, Clone, PartialEq)]
pub struct DashboardKpis {
    pub peak_oil_step: Option<usize>,
    pub gdp_peak_step: Option<usize>,
    pub gdp_drawdown_pct: f64,
    pub collapse_step: Option<usize>,
}

pub fn detect_peak_oil(result: &SimulationResult) -> Option<usize> {
    result
        .points
        .iter()
        .enumerate()
        .max_by(|(_, a), (_, b)| a.extraction_rate.total_cmp(&b.extraction_rate))
        .map(|(idx, _)| idx)
}

pub fn detect_gdp_collapse(
    result: &SimulationResult,
    drawdown_trigger: f64,
    stress_window: usize,
) -> Option<usize> {
    if result.points.is_empty() {
        return None;
    }

    let peak_idx = result
        .points
        .iter()
        .enumerate()
        .max_by(|(_, a), (_, b)| a.gdp.total_cmp(&b.gdp))
        .map(|(idx, _)| idx)?;

    let peak_gdp = result.points[peak_idx].gdp;
    if peak_gdp <= f64::EPSILON {
        return None;
    }

    let drawdown_threshold = peak_gdp * (1.0 - drawdown_trigger.clamp(0.01, 0.95));
    let mut stress_count = 0usize;

    for idx in (peak_idx + 1)..result.points.len() {
        let p = &result.points[idx];
        let drawdown_hit = p.gdp <= drawdown_threshold;
        let credit_stress = p.default_rate >= 0.015
            || p.bank_capital_ratio <= 0.08
            || p.debt_service_ratio >= 0.24
            || p.energy_price >= 2.5;

        if drawdown_hit && credit_stress {
            stress_count += 1;
            if stress_count >= stress_window.max(1) {
                return Some(idx + 1 - stress_count);
            }
        } else {
            stress_count = 0;
        }
    }

    None
}

pub fn derive_dashboard_kpis(result: &SimulationResult) -> DashboardKpis {
    let peak_oil_step = detect_peak_oil(result);
    let gdp_peak_step = result
        .points
        .iter()
        .enumerate()
        .max_by(|(_, a), (_, b)| a.gdp.total_cmp(&b.gdp))
        .map(|(idx, _)| idx);

    let gdp_drawdown_pct = if let Some(peak_idx) = gdp_peak_step {
        let peak = result.points[peak_idx].gdp;
        let trough = result
            .points
            .iter()
            .map(|p| p.gdp)
            .min_by(|a, b| a.total_cmp(b))
            .unwrap_or(peak);
        if peak.abs() <= f64::EPSILON {
            0.0
        } else {
            ((peak - trough) / peak).clamp(0.0, 1.0) * 100.0
        }
    } else {
        0.0
    };

    let collapse_step = detect_gdp_collapse(result, 0.30, 12);

    DashboardKpis {
        peak_oil_step,
        gdp_peak_step,
        gdp_drawdown_pct,
        collapse_step,
    }
}
