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
    #[must_use]
    pub const fn with_shape(mut self, shape: PinShape) -> Self {
        self.shape = Some(shape);
        self
    }

    /// Sets the size of the pin.
    #[must_use]
    pub const fn with_size(mut self, size: f32) -> Self {
        self.size = Some(size);
        self
    }

    /// Sets the fill color of the pin.
    #[must_use]
    pub const fn with_fill(mut self, fill: Color32) -> Self {
        self.fill = Some(fill);
        self
    }

    /// Sets the outline stroke of the pin.
    #[must_use]
    pub const fn with_stroke(mut self, stroke: Stroke) -> Self {
        self.stroke = Some(stroke);
        self
    }

    /// Sets the style of the wire connected to the pin.
    #[must_use]
    pub const fn with_wire_style(mut self, wire_style: WireStyle) -> Self {
        self.wire_style = Some(wire_style);
        self
    }

    /// Creates a circle pin.
    #[must_use]
    pub fn circle() -> Self {
        PinInfo {
            shape: Some(PinShape::Circle),
            ..Default::default()
        }
    }

    /// Creates a triangle pin.
    #[must_use]
    pub fn triangle() -> Self {
        PinInfo {
            shape: Some(PinShape::Triangle),
            ..Default::default()
        }
    }

    /// Creates a square pin.
    #[must_use]
    pub fn square() -> Self {
        PinInfo {
            shape: Some(PinShape::Square),
            ..Default::default()
        }
    }

    /// Creates a star pin.
    #[must_use]
    pub fn star() -> Self {
        PinInfo {
            shape: Some(PinShape::Star),
            ..Default::default()
        }
    }

    /// Returns the shape of the pin.
    #[must_use]
    pub fn get_shape(&self, snarl_style: &SnarlStyle) -> PinShape {
        self.shape.unwrap_or_else(|| snarl_style.get_pin_shape())
    }

    /// Returns fill color of the pin.
    #[must_use]
    pub fn get_fill(&self, snarl_style: &SnarlStyle, style: &Style) -> Color32 {
        self.fill.unwrap_or_else(|| snarl_style.get_pin_fill(style))
    }

    /// Returns outline stroke of the pin.
    #[must_use]
    pub fn get_stroke(&self, snarl_style: &SnarlStyle, style: &Style, scale: f32) -> Stroke {
        self.stroke
            .zoomed(scale)
            .unwrap_or_else(|| snarl_style.get_pin_stroke(scale, style))
    }

    /// Draws the pin and returns color.
    ///
    /// Wires are drawn with returned color by default.
    #[must_use]
    pub fn draw(
        &self,
        pos: Pos2,
        size: f32,
        snarl_style: &SnarlStyle,
        style: &Style,
        painter: &Painter,
        scale: f32,
    ) -> Color32 {
        let shape = self.get_shape(snarl_style);
        let fill = self.get_fill(snarl_style, style);
        let stroke = self.get_stroke(snarl_style, style, scale);
        let size = self.size.zoomed(scale).unwrap_or(size);
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
                pos + size * 0.700_000 * vec2(0.0, -1.0),
                pos + size * 0.267_376 * vec2(-0.587_785, -0.809_017),
                pos + size * 0.700_000 * vec2(-0.951_057, -0.309_017),
                pos + size * 0.267_376 * vec2(-0.951_057, 0.309_017),
                pos + size * 0.700_000 * vec2(-0.587_785, 0.809_017),
                pos + size * 0.267_376 * vec2(0.0, 1.0),
                pos + size * 0.700_000 * vec2(0.587_785, 0.809_017),
                pos + size * 0.267_376 * vec2(0.951_057, 0.309_017),
                pos + size * 0.700_000 * vec2(0.951_057, -0.309_017),
                pos + size * 0.267_376 * vec2(0.587_785, -0.809_017),
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
