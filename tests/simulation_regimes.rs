use ponzisim::{SimConfig, run_simulation};

fn peak(values: &[f64]) -> f64 {
    values
        .iter()
        .copied()
        .max_by(|a, b| a.total_cmp(b))
        .unwrap_or(0.0)
}

#[test]
fn default_config_exhibits_endogenous_decline() {
    let cfg = SimConfig::default();
    let result = run_simulation(&cfg);

    let gdp_values: Vec<f64> = result.points.iter().map(|p| p.gdp).collect();
    let peak_gdp = peak(&gdp_values);
    let final_gdp = *gdp_values.last().unwrap_or(&0.0);

    assert!(peak_gdp > 0.0);
    assert!(
        final_gdp < peak_gdp * 0.9,
        "final={final_gdp}, peak={peak_gdp}"
    );
    assert!(
        result.gdp_collapse_step.is_some(),
        "collapse step should emerge without forcing"
    );
}

#[test]
fn benign_config_can_avoid_collapse_in_horizon() {
    let mut cfg = SimConfig::default();
    cfg.interest_apr = 0.02;
    cfg.initial_debt_to_gdp = 0.8;
    cfg.min_bank_capital_ratio = 0.18;
    cfg.urr = 20_000.0;
    cfg.extraction_k = 0.055;
    cfg.initial_q_frac = 0.08;
    cfg.energy_efficiency_trend = 0.022;

    let result = run_simulation(&cfg);
    let gdp_values: Vec<f64> = result.points.iter().map(|p| p.gdp).collect();
    let peak_gdp = peak(&gdp_values);
    let final_gdp = *gdp_values.last().unwrap_or(&0.0);
    assert!(
        result.gdp_collapse_step.is_none(),
        "collapse unexpectedly detected"
    );
    assert!(
        final_gdp >= peak_gdp * 0.95,
        "benign regime degraded too much: final={final_gdp}, peak={peak_gdp}"
    );
}

#[test]
fn simulation_metrics_respect_invariants() {
    let cfg = SimConfig::default();
    let result = run_simulation(&cfg);

    assert!(!result.points.is_empty());

    for point in result.points {
        assert!(point.gdp.is_finite());
        assert!(point.net_energy.is_finite());
        assert!(point.extraction_rate.is_finite());
        assert!(point.debt_total.is_finite());
        assert!(point.debt_to_gdp.is_finite());
        assert!(point.debt_service_ratio.is_finite());
        assert!(point.default_rate.is_finite());
        assert!(point.bank_capital_ratio.is_finite());
        assert!(point.loan_flow.is_finite());
        assert!(point.write_off_flow.is_finite());
        assert!(point.energy_price.is_finite());

        assert!(point.extraction_rate >= -1e-9);
        assert!(point.net_energy >= -1e-9);
        assert!(point.default_rate >= -1e-9 && point.default_rate <= 1.0 + 1e-9);
        assert!(point.bank_capital_ratio >= -1e-9);
        assert!(point.energy_price >= 0.0);
    }
}
