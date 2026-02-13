use std::f64::consts::PI;

use crate::metrics::{detect_gdp_collapse, detect_peak_oil};
use crate::model::{MetricsPoint, SimConfig, SimulationResult};

#[derive(Debug, Clone)]
struct CohortState {
    weight: f64,
    productivity: f64,
    energy_intensity: f64,
    debt: f64,
    liquidity: f64,
    defaulted_share: f64,
}

#[derive(Debug, Clone)]
struct BankState {
    capital: f64,
}

fn clamp01(x: f64) -> f64 {
    x.clamp(0.0, 1.0)
}

fn hubbert_flow(q: f64, urr: f64, k: f64, tech_factor: f64) -> f64 {
    let frac = (1.0 - (q / urr)).max(0.0);
    (k * q * frac * tech_factor).max(0.0)
}

fn lending_supply_factor(bank_capital_ratio: f64, min_capital_ratio: f64) -> f64 {
    let stress = bank_capital_ratio / min_capital_ratio.max(1e-9);
    if stress >= 1.25 {
        1.0
    } else if stress >= 1.0 {
        0.85 + 0.6 * (stress - 1.0)
    } else {
        (stress.powi(2) * 0.85).clamp(0.0, 0.85)
    }
}

fn update_debt_stock(
    old_debt: f64,
    rate_per_step: f64,
    new_loans: f64,
    repayment: f64,
    writeoffs: f64,
) -> f64 {
    (old_debt * (1.0 + rate_per_step) + new_loans - repayment - writeoffs).max(0.0)
}

fn init_cohorts(cfg: &SimConfig) -> Vec<CohortState> {
    let count = cfg.cohort_count.max(1);
    let weight = cfg.effective_population as f64 / count as f64;

    let mut cohorts = Vec::with_capacity(count);

    for i in 0..count {
        let x = i as f64 / count as f64;
        let productivity = (0.75 + 0.45 * (2.0 * PI * x).sin().abs()).max(0.1);
        let energy_intensity = (1.35 - 0.45 * (2.0 * PI * x).cos()).max(0.2);
        let liquidity = 0.4 * productivity;
        cohorts.push(CohortState {
            weight,
            productivity,
            energy_intensity,
            debt: 0.0,
            liquidity,
            defaulted_share: 0.0,
        });
    }

    let baseline_gdp: f64 = cohorts.iter().map(|c| c.productivity * c.weight).sum();
    let target_debt_total = cfg.initial_debt_to_gdp * baseline_gdp;
    let denom: f64 = cohorts
        .iter()
        .map(|c| (c.productivity * c.weight).max(0.0))
        .sum::<f64>()
        .max(1e-9);

    for cohort in &mut cohorts {
        let share = (cohort.productivity * cohort.weight) / denom;
        cohort.debt = (target_debt_total * share) / cohort.weight.max(1e-9);
    }

    cohorts
}

pub fn run_simulation(cfg: &SimConfig) -> SimulationResult {
    let cfg = cfg.clone().clamped();
    let mut cohorts = init_cohorts(&cfg);

    let mut total_initial_debt: f64 = cohorts.iter().map(|c| c.debt * c.weight).sum();
    if !total_initial_debt.is_finite() || total_initial_debt <= 0.0 {
        total_initial_debt = 1.0;
    }

    let mut bank = BankState {
        capital: total_initial_debt * (cfg.min_bank_capital_ratio + 0.045),
    };

    let mut q = cfg.urr * cfg.initial_q_frac;
    let mut points = Vec::with_capacity(cfg.steps + 1);

    for step in 0..=cfg.steps {
        let t_years = step as f64 * cfg.dt_years;

        let efficiency_factor = (-cfg.energy_efficiency_trend * t_years)
            .exp()
            .clamp(0.35, 2.2);
        let extraction_tech = (1.0 + 0.35 * cfg.energy_efficiency_trend * t_years).clamp(0.5, 1.8);

        let raw = hubbert_flow(q, cfg.urr, cfg.extraction_k, extraction_tech);
        let cap = ((cfg.urr - q) / cfg.dt_years).max(0.0);
        let extraction_rate = raw.min(cap);

        let depletion = clamp01(q / cfg.urr.max(1e-9));
        let eroi = (18.0 * (1.0 - 0.86 * depletion)).max(2.2);
        let net_energy = (extraction_rate * (1.0 - 1.0 / eroi)).max(0.0);

        let mut potential_energy_demand = 0.0;

        for c in &cohorts {
            let active_share = 1.0 - c.defaulted_share;
            let potential_output = (c.productivity * active_share * c.weight).max(0.0);
            potential_energy_demand += potential_output * c.energy_intensity * efficiency_factor;
        }

        let scarcity = (potential_energy_demand / (net_energy + 1e-6)).max(0.0);
        let energy_price = (0.9 + scarcity.powf(1.35)).clamp(0.2, 14.0);
        let energy_allocation = if potential_energy_demand <= 1e-9 {
            1.0
        } else {
            (net_energy / potential_energy_demand).min(1.0)
        };

        let debt_total_before: f64 = cohorts
            .iter()
            .map(|c| c.debt * c.weight)
            .sum::<f64>()
            .max(1e-6);
        let bank_capital_ratio_before = bank.capital / debt_total_before;
        let supply_factor =
            lending_supply_factor(bank_capital_ratio_before, cfg.min_bank_capital_ratio);

        let mut credit_demands = vec![0.0; cohorts.len()];
        let mut interest_due = vec![0.0; cohorts.len()];
        let mut principal_due = vec![0.0; cohorts.len()];
        let mut energy_costs = vec![0.0; cohorts.len()];
        let mut revenues = vec![0.0; cohorts.len()];
        let mut free_cash_flows = vec![0.0; cohorts.len()];

        let mut total_credit_demand = 0.0;

        for (idx, cohort) in cohorts.iter().enumerate() {
            let active_share = 1.0 - cohort.defaulted_share;
            let potential_output = cohort.productivity * active_share * cohort.weight;
            let realized_output = potential_output * energy_allocation;

            let op_cost = 0.43 * realized_output;
            let energy_cost =
                realized_output * cohort.energy_intensity * efficiency_factor * energy_price * 0.22;
            let fcf = realized_output - op_cost - energy_cost;

            let interest = cohort.debt * cohort.weight * cfg.interest_apr * cfg.dt_years;
            let principal = cohort.debt * cohort.weight * 0.055 * cfg.dt_years;
            let investment_need = (0.14 * realized_output).max(0.0);

            let liquidity = cohort.liquidity * cohort.weight;
            let demand = (interest + principal + investment_need - (fcf + liquidity)).max(0.0);

            revenues[idx] = realized_output;
            free_cash_flows[idx] = fcf;
            energy_costs[idx] = energy_cost;
            interest_due[idx] = interest;
            principal_due[idx] = principal;
            credit_demands[idx] = demand;
            total_credit_demand += demand;
        }

        let total_new_loans = total_credit_demand * supply_factor;

        let mut gdp = 0.0;
        let mut total_debt_service_due = 0.0;
        let mut total_defaults = 0.0;
        let mut total_writeoffs = 0.0;
        let mut total_interest_paid = 0.0;

        for (idx, cohort) in cohorts.iter_mut().enumerate() {
            let active_share = 1.0 - cohort.defaulted_share;
            let demand = credit_demands[idx];
            let granted = if total_credit_demand > 1e-9 {
                total_new_loans * demand / total_credit_demand
            } else {
                0.0
            };

            let revenue = revenues[idx];
            let fcf = free_cash_flows[idx];
            let interest = interest_due[idx];
            let principal = principal_due[idx];
            let debt_service_due = interest + principal;

            let available_cash = (cohort.liquidity * cohort.weight + fcf + granted).max(0.0);
            let debt_service_paid = available_cash.min(debt_service_due);
            let interest_paid = interest.min(debt_service_paid);
            let principal_paid = (debt_service_paid - interest_paid).max(0.0);
            let shortfall = (debt_service_due - debt_service_paid).max(0.0);

            let debt_service_ratio = debt_service_due / (revenue + 1e-6);
            let energy_burden = energy_costs[idx] / (revenue + 1e-6);
            let leverage = cohort.debt * cohort.weight / (revenue + 1.0);

            let stress_signal = 2.6 * (shortfall / (debt_service_due + 1e-6))
                + 1.8 * (energy_burden - 0.32)
                + 1.3 * (debt_service_ratio - 0.28)
                + 1.0 * (leverage - 2.6)
                + 1.0 * (1.0 - supply_factor)
                + 0.8 * scarcity.max(0.0).ln_1p();

            let stress_excess = (stress_signal - 1.0).max(0.0);
            let default_hazard = (1.0 - (-stress_excess).exp()).clamp(0.0, 1.0);
            let newly_defaulted_share =
                (1.0 - cohort.defaulted_share) * default_hazard * cfg.dt_years * 1.2;
            let newly_defaulted_share = newly_defaulted_share.clamp(0.0, 0.35);

            let existing_debt_total = cohort.debt * cohort.weight;
            let writeoff = existing_debt_total * newly_defaulted_share * 0.9;

            let unpaid_interest = (interest - interest_paid).max(0.0);

            let new_debt_total = update_debt_stock(
                existing_debt_total,
                0.0,
                granted + unpaid_interest,
                principal_paid,
                writeoff,
            );

            cohort.debt = new_debt_total / cohort.weight;
            let end_liquidity = (available_cash - debt_service_paid).max(0.0);
            cohort.liquidity = end_liquidity / cohort.weight;
            let recovery_signal = (1.0 - default_hazard) * (1.0 - (scarcity / 2.8).clamp(0.0, 1.0));
            let recovered_share =
                (0.05 * recovery_signal * cfg.dt_years).min(cohort.defaulted_share);
            cohort.defaulted_share = (cohort.defaulted_share + newly_defaulted_share
                - recovered_share)
                .clamp(0.0, 0.995);

            let innovation = (granted / (revenue + 1.0)).clamp(0.0, 0.4) * 0.08;
            let stress_drag = (default_hazard * 0.05 + scarcity * 0.01).clamp(0.0, 0.09);
            cohort.productivity = (cohort.productivity
                * (1.0 + (innovation - stress_drag) * cfg.dt_years))
                .clamp(0.05, 8.0);
            cohort.energy_intensity = (cohort.energy_intensity
                * (1.0 - cfg.energy_efficiency_trend * cfg.dt_years))
                .clamp(0.05, 3.5);
            if !active_share.is_finite() {
                cohort.defaulted_share = 0.0;
            }

            gdp += revenue;
            total_debt_service_due += debt_service_due;
            total_defaults += newly_defaulted_share * cohort.weight;
            total_writeoffs += writeoff;
            total_interest_paid += interest_paid;
        }

        bank.capital += total_interest_paid * 0.14 - total_writeoffs;
        bank.capital = bank.capital.max(0.01);

        let debt_total_after: f64 = cohorts
            .iter()
            .map(|c| c.debt * c.weight)
            .sum::<f64>()
            .max(0.0);
        let debt_to_gdp = if gdp <= 1e-9 {
            0.0
        } else {
            debt_total_after / gdp
        };
        let debt_service_ratio = if gdp <= 1e-9 {
            0.0
        } else {
            total_debt_service_due / gdp
        };
        let default_rate = total_defaults / cfg.effective_population as f64;
        let bank_capital_ratio = if debt_total_after <= 1e-9 {
            1.0
        } else {
            (bank.capital / debt_total_after).max(0.0)
        };

        points.push(MetricsPoint {
            t_years,
            gdp,
            net_energy,
            extraction_rate,
            debt_total: debt_total_after,
            debt_to_gdp,
            debt_service_ratio,
            default_rate,
            bank_capital_ratio,
            loan_flow: total_new_loans,
            write_off_flow: total_writeoffs,
            energy_price,
        });

        q = (q + extraction_rate * cfg.dt_years).min(cfg.urr);
    }

    let mut result = SimulationResult {
        points,
        peak_oil_step: None,
        gdp_collapse_step: None,
    };

    result.peak_oil_step = detect_peak_oil(&result);
    result.gdp_collapse_step = detect_gdp_collapse(&result, 0.30, 12);
    result
}

#[cfg(test)]
mod tests {
    use super::{hubbert_flow, lending_supply_factor, run_simulation, update_debt_stock};
    use crate::model::SimConfig;

    #[test]
    fn interest_compounding_math_is_consistent() {
        let debt = update_debt_stock(100.0, 0.01, 5.0, 3.0, 1.0);
        let expected = 100.0 * 1.01 + 5.0 - 3.0 - 1.0;
        assert!((debt - expected).abs() < 1e-9);
    }

    #[test]
    fn hubbert_curve_peaks_near_half_urr() {
        let urr = 10_000.0;
        let k = 0.08;
        let dt = 1.0 / 12.0;
        let mut q = urr * 0.05;

        let mut peak_q = q;
        let mut peak_flow = 0.0;

        for _ in 0..1_200 {
            let flow = hubbert_flow(q, urr, k, 1.0);
            if flow > peak_flow {
                peak_flow = flow;
                peak_q = q;
            }
            q = (q + flow * dt).min(urr);
        }

        let peak_frac = peak_q / urr;
        assert!((peak_frac - 0.5).abs() < 0.12, "peak_frac={peak_frac}");
    }

    #[test]
    fn bank_capital_constraint_throttles_lending() {
        let min = 0.10;
        let low = lending_supply_factor(0.05, min);
        let mid = lending_supply_factor(0.10, min);
        let high = lending_supply_factor(0.14, min);

        assert!(low < mid);
        assert!(mid < high);
        assert!(high <= 1.0);
    }

    #[test]
    fn simulation_is_deterministic() {
        let cfg = SimConfig::default();
        let a = run_simulation(&cfg);
        let b = run_simulation(&cfg);
        assert_eq!(a.points.len(), b.points.len());

        for (pa, pb) in a.points.iter().zip(b.points.iter()) {
            assert!((pa.gdp - pb.gdp).abs() < 1e-12);
            assert!((pa.net_energy - pb.net_energy).abs() < 1e-12);
            assert!((pa.debt_total - pb.debt_total).abs() < 1e-12);
        }

        assert_eq!(a.peak_oil_step, b.peak_oil_step);
        assert_eq!(a.gdp_collapse_step, b.gdp_collapse_step);
    }
}
