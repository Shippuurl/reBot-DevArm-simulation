//! Small, brand-neutral line icons drawn with egui's painter.
//!
//! The icon set follows the same simple stroke language as Lucide, which is
//! also used as a reference by shadcn-rs. Drawing the handful of icons locally
//! avoids an egui-version-specific icon-font dependency and keeps text crisp at
//! any scale.

use eframe::egui::{Color32, Painter, Pos2, Rect, Stroke, Ui, Vec2};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Icon {
    Overview,
    Robot,
    Joints,
    Frames,
    Sensors,
}

pub fn draw(ui: &mut Ui, icon: Icon, rect: Rect, color: Color32) {
    let painter = ui.painter();
    draw_on(painter, icon, rect, color);
}

fn draw_on(painter: &Painter, icon: Icon, rect: Rect, color: Color32) {
    let stroke = Stroke::new(1.35, color);
    let c = rect.center();
    let x = rect.left();
    let y = rect.top();
    let w = rect.width();
    let h = rect.height();
    match icon {
        Icon::Overview => {
            painter.line_segment(
                [
                    Pos2::new(x + w * 0.16, y + h * 0.48),
                    Pos2::new(c.x, y + h * 0.14),
                ],
                stroke,
            );
            painter.line_segment(
                [
                    Pos2::new(c.x, y + h * 0.14),
                    Pos2::new(x + w * 0.84, y + h * 0.48),
                ],
                stroke,
            );
            painter.line_segment(
                [
                    Pos2::new(x + w * 0.25, y + h * 0.42),
                    Pos2::new(x + w * 0.25, y + h * 0.86),
                ],
                stroke,
            );
            painter.line_segment(
                [
                    Pos2::new(x + w * 0.75, y + h * 0.42),
                    Pos2::new(x + w * 0.75, y + h * 0.86),
                ],
                stroke,
            );
            painter.line_segment(
                [
                    Pos2::new(x + w * 0.25, y + h * 0.86),
                    Pos2::new(x + w * 0.75, y + h * 0.86),
                ],
                stroke,
            );
        }
        Icon::Robot => {
            painter.rect_stroke(
                Rect::from_min_max(
                    Pos2::new(x + w * 0.16, y + h * 0.25),
                    Pos2::new(x + w * 0.84, y + h * 0.82),
                ),
                2.0,
                stroke,
                egui::epaint::StrokeKind::Inside,
            );
            painter.line_segment(
                [Pos2::new(c.x, y + h * 0.25), Pos2::new(c.x, y + h * 0.08)],
                stroke,
            );
            painter.circle_filled(Pos2::new(c.x, y + h * 0.06), 1.6, color);
            painter.circle_filled(Pos2::new(x + w * 0.38, y + h * 0.5), 1.7, color);
            painter.circle_filled(Pos2::new(x + w * 0.62, y + h * 0.5), 1.7, color);
            painter.line_segment(
                [
                    Pos2::new(x + w * 0.35, y + h * 0.68),
                    Pos2::new(x + w * 0.65, y + h * 0.68),
                ],
                stroke,
            );
        }
        Icon::Joints => {
            painter.circle_stroke(Pos2::new(x + w * 0.26, y + h * 0.30), w * 0.12, stroke);
            painter.circle_stroke(Pos2::new(c.x, y + h * 0.58), w * 0.12, stroke);
            painter.circle_stroke(Pos2::new(x + w * 0.74, y + h * 0.30), w * 0.12, stroke);
            painter.line_segment(
                [
                    Pos2::new(x + w * 0.34, y + h * 0.36),
                    Pos2::new(x + w * 0.45, y + h * 0.51),
                ],
                stroke,
            );
            painter.line_segment(
                [
                    Pos2::new(x + w * 0.55, y + h * 0.51),
                    Pos2::new(x + w * 0.66, y + h * 0.36),
                ],
                stroke,
            );
            painter.line_segment(
                [
                    Pos2::new(x + w * 0.5, y + h * 0.7),
                    Pos2::new(x + w * 0.5, y + h * 0.9),
                ],
                stroke,
            );
        }
        Icon::Frames => {
            painter.line_segment(
                [
                    Pos2::new(c.x, y + h * 0.5),
                    Pos2::new(x + w * 0.84, y + h * 0.5),
                ],
                stroke,
            );
            painter.line_segment(
                [
                    Pos2::new(c.x, y + h * 0.5),
                    Pos2::new(x + w * 0.3, y + h * 0.16),
                ],
                stroke,
            );
            painter.line_segment(
                [
                    Pos2::new(c.x, y + h * 0.5),
                    Pos2::new(x + w * 0.3, y + h * 0.84),
                ],
                stroke,
            );
            painter.circle_filled(c, 2.2, color);
            painter.text(
                Pos2::new(x + w * 0.86, y + h * 0.5),
                egui::Align2::LEFT_CENTER,
                "X",
                egui::FontId::monospace(8.0),
                color,
            );
        }
        Icon::Sensors => {
            painter.rect_stroke(
                Rect::from_min_max(
                    Pos2::new(x + w * 0.18, y + h * 0.22),
                    Pos2::new(x + w * 0.82, y + h * 0.78),
                ),
                2.0,
                stroke,
                egui::epaint::StrokeKind::Inside,
            );
            painter.circle_stroke(c, w * 0.17, stroke);
            painter.circle_filled(c, 1.8, color);
            painter.line_segment(
                [
                    Pos2::new(x + w * 0.5, y + h * 0.08),
                    Pos2::new(x + w * 0.5, y + h * 0.22),
                ],
                stroke,
            );
            painter.line_segment(
                [
                    Pos2::new(x + w * 0.5, y + h * 0.78),
                    Pos2::new(x + w * 0.5, y + h * 0.92),
                ],
                stroke,
            );
        }
    }
}

pub fn icon_size() -> Vec2 {
    Vec2::splat(16.0)
}
