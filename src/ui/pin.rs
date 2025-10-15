use egui::{Color32, Painter, Rect, Shape, Stroke, Style, Vec2, epaint::PathShape, pos2, vec2};

use crate::{InPinId, OutPinId};

use super::{SnarlStyle, WireStyle};

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

/// Contains information about a pin's wire.
/// Used to draw the wire.
/// When two pins are connected, the wire is drawn between them,
/// using merged `PinWireInfo` from both pins.
pub struct PinWireInfo {
    /// Desired color of the wire.
    pub color: Color32,

    /// Desired style of the wire.
    /// Zoomed with current scale.
    pub style: WireStyle,
}

/// Uses `Painter` to draw a pin.
pub trait SnarlPin {
    /// Calculates pin Rect from the given parameters.
    fn pin_rect(&self, x: f32, y0: f32, y1: f32, size: f32) -> Rect {
        // Center vertically by default.
        let y = (y0 + y1) * 0.5;
        let pin_pos = pos2(x, y);
        Rect::from_center_size(pin_pos, vec2(size, size))
    }

    /// Draws the pin.
    ///
    /// `rect` is the interaction rectangle of the pin.
    /// Pin should fit in it.
    /// `painter` is used to add pin's shapes to the UI.
    ///
    /// Returns the color
    #[must_use]
    fn draw(
        self,
        snarl_style: &SnarlStyle,
        style: &Style,
        rect: Rect,
        painter: &Painter,
    ) -> PinWireInfo;
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

    /// Fill color of the pin.
    pub fill: Option<Color32>,

    /// Outline stroke of the pin.
    pub stroke: Option<Stroke>,

    /// Color of the wire connected to the pin.
    /// If `None`, the pin's fill color is used.
    pub wire_color: Option<Color32>,

    /// Style of the wire connected to the pin.
    pub wire_style: Option<WireStyle>,

    /// Custom vertical position of a pin
    pub position: Option<f32>,
}

impl PinInfo {
    /// Sets the shape of the pin.
    #[must_use]
    pub const fn with_shape(mut self, shape: PinShape) -> Self {
        self.shape = Some(shape);
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

    /// Sets the color of the wire connected to the pin.
    #[must_use]
    pub const fn with_wire_color(mut self, wire_color: Color32) -> Self {
        self.wire_color = Some(wire_color);
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
    pub fn get_stroke(&self, snarl_style: &SnarlStyle, style: &Style) -> Stroke {
        self.stroke
            .unwrap_or_else(|| snarl_style.get_pin_stroke(style))
    }

    /// Draws the pin and returns color.
    ///
    /// Wires are drawn with returned color by default.
    #[must_use]
    pub fn draw(
        &self,
        snarl_style: &SnarlStyle,
        style: &Style,
        rect: Rect,
        painter: &Painter,
    ) -> PinWireInfo {
        let shape = self.get_shape(snarl_style);
        let fill = self.get_fill(snarl_style, style);
        let stroke = self.get_stroke(snarl_style, style);
        draw_pin(painter, shape, fill, stroke, rect);

        PinWireInfo {
            color: self.wire_color.unwrap_or(fill),
            style: self
                .wire_style
                .unwrap_or_else(|| snarl_style.get_wire_style()),
        }
    }
}

impl SnarlPin for PinInfo {
    fn draw(
        self,
        snarl_style: &SnarlStyle,
        style: &Style,
        rect: Rect,
        painter: &Painter,
    ) -> PinWireInfo {
        Self::draw(&self, snarl_style, style, rect, painter)
    }
}

pub fn draw_pin(painter: &Painter, shape: PinShape, fill: Color32, stroke: Stroke, rect: Rect) {
    let center = rect.center();
    let size = f32::min(rect.width(), rect.height());

    match shape {
        PinShape::Circle => {
            painter.circle(center, size / 2.0, fill, stroke);
        }
        PinShape::Triangle => {
            const A: Vec2 = vec2(-0.649_519, 0.4875);
            const B: Vec2 = vec2(0.649_519, 0.4875);
            const C: Vec2 = vec2(0.0, -0.6375);

            let points = vec![center + A * size, center + B * size, center + C * size];

            painter.add(Shape::Path(PathShape {
                points,
                closed: true,
                fill,
                stroke: stroke.into(),
            }));
        }
        PinShape::Square => {
            let points = vec![
                center + vec2(-0.5, -0.5) * size,
                center + vec2(0.5, -0.5) * size,
                center + vec2(0.5, 0.5) * size,
                center + vec2(-0.5, 0.5) * size,
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
                center + size * 0.700_000 * vec2(0.0, -1.0),
                center + size * 0.267_376 * vec2(-0.587_785, -0.809_017),
                center + size * 0.700_000 * vec2(-0.951_057, -0.309_017),
                center + size * 0.267_376 * vec2(-0.951_057, 0.309_017),
                center + size * 0.700_000 * vec2(-0.587_785, 0.809_017),
                center + size * 0.267_376 * vec2(0.0, 1.0),
                center + size * 0.700_000 * vec2(0.587_785, 0.809_017),
                center + size * 0.267_376 * vec2(0.951_057, 0.309_017),
                center + size * 0.700_000 * vec2(0.951_057, -0.309_017),
                center + size * 0.267_376 * vec2(0.587_785, -0.809_017),
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
