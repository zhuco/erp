//! Small, dependency-free charts for report summaries.
//!
//! The host owns the meaning of the labels and values.  Values remain fixed
//! point integers at the API boundary; conversion to `f64` is only used for
//! screen geometry.

use egui::{Align2, Color32, FontId, Id, Pos2, Rect, Sense, Stroke, StrokeKind, Ui, Vec2};

const MAX_POINTS: usize = 256;
const MAX_LABEL_BYTES: usize = 256;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ChartKind {
    Bar,
    Line,
}

/// Paint a compact report chart into the available width.
pub fn show(
    ui: &mut Ui,
    id: Id,
    kind: ChartKind,
    points: &[(String, i128)],
    scale: u32,
) -> Result<(), String> {
    validate(points, scale)?;
    let values: Vec<f64> = points
        .iter()
        .map(|(_, value)| (*value as f64) / pow10(scale))
        .collect();
    let (low, high) = bounds(&values);

    let width = ui.available_width().max(1.0);
    let height = ui.available_height().clamp(180.0, 260.0);
    let (rect, _) = ui.allocate_exact_size(Vec2::new(width, height), Sense::hover());
    let painter = ui.painter_at(rect);
    let text_color = ui.visuals().text_color();
    let grid_color = ui.visuals().widgets.noninteractive.bg_stroke.color;
    let accent = ui.visuals().selection.bg_fill;

    // Reserve room for the compact y-axis labels and x-axis labels.
    let left_margin = rect.width().min(48.0);
    let plot = Rect::from_min_max(
        Pos2::new(rect.left() + left_margin, rect.top() + 10.0),
        Pos2::new(rect.right(), rect.bottom() - 28.0),
    );
    let baseline = value_y(0.0, low, high, plot);
    painter.line_segment(
        [
            Pos2::new(plot.left(), baseline),
            Pos2::new(plot.right(), baseline),
        ],
        Stroke::new(1.0, grid_color),
    );
    for tick in [low, 0.0, high] {
        let y = value_y(tick, low, high, plot);
        painter.line_segment(
            [Pos2::new(plot.left(), y), Pos2::new(plot.right(), y)],
            Stroke::new(1.0, grid_color.gamma_multiply(0.55)),
        );
        painter.text(
            Pos2::new(plot.left() - 5.0, y),
            Align2::RIGHT_CENTER,
            compact_number(tick),
            FontId::proportional(11.0),
            text_color,
        );
    }

    let n = points.len();
    if n > 0 {
        let step = plot.width() / n as f32;
        let centers: Vec<Pos2> = values
            .iter()
            .enumerate()
            .map(|(index, value)| {
                Pos2::new(
                    plot.left() + step * (index as f32 + 0.5),
                    value_y(*value, low, high, plot),
                )
            })
            .collect();

        match kind {
            ChartKind::Bar => {
                for (index, ((label, raw), center)) in points.iter().zip(centers.iter()).enumerate()
                {
                    let left = plot.left() + step * index as f32 + step * 0.14;
                    let right = plot.left() + step * (index as f32 + 1.0) - step * 0.14;
                    let bar = Rect::from_min_max(
                        Pos2::new(left, center.y.min(baseline)),
                        Pos2::new(right.max(left + 1.0), center.y.max(baseline)),
                    );
                    painter.rect_filled(bar, 2.0, accent);
                    let response = ui.interact(bar, id.with(index), Sense::hover());
                    response.on_hover_text(format!("{label}: {}", format_fixed(*raw, scale)));
                    draw_x_label(
                        &painter,
                        label,
                        index,
                        n,
                        center.x,
                        plot.bottom(),
                        text_color,
                    );
                }
            }
            ChartKind::Line => {
                for pair in centers.windows(2) {
                    painter.line_segment([pair[0], pair[1]], Stroke::new(2.0, accent));
                }
                for (index, ((label, raw), center)) in points.iter().zip(centers.iter()).enumerate()
                {
                    painter.circle_filled(*center, 3.0, accent);
                    let hit = Rect::from_center_size(*center, Vec2::splat(14.0));
                    let response = ui.interact(hit, id.with(index), Sense::hover());
                    response.on_hover_text(format!("{label}: {}", format_fixed(*raw, scale)));
                    draw_x_label(
                        &painter,
                        label,
                        index,
                        n,
                        center.x,
                        plot.bottom(),
                        text_color,
                    );
                }
            }
        }
    }
    painter.rect_stroke(plot, 0.0, Stroke::new(1.0, grid_color), StrokeKind::Inside);
    Ok(())
}

fn validate(points: &[(String, i128)], scale: u32) -> Result<(), String> {
    if points.len() > MAX_POINTS {
        return Err(format!("chart supports at most {MAX_POINTS} points"));
    }
    if scale > 8 {
        return Err("chart scale must be at most 8".into());
    }
    if let Some((index, (_label, _))) = points
        .iter()
        .enumerate()
        .find(|(_, (label, _))| label.len() > MAX_LABEL_BYTES)
    {
        return Err(format!(
            "chart label {index} exceeds {MAX_LABEL_BYTES} bytes"
        ));
    }
    Ok(())
}

fn pow10(scale: u32) -> f64 {
    10_f64.powi(scale as i32)
}

fn bounds(values: &[f64]) -> (f64, f64) {
    let mut low = values
        .iter()
        .copied()
        .filter(|value| value.is_finite())
        .fold(0.0_f64, f64::min);
    let mut high = values
        .iter()
        .copied()
        .filter(|value| value.is_finite())
        .fold(0.0_f64, f64::max);
    if low == high {
        let pad = if low == 0.0 {
            1.0
        } else {
            low.abs().max(1.0) * 0.1
        };
        low -= pad;
        high += pad;
    }
    (low, high)
}

fn value_y(value: f64, low: f64, high: f64, plot: Rect) -> f32 {
    let fraction = if value.is_finite() && low.is_finite() && high.is_finite() && high > low {
        ((value - low) / (high - low)).clamp(0.0, 1.0)
    } else {
        0.5
    };
    plot.bottom() - plot.height() * fraction as f32
}

fn format_fixed(value: i128, scale: u32) -> String {
    if scale == 0 {
        return value.to_string();
    }
    let base = 10_i128.pow(scale);
    let negative = value < 0;
    let magnitude = value.unsigned_abs();
    let whole = magnitude / base as u128;
    let fraction = magnitude % base as u128;
    let prefix = if negative { "-" } else { "" };
    format!("{prefix}{whole}.{fraction:0width$}", width = scale as usize)
}

fn compact_number(value: f64) -> String {
    if value == 0.0 {
        "0".into()
    } else if value.abs() >= 1_000_000.0 {
        format!("{value:.2e}")
    } else {
        format!("{value:.3}")
    }
}

fn draw_x_label(
    painter: &egui::Painter,
    label: &str,
    index: usize,
    count: usize,
    x: f32,
    y: f32,
    color: Color32,
) {
    if index == 0 || index + 1 == count || count <= 8 || index.is_multiple_of((count / 6).max(1)) {
        painter.text(
            Pos2::new(x, y + 5.0),
            Align2::CENTER_TOP,
            label,
            FontId::proportional(10.0),
            color,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_negative_fixed_point_is_preserved() {
        assert_eq!(format_fixed(-50, 2), "-0.50");
        assert_eq!(
            format_fixed(i128::MIN, 6),
            "-170141183460469231731687303715884.105728"
        );
    }

    #[test]
    fn geometry_handles_extreme_values_without_nonfinite_results() {
        let values = [i128::MIN as f64 / 1e6, i128::MAX as f64 / 1e6];
        let (low, high) = bounds(&values);
        let rect = Rect::from_min_size(Pos2::ZERO, Vec2::new(800.0, 200.0));
        assert!(low.is_finite() && high.is_finite());
        assert!(value_y(values[0], low, high, rect).is_finite());
        assert!(value_y(values[1], low, high, rect).is_finite());
        assert!(value_y(f64::NAN, f64::NEG_INFINITY, f64::INFINITY, rect).is_finite());
    }
}
