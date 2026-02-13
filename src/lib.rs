pub mod metrics;
pub mod model;
pub mod sim;

#[cfg(target_arch = "wasm32")]
pub mod charts;
#[cfg(target_arch = "wasm32")]
pub mod ui;

pub use metrics::{DashboardKpis, derive_dashboard_kpis};
pub use model::{MetricsPoint, SimConfig, SimulationResult};
pub use sim::run_simulation;
