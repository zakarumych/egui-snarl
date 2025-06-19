//! This module provides functionality for showing [`Snarl`] graph in [`Ui`].

use std::{collections::HashMap, hash::Hash};

use egui::{
    Align, Color32, CornerRadius, Frame, Id, LayerId, Layout, Margin, Modifiers, PointerButton,
    Pos2, Rect, Scene, Sense, Shape, Stroke, StrokeKind, Style, Ui, UiBuilder, UiKind, UiStackInfo,
    Vec2,
    collapsing_header::paint_default_icon,
    emath::{GuiRounding, TSTransform},
    epaint::Shadow,
    pos2,
    response::Flags,
    vec2,
};
use egui_scale::EguiScale;
use smallvec::SmallVec;

use crate::{InPin, InPinId, Node, NodeId, OutPin, OutPinId, Snarl};

mod background_pattern;
mod pin;
mod scale;
mod state;
mod viewer;
mod wire;

use self::{
    pin::AnyPin,
    state::{NewWires, NodeState, RowHeights, SnarlState},
    wire::{draw_wire, hit_wire, pick_wire_style},
};

pub use self::{
    background_pattern::{BackgroundPattern, Grid},
    pin::{AnyPins, PinInfo, PinShape, PinWireInfo, SnarlPin},
    state::get_selected_nodes,
    viewer::SnarlViewer,
    wire::{WireLayer, WireStyle},
};

/// Controls how header, pins, body and footer are placed in the node.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "egui-probe", derive(egui_probe::EguiProbe))]
pub enum NodeLayoutKind {
    /// Input pins, body and output pins are placed horizontally.
    /// With header on top and footer on bottom.
    ///
    /// +---------------------+
    /// |       Header        |
    /// +----+-----------+----+
    /// | In |           | Out|
    /// | In |   Body    | Out|
    /// | In |           | Out|
    /// | In |           |    |
    /// +----+-----------+----+
    /// |       Footer        |
    /// +---------------------+
    ///
    #[default]
    Coil,

    /// All elements are placed in vertical stack.
    /// Header is on top, then input pins, body, output pins and footer.
    ///
    /// +---------------------+
    /// |       Header        |
    /// +---------------------+
    /// | In                  |
    /// | In                  |
    /// | In                  |
    /// | In                  |
    /// +---------------------+
    /// |       Body          |
    /// +---------------------+
    /// |                 Out |
    /// |                 Out |
    /// |                 Out |
    /// +---------------------+
    /// |       Footer        |
    /// +---------------------+
    Sandwich,

    /// All elements are placed in vertical stack.
    /// Header is on top, then output pins, body, input pins and footer.
    ///
    /// +---------------------+
    /// |       Header        |
    /// +---------------------+
    /// |                 Out |
    /// |                 Out |
    /// |                 Out |
    /// +---------------------+
    /// |       Body          |
    /// +---------------------+
    /// | In                  |
    /// | In                  |
    /// | In                  |
    /// | In                  |
    /// +---------------------+
    /// |       Footer        |
    /// +---------------------+
    FlippedSandwich,
    // TODO: Add vertical layouts.
}

/// Controls how node elements are laid out.
///
///
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "egui-probe", derive(egui_probe::EguiProbe))]
pub struct NodeLayout {
    /// Controls method of laying out node elements.
    pub kind: NodeLayoutKind,

    /// Controls minimal height of pin rows.
    pub min_pin_row_height: f32,

    /// Controls how pin rows heights are set.
    /// If true, all pin rows will have the same height, matching the largest content.
    /// False by default.
    pub equal_pin_row_heights: bool,
}

impl NodeLayout {
    /// Creates new [`NodeLayout`] with `Coil` kind and flexible pin heights.
    #[inline]
    pub const fn coil() -> Self {
        NodeLayout {
            kind: NodeLayoutKind::Coil,
            min_pin_row_height: 0.0,
            equal_pin_row_heights: false,
        }
    }

    /// Creates new [`NodeLayout`] with `Sandwich` kind and flexible pin heights.
    #[inline]
    pub const fn sandwich() -> Self {
        NodeLayout {
            kind: NodeLayoutKind::Sandwich,
            min_pin_row_height: 0.0,
            equal_pin_row_heights: false,
        }
    }

    /// Creates new [`NodeLayout`] with `FlippedSandwich` kind and flexible pin heights.
    #[inline]
    pub const fn flipped_sandwich() -> Self {
        NodeLayout {
            kind: NodeLayoutKind::FlippedSandwich,
            min_pin_row_height: 0.0,
            equal_pin_row_heights: false,
        }
    }

    /// Returns new [`NodeLayout`] with same `kind` and specified pin heights.
    pub const fn with_equal_pin_rows(self) -> Self {
        NodeLayout {
            kind: self.kind,
            min_pin_row_height: self.min_pin_row_height,
            equal_pin_row_heights: true,
        }
    }

    /// Returns new [`NodeLayout`] with same `kind` and specified minimum pin row height.
    pub const fn with_min_pin_row_height(self, min_pin_row_height: f32) -> Self {
        NodeLayout {
            kind: self.kind,
            min_pin_row_height,
            equal_pin_row_heights: self.equal_pin_row_heights,
        }
    }
}

impl From<NodeLayoutKind> for NodeLayout {
    #[inline]
    fn from(kind: NodeLayoutKind) -> Self {
        NodeLayout {
            kind,
            min_pin_row_height: 0.0,
            equal_pin_row_heights: false,
        }
    }
}

impl Default for NodeLayout {
    #[inline]
    fn default() -> Self {
        NodeLayout::coil()
    }
}

#[derive(Clone, Debug)]
enum OuterHeights {
    Flexible { rows: RowHeights },
    Matching { max: f32 },
    Tight,
}

#[derive(Clone, Debug)]
struct Heights {
    rows: RowHeights,
    outer: OuterHeights,
    min_outer: f32,
}

impl Heights {
    fn get(&self, idx: usize) -> (f32, f32) {
        let inner = match self.rows.get(idx) {
            Some(&value) => value,
            None => 0.0,
        };

        let outer = match &self.outer {
            OuterHeights::Flexible { rows } => match rows.get(idx) {
                Some(&outer) => outer.max(inner),
                None => inner,
            },
            OuterHeights::Matching { max } => max.max(inner),
            OuterHeights::Tight => inner,
        };

        (inner, outer.max(self.min_outer))
    }
}

impl NodeLayout {
    fn input_heights(&self, state: &NodeState) -> Heights {
        let rows = state.input_heights().clone();

        let outer = match (self.kind, self.equal_pin_row_heights) {
            (NodeLayoutKind::Coil, false) => OuterHeights::Flexible {
                rows: state.output_heights().clone(),
            },
            (_, true) => {
                let mut max_height = 0.0f32;
                for &h in state.input_heights() {
                    max_height = max_height.max(h);
                }
                for &h in state.output_heights() {
                    max_height = max_height.max(h);
                }
                OuterHeights::Matching { max: max_height }
            }
            (_, false) => OuterHeights::Tight,
        };

        Heights {
            rows,
            outer,
            min_outer: self.min_pin_row_height,
        }
    }

    fn output_heights(&self, state: &NodeState) -> Heights {
        let rows = state.output_heights().clone();

        let outer = match (self.kind, self.equal_pin_row_heights) {
            (NodeLayoutKind::Coil, false) => OuterHeights::Flexible {
                rows: state.input_heights().clone(),
            },
            (_, true) => {
                let mut max_height = 0.0f32;
                for &h in state.input_heights() {
                    max_height = max_height.max(h);
                }
                for &h in state.output_heights() {
                    max_height = max_height.max(h);
                }
                OuterHeights::Matching { max: max_height }
            }
            (_, false) => OuterHeights::Tight,
        };

        Heights {
            rows,
            outer,
            min_outer: self.min_pin_row_height,
        }
    }
}

/// Controls style of node selection rect.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "egui-probe", derive(egui_probe::EguiProbe))]
pub struct SelectionStyle {
    /// Margin between selection rect and node frame.
    pub margin: Margin,

    /// Rounding of selection rect.
    pub rounding: CornerRadius,

    /// Fill color of selection rect.
    pub fill: Color32,

    /// Stroke of selection rect.
    pub stroke: Stroke,
}

/// Controls how pins are placed in the node.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "egui-probe", derive(egui_probe::EguiProbe))]
pub enum PinPlacement {
    /// Pins are placed inside the node frame.
    #[default]
    Inside,

    /// Pins are placed on the edge of the node frame.
    Edge,

    /// Pins are placed outside the node frame.
    Outside {
        /// Margin between node frame and pins.
        margin: f32,
    },
}

/// Style for rendering Snarl.
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "egui-probe", derive(egui_probe::EguiProbe))]
pub struct SnarlStyle {
    /// Controls how nodes are laid out.
    /// Defaults to [`NodeLayoutKind::Coil`].
    #[cfg_attr(
        feature = "serde",
        serde(skip_serializing_if = "Option::is_none", default)
    )]
    pub node_layout: Option<NodeLayout>,

    /// Frame used to draw nodes.
    /// Defaults to [`Frame::window`] constructed from current ui's style.
    #[cfg_attr(
        feature = "serde",
        serde(
            skip_serializing_if = "Option::is_none",
            default,
            with = "serde_frame_option"
        )
    )]
    pub node_frame: Option<Frame>,

    /// Frame used to draw node headers.
    /// Defaults to [`node_frame`] without shadow and transparent fill.
    ///
    /// If set, it should not have shadow and fill should be either opaque of fully transparent
    /// unless layering of header fill color with node fill color is desired.
    #[cfg_attr(
        feature = "serde",
        serde(
            skip_serializing_if = "Option::is_none",
            default,
            with = "serde_frame_option"
        )
    )]
    pub header_frame: Option<Frame>,

    /// Blank space for dragging node by its header.
    /// Elements in the header are placed after this space.
    #[cfg_attr(
        feature = "serde",
        serde(skip_serializing_if = "Option::is_none", default)
    )]
    pub header_drag_space: Option<Vec2>,

    /// Whether nodes can be collapsed.
    /// If true, headers will have collapsing button.
    /// When collapsed, node will not show its pins, body and footer.
    #[cfg_attr(
        feature = "serde",
        serde(skip_serializing_if = "Option::is_none", default)
    )]
    pub collapsible: Option<bool>,

    /// Size of pins.
    #[cfg_attr(feature = "egui-probe", egui_probe(range = 0.0..))]
    #[cfg_attr(
        feature = "serde",
        serde(skip_serializing_if = "Option::is_none", default)
    )]
    pub pin_size: Option<f32>,

    /// Default fill color for pins.
    #[cfg_attr(
        feature = "serde",
        serde(skip_serializing_if = "Option::is_none", default)
    )]
    pub pin_fill: Option<Color32>,

    /// Default stroke for pins.
    #[cfg_attr(
        feature = "serde",
        serde(skip_serializing_if = "Option::is_none", default)
    )]
    pub pin_stroke: Option<Stroke>,

    /// Shape of pins.
    #[cfg_attr(
        feature = "serde",
        serde(skip_serializing_if = "Option::is_none", default)
    )]
    pub pin_shape: Option<PinShape>,

    /// Placement of pins.
    #[cfg_attr(
        feature = "serde",
        serde(skip_serializing_if = "Option::is_none", default)
    )]
    pub pin_placement: Option<PinPlacement>,

    /// Width of wires.
    #[cfg_attr(feature = "egui-probe", egui_probe(range = 0.0..))]
    #[cfg_attr(
        feature = "serde",
        serde(skip_serializing_if = "Option::is_none", default)
    )]
    pub wire_width: Option<f32>,

    /// Size of wire frame which controls curvature of wires.
    #[cfg_attr(feature = "egui-probe", egui_probe(range = 0.0..))]
    #[cfg_attr(
        feature = "serde",
        serde(skip_serializing_if = "Option::is_none", default)
    )]
    pub wire_frame_size: Option<f32>,

    /// Whether to downscale wire frame when nodes are close.
    #[cfg_attr(
        feature = "serde",
        serde(skip_serializing_if = "Option::is_none", default)
    )]
    pub downscale_wire_frame: Option<bool>,

    /// Weather to upscale wire frame when nodes are far.
    #[cfg_attr(
        feature = "serde",
        serde(skip_serializing_if = "Option::is_none", default)
    )]
    pub upscale_wire_frame: Option<bool>,

    /// Controls default style of wires.
    #[cfg_attr(
        feature = "serde",
        serde(skip_serializing_if = "Option::is_none", default)
    )]
    pub wire_style: Option<WireStyle>,

    /// Layer where wires are rendered.
    #[cfg_attr(
        feature = "serde",
        serde(skip_serializing_if = "Option::is_none", default)
    )]
    pub wire_layer: Option<WireLayer>,

    /// Frame used to draw background
    #[cfg_attr(
        feature = "serde",
        serde(
            skip_serializing_if = "Option::is_none",
            default,
            with = "serde_frame_option"
        )
    )]
    pub bg_frame: Option<Frame>,

    /// Background pattern.
    /// Defaults to [`BackgroundPattern::Grid`].
    #[cfg_attr(
        feature = "serde",
        serde(skip_serializing_if = "Option::is_none", default)
    )]
    pub bg_pattern: Option<BackgroundPattern>,

    /// Stroke for background pattern.
    /// Defaults to `ui.visuals().widgets.noninteractive.bg_stroke`.
    #[cfg_attr(
        feature = "serde",
        serde(skip_serializing_if = "Option::is_none", default)
    )]
    pub bg_pattern_stroke: Option<Stroke>,

    /// Minimum viewport scale that can be set.
    #[cfg_attr(feature = "egui-probe", egui_probe(range = 0.0..=1.0))]
    #[cfg_attr(
        feature = "serde",
        serde(skip_serializing_if = "Option::is_none", default)
    )]
    pub min_scale: Option<f32>,

    /// Maximum viewport scale that can be set.
    #[cfg_attr(feature = "egui-probe", egui_probe(range = 1.0..))]
    #[cfg_attr(
        feature = "serde",
        serde(skip_serializing_if = "Option::is_none", default)
    )]
    pub max_scale: Option<f32>,

    /// Enable centering by double click on background
    #[cfg_attr(
        feature = "serde",
        serde(skip_serializing_if = "Option::is_none", default)
    )]
    pub centering: Option<bool>,

    /// Stroke for selection.
    #[cfg_attr(
        feature = "serde",
        serde(skip_serializing_if = "Option::is_none", default)
    )]
    pub select_stoke: Option<Stroke>,

    /// Fill for selection.
    #[cfg_attr(
        feature = "serde",
        serde(skip_serializing_if = "Option::is_none", default)
    )]
    pub select_fill: Option<Color32>,

    /// Flag to control how rect selection works.
    /// If set to true, only nodes fully contained in selection rect will be selected.
    /// If set to false, nodes intersecting with selection rect will be selected.
    pub select_rect_contained: Option<bool>,

    /// Style for node selection.
    #[cfg_attr(
        feature = "serde",
        serde(skip_serializing_if = "Option::is_none", default)
    )]
    pub select_style: Option<SelectionStyle>,

    /// Controls whether to show magnified text in crisp mode.
    /// This zooms UI style to max scale and scales down the scene.
    #[cfg_attr(
        feature = "serde",
        serde(skip_serializing_if = "Option::is_none", default)
    )]
    pub crisp_magnified_text: Option<bool>,

    /// Controls smoothness of wire curves.
    #[cfg_attr(
        feature = "serde",
        serde(skip_serializing_if = "Option::is_none", default)
    )]
    #[cfg_attr(
        feature = "egui-probe",
        egui_probe(range = 0.0f32..=10.0f32 by 0.05f32)
    )]
    pub wire_smoothness: Option<f32>,

    #[doc(hidden)]
    #[cfg_attr(feature = "egui-probe", egui_probe(skip))]
    #[cfg_attr(feature = "serde", serde(skip_serializing, default))]
    /// Do not access other than with .., here to emulate `#[non_exhaustive(pub)]`
    pub _non_exhaustive: (),
}

impl SnarlStyle {
    fn get_node_layout(&self) -> NodeLayout {
        self.node_layout.unwrap_or(NodeLayout::default())
    }

    fn get_pin_size(&self, style: &Style) -> f32 {
        self.pin_size.unwrap_or(style.spacing.interact_size.y * 0.6)
    }

    fn get_pin_fill(&self, style: &Style) -> Color32 {
        self.pin_fill
            .unwrap_or(style.visuals.widgets.active.bg_fill)
    }

    fn get_pin_stroke(&self, style: &Style) -> Stroke {
        self.pin_stroke.unwrap_or_else(|| {
            Stroke::new(
                style.visuals.widgets.active.bg_stroke.width,
                style.visuals.widgets.active.bg_stroke.color,
            )
        })
    }

    fn get_pin_shape(&self) -> PinShape {
        self.pin_shape.unwrap_or(PinShape::Circle)
    }

    fn get_pin_placement(&self) -> PinPlacement {
        self.pin_placement.unwrap_or_default()
    }

    fn get_wire_width(&self, style: &Style) -> f32 {
        self.wire_width
            .unwrap_or_else(|| self.get_pin_size(style) * 0.1)
    }

    fn get_wire_frame_size(&self, style: &Style) -> f32 {
        self.wire_frame_size
            .unwrap_or_else(|| self.get_pin_size(style) * 3.0)
    }

    fn get_downscale_wire_frame(&self) -> bool {
        self.downscale_wire_frame.unwrap_or(true)
    }

    fn get_upscale_wire_frame(&self) -> bool {
        self.upscale_wire_frame.unwrap_or(false)
    }

    fn get_wire_style(&self) -> WireStyle {
        self.wire_style.unwrap_or(WireStyle::Bezier5)
    }

    fn get_wire_layer(&self) -> WireLayer {
        self.wire_layer.unwrap_or(WireLayer::BehindNodes)
    }

    fn get_header_drag_space(&self, style: &Style) -> Vec2 {
        self.header_drag_space
            .unwrap_or_else(|| vec2(style.spacing.icon_width, style.spacing.icon_width))
    }

    fn get_collapsible(&self) -> bool {
        self.collapsible.unwrap_or(true)
    }

    fn get_bg_frame(&self, style: &Style) -> Frame {
        self.bg_frame.unwrap_or_else(|| Frame::canvas(style))
    }

    fn get_bg_pattern_stroke(&self, style: &Style) -> Stroke {
        self.bg_pattern_stroke
            .unwrap_or(style.visuals.widgets.noninteractive.bg_stroke)
    }

    fn get_min_scale(&self) -> f32 {
        self.min_scale.unwrap_or(0.2)
    }

    fn get_max_scale(&self) -> f32 {
        self.max_scale.unwrap_or(2.0)
    }

    fn get_node_frame(&self, style: &Style) -> Frame {
        self.node_frame.unwrap_or_else(|| Frame::window(style))
    }

    fn get_header_frame(&self, style: &Style) -> Frame {
        self.header_frame
            .unwrap_or_else(|| self.get_node_frame(style).shadow(Shadow::NONE))
    }

    fn get_centering(&self) -> bool {
        self.centering.unwrap_or(true)
    }

    fn get_select_stroke(&self, style: &Style) -> Stroke {
        self.select_stoke.unwrap_or_else(|| {
            Stroke::new(
                style.visuals.selection.stroke.width,
                style.visuals.selection.stroke.color.gamma_multiply(0.5),
            )
        })
    }

    fn get_select_fill(&self, style: &Style) -> Color32 {
        self.select_fill
            .unwrap_or_else(|| style.visuals.selection.bg_fill.gamma_multiply(0.3))
    }

    fn get_select_rect_contained(&self) -> bool {
        self.select_rect_contained.unwrap_or(false)
    }

    fn get_select_style(&self, style: &Style) -> SelectionStyle {
        self.select_style.unwrap_or_else(|| SelectionStyle {
            margin: style.spacing.window_margin,
            rounding: style.visuals.window_corner_radius,
            fill: self.get_select_fill(style),
            stroke: self.get_select_stroke(style),
        })
    }

    fn get_crisp_magnified_text(&self) -> bool {
        self.crisp_magnified_text.unwrap_or(false)
    }

    fn get_wire_smoothness(&self) -> f32 {
        self.wire_smoothness.unwrap_or(1.0)
    }
}

#[cfg(feature = "serde")]
mod serde_frame_option {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    #[derive(Serialize, Deserialize)]
    pub struct Frame {
        pub inner_margin: egui::Margin,
        pub outer_margin: egui::Margin,
        pub rounding: egui::CornerRadius,
        pub shadow: egui::epaint::Shadow,
        pub fill: egui::Color32,
        pub stroke: egui::Stroke,
    }

    #[allow(clippy::ref_option)]
    pub fn serialize<S>(frame: &Option<egui::Frame>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match frame {
            Some(frame) => Frame {
                inner_margin: frame.inner_margin,
                outer_margin: frame.outer_margin,
                rounding: frame.corner_radius,
                shadow: frame.shadow,
                fill: frame.fill,
                stroke: frame.stroke,
            }
            .serialize(serializer),
            None => serializer.serialize_none(),
        }
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<egui::Frame>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let frame_opt = Option::<Frame>::deserialize(deserializer)?;
        Ok(frame_opt.map(|frame| egui::Frame {
            inner_margin: frame.inner_margin,
            outer_margin: frame.outer_margin,
            corner_radius: frame.rounding,
            shadow: frame.shadow,
            fill: frame.fill,
            stroke: frame.stroke,
        }))
    }
}

impl SnarlStyle {
    /// Creates new [`SnarlStyle`] filled with default values.
    #[must_use]
    pub const fn new() -> Self {
        SnarlStyle {
            node_layout: None,
            pin_size: None,
            pin_fill: None,
            pin_stroke: None,
            pin_shape: None,
            pin_placement: None,
            wire_width: None,
            wire_frame_size: None,
            downscale_wire_frame: None,
            upscale_wire_frame: None,
            wire_style: None,
            wire_layer: None,
            header_drag_space: None,
            collapsible: None,

            bg_frame: None,
            bg_pattern: None,
            bg_pattern_stroke: None,

            min_scale: None,
            max_scale: None,
            node_frame: None,
            header_frame: None,
            centering: None,
            select_stoke: None,
            select_fill: None,
            select_rect_contained: None,
            select_style: None,
            crisp_magnified_text: None,
            wire_smoothness: None,

            _non_exhaustive: (),
        }
    }
}

impl Default for SnarlStyle {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

struct DrawNodeResponse {
    node_moved: Option<(NodeId, Vec2)>,
    node_to_top: Option<NodeId>,
    drag_released: bool,
    pin_hovered: Option<AnyPin>,
    final_rect: Rect,
}

struct DrawPinsResponse {
    drag_released: bool,
    pin_hovered: Option<AnyPin>,
    final_rect: Rect,
    new_heights: RowHeights,
}

struct DrawBodyResponse {
    final_rect: Rect,
}

struct PinResponse {
    pos: Pos2,
    wire_color: Color32,
    wire_style: WireStyle,
}

/// Widget to display [`Snarl`] graph in [`Ui`].
#[derive(Clone, Copy, Debug)]
pub struct SnarlWidget {
    id_salt: Id,
    id: Option<Id>,
    style: SnarlStyle,
    min_size: Vec2,
    max_size: Vec2,
}

impl Default for SnarlWidget {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl SnarlWidget {
    /// Returns new [`SnarlWidget`] with default parameters.
    #[inline]
    #[must_use]
    pub fn new() -> Self {
        SnarlWidget {
            id_salt: Id::new(":snarl:"),
            id: None,
            style: SnarlStyle::new(),
            min_size: Vec2::ZERO,
            max_size: Vec2::INFINITY,
        }
    }

    /// Assign an explicit and globally unique [`Id`].
    ///
    /// Use this if you want to persist the state of the widget
    /// when it changes position in the widget hierarchy.
    ///
    /// Prefer using [`SnarlWidget::id_salt`] otherwise.
    #[inline]
    #[must_use]
    pub fn id(mut self, id: Id) -> Self {
        self.id = Some(id);
        self
    }

    /// Assign a source for the unique [`Id`]
    ///
    /// It must be locally unique for the current [`Ui`] hierarchy position.
    ///
    /// Ignored if [`SnarlWidget::id`] was set.
    #[inline]
    #[must_use]
    pub fn id_salt(mut self, id_salt: impl Hash) -> Self {
        self.id_salt = Id::new(id_salt);
        self
    }

    /// Set style parameters for the [`Snarl`] widget.
    #[inline]
    #[must_use]
    pub fn style(mut self, style: SnarlStyle) -> Self {
        self.style = style;
        self
    }

    /// Set minimum size of the [`Snarl`] widget.
    #[inline]
    #[must_use]
    pub fn min_size(mut self, min_size: Vec2) -> Self {
        self.min_size = min_size;
        self
    }

    /// Set maximum size of the [`Snarl`] widget.
    #[inline]
    #[must_use]
    pub fn max_size(mut self, max_size: Vec2) -> Self {
        self.max_size = max_size;
        self
    }

    #[inline]
    fn get_id(&self, ui_id: Id) -> Id {
        self.id.unwrap_or_else(|| ui_id.with(self.id_salt))
    }

    /// Render [`Snarl`] using given viewer and style into the [`Ui`].
    #[inline]
    pub fn show<T, V>(&self, snarl: &mut Snarl<T>, viewer: &mut V, ui: &mut Ui) -> egui::Response
    where
        V: SnarlViewer<T>,
    {
        let snarl_id = self.get_id(ui.id());

        show_snarl(
            snarl_id,
            self.style,
            self.min_size,
            self.max_size,
            snarl,
            viewer,
            ui,
        )
    }
}

#[inline(never)]
fn show_snarl<T, V>(
    snarl_id: Id,
    mut style: SnarlStyle,
    min_size: Vec2,
    max_size: Vec2,
    snarl: &mut Snarl<T>,
    viewer: &mut V,
    ui: &mut Ui,
) -> egui::Response
where
    V: SnarlViewer<T>,
{
    #![allow(clippy::too_many_lines)]

    let (mut latest_pos, modifiers) = ui.ctx().input(|i| (i.pointer.latest_pos(), i.modifiers));

    let bg_frame = style.get_bg_frame(ui.style());

    let outer_size_bounds = ui.available_size_before_wrap().max(min_size).min(max_size);

    let outer_resp = ui.allocate_response(outer_size_bounds, Sense::hover());

    ui.painter().add(bg_frame.paint(outer_resp.rect));

    let mut content_rect = outer_resp.rect - bg_frame.total_margin();

    // Make sure we don't shrink to the negative:
    content_rect.max.x = content_rect.max.x.max(content_rect.min.x);
    content_rect.max.y = content_rect.max.y.max(content_rect.min.y);

    let snarl_layer_id = LayerId::new(ui.layer_id().order, snarl_id);

    ui.ctx().set_sublayer(ui.layer_id(), snarl_layer_id);

    let mut min_scale = style.get_min_scale();
    let mut max_scale = style.get_max_scale();

    let ui_rect = content_rect;

    let mut snarl_state =
        SnarlState::load(ui.ctx(), snarl_id, snarl, ui_rect, min_scale, max_scale);
    let mut to_global = snarl_state.to_global();

    let clip_rect = ui.clip_rect();

    let mut ui = ui.new_child(
        UiBuilder::new()
            .ui_stack_info(UiStackInfo::new(UiKind::Frame).with_frame(bg_frame))
            .layer_id(snarl_layer_id)
            .max_rect(Rect::EVERYTHING)
            .sense(Sense::click_and_drag()),
    );

    if style.get_crisp_magnified_text() {
        style.scale(max_scale);
        ui.style_mut().scale(max_scale);

        min_scale /= max_scale;
        max_scale = 1.0;
    }

    clamp_scale(&mut to_global, min_scale, max_scale, ui_rect);

    let mut snarl_resp = ui.response();
    Scene::new()
        .zoom_range(min_scale..=max_scale)
        .register_pan_and_zoom(&ui, &mut snarl_resp, &mut to_global);

    if snarl_resp.changed() {
        ui.ctx().request_repaint();
    }

    // Inform viewer about current transform.
    viewer.current_transform(&mut to_global, snarl);

    snarl_state.set_to_global(to_global);

    let to_global = to_global;
    let from_global = to_global.inverse();

    // Graph viewport
    let viewport = (from_global * ui_rect).round_ui();
    let viewport_clip = from_global * clip_rect;

    ui.set_clip_rect(viewport.intersect(viewport_clip));
    ui.expand_to_include_rect(viewport);

    // Set transform for snarl layer.
    ui.ctx().set_transform_layer(snarl_layer_id, to_global);

    // Map latest pointer position to graph space.
    latest_pos = latest_pos.map(|pos| from_global * pos);

    viewer.draw_background(
        style.bg_pattern.as_ref(),
        &viewport,
        &style,
        ui.style(),
        ui.painter(),
        snarl,
    );

    let mut node_moved = None;
    let mut node_to_top = None;

    // Process selection rect.
    let mut rect_selection_ended = None;
    if modifiers.shift || snarl_state.is_rect_selection() {
        let select_resp = ui.interact(snarl_resp.rect, snarl_id.with("select"), Sense::drag());

        if select_resp.dragged_by(PointerButton::Primary) {
            if let Some(pos) = select_resp.interact_pointer_pos() {
                if snarl_state.is_rect_selection() {
                    snarl_state.update_rect_selection(pos);
                } else {
                    snarl_state.start_rect_selection(pos);
                }
            }
        }

        if select_resp.drag_stopped_by(PointerButton::Primary) {
            if let Some(select_rect) = snarl_state.rect_selection() {
                rect_selection_ended = Some(select_rect);
            }
            snarl_state.stop_rect_selection();
        }
    }

    let wire_frame_size = style.get_wire_frame_size(ui.style());
    let wire_width = style.get_wire_width(ui.style());
    let wire_threshold = style.get_wire_smoothness();

    let wire_shape_idx = match style.get_wire_layer() {
        WireLayer::BehindNodes => Some(ui.painter().add(Shape::Noop)),
        WireLayer::AboveNodes => None,
    };

    let mut input_info = HashMap::new();
    let mut output_info = HashMap::new();

    let mut pin_hovered = None;

    let draw_order = snarl_state.update_draw_order(snarl);
    let mut drag_released = false;

    let mut nodes_bb = Rect::NOTHING;
    let mut node_rects = Vec::new();

    for node_idx in draw_order {
        if !snarl.nodes.contains(node_idx.0) {
            continue;
        }

        // show_node(node_idx);
        let response = draw_node(
            snarl,
            &mut ui,
            node_idx,
            viewer,
            &mut snarl_state,
            &style,
            snarl_id,
            &mut input_info,
            modifiers,
            &mut output_info,
        );

        if let Some(response) = response {
            if let Some(v) = response.node_to_top {
                node_to_top = Some(v);
            }
            if let Some(v) = response.node_moved {
                node_moved = Some(v);
            }
            if let Some(v) = response.pin_hovered {
                pin_hovered = Some(v);
            }
            drag_released |= response.drag_released;

            nodes_bb = nodes_bb.union(response.final_rect);
            if rect_selection_ended.is_some() {
                node_rects.push((node_idx, response.final_rect));
            }
        }
    }

    let mut hovered_wire = None;
    let mut hovered_wire_disconnect = false;
    let mut wire_shapes = Vec::new();

    // Draw and interact with wires
    for wire in snarl.wires.iter() {
        let Some(from_r) = output_info.get(&wire.out_pin) else {
            continue;
        };
        let Some(to_r) = input_info.get(&wire.in_pin) else {
            continue;
        };

        if !snarl_state.has_new_wires() && snarl_resp.contains_pointer() && hovered_wire.is_none() {
            // Try to find hovered wire
            // If not dragging new wire
            // And not hovering over item above.

            if let Some(latest_pos) = latest_pos {
                let wire_hit = hit_wire(
                    ui.ctx(),
                    snarl_id,
                    wire,
                    wire_frame_size,
                    style.get_upscale_wire_frame(),
                    style.get_downscale_wire_frame(),
                    from_r.pos,
                    to_r.pos,
                    latest_pos,
                    wire_threshold,
                    wire_width.max(2.0),
                    pick_wire_style(from_r.wire_style, to_r.wire_style),
                );

                if wire_hit {
                    hovered_wire = Some(wire);

                    let wire_r =
                        ui.interact(snarl_resp.rect, ui.make_persistent_id(wire), Sense::click());

                    //Remove hovered wire by second click
                    hovered_wire_disconnect |= wire_r.clicked_by(PointerButton::Secondary) | wire_r.clicked_by(PointerButton::Primary);
                }
            }
        }

        let color = mix_colors(from_r.wire_color, to_r.wire_color);

        let mut draw_width = wire_width;
        if hovered_wire == Some(wire) {
            draw_width *= 1.5;
        }

        draw_wire(
            &ui,
            snarl_id,
            Some(wire),
            &mut wire_shapes,
            wire_frame_size,
            style.get_upscale_wire_frame(),
            style.get_downscale_wire_frame(),
            from_r.pos,
            to_r.pos,
            Stroke::new(draw_width, color),
            wire_threshold,
            pick_wire_style(from_r.wire_style, to_r.wire_style),
        );
    }

    // Remove hovered wire by second click
    if hovered_wire_disconnect {
        if let Some(wire) = hovered_wire {
            let out_pin = OutPin::new(snarl, wire.out_pin);
            let in_pin = InPin::new(snarl, wire.in_pin);
            viewer.disconnect(&out_pin, &in_pin, snarl);
        }
    }

    if let Some(select_rect) = rect_selection_ended {
        let select_nodes = node_rects.into_iter().filter_map(|(id, rect)| {
            let select = if style.get_select_rect_contained() {
                select_rect.contains_rect(rect)
            } else {
                select_rect.intersects(rect)
            };

            if select { Some(id) } else { None }
        });

        if modifiers.command {
            snarl_state.deselect_many_nodes(select_nodes);
        } else {
            snarl_state.select_many_nodes(!modifiers.shift, select_nodes);
        }
    }

    if let Some(select_rect) = snarl_state.rect_selection() {
        ui.painter().rect(
            select_rect,
            0.0,
            style.get_select_fill(ui.style()),
            style.get_select_stroke(ui.style()),
            StrokeKind::Inside,
        );
    }

    // If right button is clicked while new wire is being dragged, cancel it.
    // This is to provide way to 'not open' the link graph node menu, but just
    // releasing the new wire to empty space.
    //
    // This uses `button_down` directly, instead of `clicked_by` to improve
    // responsiveness of the cancel action.
    if snarl_state.has_new_wires() && ui.input(|x| x.pointer.button_down(PointerButton::Secondary))
    {
        let _ = snarl_state.take_new_wires();
        snarl_resp.flags.remove(Flags::CLICKED);
    }

    // Do centering unless no nodes are present.
    if style.get_centering() && snarl_resp.double_clicked() && nodes_bb.is_finite() {
        let nodes_bb = nodes_bb.expand(100.0);
        snarl_state.look_at(nodes_bb, ui_rect, min_scale, max_scale);
    }

    if modifiers.command && snarl_resp.clicked_by(PointerButton::Primary) {
        snarl_state.deselect_all_nodes();
    }

    // Wire end position will be overridden when link graph menu is opened.
    let mut wire_end_pos = latest_pos.unwrap_or(snarl_resp.rect.center());

    if drag_released {
        let new_wires = snarl_state.take_new_wires();
        if new_wires.is_some() {
            ui.ctx().request_repaint();
        }
        match (new_wires, pin_hovered) {
            (Some(NewWires::In(in_pins)), Some(AnyPin::Out(out_pin))) => {
                for in_pin in in_pins {
                    viewer.connect(
                        &OutPin::new(snarl, out_pin),
                        &InPin::new(snarl, in_pin),
                        snarl,
                    );
                }
            }
            (Some(NewWires::Out(out_pins)), Some(AnyPin::In(in_pin))) => {
                for out_pin in out_pins {
                    viewer.connect(
                        &OutPin::new(snarl, out_pin),
                        &InPin::new(snarl, in_pin),
                        snarl,
                    );
                }
            }
            (Some(new_wires), None) if snarl_resp.hovered() => {
                let pins = match &new_wires {
                    NewWires::In(x) => AnyPins::In(x),
                    NewWires::Out(x) => AnyPins::Out(x),
                };

                if viewer.has_dropped_wire_menu(pins, snarl) {
                    // A wire is dropped without connecting to a pin.
                    // Show context menu for the wire drop.
                    snarl_state.set_new_wires_menu(new_wires);

                    // Force open context menu.
                    snarl_resp.flags.insert(Flags::LONG_TOUCHED);
                }
            }
            _ => {}
        }
    }

    if let Some(interact_pos) = ui.ctx().input(|i| i.pointer.interact_pos()) {
        if let Some(new_wires) = snarl_state.take_new_wires_menu() {
            let pins = match &new_wires {
                NewWires::In(x) => AnyPins::In(x),
                NewWires::Out(x) => AnyPins::Out(x),
            };

            if viewer.has_dropped_wire_menu(pins, snarl) {
                snarl_resp.context_menu(|ui| {
                    let pins = match &new_wires {
                        NewWires::In(x) => AnyPins::In(x),
                        NewWires::Out(x) => AnyPins::Out(x),
                    };

                    let menu_pos = from_global * ui.cursor().min;

                    // Override wire end position when the wire-drop context menu is opened.
                    wire_end_pos = menu_pos;

                    // The context menu is opened as *link* graph menu.
                    viewer.show_dropped_wire_menu(menu_pos, ui, pins, snarl);

                    // Even though menu could be closed in `show_dropped_wire_menu`,
                    // we need to revert the new wires here, because menu state is inaccessible.
                    // Next frame context menu won't be shown and wires will be removed.
                    snarl_state.set_new_wires_menu(new_wires);
                });
            }
        } else if viewer.has_graph_menu(interact_pos, snarl) {
            snarl_resp.context_menu(|ui| {
                let menu_pos = from_global * ui.cursor().min;

                viewer.show_graph_menu(menu_pos, ui, snarl);
            });
        }
    }

    match snarl_state.new_wires() {
        None => {}
        Some(NewWires::In(pins)) => {
            for pin in pins {
                let from_pos = wire_end_pos;
                let to_r = &input_info[pin];

                draw_wire(
                    &ui,
                    snarl_id,
                    None,
                    &mut wire_shapes,
                    wire_frame_size,
                    style.get_upscale_wire_frame(),
                    style.get_downscale_wire_frame(),
                    from_pos,
                    to_r.pos,
                    Stroke::new(wire_width, to_r.wire_color),
                    wire_threshold,
                    to_r.wire_style,
                );
            }
        }
        Some(NewWires::Out(pins)) => {
            for pin in pins {
                let from_r = &output_info[pin];
                let to_pos = wire_end_pos;

                draw_wire(
                    &ui,
                    snarl_id,
                    None,
                    &mut wire_shapes,
                    wire_frame_size,
                    style.get_upscale_wire_frame(),
                    style.get_downscale_wire_frame(),
                    from_r.pos,
                    to_pos,
                    Stroke::new(wire_width, from_r.wire_color),
                    wire_threshold,
                    from_r.wire_style,
                );
            }
        }
    }

    match wire_shape_idx {
        None => {
            ui.painter().add(Shape::Vec(wire_shapes));
        }
        Some(idx) => {
            ui.painter().set(idx, Shape::Vec(wire_shapes));
        }
    }

    ui.advance_cursor_after_rect(Rect::from_min_size(snarl_resp.rect.min, Vec2::ZERO));

    if let Some(node) = node_to_top {
        if snarl.nodes.contains(node.0) {
            snarl_state.node_to_top(node);
        }
    }

    if let Some((node, delta)) = node_moved {
        if snarl.nodes.contains(node.0) {
            ui.ctx().request_repaint();
            if snarl_state.selected_nodes().contains(&node) {
                for node in snarl_state.selected_nodes() {
                    let node = &mut snarl.nodes[node.0];
                    node.pos += delta;
                }
            } else {
                let node = &mut snarl.nodes[node.0];
                node.pos += delta;
            }
        }
    }

    snarl_state.store(snarl, ui.ctx());

    snarl_resp
}

#[allow(clippy::too_many_arguments)]
#[inline(never)]
fn draw_inputs<T, V>(
    snarl: &mut Snarl<T>,
    viewer: &mut V,
    node: NodeId,
    inputs: &[InPin],
    pin_size: f32,
    style: &SnarlStyle,
    node_ui: &mut Ui,
    inputs_rect: Rect,
    payload_clip_rect: Rect,
    input_x: f32,
    min_pin_y_top: f32,
    min_pin_y_bottom: f32,
    input_spacing: Option<f32>,
    snarl_state: &mut SnarlState,
    modifiers: Modifiers,
    input_positions: &mut HashMap<InPinId, PinResponse>,
    heights: Heights,
) -> DrawPinsResponse
where
    V: SnarlViewer<T>,
{
    let mut drag_released = false;
    let mut pin_hovered = None;

    // Input pins on the left.
    let mut inputs_ui = node_ui.new_child(
        UiBuilder::new()
            .max_rect(inputs_rect.round_ui())
            .layout(Layout::top_down(Align::Min))
            .id_salt("inputs"),
    );

    let snarl_clip_rect = node_ui.clip_rect();
    inputs_ui.shrink_clip_rect(payload_clip_rect);

    let pin_layout = Layout::left_to_right(Align::Min);
    let mut new_heights = SmallVec::with_capacity(inputs.len());

    for in_pin in inputs {
        // Show input pin.
        let cursor = inputs_ui.cursor();
        let (height, height_outer) = heights.get(in_pin.id.input);

        let margin = (height_outer - height) / 2.0;
        let outer_rect = cursor.with_max_y(cursor.top() + height_outer);
        let inner_rect = outer_rect.shrink2(vec2(0.0, margin));

        let builder = UiBuilder::new().layout(pin_layout).max_rect(inner_rect);

        inputs_ui.allocate_new_ui(builder, |pin_ui| {
            if let Some(input_spacing) = input_spacing {
                let min = pin_ui.next_widget_position();
                pin_ui.advance_cursor_after_rect(Rect::from_min_size(
                    min,
                    vec2(input_spacing, pin_size),
                ));
            }

            let y0 = pin_ui.max_rect().min.y;
            let y1 = pin_ui.max_rect().max.y;

            // Show input content
            let snarl_pin = viewer.show_input(in_pin, pin_ui, snarl);
            if !snarl.nodes.contains(node.0) {
                // If removed
                return;
            }

            let pin_rect = snarl_pin.pin_rect(
                input_x,
                min_pin_y_top.max(y0),
                min_pin_y_bottom.max(y1),
                pin_size,
            );

            // Interact with pin shape.
            pin_ui.set_clip_rect(snarl_clip_rect);

            let r = pin_ui.interact(pin_rect, pin_ui.next_auto_id(), Sense::click_and_drag());

            pin_ui.skip_ahead_auto_ids(1);

            if r.clicked_by(PointerButton::Secondary) {
                if snarl_state.has_new_wires() {
                    snarl_state.remove_new_wire_in(in_pin.id);
                } else {
                    viewer.drop_inputs(in_pin, snarl);
                    if !snarl.nodes.contains(node.0) {
                        // If removed
                        return;
                    }
                }
            }
            if r.drag_started_by(PointerButton::Primary) {
                if modifiers.command {
                    snarl_state.start_new_wires_out(&in_pin.remotes);
                    if !modifiers.shift {
                        snarl.drop_inputs(in_pin.id);
                        if !snarl.nodes.contains(node.0) {
                            // If removed
                            return;
                        }
                    }
                } else {
                    snarl_state.start_new_wire_in(in_pin.id);
                }
            }

            if r.drag_stopped() {
                drag_released = true;
            }

            let mut visual_pin_rect = r.rect;

            if r.contains_pointer() {
                if snarl_state.has_new_wires_in() {
                    if modifiers.shift && !modifiers.command {
                        snarl_state.add_new_wire_in(in_pin.id);
                    }
                    if !modifiers.shift && modifiers.command {
                        snarl_state.remove_new_wire_in(in_pin.id);
                    }
                }
                pin_hovered = Some(AnyPin::In(in_pin.id));
                visual_pin_rect = visual_pin_rect.scale_from_center(1.2);
            }

            let wire_info =
                snarl_pin.draw(style, pin_ui.style(), visual_pin_rect, pin_ui.painter());

            input_positions.insert(
                in_pin.id,
                PinResponse {
                    pos: r.rect.center(),
                    wire_color: wire_info.color,
                    wire_style: wire_info.style,
                },
            );

            new_heights.push(pin_ui.min_rect().height());

            pin_ui.expand_to_include_y(outer_rect.bottom());
        });
    }

    let final_rect = inputs_ui.min_rect();
    node_ui.expand_to_include_rect(final_rect.intersect(payload_clip_rect));

    DrawPinsResponse {
        drag_released,
        pin_hovered,
        final_rect,
        new_heights,
    }
}

#[allow(clippy::too_many_arguments)]
fn draw_outputs<T, V>(
    snarl: &mut Snarl<T>,
    viewer: &mut V,
    node: NodeId,
    outputs: &[OutPin],
    pin_size: f32,
    style: &SnarlStyle,
    node_ui: &mut Ui,
    outputs_rect: Rect,
    payload_clip_rect: Rect,
    output_x: f32,
    min_pin_y_top: f32,
    min_pin_y_bottom: f32,
    output_spacing: Option<f32>,
    snarl_state: &mut SnarlState,
    modifiers: Modifiers,
    output_positions: &mut HashMap<OutPinId, PinResponse>,
    heights: Heights,
) -> DrawPinsResponse
where
    V: SnarlViewer<T>,
{
    let mut drag_released = false;
    let mut pin_hovered = None;

    let mut outputs_ui = node_ui.new_child(
        UiBuilder::new()
            .max_rect(outputs_rect.round_ui())
            .layout(Layout::top_down(Align::Max))
            .id_salt("outputs"),
    );

    let snarl_clip_rect = node_ui.clip_rect();
    outputs_ui.shrink_clip_rect(payload_clip_rect);

    let pin_layout = Layout::right_to_left(Align::Min);
    let mut new_heights = SmallVec::with_capacity(outputs.len());

    // Output pins on the right.
    for out_pin in outputs {
        // Show output pin.
        let cursor = outputs_ui.cursor();
        let (height, height_outer) = heights.get(out_pin.id.output);

        let margin = (height_outer - height) / 2.0;
        let outer_rect = cursor.with_max_y(cursor.top() + height_outer);
        let inner_rect = outer_rect.shrink2(vec2(0.0, margin));

        let builder = UiBuilder::new().layout(pin_layout).max_rect(inner_rect);

        outputs_ui.allocate_new_ui(builder, |pin_ui| {
            // Allocate space for pin shape.
            if let Some(output_spacing) = output_spacing {
                let min = pin_ui.next_widget_position();
                pin_ui.advance_cursor_after_rect(Rect::from_min_size(
                    min,
                    vec2(output_spacing, pin_size),
                ));
            }

            let y0 = pin_ui.max_rect().min.y;
            let y1 = pin_ui.max_rect().max.y;

            // Show output content
            let snarl_pin = viewer.show_output(out_pin, pin_ui, snarl);
            if !snarl.nodes.contains(node.0) {
                // If removed
                return;
            }

            let pin_rect = snarl_pin.pin_rect(
                output_x,
                min_pin_y_top.max(y0),
                min_pin_y_bottom.max(y1),
                pin_size,
            );

            pin_ui.set_clip_rect(snarl_clip_rect);

            let r = pin_ui.interact(pin_rect, pin_ui.next_auto_id(), Sense::click_and_drag());

            pin_ui.skip_ahead_auto_ids(1);

            if r.clicked_by(PointerButton::Secondary) {
                if snarl_state.has_new_wires() {
                    snarl_state.remove_new_wire_out(out_pin.id);
                } else {
                    viewer.drop_outputs(out_pin, snarl);
                    if !snarl.nodes.contains(node.0) {
                        // If removed
                        return;
                    }
                }
            }
            if r.drag_started_by(PointerButton::Primary) {
                if modifiers.command {
                    snarl_state.start_new_wires_in(&out_pin.remotes);

                    if !modifiers.shift {
                        snarl.drop_outputs(out_pin.id);
                        if !snarl.nodes.contains(node.0) {
                            // If removed
                            return;
                        }
                    }
                } else {
                    snarl_state.start_new_wire_out(out_pin.id);
                }
            }

            if r.drag_stopped() {
                drag_released = true;
            }

            let mut visual_pin_rect = r.rect;

            if r.contains_pointer() {
                if snarl_state.has_new_wires_out() {
                    if modifiers.shift && !modifiers.command {
                        snarl_state.add_new_wire_out(out_pin.id);
                    }
                    if !modifiers.shift && modifiers.command {
                        snarl_state.remove_new_wire_out(out_pin.id);
                    }
                }
                pin_hovered = Some(AnyPin::Out(out_pin.id));
                visual_pin_rect = visual_pin_rect.scale_from_center(1.2);
            }

            let wire_info =
                snarl_pin.draw(style, pin_ui.style(), visual_pin_rect, pin_ui.painter());

            output_positions.insert(
                out_pin.id,
                PinResponse {
                    pos: r.rect.center(),
                    wire_color: wire_info.color,
                    wire_style: wire_info.style,
                },
            );

            new_heights.push(pin_ui.min_rect().height());

            pin_ui.expand_to_include_y(outer_rect.bottom());
        });
    }
    let final_rect = outputs_ui.min_rect();
    node_ui.expand_to_include_rect(final_rect.intersect(payload_clip_rect));

    DrawPinsResponse {
        drag_released,
        pin_hovered,
        final_rect,
        new_heights,
    }
}

#[allow(clippy::too_many_arguments)]
fn draw_body<T, V>(
    snarl: &mut Snarl<T>,
    viewer: &mut V,
    node: NodeId,
    inputs: &[InPin],
    outputs: &[OutPin],
    ui: &mut Ui,
    body_rect: Rect,
    payload_clip_rect: Rect,
    _snarl_state: &SnarlState,
) -> DrawBodyResponse
where
    V: SnarlViewer<T>,
{
    let mut body_ui = ui.new_child(
        UiBuilder::new()
            .max_rect(body_rect.round_ui())
            .layout(Layout::left_to_right(Align::Min))
            .id_salt("body"),
    );

    body_ui.shrink_clip_rect(payload_clip_rect);

    viewer.show_body(node, inputs, outputs, &mut body_ui, snarl);

    let final_rect = body_ui.min_rect();
    ui.expand_to_include_rect(final_rect.intersect(payload_clip_rect));
    // node_state.set_body_width(body_size.x);

    DrawBodyResponse { final_rect }
}

//First step for split big function to parts
/// Draw one node. Return Pins info
#[inline]
#[allow(clippy::too_many_lines)]
#[allow(clippy::too_many_arguments)]
fn draw_node<T, V>(
    snarl: &mut Snarl<T>,
    ui: &mut Ui,
    node: NodeId,
    viewer: &mut V,
    snarl_state: &mut SnarlState,
    style: &SnarlStyle,
    snarl_id: Id,
    input_positions: &mut HashMap<InPinId, PinResponse>,
    modifiers: Modifiers,
    output_positions: &mut HashMap<OutPinId, PinResponse>,
) -> Option<DrawNodeResponse>
where
    V: SnarlViewer<T>,
{
    let Node {
        pos,
        open,
        ref value,
    } = snarl.nodes[node.0];

    // Collect pins
    let inputs_count = viewer.inputs(value);
    let outputs_count = viewer.outputs(value);

    let inputs = (0..inputs_count)
        .map(|idx| InPin::new(snarl, InPinId { node, input: idx }))
        .collect::<Vec<_>>();

    let outputs = (0..outputs_count)
        .map(|idx| OutPin::new(snarl, OutPinId { node, output: idx }))
        .collect::<Vec<_>>();

    let node_pos = pos.round_ui();

    // Generate persistent id for the node.
    let node_id = snarl_id.with(("snarl-node", node));

    let openness = ui.ctx().animate_bool(node_id, open);

    let mut node_state = NodeState::load(ui.ctx(), node_id, ui.spacing());

    let node_rect = node_state.node_rect(node_pos, openness);

    let mut node_to_top = None;
    let mut node_moved = None;
    let mut drag_released = false;
    let mut pin_hovered = None;

    let node_frame = viewer.node_frame(
        style.get_node_frame(ui.style()),
        node,
        &inputs,
        &outputs,
        snarl,
    );

    let header_frame = viewer.header_frame(
        style.get_header_frame(ui.style()),
        node,
        &inputs,
        &outputs,
        snarl,
    );

    // Rect for node + frame margin.
    let node_frame_rect = node_rect + node_frame.total_margin();

    if snarl_state.selected_nodes().contains(&node) {
        let select_style = style.get_select_style(ui.style());

        let select_rect = node_frame_rect + select_style.margin;

        ui.painter().rect(
            select_rect,
            select_style.rounding,
            select_style.fill,
            select_style.stroke,
            StrokeKind::Inside,
        );
    }

    // Size of the pin.
    // Side of the square or diameter of the circle.
    let pin_size = style.get_pin_size(ui.style()).max(0.0);

    let pin_placement = style.get_pin_placement();

    let header_drag_space = style.get_header_drag_space(ui.style()).max(Vec2::ZERO);

    // Interact with node frame.
    let r = ui.interact(
        node_frame_rect,
        node_id.with("frame"),
        Sense::click_and_drag(),
    );

    if !modifiers.shift && !modifiers.command && r.dragged_by(PointerButton::Primary) {
        node_moved = Some((node, r.drag_delta()));
    }

    if r.clicked_by(PointerButton::Primary) || r.dragged_by(PointerButton::Primary) {
        if modifiers.shift {
            snarl_state.select_one_node(modifiers.command, node);
        } else if modifiers.command {
            snarl_state.deselect_one_node(node);
        }
    }

    if r.clicked() || r.dragged() {
        node_to_top = Some(node);
    }

    if viewer.has_node_menu(&snarl.nodes[node.0].value) {
        r.context_menu(|ui| {
            viewer.show_node_menu(node, &inputs, &outputs, ui, snarl);
        });
    }

    if !snarl.nodes.contains(node.0) {
        node_state.clear(ui.ctx());
        // If removed
        return None;
    }

    if viewer.has_on_hover_popup(&snarl.nodes[node.0].value) {
        r.on_hover_ui_at_pointer(|ui| {
            viewer.show_on_hover_popup(node, &inputs, &outputs, ui, snarl);
        });
    }

    if !snarl.nodes.contains(node.0) {
        node_state.clear(ui.ctx());
        // If removed
        return None;
    }

    let node_ui = &mut ui.new_child(
        UiBuilder::new()
            .max_rect(node_frame_rect.round_ui())
            .layout(Layout::top_down(Align::Center))
            .id_salt(node_id),
    );

    let mut new_pins_size = Vec2::ZERO;

    let r = node_frame.show(node_ui, |ui| {
        if viewer.has_node_style(node, &inputs, &outputs, snarl) {
            viewer.apply_node_style(ui.style_mut(), node, &inputs, &outputs, snarl);
        }

        // Input pins' center side by X axis.
        let input_x = match pin_placement {
            PinPlacement::Inside => pin_size.mul_add(
                0.5,
                node_frame_rect.left() + node_frame.inner_margin.leftf(),
            ),
            PinPlacement::Edge => node_frame_rect.left(),
            PinPlacement::Outside { margin } => {
                pin_size.mul_add(-0.5, node_frame_rect.left() - margin)
            }
        };

        // Input pins' spacing required.
        let input_spacing = match pin_placement {
            PinPlacement::Inside => Some(pin_size),
            PinPlacement::Edge => Some(
                pin_size
                    .mul_add(0.5, -node_frame.inner_margin.leftf())
                    .max(0.0),
            ),
            PinPlacement::Outside { .. } => None,
        };

        // Output pins' center side by X axis.
        let output_x = match pin_placement {
            PinPlacement::Inside => pin_size.mul_add(
                -0.5,
                node_frame_rect.right() - node_frame.inner_margin.rightf(),
            ),
            PinPlacement::Edge => node_frame_rect.right(),
            PinPlacement::Outside { margin } => {
                pin_size.mul_add(0.5, node_frame_rect.right() + margin)
            }
        };

        // Output pins' spacing required.
        let output_spacing = match pin_placement {
            PinPlacement::Inside => Some(pin_size),
            PinPlacement::Edge => Some(
                pin_size
                    .mul_add(0.5, -node_frame.inner_margin.rightf())
                    .max(0.0),
            ),
            PinPlacement::Outside { .. } => None,
        };

        // Input/output pin block

        if (openness < 1.0 && open) || (openness > 0.0 && !open) {
            ui.ctx().request_repaint();
        }

        // Pins are placed under the header and must not go outside of the header frame.
        let payload_rect = Rect::from_min_max(
            pos2(
                node_rect.min.x,
                node_rect.min.y
                    + node_state.header_height()
                    + header_frame.total_margin().bottom
                    + ui.spacing().item_spacing.y
                    - node_state.payload_offset(openness),
            ),
            node_rect.max,
        );

        let node_layout =
            viewer.node_layout(style.get_node_layout(), node, &inputs, &outputs, snarl);

        let payload_clip_rect =
            Rect::from_min_max(node_rect.min, pos2(node_rect.max.x, f32::INFINITY));

        let pins_rect = match node_layout.kind {
            NodeLayoutKind::Coil => {
                // Show input pins.
                let r = draw_inputs(
                    snarl,
                    viewer,
                    node,
                    &inputs,
                    pin_size,
                    style,
                    ui,
                    payload_rect,
                    payload_clip_rect,
                    input_x,
                    node_rect.min.y,
                    node_rect.min.y + node_state.header_height(),
                    input_spacing,
                    snarl_state,
                    modifiers,
                    input_positions,
                    node_layout.input_heights(&node_state),
                );

                let new_input_heights = r.new_heights;

                drag_released |= r.drag_released;

                if r.pin_hovered.is_some() {
                    pin_hovered = r.pin_hovered;
                }

                let inputs_rect = r.final_rect;
                let inputs_size = inputs_rect.size();

                if !snarl.nodes.contains(node.0) {
                    // If removed
                    return;
                }

                // Show output pins.

                let r = draw_outputs(
                    snarl,
                    viewer,
                    node,
                    &outputs,
                    pin_size,
                    style,
                    ui,
                    payload_rect,
                    payload_clip_rect,
                    output_x,
                    node_rect.min.y,
                    node_rect.min.y + node_state.header_height(),
                    output_spacing,
                    snarl_state,
                    modifiers,
                    output_positions,
                    node_layout.output_heights(&node_state),
                );

                let new_output_heights = r.new_heights;

                drag_released |= r.drag_released;

                if r.pin_hovered.is_some() {
                    pin_hovered = r.pin_hovered;
                }

                let outputs_rect = r.final_rect;
                let outputs_size = outputs_rect.size();

                if !snarl.nodes.contains(node.0) {
                    // If removed
                    return;
                }

                node_state.set_input_heights(new_input_heights);
                node_state.set_output_heights(new_output_heights);

                new_pins_size = vec2(
                    inputs_size.x + outputs_size.x + ui.spacing().item_spacing.x,
                    f32::max(inputs_size.y, outputs_size.y),
                );

                let mut pins_rect = inputs_rect.union(outputs_rect);

                // Show body if there's one.
                if viewer.has_body(&snarl.nodes.get(node.0).unwrap().value) {
                    let body_rect = Rect::from_min_max(
                        pos2(
                            inputs_rect.right() + ui.spacing().item_spacing.x,
                            payload_rect.top(),
                        ),
                        pos2(
                            outputs_rect.left() - ui.spacing().item_spacing.x,
                            payload_rect.bottom(),
                        ),
                    );

                    let r = draw_body(
                        snarl,
                        viewer,
                        node,
                        &inputs,
                        &outputs,
                        ui,
                        body_rect,
                        payload_clip_rect,
                        snarl_state,
                    );

                    new_pins_size.x += r.final_rect.width() + ui.spacing().item_spacing.x;
                    new_pins_size.y = f32::max(new_pins_size.y, r.final_rect.height());

                    pins_rect = pins_rect.union(body_rect);

                    if !snarl.nodes.contains(node.0) {
                        // If removed
                        return;
                    }
                }

                pins_rect
            }
            NodeLayoutKind::Sandwich => {
                // Show input pins.

                let r = draw_inputs(
                    snarl,
                    viewer,
                    node,
                    &inputs,
                    pin_size,
                    style,
                    ui,
                    payload_rect,
                    payload_clip_rect,
                    input_x,
                    node_rect.min.y,
                    node_rect.min.y + node_state.header_height(),
                    input_spacing,
                    snarl_state,
                    modifiers,
                    input_positions,
                    node_layout.input_heights(&node_state),
                );

                let new_input_heights = r.new_heights;

                drag_released |= r.drag_released;

                if r.pin_hovered.is_some() {
                    pin_hovered = r.pin_hovered;
                }

                let inputs_rect = r.final_rect;

                new_pins_size = inputs_rect.size();

                let mut next_y = inputs_rect.bottom() + ui.spacing().item_spacing.y;

                if !snarl.nodes.contains(node.0) {
                    // If removed
                    return;
                }

                let mut pins_rect = inputs_rect;

                // Show body if there's one.
                if viewer.has_body(&snarl.nodes.get(node.0).unwrap().value) {
                    let body_rect = payload_rect.intersect(Rect::everything_below(next_y));

                    let r = draw_body(
                        snarl,
                        viewer,
                        node,
                        &inputs,
                        &outputs,
                        ui,
                        body_rect,
                        payload_clip_rect,
                        snarl_state,
                    );

                    let body_rect = r.final_rect;

                    new_pins_size.x = f32::max(new_pins_size.x, body_rect.width());
                    new_pins_size.y += body_rect.height() + ui.spacing().item_spacing.y;

                    if !snarl.nodes.contains(node.0) {
                        // If removed
                        return;
                    }

                    pins_rect = pins_rect.union(body_rect);
                    next_y = body_rect.bottom() + ui.spacing().item_spacing.y;
                }

                // Show output pins.

                let outputs_rect = payload_rect.intersect(Rect::everything_below(next_y));

                let r = draw_outputs(
                    snarl,
                    viewer,
                    node,
                    &outputs,
                    pin_size,
                    style,
                    ui,
                    outputs_rect,
                    payload_clip_rect,
                    output_x,
                    node_rect.min.y,
                    node_rect.min.y + node_state.header_height(),
                    output_spacing,
                    snarl_state,
                    modifiers,
                    output_positions,
                    node_layout.output_heights(&node_state),
                );

                let new_output_heights = r.new_heights;

                drag_released |= r.drag_released;

                if r.pin_hovered.is_some() {
                    pin_hovered = r.pin_hovered;
                }

                let outputs_rect = r.final_rect;

                if !snarl.nodes.contains(node.0) {
                    // If removed
                    return;
                }

                node_state.set_input_heights(new_input_heights);
                node_state.set_output_heights(new_output_heights);

                new_pins_size.x = f32::max(new_pins_size.x, outputs_rect.width());
                new_pins_size.y += outputs_rect.height() + ui.spacing().item_spacing.y;

                pins_rect = pins_rect.union(outputs_rect);

                pins_rect
            }
            NodeLayoutKind::FlippedSandwich => {
                // Show input pins.

                let outputs_rect = payload_rect;
                let r = draw_outputs(
                    snarl,
                    viewer,
                    node,
                    &outputs,
                    pin_size,
                    style,
                    ui,
                    outputs_rect,
                    payload_clip_rect,
                    output_x,
                    node_rect.min.y,
                    node_rect.min.y + node_state.header_height(),
                    output_spacing,
                    snarl_state,
                    modifiers,
                    output_positions,
                    node_layout.output_heights(&node_state),
                );

                let new_output_heights = r.new_heights;

                drag_released |= r.drag_released;

                if r.pin_hovered.is_some() {
                    pin_hovered = r.pin_hovered;
                }

                let outputs_rect = r.final_rect;

                new_pins_size = outputs_rect.size();

                let mut next_y = outputs_rect.bottom() + ui.spacing().item_spacing.y;

                if !snarl.nodes.contains(node.0) {
                    // If removed
                    return;
                }

                let mut pins_rect = outputs_rect;

                // Show body if there's one.
                if viewer.has_body(&snarl.nodes.get(node.0).unwrap().value) {
                    let body_rect = payload_rect.intersect(Rect::everything_below(next_y));

                    let r = draw_body(
                        snarl,
                        viewer,
                        node,
                        &inputs,
                        &outputs,
                        ui,
                        body_rect,
                        payload_clip_rect,
                        snarl_state,
                    );

                    let body_rect = r.final_rect;

                    new_pins_size.x = f32::max(new_pins_size.x, body_rect.width());
                    new_pins_size.y += body_rect.height() + ui.spacing().item_spacing.y;

                    if !snarl.nodes.contains(node.0) {
                        // If removed
                        return;
                    }

                    pins_rect = pins_rect.union(body_rect);
                    next_y = body_rect.bottom() + ui.spacing().item_spacing.y;
                }

                // Show output pins.

                let inputs_rect = payload_rect.intersect(Rect::everything_below(next_y));

                let r = draw_inputs(
                    snarl,
                    viewer,
                    node,
                    &inputs,
                    pin_size,
                    style,
                    ui,
                    inputs_rect,
                    payload_clip_rect,
                    input_x,
                    node_rect.min.y,
                    node_rect.min.y + node_state.header_height(),
                    input_spacing,
                    snarl_state,
                    modifiers,
                    input_positions,
                    node_layout.input_heights(&node_state),
                );

                let new_input_heights = r.new_heights;

                drag_released |= r.drag_released;

                if r.pin_hovered.is_some() {
                    pin_hovered = r.pin_hovered;
                }

                let inputs_rect = r.final_rect;

                if !snarl.nodes.contains(node.0) {
                    // If removed
                    return;
                }

                node_state.set_input_heights(new_input_heights);
                node_state.set_output_heights(new_output_heights);

                new_pins_size.x = f32::max(new_pins_size.x, inputs_rect.width());
                new_pins_size.y += inputs_rect.height() + ui.spacing().item_spacing.y;

                pins_rect = pins_rect.union(inputs_rect);

                pins_rect
            }
        };

        if viewer.has_footer(&snarl.nodes[node.0].value) {
            let footer_rect = Rect::from_min_max(
                pos2(
                    node_rect.left(),
                    pins_rect.bottom() + ui.spacing().item_spacing.y,
                ),
                pos2(node_rect.right(), node_rect.bottom()),
            );

            let mut footer_ui = ui.new_child(
                UiBuilder::new()
                    .max_rect(footer_rect.round_ui())
                    .layout(Layout::left_to_right(Align::Min))
                    .id_salt("footer"),
            );
            footer_ui.shrink_clip_rect(payload_clip_rect);

            viewer.show_footer(node, &inputs, &outputs, &mut footer_ui, snarl);

            let final_rect = footer_ui.min_rect();
            ui.expand_to_include_rect(final_rect.intersect(payload_clip_rect));
            let footer_size = final_rect.size();

            new_pins_size.x = f32::max(new_pins_size.x, footer_size.x);
            new_pins_size.y += footer_size.y + ui.spacing().item_spacing.y;

            if !snarl.nodes.contains(node.0) {
                // If removed
                return;
            }
        }

        // Render header frame.
        let mut header_rect = Rect::NAN;

        let mut header_frame_rect = Rect::NAN; //node_rect + header_frame.total_margin();

        // Show node's header
        let header_ui: &mut Ui = &mut ui.new_child(
            UiBuilder::new()
                .max_rect(node_rect.round_ui() + header_frame.total_margin())
                .layout(Layout::top_down(Align::Center))
                .id_salt("header"),
        );

        header_frame.show(header_ui, |ui: &mut Ui| {
            ui.with_layout(Layout::left_to_right(Align::Min), |ui| {
                if style.get_collapsible() {
                    let (_, r) = ui.allocate_exact_size(
                        vec2(ui.spacing().icon_width, ui.spacing().icon_width),
                        Sense::click(),
                    );
                    paint_default_icon(ui, openness, &r);

                    if r.clicked_by(PointerButton::Primary) {
                        // Toggle node's openness.
                        snarl.open_node(node, !open);
                    }
                }

                ui.allocate_exact_size(header_drag_space, Sense::hover());

                viewer.show_header(node, &inputs, &outputs, ui, snarl);

                header_rect = ui.min_rect();
            });

            header_frame_rect = header_rect + header_frame.total_margin();

            ui.advance_cursor_after_rect(Rect::from_min_max(
                header_rect.min,
                pos2(
                    f32::max(header_rect.max.x, node_rect.max.x),
                    header_rect.min.y,
                ),
            ));
        });

        ui.expand_to_include_rect(header_rect);
        let header_size = header_rect.size();
        node_state.set_header_height(header_size.y);

        node_state.set_size(vec2(
            f32::max(header_size.x, new_pins_size.x),
            header_size.y
                + header_frame.total_margin().bottom
                + ui.spacing().item_spacing.y
                + new_pins_size.y,
        ));
    });

    if !snarl.nodes.contains(node.0) {
        ui.ctx().request_repaint();
        node_state.clear(ui.ctx());
        // If removed
        return None;
    }

    viewer.final_node_rect(node, r.response.rect, ui, snarl);

    node_state.store(ui.ctx());
    Some(DrawNodeResponse {
        node_moved,
        node_to_top,
        drag_released,
        pin_hovered,
        final_rect: r.response.rect,
    })
}

const fn mix_colors(a: Color32, b: Color32) -> Color32 {
    #![allow(clippy::cast_possible_truncation)]

    Color32::from_rgba_premultiplied(
        ((a.r() as u16 + b.r() as u16) / 2) as u8,
        ((a.g() as u16 + b.g() as u16) / 2) as u8,
        ((a.b() as u16 + b.b() as u16) / 2) as u8,
        ((a.a() as u16 + b.a() as u16) / 2) as u8,
    )
}

// fn mix_colors(mut colors: impl Iterator<Item = Color32>) -> Option<Color32> {
//     let color = colors.next()?;

//     let mut r = color.r() as u32;
//     let mut g = color.g() as u32;
//     let mut b = color.b() as u32;
//     let mut a = color.a() as u32;
//     let mut w = 1;

//     for c in colors {
//         r += c.r() as u32;
//         g += c.g() as u32;
//         b += c.b() as u32;
//         a += c.a() as u32;
//         w += 1;
//     }

//     Some(Color32::from_rgba_premultiplied(
//         (r / w) as u8,
//         (g / w) as u8,
//         (b / w) as u8,
//         (a / w) as u8,
//     ))
// }

// fn mix_sizes(mut sizes: impl Iterator<Item = f32>) -> Option<f32> {
//     let mut size = sizes.next()?;
//     let mut w = 1;

//     for s in sizes {
//         size += s;
//         w += 1;
//     }

//     Some(size / w as f32)
// }

// fn mix_strokes(mut strokes: impl Iterator<Item = Stroke>) -> Option<Stroke> {
//     let stoke = strokes.next()?;

//     let mut width = stoke.width;
//     let mut r = stoke.color.r() as u32;
//     let mut g = stoke.color.g() as u32;
//     let mut b = stoke.color.b() as u32;
//     let mut a = stoke.color.a() as u32;

//     let mut w = 1;

//     for s in strokes {
//         width += s.width;
//         r += s.color.r() as u32;
//         g += s.color.g() as u32;
//         b += s.color.b() as u32;
//         a += s.color.a() as u32;
//         w += 1;
//     }

//     Some(Stroke {
//         width: width / w as f32,
//         color: Color32::from_rgba_premultiplied(
//             (r / w) as u8,
//             (g / w) as u8,
//             (b / w) as u8,
//             (a / w) as u8,
//         ),
//     })
// }

impl<T> Snarl<T> {
    /// Render [`Snarl`] using given viewer and style into the [`Ui`].
    #[inline]
    pub fn show<V>(&mut self, viewer: &mut V, style: &SnarlStyle, id_salt: impl Hash, ui: &mut Ui)
    where
        V: SnarlViewer<T>,
    {
        show_snarl(
            ui.make_persistent_id(id_salt),
            *style,
            Vec2::ZERO,
            Vec2::INFINITY,
            self,
            viewer,
            ui,
        );
    }
}

#[inline]
fn clamp_scale(to_global: &mut TSTransform, min_scale: f32, max_scale: f32, ui_rect: Rect) {
    if to_global.scaling >= min_scale && to_global.scaling <= max_scale {
        return;
    }

    let new_scaling = to_global.scaling.clamp(min_scale, max_scale);
    *to_global = scale_transform_around(to_global, new_scaling, ui_rect.center());
}

#[inline]
#[must_use]
fn transform_matching_points(from: Pos2, to: Pos2, scaling: f32) -> TSTransform {
    TSTransform {
        scaling,
        translation: to.to_vec2() - from.to_vec2() * scaling,
    }
}

#[inline]
#[must_use]
fn scale_transform_around(transform: &mut TSTransform, scaling: f32, point: Pos2) -> TSTransform {
    let from = (point - transform.translation) / transform.scaling;
    transform_matching_points(from, point, scaling)
}

#[test]
const fn snarl_style_is_send_sync() {
    const fn is_send_sync<T: Send + Sync>() {}
    is_send_sync::<SnarlStyle>();
}
