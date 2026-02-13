#![cfg(target_arch = "wasm32")]

use plotters::prelude::*;
use plotters_canvas::CanvasBackend;
use web_sys::HtmlCanvasElement;

use crate::model::MetricsPoint;

#[derive(Clone, PartialEq)]
pub struct SeriesData {
    pub label: String,
    pub color: (u8, u8, u8),
    pub points: Vec<(f64, f64)>,
}

impl SeriesData {
    pub fn new(label: impl Into<String>, color: (u8, u8, u8), points: Vec<(f64, f64)>) -> Self {
        Self {
            label: label.into(),
            color,
            points,
        }
    }
}

fn range_from_series(series: &[SeriesData]) -> Option<((f64, f64), (f64, f64))> {
    let mut x_min = f64::INFINITY;
    let mut x_max = f64::NEG_INFINITY;
    let mut y_min = f64::INFINITY;
    let mut y_max = f64::NEG_INFINITY;

    for s in series {
        for (x, y) in &s.points {
            x_min = x_min.min(*x);
            x_max = x_max.max(*x);
            y_min = y_min.min(*y);
            y_max = y_max.max(*y);
        }
    }

    if !x_min.is_finite() || !x_max.is_finite() || !y_min.is_finite() || !y_max.is_finite() {
        return None;
    }

    if (x_max - x_min).abs() < f64::EPSILON {
        x_max += 1.0;
    }

    if (y_max - y_min).abs() < f64::EPSILON {
        y_max += 1.0;
        y_min = (y_min - 1.0).max(0.0);
    }

    Some(((x_min, x_max), (y_min, y_max)))
}

pub fn draw_line_chart(
    canvas: &HtmlCanvasElement,
    title: &str,
    y_label: &str,
    series: &[SeriesData],
) -> Result<(), String> {
    let backend = CanvasBackend::with_canvas_object(canvas.clone())
        .ok_or_else(|| "canvas backend unavailable".to_string())?;
    let root = backend.into_drawing_area();
    root.fill(&RGBColor(247, 248, 255))
        .map_err(|e| format!("fill: {e:?}"))?;

    if series.is_empty() {
        root.present().map_err(|e| format!("present: {e:?}"))?;
        return Ok(());
    }

    let ((x_min, x_max), (y_min, y_max)) =
        range_from_series(series).ok_or_else(|| "empty series range".to_string())?;
    let y_floor = y_min.min(0.0);
    let y_top = (y_max * 1.08).max(y_floor + 1e-6);

    let mut chart = ChartBuilder::on(&root)
        .caption(
            title,
            ("Trebuchet MS", 18)
                .into_font()
                .color(&RGBColor(14, 44, 77)),
        )
        .margin(14)
        .x_label_area_size(35)
        .y_label_area_size(54)
        .build_cartesian_2d(x_min..x_max, y_floor..y_top)
        .map_err(|e| format!("build chart: {e:?}"))?;

    chart
        .configure_mesh()
        .light_line_style(RGBColor(224, 229, 239))
        .bold_line_style(RGBColor(196, 204, 219))
        .x_labels(8)
        .y_labels(8)
        .x_desc("Years")
        .y_desc(y_label)
        .axis_desc_style(
            ("Trebuchet MS", 12)
                .into_font()
                .color(&RGBColor(44, 58, 82)),
        )
        .label_style(
            ("Trebuchet MS", 11)
                .into_font()
                .color(&RGBColor(78, 92, 118)),
        )
        .draw()
        .map_err(|e| format!("mesh: {e:?}"))?;

    for s in series {
        let color = RGBColor(s.color.0, s.color.1, s.color.2);
        chart
            .draw_series(LineSeries::new(s.points.clone(), color.stroke_width(2)))
            .map_err(|e| format!("line: {e:?}"))?
            .label(s.label.clone())
            .legend(move |(x, y)| {
                PathElement::new(vec![(x, y), (x + 16, y)], color.stroke_width(3))
            });
    }

    chart
        .configure_series_labels()
        .background_style(RGBColor(255, 255, 255).mix(0.92))
        .border_style(RGBColor(172, 186, 211))
        .label_font(("Trebuchet MS", 11).into_font())
        .draw()
        .map_err(|e| format!("legend: {e:?}"))?;

    root.present().map_err(|e| format!("present: {e:?}"))?;
    Ok(())
}

pub fn draw_master_chart(
    canvas: &HtmlCanvasElement,
    points: &[MetricsPoint],
) -> Result<(), String> {
    let backend = CanvasBackend::with_canvas_object(canvas.clone())
        .ok_or_else(|| "canvas backend unavailable".to_string())?;
    let root = backend.into_drawing_area();
    root.fill(&RGBColor(246, 249, 255))
        .map_err(|e| format!("fill: {e:?}"))?;

    if points.is_empty() {
        root.present().map_err(|e| format!("present: {e:?}"))?;
        return Ok(());
    }

    let x_min = points.first().map(|p| p.t_years).unwrap_or(0.0);
    let mut x_max = points.last().map(|p| p.t_years).unwrap_or(1.0);
    if (x_max - x_min).abs() < f64::EPSILON {
        x_max += 1.0;
    }

    let gdp_max = points
        .iter()
        .map(|p| p.gdp)
        .max_by(|a, b| a.total_cmp(b))
        .unwrap_or(1.0)
        .max(1.0);
    let energy_max = points
        .iter()
        .map(|p| p.net_energy)
        .max_by(|a, b| a.total_cmp(b))
        .unwrap_or(1.0)
        .max(1.0);

    let chart = ChartBuilder::on(&root)
        .caption(
            "Master System Trajectory: GDP vs Net Energy",
            ("Trebuchet MS", 20)
                .into_font()
                .color(&RGBColor(14, 44, 77)),
        )
        .margin(14)
        .x_label_area_size(38)
        .y_label_area_size(58)
        .right_y_label_area_size(58)
        .build_cartesian_2d(x_min..x_max, 0.0..(gdp_max * 1.08))
        .map_err(|e| format!("build master chart: {e:?}"))?;

    let mut chart = chart.set_secondary_coord(x_min..x_max, 0.0..(energy_max * 1.08));

    chart
        .configure_mesh()
        .x_labels(10)
        .y_labels(8)
        .x_desc("Years")
        .y_desc("GDP (normalized units)")
        .axis_desc_style(
            ("Trebuchet MS", 12)
                .into_font()
                .color(&RGBColor(36, 60, 94)),
        )
        .label_style(
            ("Trebuchet MS", 11)
                .into_font()
                .color(&RGBColor(71, 85, 112)),
        )
        .light_line_style(RGBColor(223, 229, 242))
        .bold_line_style(RGBColor(192, 202, 223))
        .draw()
        .map_err(|e| format!("master mesh: {e:?}"))?;

    chart
        .configure_secondary_axes()
        .y_desc("Net Energy")
        .axis_desc_style(
            ("Trebuchet MS", 12)
                .into_font()
                .color(&RGBColor(36, 60, 94)),
        )
        .label_style(
            ("Trebuchet MS", 11)
                .into_font()
                .color(&RGBColor(71, 85, 112)),
        )
        .draw()
        .map_err(|e| format!("secondary mesh: {e:?}"))?;

    let gdp_series: Vec<(f64, f64)> = points.iter().map(|p| (p.t_years, p.gdp)).collect();
    let energy_series: Vec<(f64, f64)> = points.iter().map(|p| (p.t_years, p.net_energy)).collect();

    chart
        .draw_series(LineSeries::new(
            gdp_series,
            RGBColor(197, 38, 49).stroke_width(3),
        ))
        .map_err(|e| format!("gdp line: {e:?}"))?
        .label("GDP")
        .legend(|(x, y)| {
            PathElement::new(
                vec![(x, y), (x + 16, y)],
                RGBColor(197, 38, 49).stroke_width(3),
            )
        });

    chart
        .draw_secondary_series(LineSeries::new(
            energy_series,
            RGBColor(35, 92, 171).stroke_width(3),
        ))
        .map_err(|e| format!("energy line: {e:?}"))?
        .label("Net Energy")
        .legend(|(x, y)| {
            PathElement::new(
                vec![(x, y), (x + 16, y)],
                RGBColor(35, 92, 171).stroke_width(3),
            )
        });

    chart
        .configure_series_labels()
        .background_style(RGBColor(255, 255, 255).mix(0.92))
        .border_style(RGBColor(176, 189, 214))
        .label_font(("Trebuchet MS", 11).into_font())
        .draw()
        .map_err(|e| format!("master legend: {e:?}"))?;

    root.present().map_err(|e| format!("present: {e:?}"))?;
    Ok(())
}
