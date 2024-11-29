use egui::{epaint::PathShape, vec2, Color32, Painter, Pos2, Shape, Stroke, Style, Vec2};

use crate::{InPinId, OutPinId};

use super::{zoom::Zoom, SnarlStyle, WireStyle};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AnyPin {
    Out(OutPinId),
    In(InPinId),
}

/// In the current context, these are the I/O pins of the 'source' node that the newly
/// created node's I/O pins will connect to.
#[derive(Debug)]
pub enum AnyPins<'a> {
    /// Output pins.
    Out(&'a [OutPinId]),
    /// Input pins
    In(&'a [InPinId]),
}

/// Shape of a pin.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "egui-probe", derive(egui_probe::EguiProbe))]
pub enum PinShape {
    /// Circle shape.
    #[default]
    Circle,

    /// Triangle shape.
    Triangle,

    /// Square shape.
    Square,

    /// Star shape.
    Star,
}

/// Information about a pin returned by `SnarlViewer::show_input` and `SnarlViewer::show_output`.
///
/// All fields are optional.
/// If a field is `None`, the default value is used derived from the graph style.
#[derive(Default)]
pub struct PinInfo {
    /// Shape of the pin.
    pub shape: Option<PinShape>,

    /// Size of the pin.
    pub size: Option<f32>,

    /// Fill color of the pin.
    pub fill: Option<Color32>,

    /// Outline stroke of the pin.
    pub stroke: Option<Stroke>,

    /// Style of the wire connected to the pin.
    pub wire_style: Option<WireStyle>,
}

impl PinInfo {
    /// Sets the shape of the pin.
    pub fn with_shape(mut self, shape: PinShape) -> Self {
        self.shape = Some(shape);
        self
    }

    /// Sets the size of the pin.
    pub fn with_size(mut self, size: f32) -> Self {
        self.size = Some(size);
        self
    }

    /// Sets the fill color of the pin.
    pub fn with_fill(mut self, fill: Color32) -> Self {
        self.fill = Some(fill);
        self
    }

    /// Sets the outline stroke of the pin.
    pub fn with_stroke(mut self, stroke: Stroke) -> Self {
        self.stroke = Some(stroke);
        self
    }

    /// Sets the style of the wire connected to the pin.
    pub fn with_wire_style(mut self, wire_style: WireStyle) -> Self {
        self.wire_style = Some(wire_style);
        self
    }

    /// Creates a circle pin.
    pub fn circle() -> Self {
        PinInfo {
            shape: Some(PinShape::Circle),
            ..Default::default()
        }
    }

    /// Creates a triangle pin.
    pub fn triangle() -> Self {
        PinInfo {
            shape: Some(PinShape::Triangle),
            ..Default::default()
        }
    }

    /// Creates a square pin.
    pub fn square() -> Self {
        PinInfo {
            shape: Some(PinShape::Square),
            ..Default::default()
        }
    }

    /// Creates a star pin.
    pub fn star() -> Self {
        PinInfo {
            shape: Some(PinShape::Star),
            ..Default::default()
        }
    }

    /// Draws the pin and returns color.
    ///
    /// Wires are drawn with returned color by default.
    pub fn draw(
        &self,
        pos: Pos2,
        size: f32,
        snarl_style: &SnarlStyle,
        style: &Style,
        painter: &Painter,
        scale: f32,
    ) -> Color32 {
        let shape = self.shape.unwrap_or(snarl_style.get_pin_shape());
        let fill = self.fill.unwrap_or(snarl_style.get_pin_fill(style));
        let stroke = self
            .stroke
            .zoomed(scale)
            .unwrap_or(snarl_style.get_pin_stroke(scale, style));
        let size = self.size.map_or(size, |s| s * size);

        draw_pin(painter, shape, fill, stroke, pos, size);

        fill
    }
}

pub fn draw_pin(
    painter: &Painter,
    shape: PinShape,
    fill: Color32,
    stroke: Stroke,
    pos: Pos2,
    size: f32,
) {
    match shape {
        PinShape::Circle => {
            painter.circle(pos, size * 2.0 / std::f32::consts::PI, fill, stroke);
        }
        PinShape::Triangle => {
            const A: Vec2 = vec2(-0.649_519, 0.4875);
            const B: Vec2 = vec2(0.649_519, 0.4875);
            const C: Vec2 = vec2(0.0, -0.6375);

            let points = vec![pos + A * size, pos + B * size, pos + C * size];

            painter.add(Shape::Path(PathShape {
                points,
                closed: true,
                fill,
                stroke: stroke.into(),
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
                fill,
                stroke: stroke.into(),
            }));
        }

        PinShape::Star => {
            let points = vec![
                pos + size * 0.700000 * vec2(0.0, -1.0),
                pos + size * 0.267376 * vec2(-0.587785, -0.809017),
                pos + size * 0.700000 * vec2(-0.951057, -0.309017),
                pos + size * 0.267376 * vec2(-0.951057, 0.309017),
                pos + size * 0.700000 * vec2(-0.587785, 0.809017),
                pos + size * 0.267376 * vec2(0.0, 1.0),
                pos + size * 0.700000 * vec2(0.587785, 0.809017),
                pos + size * 0.267376 * vec2(0.951057, 0.309017),
                pos + size * 0.700000 * vec2(0.951057, -0.309017),
                pos + size * 0.267376 * vec2(0.587785, -0.809017),
            ];

            painter.add(Shape::Path(PathShape {
                points,
                closed: true,
                fill,
                stroke: stroke.into(),
            }));
        }
    }
}
