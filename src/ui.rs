#![cfg(target_arch = "wasm32")]

use gloo_timers::callback::Timeout;
use web_sys::{HtmlCanvasElement, HtmlInputElement};
use yew::TargetCast;
use yew::prelude::*;

use crate::charts::{SeriesData, draw_line_chart, draw_master_chart};
use crate::metrics::derive_dashboard_kpis;
use crate::model::{MetricsPoint, SimConfig};
use crate::sim::run_simulation;

fn year_at_step(step: usize, cfg: &SimConfig) -> f64 {
    step as f64 * cfg.dt_years
}

#[derive(Properties, PartialEq, Clone)]
struct MasterChartProps {
    points: Vec<MetricsPoint>,
}

#[function_component(MasterChart)]
fn master_chart(props: &MasterChartProps) -> Html {
    let canvas_ref = use_node_ref();

    {
        let canvas_ref = canvas_ref.clone();
        let points = props.points.clone();
        use_effect_with(points, move |points| {
            if let Some(canvas) = canvas_ref.cast::<HtmlCanvasElement>() {
                let _ = draw_master_chart(&canvas, points);
            }
            || ()
        });
    }

    html! {
        <canvas ref={canvas_ref} class="chart-canvas master-canvas" width="1040" height="420"></canvas>
    }
}

#[derive(Properties, PartialEq, Clone)]
struct SeriesChartProps {
    title: AttrValue,
    y_label: AttrValue,
    series: Vec<SeriesData>,
}

#[function_component(SeriesChart)]
fn series_chart(props: &SeriesChartProps) -> Html {
    let canvas_ref = use_node_ref();

    {
        let canvas_ref = canvas_ref.clone();
        let title = props.title.to_string();
        let y_label = props.y_label.to_string();
        let series = props.series.clone();

        use_effect_with(
            (title.clone(), y_label.clone(), series.clone()),
            move |_| {
                if let Some(canvas) = canvas_ref.cast::<HtmlCanvasElement>() {
                    let _ = draw_line_chart(&canvas, &title, &y_label, &series);
                }
                || ()
            },
        );
    }

    html! {
        <canvas ref={canvas_ref} class="chart-canvas" width="500" height="280"></canvas>
    }
}

#[derive(Properties, PartialEq)]
struct ControlFieldProps {
    label: AttrValue,
    value_display: AttrValue,
    min: f64,
    max: f64,
    step: f64,
    value: f64,
    on_range: Callback<InputEvent>,
    on_number: Callback<InputEvent>,
}

#[function_component(ControlField)]
fn control_field(props: &ControlFieldProps) -> Html {
    html! {
        <div class="control-field">
            <div class="control-head">
                <span class="control-label">{props.label.clone()}</span>
                <span class="control-value">{props.value_display.clone()}</span>
            </div>
            <input
                type="range"
                min={props.min.to_string()}
                max={props.max.to_string()}
                step={props.step.to_string()}
                value={props.value.to_string()}
                oninput={props.on_range.clone()}
            />
            <input
                type="number"
                min={props.min.to_string()}
                max={props.max.to_string()}
                step={props.step.to_string()}
                value={props.value.to_string()}
                oninput={props.on_number.clone()}
            />
        </div>
    }
}

#[function_component(App)]
pub fn app() -> Html {
    let defaults = SimConfig::default().clamped();

    let config = use_state(|| defaults.clone());
    let result = use_state(|| run_simulation(&defaults));
    let debounce_handle = use_mut_ref(|| Option::<Timeout>::None);

    {
        let config_snapshot = (*config).clone();
        let result = result.clone();
        let debounce_handle = debounce_handle.clone();
        use_effect_with(config_snapshot, move |cfg| {
            if let Some(timeout) = debounce_handle.borrow_mut().take() {
                timeout.cancel();
            }

            let cfg_clone = cfg.clone();
            let result = result.clone();
            let handle = Timeout::new(180, move || {
                result.set(run_simulation(&cfg_clone));
            });
            *debounce_handle.borrow_mut() = Some(handle);
            || ()
        });
    }

    let apply_number = {
        let config = config.clone();
        move |mutator: fn(&mut SimConfig, f64), raw: f64| {
            let mut next = (*config).clone();
            mutator(&mut next, raw);
            config.set(next.clamped());
        }
    };

    let on_steps = {
        let apply_number = apply_number.clone();
        Callback::from(move |e: InputEvent| {
            let raw = e
                .target_unchecked_into::<HtmlInputElement>()
                .value_as_number();
            apply_number(|cfg, v| cfg.steps = v.round() as usize, raw);
        })
    };

    let on_cohorts = {
        let apply_number = apply_number.clone();
        Callback::from(move |e: InputEvent| {
            let raw = e
                .target_unchecked_into::<HtmlInputElement>()
                .value_as_number();
            apply_number(|cfg, v| cfg.cohort_count = v.round() as usize, raw);
        })
    };

    let on_population = {
        let apply_number = apply_number.clone();
        Callback::from(move |e: InputEvent| {
            let raw = e
                .target_unchecked_into::<HtmlInputElement>()
                .value_as_number();
            apply_number(|cfg, v| cfg.effective_population = v.round() as u64, raw);
        })
    };

    let on_interest = {
        let apply_number = apply_number.clone();
        Callback::from(move |e: InputEvent| {
            let raw = e
                .target_unchecked_into::<HtmlInputElement>()
                .value_as_number();
            apply_number(|cfg, v| cfg.interest_apr = v / 100.0, raw);
        })
    };

    let on_debt_gdp = {
        let apply_number = apply_number.clone();
        Callback::from(move |e: InputEvent| {
            let raw = e
                .target_unchecked_into::<HtmlInputElement>()
                .value_as_number();
            apply_number(|cfg, v| cfg.initial_debt_to_gdp = v, raw);
        })
    };

    let on_capital = {
        let apply_number = apply_number.clone();
        Callback::from(move |e: InputEvent| {
            let raw = e
                .target_unchecked_into::<HtmlInputElement>()
                .value_as_number();
            apply_number(|cfg, v| cfg.min_bank_capital_ratio = v / 100.0, raw);
        })
    };

    let on_urr = {
        let apply_number = apply_number.clone();
        Callback::from(move |e: InputEvent| {
            let raw = e
                .target_unchecked_into::<HtmlInputElement>()
                .value_as_number();
            apply_number(|cfg, v| cfg.urr = v, raw);
        })
    };

    let on_k = {
        let apply_number = apply_number.clone();
        Callback::from(move |e: InputEvent| {
            let raw = e
                .target_unchecked_into::<HtmlInputElement>()
                .value_as_number();
            apply_number(|cfg, v| cfg.extraction_k = v, raw);
        })
    };

    let on_qfrac = {
        let apply_number = apply_number.clone();
        Callback::from(move |e: InputEvent| {
            let raw = e
                .target_unchecked_into::<HtmlInputElement>()
                .value_as_number();
            apply_number(|cfg, v| cfg.initial_q_frac = v, raw);
        })
    };

    let on_efficiency = {
        let apply_number = apply_number.clone();
        Callback::from(move |e: InputEvent| {
            let raw = e
                .target_unchecked_into::<HtmlInputElement>()
                .value_as_number();
            apply_number(|cfg, v| cfg.energy_efficiency_trend = v / 100.0, raw);
        })
    };

    let on_run = {
        let config = config.clone();
        let result = result.clone();
        let debounce_handle = debounce_handle.clone();
        Callback::from(move |_| {
            if let Some(timeout) = debounce_handle.borrow_mut().take() {
                timeout.cancel();
            }
            result.set(run_simulation(&config));
        })
    };

    let on_reset = {
        let config = config.clone();
        let result = result.clone();
        let debounce_handle = debounce_handle.clone();
        Callback::from(move |_| {
            if let Some(timeout) = debounce_handle.borrow_mut().take() {
                timeout.cancel();
            }
            let fresh = SimConfig::default().clamped();
            result.set(run_simulation(&fresh));
            config.set(fresh);
        })
    };

    let points = (*result).points.clone();
    let kpis = derive_dashboard_kpis(&result);

    let peak_oil_year = kpis.peak_oil_step.map(|step| year_at_step(step, &config));
    let gdp_peak_year = kpis.gdp_peak_step.map(|step| year_at_step(step, &config));
    let collapse_year = kpis.collapse_step.map(|step| year_at_step(step, &config));

    let debt_to_gdp_series = vec![SeriesData::new(
        "Debt / GDP",
        (193, 66, 66),
        points.iter().map(|p| (p.t_years, p.debt_to_gdp)).collect(),
    )];

    let debt_service_series = vec![SeriesData::new(
        "Debt Service Ratio",
        (204, 116, 38),
        points
            .iter()
            .map(|p| (p.t_years, p.debt_service_ratio))
            .collect(),
    )];

    let default_series = vec![SeriesData::new(
        "Default Rate",
        (125, 49, 174),
        points.iter().map(|p| (p.t_years, p.default_rate)).collect(),
    )];

    let extraction_series = vec![SeriesData::new(
        "Extraction Flow",
        (31, 94, 148),
        points
            .iter()
            .map(|p| (p.t_years, p.extraction_rate))
            .collect(),
    )];

    let energy_price_series = vec![SeriesData::new(
        "Energy Price",
        (26, 146, 119),
        points.iter().map(|p| (p.t_years, p.energy_price)).collect(),
    )];

    let bank_capital_series = vec![SeriesData::new(
        "Bank Capital Ratio",
        (90, 112, 178),
        points
            .iter()
            .map(|p| (p.t_years, p.bank_capital_ratio))
            .collect(),
    )];

    let loan_vs_writeoff = vec![
        SeriesData::new(
            "Loan Issuance",
            (37, 122, 72),
            points.iter().map(|p| (p.t_years, p.loan_flow)).collect(),
        ),
        SeriesData::new(
            "Write-Offs",
            (187, 39, 52),
            points
                .iter()
                .map(|p| (p.t_years, p.write_off_flow))
                .collect(),
        ),
    ];

    html! {
        <div class="app-shell">
            <aside class="control-rail">
                <h1>{"Ponzisim"}</h1>
                <p class="subtitle">{"Debt expansion, finite extraction, and endogenous collapse dynamics"}</p>

                <ControlField label="Horizon (months)" value_display={config.steps.to_string()} min={120.0} max={1200.0} step={1.0} value={config.steps as f64} on_range={on_steps.clone()} on_number={on_steps} />
                <ControlField label="Cohort Agents" value_display={config.cohort_count.to_string()} min={100.0} max={5000.0} step={10.0} value={config.cohort_count as f64} on_range={on_cohorts.clone()} on_number={on_cohorts} />
                <ControlField label="Effective Population" value_display={config.effective_population.to_string()} min={100000.0} max={10000000.0} step={10000.0} value={config.effective_population as f64} on_range={on_population.clone()} on_number={on_population} />
                <ControlField label="Interest APR (%)" value_display={format!("{:.2}%", config.interest_apr * 100.0)} min={0.0} max={35.0} step={0.1} value={config.interest_apr * 100.0} on_range={on_interest.clone()} on_number={on_interest} />
                <ControlField label="Initial Debt / GDP" value_display={format!("{:.2}", config.initial_debt_to_gdp)} min={0.0} max={8.0} step={0.01} value={config.initial_debt_to_gdp} on_range={on_debt_gdp.clone()} on_number={on_debt_gdp} />
                <ControlField label="Min Bank Capital (%)" value_display={format!("{:.2}%", config.min_bank_capital_ratio * 100.0)} min={2.0} max={30.0} step={0.1} value={config.min_bank_capital_ratio * 100.0} on_range={on_capital.clone()} on_number={on_capital} />
                <ControlField label="URR (resource stock)" value_display={format!("{:.0}", config.urr)} min={1000.0} max={30000.0} step={50.0} value={config.urr} on_range={on_urr.clone()} on_number={on_urr} />
                <ControlField label="Extraction k" value_display={format!("{:.3}", config.extraction_k)} min={0.01} max={0.25} step={0.001} value={config.extraction_k} on_range={on_k.clone()} on_number={on_k} />
                <ControlField label="Initial Q / URR" value_display={format!("{:.3}", config.initial_q_frac)} min={0.01} max={0.90} step={0.001} value={config.initial_q_frac} on_range={on_qfrac.clone()} on_number={on_qfrac} />
                <ControlField label="Energy Efficiency Trend (%/yr)" value_display={format!("{:.2}%", config.energy_efficiency_trend * 100.0)} min={-5.0} max={8.0} step={0.05} value={config.energy_efficiency_trend * 100.0} on_range={on_efficiency.clone()} on_number={on_efficiency} />

                <div class="action-row">
                    <button class="btn-run" onclick={on_run}>{"Run Simulation"}</button>
                    <button class="btn-reset" onclick={on_reset}>{"Reset Defaults"}</button>
                </div>
            </aside>

            <main class="dashboard">
                <section class="kpi-grid">
                    <article class="kpi-card">
                        <h3>{"Peak Oil"}</h3>
                        <p>{peak_oil_year.map(|y| format!("Year {:.2}", y)).unwrap_or_else(|| "N/A".to_string())}</p>
                    </article>
                    <article class="kpi-card">
                        <h3>{"GDP Peak"}</h3>
                        <p>{gdp_peak_year.map(|y| format!("Year {:.2}", y)).unwrap_or_else(|| "N/A".to_string())}</p>
                    </article>
                    <article class="kpi-card">
                        <h3>{"GDP Drawdown"}</h3>
                        <p>{format!("{:.2}%", kpis.gdp_drawdown_pct)}</p>
                    </article>
                    <article class="kpi-card">
                        <h3>{"Collapse Signal"}</h3>
                        <p>{collapse_year.map(|y| format!("Detected at year {:.2}", y)).unwrap_or_else(|| "Not detected in horizon".to_string())}</p>
                    </article>
                </section>

                <section class="master-panel">
                    <MasterChart points={points.clone()} />
                </section>

                <section class="collapse-note">
                    {
                        if let Some(year) = collapse_year {
                            html! { <p>{format!("Endogenous collapse marker appears around year {:.2} when sustained GDP drawdown and credit stress co-occur.", year)}</p> }
                        } else {
                            html! { <p>{"No collapse marker is currently triggered; adjust debt, interest, extraction, and capital constraints to explore different trajectories."}</p> }
                        }
                    }
                </section>

                <section class="chart-grid">
                    <div class="chart-card"><SeriesChart title="Debt Burden" y_label="Debt / GDP" series={debt_to_gdp_series} /></div>
                    <div class="chart-card"><SeriesChart title="Debt Servicing" y_label="Debt Service / GDP" series={debt_service_series} /></div>
                    <div class="chart-card"><SeriesChart title="Defaults" y_label="Default Rate" series={default_series} /></div>
                    <div class="chart-card"><SeriesChart title="Peak Oil Dynamics" y_label="Extraction Flow" series={extraction_series} /></div>
                    <div class="chart-card"><SeriesChart title="Energy Scarcity Price" y_label="Price" series={energy_price_series} /></div>
                    <div class="chart-card"><SeriesChart title="Bank Solvency" y_label="Capital Ratio" series={bank_capital_series} /></div>
                    <div class="chart-card chart-wide"><SeriesChart title="Loan Issuance vs Write-Offs" y_label="Flow" series={loan_vs_writeoff} /></div>
                </section>
            </main>
        </div>
    }
}
