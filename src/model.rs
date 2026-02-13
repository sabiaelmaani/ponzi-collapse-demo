use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SimConfig {
    pub steps: usize,
    pub dt_years: f64,
    pub cohort_count: usize,
    pub effective_population: u64,
    pub interest_apr: f64,
    pub initial_debt_to_gdp: f64,
    pub min_bank_capital_ratio: f64,
    pub urr: f64,
    pub extraction_k: f64,
    pub initial_q_frac: f64,
    pub energy_efficiency_trend: f64,
}

impl Default for SimConfig {
    fn default() -> Self {
        Self {
            steps: 240,
            dt_years: 1.0 / 12.0,
            cohort_count: 1_000,
            effective_population: 1_000_000,
            interest_apr: 0.10,
            initial_debt_to_gdp: 2.8,
            min_bank_capital_ratio: 0.10,
            urr: 6_000.0,
            extraction_k: 0.14,
            initial_q_frac: 0.42,
            energy_efficiency_trend: 0.009,
        }
    }
}

impl SimConfig {
    pub fn clamped(mut self) -> Self {
        self.steps = self.steps.clamp(12, 1_200);
        self.dt_years = self.dt_years.clamp(1.0 / 24.0, 0.5);
        self.cohort_count = self.cohort_count.clamp(25, 10_000);
        self.effective_population = self.effective_population.clamp(10_000, 10_000_000);
        self.interest_apr = self.interest_apr.clamp(0.0, 0.35);
        self.initial_debt_to_gdp = self.initial_debt_to_gdp.clamp(0.0, 8.0);
        self.min_bank_capital_ratio = self.min_bank_capital_ratio.clamp(0.02, 0.4);
        self.urr = self.urr.clamp(100.0, 100_000.0);
        self.extraction_k = self.extraction_k.clamp(0.001, 0.35);
        self.initial_q_frac = self.initial_q_frac.clamp(0.001, 0.95);
        self.energy_efficiency_trend = self.energy_efficiency_trend.clamp(-0.05, 0.08);
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MetricsPoint {
    pub t_years: f64,
    pub gdp: f64,
    pub net_energy: f64,
    pub extraction_rate: f64,
    pub debt_total: f64,
    pub debt_to_gdp: f64,
    pub debt_service_ratio: f64,
    pub default_rate: f64,
    pub bank_capital_ratio: f64,
    pub loan_flow: f64,
    pub write_off_flow: f64,
    pub energy_price: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SimulationResult {
    pub points: Vec<MetricsPoint>,
    pub peak_oil_step: Option<usize>,
    pub gdp_collapse_step: Option<usize>,
}
