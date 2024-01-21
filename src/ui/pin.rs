use egui::{epaint::PathShape, vec2, Color32, Painter, Pos2, Rect, Shape, Stroke, Vec2};

use crate::{InPinId, OutPinId};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AnyPin {
    Out(OutPinId),
    In(InPinId),
}

/// Shape of a pin.
pub enum PinShape {
    /// Circle shape.
    Circle,

    /// Triangle shape.
    Triangle,

    /// Square shape.
    Square,

    /// Star
    Star,

    /// Custom shape.
    Custom(Box<dyn FnOnce(&Painter, Rect, Color32, Stroke)>),
}

/// Information about a pin returned by `SnarlViewer::show_input` and `SnarlViewer::show_output`.
pub struct PinInfo {
    pub shape: PinShape,
    pub size: f32,
    pub fill: Color32,
    pub stroke: Stroke,
}

impl Default for PinInfo {
    fn default() -> Self {
        PinInfo {
            shape: PinShape::Circle,
            size: 1.0,
            fill: Color32::GRAY,
            stroke: Stroke::new(1.0, Color32::BLACK),
        }
    }
}

impl PinInfo {
    pub fn with_shape(mut self, shape: PinShape) -> Self {
        self.shape = shape;
        self
    }

    pub fn with_size(mut self, size: f32) -> Self {
        self.size = size;
        self
    }

    pub fn with_fill(mut self, fill: Color32) -> Self {
        self.fill = fill;
        self
    }

    pub fn with_stroke(mut self, stroke: Stroke) -> Self {
        self.stroke = stroke;
        self
    }

    pub fn circle() -> Self {
        PinInfo {
            shape: PinShape::Circle,
            ..Default::default()
        }
    }

    pub fn triangle() -> Self {
        PinInfo {
            shape: PinShape::Triangle,
            ..Default::default()
        }
    }

    pub fn square() -> Self {
        PinInfo {
            shape: PinShape::Square,
            ..Default::default()
        }
    }
}

pub fn draw_pin(painter: &Painter, pin: PinInfo, pos: Pos2, base_size: f32) {
    let size = base_size * pin.size;
    match pin.shape {
        PinShape::Circle => {
            painter.circle(pos, size * 0.5, pin.fill, pin.stroke);
        }
        PinShape::Triangle => {
            const A: Vec2 = vec2(-0.649_519, 0.4875);
            const B: Vec2 = vec2(0.649_519, 0.4875);
            const C: Vec2 = vec2(0.0, -0.6375);

            let points = vec![pos + A * size, pos + B * size, pos + C * size];

            painter.add(Shape::Path(PathShape {
                points,
                closed: true,
                fill: pin.fill,
                stroke: pin.stroke,
            }));
        }
        PinShape::Square => {
            let points = vec![
                pos + vec2(-0.5, -0.5) * size,
                pos + vec2(0.5, -0.5) * size,
                pos + vec2(0.5, 0.5) * size,
                pos + vec2(-0.5, 0.5) * size,
            ];

            painter.add(Shape::Path(PathShape {
                points,
                closed: true,
                fill: pin.fill,
                stroke: pin.stroke,
            }));
        }

        PinShape::Star => {
            let points = vec![
                pos + (vec2(12.0, 2.0) - vec2(12.0, 12.0)) / 22.0 * size,
                pos + (vec2(15.09, 8.26) - vec2(12.0, 12.0)) / 22.0 * size,
                pos + (vec2(22.0, 9.27) - vec2(12.0, 12.0)) / 22.0 * size,
                pos + (vec2(17.0, 14.14) - vec2(12.0, 12.0)) / 22.0 * size,
                pos + (vec2(18.18, 21.02) - vec2(12.0, 12.0)) / 22.0 * size,
                pos + (vec2(12.0, 17.77) - vec2(12.0, 12.0)) / 22.0 * size,
                pos + (vec2(5.82, 21.02) - vec2(12.0, 12.0)) / 22.0 * size,
                pos + (vec2(7.0, 14.14) - vec2(12.0, 12.0)) / 22.0 * size,
                pos + (vec2(2.0, 9.27) - vec2(12.0, 12.0)) / 22.0 * size,
                pos + (vec2(8.91, 8.26) - vec2(12.0, 12.0)) / 22.0 * size,
                pos + (vec2(12.0, 2.0) - vec2(12.0, 12.0)) / 22.0 * size,
            ];

            painter.add(Shape::Path(PathShape {
                points,
                closed: true,
                fill: pin.fill,
                stroke: pin.stroke,
            }));
        }

        PinShape::Custom(f) => f(
            painter,
            Rect::from_center_size(pos, vec2(size, size)),
            pin.fill,
            pin.stroke,
        ),
    }
}
