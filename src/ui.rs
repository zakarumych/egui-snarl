//! This module provides functionality for showing [`Snarl`] graph in [`Ui`].

use std::{collections::HashMap, hash::Hash};

use egui::{
    collapsing_header::paint_default_icon, epaint::Shadow, pos2, vec2, Align, Color32, Frame, Id,
    Layout, Margin, Modifiers, PointerButton, Pos2, Rect, Rounding, Sense, Shape, Stroke, Style,
    Ui, UiBuilder, Vec2,
};

use crate::{InPin, InPinId, Node, NodeId, OutPin, OutPinId, Snarl};

mod background_pattern;
mod pin;
mod state;
mod viewer;
mod wire;
mod zoom;

use self::{
    pin::AnyPin,
    state::{NewWires, NodeState, SnarlState},
    wire::{draw_wire, hit_wire, pick_wire_style},
    zoom::Zoom,
};

pub use self::{
    background_pattern::{BackgroundPattern, Grid, Viewport},
    pin::{AnyPins, PinInfo, PinShape},
    viewer::SnarlViewer,
    wire::{WireLayer, WireStyle},
};

/// Controls how header, pins, body and footer are laid out in the node.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "egui-probe", derive(egui_probe::EguiProbe))]
pub enum NodeLayout {
    /// Input pins, body and output pins are placed horizontally.
    /// With header on top and footer on bottom.
    ///
    /// +---------------------+
    /// |       Header        |
    /// +----+-----------+----+
    /// | In |   Body    | Out|
    /// +----+-----------+----+
    /// |       Footer        |
    /// +---------------------+
    ///
    #[default]
    Basic,

    /// All elements are placed in vertical stack.
    /// Header is on top, then input pins, body, output pins and footer.
    ///
    /// +---------------------+
    /// |       Header        |
    /// +---------------------+
    /// | In                  |
    /// +---------------------+
    /// |       Body          |
    /// +---------------------+
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
    /// +---------------------+
    /// |       Body          |
    /// +---------------------+
    /// | In                  |
    /// +---------------------+
    /// |       Footer        |
    /// +---------------------+
    FlippedSandwich,
}

/// Controls style of node selection rect.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "egui-probe", derive(egui_probe::EguiProbe))]
pub struct SelectionStyle {
    /// Margin between selection rect and node frame.
    pub margin: Margin,

    /// Rounding of selection rect.
    pub rounding: Rounding,

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
#[derive(Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "egui-probe", derive(egui_probe::EguiProbe))]
pub struct SnarlStyle {
    /// Controls how nodes are laid out.
    /// Defaults to [`NodeLayout::Basic`].
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

    /// Velocity of viewport scale when scaling with mouse wheel.
    #[cfg_attr(feature = "egui-probe", egui_probe(range = 0.0..))]
    #[cfg_attr(
        feature = "serde",
        serde(skip_serializing_if = "Option::is_none", default)
    )]
    pub scale_velocity: Option<f32>,

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

    #[doc(hidden)]
    #[cfg_attr(feature = "egui-probe", egui_probe(skip))]
    #[cfg_attr(feature = "serde", serde(skip_serializing, default))]
    /// Do not access other than with .., here to emulate `#[non_exhaustive(pub)]`
    pub _non_exhaustive: (),
}

impl SnarlStyle {
    fn get_node_layout(&self) -> NodeLayout {
        self.node_layout.unwrap_or(NodeLayout::Basic)
    }

    fn get_pin_size(&self, scale: f32, style: &Style) -> f32 {
        self.pin_size
            .zoomed(scale)
            .unwrap_or(style.spacing.interact_size.y * 0.6)
    }

    fn get_pin_fill(&self, style: &Style) -> Color32 {
        self.pin_fill
            .unwrap_or(style.visuals.widgets.active.bg_fill)
    }

    fn get_pin_stroke(&self, scale: f32, style: &Style) -> Stroke {
        self.pin_stroke.zoomed(scale).unwrap_or(Stroke::new(
            style.visuals.widgets.active.bg_stroke.width,
            style.visuals.widgets.active.bg_stroke.color,
        ))
    }

    fn get_pin_shape(&self) -> PinShape {
        self.pin_shape.unwrap_or(PinShape::Circle)
    }

    fn get_pin_placement(&self) -> PinPlacement {
        self.pin_placement.unwrap_or_default()
    }

    fn get_wire_width(&self, scale: f32, style: &Style) -> f32 {
        self.wire_width
            .zoomed(scale)
            .unwrap_or(self.get_pin_size(scale, style) * 0.1)
    }

    fn get_wire_frame_size(&self, scale: f32, style: &Style) -> f32 {
        self.wire_frame_size
            .zoomed(scale)
            .unwrap_or(self.get_pin_size(scale, style) * 3.0)
    }

    fn get_downscale_wire_frame(&self) -> bool {
        self.downscale_wire_frame.unwrap_or(true)
    }

    fn get_upscale_wire_frame(&self) -> bool {
        self.upscale_wire_frame.unwrap_or(false)
    }

    fn get_wire_style(&self, scale: f32) -> WireStyle {
        self.wire_style.zoomed(scale).unwrap_or(WireStyle::Bezier5)
    }

    fn get_wire_layer(&self) -> WireLayer {
        self.wire_layer.unwrap_or(WireLayer::BehindNodes)
    }

    fn get_header_drag_space(&self, scale: f32, style: &Style) -> Vec2 {
        self.header_drag_space
            .zoomed(scale)
            .unwrap_or(vec2(style.spacing.icon_width, style.spacing.icon_width))
    }

    fn get_collapsible(&self) -> bool {
        self.collapsible.unwrap_or(true)
    }

    fn get_bg_frame(&self, style: &Style) -> Frame {
        self.bg_frame.unwrap_or(Frame::canvas(style))
    }

    fn get_bg_pattern_stroke(&self, scale: f32, style: &Style) -> Stroke {
        self.bg_pattern_stroke
            .zoomed(scale)
            .unwrap_or(style.visuals.widgets.noninteractive.bg_stroke)
    }

    fn get_min_scale(&self) -> f32 {
        self.min_scale.unwrap_or(0.2)
    }

    fn get_max_scale(&self) -> f32 {
        self.max_scale.unwrap_or(5.0)
    }

    fn get_scale_velocity(&self) -> f32 {
        self.scale_velocity.unwrap_or(0.005)
    }

    fn get_node_frame(&self, scale: f32, style: &Style) -> Frame {
        self.node_frame
            .zoomed(scale)
            .unwrap_or_else(|| Frame::window(style))
    }

    fn get_header_frame(&self, scale: f32, style: &Style) -> Frame {
        self.header_frame
            .zoomed(scale)
            .unwrap_or_else(|| self.get_node_frame(scale, style).shadow(Shadow::NONE))
    }

    fn get_centering(&self) -> bool {
        self.centering.unwrap_or(true)
    }

    fn get_select_stroke(&self, scale: f32, style: &Style) -> Stroke {
        self.select_stoke.zoomed(scale).unwrap_or(Stroke::new(
            style.visuals.selection.stroke.width,
            style.visuals.selection.stroke.color.gamma_multiply(0.5),
        ))
    }

    fn get_select_fill(&self, style: &Style) -> Color32 {
        self.select_fill
            .unwrap_or(style.visuals.selection.bg_fill.gamma_multiply(0.3))
    }

    fn get_select_rect_contained(&self) -> bool {
        self.select_rect_contained.unwrap_or(false)
    }

    fn get_select_style(&self, scale: f32, style: &Style) -> SelectionStyle {
        self.select_style.zoomed(scale).unwrap_or(SelectionStyle {
            margin: style.spacing.window_margin,
            rounding: style.visuals.window_rounding,
            fill: self.get_select_fill(style),
            stroke: self.get_select_stroke(scale, style),
        })
    }
}

#[cfg(feature = "serde")]
mod serde_frame_option {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    #[derive(Serialize, Deserialize)]
    pub struct Frame {
        pub inner_margin: egui::Margin,
        pub outer_margin: egui::Margin,
        pub rounding: egui::Rounding,
        pub shadow: egui::epaint::Shadow,
        pub fill: egui::Color32,
        pub stroke: egui::Stroke,
    }

    pub fn serialize<S>(frame: &Option<egui::Frame>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match frame {
            Some(frame) => Frame {
                inner_margin: frame.inner_margin,
                outer_margin: frame.outer_margin,
                rounding: frame.rounding,
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
            rounding: frame.rounding,
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
            scale_velocity: None,
            node_frame: None,
            header_frame: None,
            centering: None,
            select_stoke: None,
            select_fill: None,
            select_rect_contained: None,
            select_style: None,

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

struct Input {
    hover_pos: Option<Pos2>,
    interact_pos: Option<Pos2>,
    scroll_delta: f32,
    // primary_pressed: bool,
    secondary_pressed: bool,
    modifiers: Modifiers,
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
}

struct DrawBodyResponse {
    final_rect: Rect,
}

struct PinResponse {
    pos: Pos2,
    pin_color: Color32,
    wire_style: Option<WireStyle>,
}

impl<T> Snarl<T> {
    fn draw_background<V>(
        &self,
        viewer: &mut V,
        style: &SnarlStyle,
        snarl_state: &SnarlState,
        viewport: &Rect,
        ui: &mut Ui,
    ) where
        V: SnarlViewer<T>,
    {
        let viewport = Viewport {
            rect: *viewport,
            scale: snarl_state.scale(),
            offset: snarl_state.offset(),
        };

        viewer.draw_background(
            style.bg_pattern.as_ref(),
            &viewport,
            style,
            ui.style(),
            ui.painter(),
            self,
        );
    }

    /// Render [`Snarl`] using given viewer and style into the [`Ui`].

    pub fn show<V>(&mut self, viewer: &mut V, style: &SnarlStyle, id_salt: impl Hash, ui: &mut Ui)
    where
        V: SnarlViewer<T>,
    {
        #![allow(clippy::too_many_lines)]

        let snarl_id = ui.make_persistent_id(id_salt);

        // Draw background pattern.
        let bg_frame = style.get_bg_frame(ui.style());

        let input = ui.ctx().input(|i| Input {
            scroll_delta: i.raw_scroll_delta.y,
            hover_pos: i.pointer.hover_pos(),
            interact_pos: i.pointer.interact_pos(),
            modifiers: i.modifiers,
            // primary_pressed: i.pointer.primary_pressed(),
            secondary_pressed: i.pointer.secondary_pressed(),
        });

        bg_frame.show(ui, |ui| {
            let mut node_moved = None;
            let mut node_to_top = None;

            let mut bg_r = ui.allocate_rect(ui.max_rect(), Sense::click_and_drag());
            let viewport = bg_r.rect;
            ui.set_clip_rect(viewport);

            let pivot = input.hover_pos.unwrap_or_else(|| viewport.center());

            let mut snarl_state =
                SnarlState::load(ui.ctx(), snarl_id, pivot, viewport, self, style);

            ui.style_mut().zoom(snarl_state.scale());

            // let mut node_style: Style = (**ui.style()).clone();
            // node_style.zoom(snarl_state.scale());

            //Draw background
            self.draw_background(viewer, style, &snarl_state, &viewport, ui);

            let wire_frame_size = style.get_wire_frame_size(snarl_state.scale(), ui.style());
            let wire_width = style.get_wire_width(snarl_state.scale(), ui.style());

            let wire_shape_idx = match style.get_wire_layer() {
                WireLayer::BehindNodes => Some(ui.painter().add(Shape::Noop)),
                WireLayer::AboveNodes => None,
            };

            // Zooming
            match input.hover_pos {
                Some(hover_pos)
                    if viewport.contains(hover_pos) && ui.rect_contains_pointer(viewport) =>
                {
                    if input.scroll_delta != 0.0 {
                        let new_scale = (snarl_state.scale()
                            * (1.0 + input.scroll_delta * style.get_scale_velocity()))
                        .clamp(style.get_min_scale(), style.get_max_scale());

                        snarl_state.set_scale(new_scale);
                    }
                }
                _ => {}
            }
            let mut input_info = HashMap::new();
            let mut output_info = HashMap::new();

            let mut pin_hovered = None;

            let draw_order = snarl_state.update_draw_order(self);
            let mut drag_released = false;

            let mut centers_sum = vec2(0.0, 0.0);
            let mut centers_weight = 0;

            let mut node_rects = Vec::new();

            for node_idx in draw_order {
                if !self.nodes.contains(node_idx.0) {
                    continue;
                }

                // show_node(node_idx);
                let response = self.draw_node(
                    ui,
                    node_idx,
                    viewer,
                    &mut snarl_state,
                    style,
                    snarl_id,
                    &mut input_info,
                    &input,
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

                    centers_sum += response.final_rect.center().to_vec2();
                    centers_weight += 1;

                    if snarl_state.is_rect_selection() {
                        node_rects.push((node_idx, response.final_rect));
                    }
                }
            }

            let mut hovered_wire = None;
            let mut hovered_wire_disconnect = false;
            let mut wire_shapes = Vec::new();
            let mut wire_hit = false;

            for wire in self.wires.iter() {
                let Some(from_r) = output_info.get(&wire.out_pin) else {
                    continue;
                };
                let Some(to_r) = input_info.get(&wire.in_pin) else {
                    continue;
                };

                if !wire_hit && !snarl_state.has_new_wires() && bg_r.hovered() && !bg_r.dragged() {
                    // Try to find hovered wire
                    // If not draggin new wire
                    // And not hovering over item above.

                    if let Some(interact_pos) = input.interact_pos {
                        wire_hit = hit_wire(
                            interact_pos,
                            wire_frame_size,
                            style.get_upscale_wire_frame(),
                            style.get_downscale_wire_frame(),
                            from_r.pos,
                            to_r.pos,
                            wire_width.max(1.5),
                            pick_wire_style(
                                style.get_wire_style(snarl_state.scale()),
                                from_r.wire_style,
                                to_r.wire_style,
                            )
                            .zoomed(snarl_state.scale()),
                        );

                        if wire_hit {
                            hovered_wire = Some(wire);

                            //Remove hovered wire by second click
                            hovered_wire_disconnect |= bg_r.clicked_by(PointerButton::Secondary);

                            // Background is not hovered then.
                            bg_r.hovered = false;
                            bg_r.clicked = false;
                        }
                    }
                }

                let color = mix_colors(from_r.pin_color, to_r.pin_color);

                let mut draw_width = wire_width;
                if hovered_wire == Some(wire) {
                    draw_width *= 1.5;
                }

                draw_wire(
                    ui,
                    &mut wire_shapes,
                    wire_frame_size,
                    style.get_upscale_wire_frame(),
                    style.get_downscale_wire_frame(),
                    from_r.pos,
                    to_r.pos,
                    Stroke::new(draw_width, color),
                    pick_wire_style(
                        style.get_wire_style(snarl_state.scale()),
                        from_r.wire_style.zoomed(snarl_state.scale()),
                        to_r.wire_style.zoomed(snarl_state.scale()),
                    ),
                );
            }

            //Remove hovered wire by second click
            if hovered_wire_disconnect {
                if let Some(wire) = hovered_wire {
                    let out_pin = OutPin::new(self, wire.out_pin);
                    let in_pin = InPin::new(self, wire.in_pin);
                    viewer.disconnect(&out_pin, &in_pin, self);
                }
            }

            if bg_r.drag_started_by(PointerButton::Primary) && input.modifiers.shift {
                let screen_pos = input.interact_pos.unwrap_or(viewport.center());
                let graph_pos = snarl_state.screen_pos_to_graph(screen_pos, viewport);
                snarl_state.start_rect_selection(graph_pos);
            }

            if bg_r.dragged_by(PointerButton::Primary) {
                if snarl_state.is_rect_selection() && input.hover_pos.is_some() {
                    let screen_pos = input.hover_pos.unwrap();
                    let graph_pos = snarl_state.screen_pos_to_graph(screen_pos, viewport);
                    snarl_state.update_rect_selection(graph_pos);
                } else {
                    snarl_state.pan(-bg_r.drag_delta());
                }
            }

            if bg_r.drag_stopped_by(PointerButton::Primary) {
                if let Some(select_rect) = snarl_state.rect_selection() {
                    let select_nodes = node_rects.into_iter().filter_map(|(id, rect)| {
                        let select = match style.get_select_rect_contained() {
                            true => select_rect.contains_rect(rect),
                            false => select_rect.intersects(rect),
                        };

                        if select {
                            Some(id)
                        } else {
                            None
                        }
                    });

                    if input.modifiers.command {
                        snarl_state.deselect_many_nodes(select_nodes);
                    } else {
                        snarl_state.select_many_nodes(!input.modifiers.shift, select_nodes);
                    }

                    snarl_state.stop_rect_selection();
                }
            }

            if let Some(select_rect) = snarl_state.rect_selection() {
                ui.painter().rect(
                    snarl_state.graph_rect_to_screen(select_rect, viewport),
                    0.0,
                    style.get_select_fill(ui.style()),
                    style.get_select_stroke(snarl_state.scale(), ui.style()),
                );
            }

            // If right button is clicked while new wire is being dragged, cancel it.
            // This is to provide way to 'not open' the link graph node menu, but just
            // releasing the new wire to empty space.
            //
            // This uses `button_down` directly, instead of `clicked_by` to improve
            // responsiveness of the cancel action.
            if snarl_state.has_new_wires()
                && ui.input(|x| x.pointer.button_down(PointerButton::Secondary))
            {
                let _ = snarl_state.take_wires();
                bg_r.clicked = false;
            }

            // Do centering unless no nodes are present.
            if style.get_centering() && bg_r.double_clicked() && centers_weight > 0 {
                centers_sum /= centers_weight as f32;
                snarl_state.set_offset(centers_sum * snarl_state.scale());
            }

            if input.modifiers.command && bg_r.clicked_by(PointerButton::Primary) {
                snarl_state.deselect_all_nodes();
            }

            // Wire end position will be overrided when link graph menu is opened.
            let mut wire_end_pos = input.hover_pos.unwrap_or_default();

            if drag_released {
                let new_wires = snarl_state.take_wires();
                if new_wires.is_some() {
                    ui.ctx().request_repaint();
                }
                match (new_wires, pin_hovered) {
                    (Some(NewWires::In(in_pins)), Some(AnyPin::Out(out_pin))) => {
                        for in_pin in in_pins {
                            viewer.connect(
                                &OutPin::new(self, out_pin),
                                &InPin::new(self, in_pin),
                                self,
                            );
                        }
                    }
                    (Some(NewWires::Out(out_pins)), Some(AnyPin::In(in_pin))) => {
                        for out_pin in out_pins {
                            viewer.connect(
                                &OutPin::new(self, out_pin),
                                &InPin::new(self, in_pin),
                                self,
                            );
                        }
                    }
                    (Some(new_wires), None) if bg_r.hovered() => {
                        // A new pin is dropped without connecting it anywhere. This
                        // will open a pop-up window for creating a new node.
                        snarl_state.revert_take_wires(new_wires);

                        // Force open context menu.
                        bg_r.long_touched = true;
                    }
                    _ => {}
                }
            }

            // Open graph menu when right-clicking on empty space.
            let mut is_menu_visible = false;

            if let Some(interact_pos) = ui.ctx().input(|i| i.pointer.interact_pos()) {
                if snarl_state.has_new_wires() {
                    let pins = match snarl_state.new_wires().unwrap() {
                        NewWires::In(x) => AnyPins::In(x),
                        NewWires::Out(x) => AnyPins::Out(x),
                    };

                    if viewer.has_dropped_wire_menu(pins, self) {
                        bg_r.context_menu(|ui| {
                            is_menu_visible = true;
                            if !snarl_state.is_link_menu_open() {
                                // Mark link menu is now visible.
                                snarl_state.open_link_menu();
                            }

                            let pins = match snarl_state.new_wires().unwrap() {
                                NewWires::In(x) => AnyPins::In(x),
                                NewWires::Out(x) => AnyPins::Out(x),
                            };

                            wire_end_pos = ui.cursor().min;

                            // The context menu is opened as *link* graph menu.
                            viewer.show_dropped_wire_menu(
                                snarl_state.screen_pos_to_graph(ui.cursor().min, viewport),
                                ui,
                                snarl_state.scale(),
                                pins,
                                self,
                            );
                        });
                    }
                } else if snarl_state.is_link_menu_open()
                    || viewer.has_graph_menu(interact_pos, self)
                {
                    bg_r.context_menu(|ui| {
                        is_menu_visible = true;
                        if !snarl_state.is_link_menu_open() {
                            // Mark link menu is now visible.
                            snarl_state.open_link_menu();
                        }

                        viewer.show_graph_menu(
                            snarl_state.screen_pos_to_graph(ui.cursor().min, viewport),
                            ui,
                            snarl_state.scale(),
                            self,
                        );
                    });
                }
            }

            if !is_menu_visible && snarl_state.is_link_menu_open() {
                // It seems that the context menu was closed. Remove new wires.
                snarl_state.close_link_menu();
            }

            match snarl_state.new_wires() {
                None => {}
                Some(NewWires::In(pins)) => {
                    for pin in pins {
                        let from_pos = wire_end_pos;
                        let to_r = &input_info[pin];

                        draw_wire(
                            ui,
                            &mut wire_shapes,
                            wire_frame_size,
                            style.get_upscale_wire_frame(),
                            style.get_downscale_wire_frame(),
                            from_pos,
                            to_r.pos,
                            Stroke::new(wire_width, to_r.pin_color),
                            to_r.wire_style
                                .zoomed(snarl_state.scale())
                                .unwrap_or(style.get_wire_style(snarl_state.scale())),
                        );
                    }
                }
                Some(NewWires::Out(pins)) => {
                    for pin in pins {
                        let from_r = &output_info[pin];
                        let to_pos = wire_end_pos;

                        draw_wire(
                            ui,
                            &mut wire_shapes,
                            wire_frame_size,
                            style.get_upscale_wire_frame(),
                            style.get_downscale_wire_frame(),
                            from_r.pos,
                            to_pos,
                            Stroke::new(wire_width, from_r.pin_color),
                            from_r
                                .wire_style
                                .zoomed(snarl_state.scale())
                                .unwrap_or(style.get_wire_style(snarl_state.scale())),
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

            ui.advance_cursor_after_rect(Rect::from_min_size(viewport.min, Vec2::ZERO));

            if let Some(node) = node_to_top {
                if self.nodes.contains(node.0) {
                    ui.ctx().request_repaint();
                    snarl_state.node_to_top(node);
                }
            }

            if let Some((node, delta)) = node_moved {
                if self.nodes.contains(node.0) {
                    ui.ctx().request_repaint();
                    if snarl_state.selected_nodes().contains(&node) {
                        for node in snarl_state.selected_nodes() {
                            let node = &mut self.nodes[node.0];
                            node.pos += delta;
                        }
                    } else {
                        let node = &mut self.nodes[node.0];
                        node.pos += delta;
                    }
                }
            }

            snarl_state.store(self, ui.ctx());
        });
    }

    #[allow(clippy::too_many_arguments)]
    fn draw_inputs<V>(
        &mut self,
        viewer: &mut V,
        node: NodeId,
        inputs: &[InPin],
        pin_size: f32,
        style: &SnarlStyle,
        ui: &mut Ui,
        inputs_rect: Rect,
        clip_rect: Rect,
        viewport: Rect,
        input_x: f32,
        min_pin_y: f32,
        input_spacing: Option<f32>,
        snarl_state: &mut SnarlState,
        input: &Input,
        input_positions: &mut HashMap<InPinId, PinResponse>,
    ) -> DrawPinsResponse
    where
        V: SnarlViewer<T>,
    {
        let mut drag_released = false;
        let mut pin_hovered = None;

        // Input pins on the left.
        let inputs_ui = &mut ui.new_child(
            UiBuilder::new()
                .max_rect(inputs_rect)
                .layout(Layout::top_down(Align::Min))
                .id_salt("inputs"),
        );

        inputs_ui.set_clip_rect(clip_rect.intersect(viewport));

        for in_pin in inputs {
            // Show input pin.
            inputs_ui.with_layout(Layout::left_to_right(Align::Min), |ui| {
                if let Some(input_spacing) = input_spacing {
                    ui.allocate_space(vec2(input_spacing, pin_size));
                }

                let y0 = ui.cursor().min.y;

                // Show input content
                let pin_info = viewer.show_input(in_pin, ui, snarl_state.scale(), self);
                if !self.nodes.contains(node.0) {
                    // If removed
                    return;
                }

                let y1 = ui.min_rect().max.y;

                // ui.end_row();

                // Centered vertically.
                let y = min_pin_y.max((y0 + y1) * 0.5);

                let pin_pos = pos2(input_x, y);

                // Interact with pin shape.
                ui.set_clip_rect(viewport);

                let r = ui.interact(
                    Rect::from_center_size(pin_pos, vec2(pin_size, pin_size)),
                    ui.next_auto_id(),
                    Sense::click_and_drag(),
                );

                ui.skip_ahead_auto_ids(1);

                if r.clicked_by(PointerButton::Secondary) {
                    if snarl_state.has_new_wires() {
                        snarl_state.remove_new_wire_in(in_pin.id);
                    } else {
                        viewer.drop_inputs(in_pin, self);
                        if !self.nodes.contains(node.0) {
                            // If removed
                            return;
                        }
                    }
                }
                if r.drag_started_by(PointerButton::Primary) {
                    if input.modifiers.command {
                        snarl_state.start_new_wires_out(&in_pin.remotes);
                        if !input.modifiers.shift {
                            self.drop_inputs(in_pin.id);
                            if !self.nodes.contains(node.0) {
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

                let mut visual_pin_size = pin_size;

                match input.hover_pos {
                    Some(hover_pos) if r.rect.contains(hover_pos) => {
                        if input.modifiers.shift {
                            snarl_state.add_new_wire_in(in_pin.id);
                        } else if input.secondary_pressed {
                            snarl_state.remove_new_wire_in(in_pin.id);
                        }
                        pin_hovered = Some(AnyPin::In(in_pin.id));
                        visual_pin_size *= 1.2;
                    }
                    _ => {}
                }

                let mut pin_painter = ui.painter().clone();
                pin_painter.set_clip_rect(viewport);

                let pin_color = viewer.draw_input_pin(
                    in_pin,
                    &pin_info,
                    r.rect.center(),
                    visual_pin_size,
                    style,
                    ui.style(),
                    &pin_painter,
                    snarl_state.scale(),
                    self,
                );

                input_positions.insert(
                    in_pin.id,
                    PinResponse {
                        pos: r.rect.center(),
                        pin_color,
                        wire_style: pin_info.wire_style,
                    },
                );
            });
        }

        let final_rect = inputs_ui.min_rect();
        ui.expand_to_include_rect(final_rect.intersect(clip_rect));

        DrawPinsResponse {
            drag_released,
            pin_hovered,
            final_rect,
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn draw_outputs<V>(
        &mut self,
        viewer: &mut V,
        node: NodeId,
        outputs: &[OutPin],
        pin_size: f32,
        style: &SnarlStyle,
        ui: &mut Ui,
        outputs_rect: Rect,
        clip_rect: Rect,
        viewport: Rect,
        output_x: f32,
        min_pin_y: f32,
        output_spacing: Option<f32>,
        snarl_state: &mut SnarlState,
        input: &Input,
        output_positions: &mut HashMap<OutPinId, PinResponse>,
    ) -> DrawPinsResponse
    where
        V: SnarlViewer<T>,
    {
        let mut drag_released = false;
        let mut pin_hovered = None;

        let outputs_ui = &mut ui.new_child(
            UiBuilder::new()
                .max_rect(outputs_rect)
                .layout(Layout::top_down(Align::Max))
                .id_salt("outputs"),
        );

        outputs_ui.set_clip_rect(clip_rect.intersect(viewport));

        // Output pins on the right.
        for out_pin in outputs {
            // Show output pin.
            outputs_ui.with_layout(Layout::right_to_left(Align::Min), |ui| {
                // Allocate space for pin shape.
                if let Some(output_spacing) = output_spacing {
                    ui.allocate_space(vec2(output_spacing, pin_size));
                }

                let y0 = ui.cursor().min.y;

                // Show output content
                let pin_info = viewer.show_output(out_pin, ui, snarl_state.scale(), self);
                if !self.nodes.contains(node.0) {
                    // If removed
                    return;
                }

                let y1 = ui.min_rect().max.y;

                // ui.end_row();

                // Centered vertically.
                let y = min_pin_y.max((y0 + y1) * 0.5);

                let pin_pos = pos2(output_x, y);

                ui.set_clip_rect(viewport);

                let r = ui.interact(
                    Rect::from_center_size(pin_pos, vec2(pin_size, pin_size)),
                    ui.next_auto_id(),
                    Sense::click_and_drag(),
                );

                ui.skip_ahead_auto_ids(1);

                if r.clicked_by(PointerButton::Secondary) {
                    if snarl_state.has_new_wires() {
                        snarl_state.remove_new_wire_out(out_pin.id);
                    } else {
                        viewer.drop_outputs(out_pin, self);
                        if !self.nodes.contains(node.0) {
                            // If removed
                            return;
                        }
                    }
                }
                if r.drag_started_by(PointerButton::Primary) {
                    if input.modifiers.command {
                        snarl_state.start_new_wires_in(&out_pin.remotes);

                        if !input.modifiers.shift {
                            self.drop_outputs(out_pin.id);
                            if !self.nodes.contains(node.0) {
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

                let mut visual_pin_size = pin_size;
                match input.hover_pos {
                    Some(hover_pos) if r.rect.contains(hover_pos) => {
                        if input.modifiers.shift {
                            snarl_state.add_new_wire_out(out_pin.id);
                        } else if input.secondary_pressed {
                            snarl_state.remove_new_wire_out(out_pin.id);
                        }
                        pin_hovered = Some(AnyPin::Out(out_pin.id));
                        visual_pin_size *= 1.2;
                    }
                    _ => {}
                }

                let mut pin_painter = ui.painter().clone();
                pin_painter.set_clip_rect(viewport);

                let pin_color = viewer.draw_output_pin(
                    out_pin,
                    &pin_info,
                    r.rect.center(),
                    visual_pin_size,
                    style,
                    ui.style(),
                    &pin_painter,
                    snarl_state.scale(),
                    self,
                );

                output_positions.insert(
                    out_pin.id,
                    PinResponse {
                        pos: r.rect.center(),
                        pin_color,
                        wire_style: pin_info.wire_style,
                    },
                );
            });
        }
        let final_rect = outputs_ui.min_rect();
        ui.expand_to_include_rect(final_rect.intersect(clip_rect));

        DrawPinsResponse {
            drag_released,
            pin_hovered,
            final_rect,
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn draw_body<V>(
        &mut self,
        viewer: &mut V,
        node: NodeId,
        inputs: &[InPin],
        outputs: &[OutPin],
        ui: &mut Ui,
        body_rect: Rect,
        clip_rect: Rect,
        viewport: Rect,
        snarl_state: &mut SnarlState,
    ) -> DrawBodyResponse
    where
        V: SnarlViewer<T>,
    {
        let mut body_ui = ui.new_child(
            UiBuilder::new()
                .max_rect(body_rect)
                .layout(Layout::left_to_right(Align::Min))
                .id_salt("body"),
        );
        body_ui.set_clip_rect(clip_rect.intersect(viewport));

        viewer.show_body(
            node,
            inputs,
            outputs,
            &mut body_ui,
            snarl_state.scale(),
            self,
        );

        let final_rect = body_ui.min_rect();
        ui.expand_to_include_rect(final_rect.intersect(clip_rect));
        // node_state.set_body_width(body_size.x);

        DrawBodyResponse { final_rect }
    }

    //First step for split big function to parts
    /// Draw one node. Return Pins info
    #[inline]
    #[allow(clippy::too_many_lines)]
    #[allow(clippy::too_many_arguments)]
    fn draw_node<V>(
        &mut self,
        ui: &mut Ui,
        node: NodeId,
        viewer: &mut V,
        snarl_state: &mut SnarlState,
        style: &SnarlStyle,
        snarl_id: Id,
        input_positions: &mut HashMap<InPinId, PinResponse>,
        input: &Input,
        output_positions: &mut HashMap<OutPinId, PinResponse>,
    ) -> Option<DrawNodeResponse>
    where
        V: SnarlViewer<T>,
    {
        let Node {
            pos,
            open,
            ref value,
        } = self.nodes[node.0];

        let viewport = ui.max_rect();

        // Collect pins
        let inputs_count = viewer.inputs(value);
        let outputs_count = viewer.outputs(value);

        let inputs = (0..inputs_count)
            .map(|idx| InPin::new(self, InPinId { node, input: idx }))
            .collect::<Vec<_>>();

        let outputs = (0..outputs_count)
            .map(|idx| OutPin::new(self, OutPinId { node, output: idx }))
            .collect::<Vec<_>>();

        let node_pos = snarl_state.graph_pos_to_screen(pos, viewport);

        // Generate persistent id for the node.
        let node_id = snarl_id.with(("snarl-node", node));

        let openness = ui.ctx().animate_bool(node_id, open);

        let mut node_state = NodeState::load(ui.ctx(), node_id, ui.spacing(), snarl_state.scale());

        let node_rect = node_state.node_rect(node_pos, openness);

        let mut node_to_top = None;
        let mut node_moved = None;
        let mut drag_released = false;
        let mut pin_hovered = None;

        let node_frame = viewer.node_frame(
            style.get_node_frame(snarl_state.scale(), ui.style()),
            node,
            &inputs,
            &outputs,
            self,
        );
        let header_frame = viewer.header_frame(
            style.get_header_frame(snarl_state.scale(), ui.style()),
            node,
            &inputs,
            &outputs,
            self,
        );

        // Rect for node + frame margin.
        let node_frame_rect = node_rect + node_frame.total_margin();

        if snarl_state.selected_nodes().contains(&node) {
            let select_style = style.get_select_style(snarl_state.scale(), ui.style());

            let select_rect = node_frame_rect + select_style.margin;

            ui.painter().rect(
                select_rect,
                select_style.rounding,
                select_style.fill,
                select_style.stroke,
            );
        }

        // Size of the pin.
        // Side of the square or diameter of the circle.
        let pin_size = style.get_pin_size(snarl_state.scale(), ui.style()).max(0.0);

        let pin_placement = style.get_pin_placement();

        let header_drag_space = style
            .get_header_drag_space(snarl_state.scale(), ui.style())
            .max(Vec2::ZERO);

        // Interact with node frame.
        let r = ui.interact(
            node_frame_rect,
            node_id.with("frame"),
            Sense::click_and_drag(),
        );

        if !input.modifiers.shift
            && !input.modifiers.command
            && r.dragged_by(PointerButton::Primary)
        {
            node_moved = Some((node, snarl_state.screen_vec_to_graph(r.drag_delta())));
        }

        if r.clicked_by(PointerButton::Primary) || r.dragged_by(PointerButton::Primary) {
            if input.modifiers.shift {
                snarl_state.select_one_node(input.modifiers.command, node);
            } else if input.modifiers.command {
                snarl_state.deselect_one_node(node);
            }
        }

        if r.clicked() || r.dragged() {
            node_to_top = Some(node);
        }

        if viewer.has_node_menu(&self.nodes[node.0].value) {
            r.context_menu(|ui| {
                viewer.show_node_menu(node, &inputs, &outputs, ui, snarl_state.scale(), self);
            });
        }

        if !self.nodes.contains(node.0) {
            node_state.clear(ui.ctx());
            // If removed
            return None;
        }

        if viewer.has_on_hover_popup(&self.nodes[node.0].value) {
            r.on_hover_ui_at_pointer(|ui| {
                viewer.show_on_hover_popup(node, &inputs, &outputs, ui, snarl_state.scale(), self);
            });
        }

        if !self.nodes.contains(node.0) {
            node_state.clear(ui.ctx());
            // If removed
            return None;
        }

        let node_ui = &mut ui.new_child(
            UiBuilder::new()
                .max_rect(node_frame_rect)
                .layout(Layout::top_down(Align::Center))
                .id_salt(node_id),
        );

        let mut new_pins_size = Vec2::ZERO;

        let r = node_frame.show(node_ui, |ui| {
            let min_pin_y = node_rect.min.y + node_state.header_height() * 0.5;

            // Input pins' center side by X axis.
            let input_x = match pin_placement {
                PinPlacement::Inside => {
                    node_frame_rect.left() + node_frame.inner_margin.left + pin_size * 0.5
                }
                PinPlacement::Edge => node_frame_rect.left(),
                PinPlacement::Outside { margin } => {
                    node_frame_rect.left() - margin * snarl_state.scale() - pin_size * 0.5
                }
            };

            // Input pins' spacing required.
            let input_spacing = match pin_placement {
                PinPlacement::Inside => Some(pin_size),
                PinPlacement::Edge => {
                    Some((pin_size * 0.5 - node_frame.inner_margin.left).max(0.0))
                }
                PinPlacement::Outside { .. } => None,
            };

            // Output pins' center side by X axis.
            let output_x = match pin_placement {
                PinPlacement::Inside => {
                    node_frame_rect.right() - node_frame.inner_margin.right - pin_size * 0.5
                }
                PinPlacement::Edge => node_frame_rect.right(),
                PinPlacement::Outside { margin } => {
                    node_frame_rect.right() + margin * snarl_state.scale() + pin_size * 0.5
                }
            };

            // Output pins' spacing required.
            let output_spacing = match pin_placement {
                PinPlacement::Inside => Some(pin_size),
                PinPlacement::Edge => {
                    Some((pin_size * 0.5 - node_frame.inner_margin.right).max(0.0))
                }
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
                viewer.node_layout(style.get_node_layout(), node, &inputs, &outputs, self);

            let payload_clip_rect =
                Rect::from_min_max(node_rect.min, pos2(node_rect.max.x, f32::INFINITY));

            let pins_rect = match node_layout {
                NodeLayout::Basic => {
                    // Show input pins.
                    let r = self.draw_inputs(
                        viewer,
                        node,
                        &inputs,
                        pin_size,
                        style,
                        ui,
                        payload_rect,
                        payload_clip_rect,
                        viewport,
                        input_x,
                        min_pin_y,
                        input_spacing,
                        snarl_state,
                        input,
                        input_positions,
                    );

                    drag_released |= r.drag_released;

                    if r.pin_hovered.is_some() {
                        pin_hovered = r.pin_hovered;
                    }

                    let inputs_rect = r.final_rect;
                    let inputs_size = inputs_rect.size();

                    if !self.nodes.contains(node.0) {
                        // If removed
                        return;
                    }

                    // Show output pins.

                    let r = self.draw_outputs(
                        viewer,
                        node,
                        &outputs,
                        pin_size,
                        style,
                        ui,
                        payload_rect,
                        payload_clip_rect,
                        viewport,
                        output_x,
                        min_pin_y,
                        output_spacing,
                        snarl_state,
                        input,
                        output_positions,
                    );

                    drag_released |= r.drag_released;

                    if r.pin_hovered.is_some() {
                        pin_hovered = r.pin_hovered;
                    }

                    let outputs_rect = r.final_rect;
                    let outputs_size = outputs_rect.size();

                    if !self.nodes.contains(node.0) {
                        // If removed
                        return;
                    }

                    new_pins_size = vec2(
                        inputs_size.x + outputs_size.x + ui.spacing().item_spacing.x,
                        f32::max(inputs_size.y, outputs_size.y),
                    );

                    let mut pins_rect = inputs_rect.union(outputs_rect);

                    // Show body if there's one.
                    if viewer.has_body(&self.nodes.get(node.0).unwrap().value) {
                        let body_left = inputs_rect.right() + ui.spacing().item_spacing.x;
                        let body_right = outputs_rect.left() - ui.spacing().item_spacing.x;
                        let body_top = payload_rect.top();
                        let body_bottom = payload_rect.bottom();

                        let body_rect = Rect::from_min_max(
                            pos2(body_left, body_top),
                            pos2(body_right, body_bottom),
                        );

                        let r = self.draw_body(
                            viewer,
                            node,
                            &inputs,
                            &outputs,
                            ui,
                            body_rect,
                            payload_clip_rect,
                            viewport,
                            snarl_state,
                        );

                        new_pins_size.x += r.final_rect.width() + ui.spacing().item_spacing.x;
                        new_pins_size.y = f32::max(new_pins_size.y, r.final_rect.height());

                        pins_rect = pins_rect.union(body_rect);

                        if !self.nodes.contains(node.0) {
                            // If removed
                            return;
                        }
                    }

                    pins_rect
                }
                NodeLayout::Sandwich => {
                    // Show input pins.

                    let inputs_rect = payload_rect;
                    let r = self.draw_inputs(
                        viewer,
                        node,
                        &inputs,
                        pin_size,
                        style,
                        ui,
                        inputs_rect,
                        payload_clip_rect,
                        viewport,
                        input_x,
                        min_pin_y,
                        input_spacing,
                        snarl_state,
                        input,
                        input_positions,
                    );

                    drag_released |= r.drag_released;

                    if r.pin_hovered.is_some() {
                        pin_hovered = r.pin_hovered;
                    }

                    let inputs_rect = r.final_rect;

                    new_pins_size = inputs_rect.size();

                    let mut next_y = inputs_rect.bottom() + ui.spacing().item_spacing.y;

                    if !self.nodes.contains(node.0) {
                        // If removed
                        return;
                    }

                    let mut pins_rect = inputs_rect;

                    // Show body if there's one.
                    if viewer.has_body(&self.nodes.get(node.0).unwrap().value) {
                        let body_rect = payload_rect.intersect(Rect::everything_below(next_y));

                        let r = self.draw_body(
                            viewer,
                            node,
                            &inputs,
                            &outputs,
                            ui,
                            body_rect,
                            payload_clip_rect,
                            viewport,
                            snarl_state,
                        );

                        let body_rect = r.final_rect;

                        new_pins_size.x = f32::max(new_pins_size.x, body_rect.width());
                        new_pins_size.y += body_rect.height() + ui.spacing().item_spacing.y;

                        if !self.nodes.contains(node.0) {
                            // If removed
                            return;
                        }

                        pins_rect = pins_rect.union(body_rect);
                        next_y = body_rect.bottom() + ui.spacing().item_spacing.y;
                    }

                    // Show output pins.

                    let outputs_rect = payload_rect.intersect(Rect::everything_below(next_y));

                    let r = self.draw_outputs(
                        viewer,
                        node,
                        &outputs,
                        pin_size,
                        style,
                        ui,
                        outputs_rect,
                        payload_clip_rect,
                        viewport,
                        output_x,
                        min_pin_y,
                        output_spacing,
                        snarl_state,
                        input,
                        output_positions,
                    );

                    drag_released |= r.drag_released;

                    if r.pin_hovered.is_some() {
                        pin_hovered = r.pin_hovered;
                    }

                    let outputs_rect = r.final_rect;

                    if !self.nodes.contains(node.0) {
                        // If removed
                        return;
                    }

                    new_pins_size.x = f32::max(new_pins_size.x, outputs_rect.width());
                    new_pins_size.y += outputs_rect.height() + ui.spacing().item_spacing.y;

                    pins_rect = pins_rect.union(outputs_rect);

                    pins_rect
                }
                NodeLayout::FlippedSandwich => {
                    // Show input pins.

                    let outputs_rect = payload_rect;
                    let r = self.draw_outputs(
                        viewer,
                        node,
                        &outputs,
                        pin_size,
                        style,
                        ui,
                        outputs_rect,
                        payload_clip_rect,
                        viewport,
                        output_x,
                        min_pin_y,
                        output_spacing,
                        snarl_state,
                        input,
                        output_positions,
                    );

                    drag_released |= r.drag_released;

                    if r.pin_hovered.is_some() {
                        pin_hovered = r.pin_hovered;
                    }

                    let outputs_rect = r.final_rect;

                    new_pins_size = outputs_rect.size();

                    let mut next_y = outputs_rect.bottom() + ui.spacing().item_spacing.y;

                    if !self.nodes.contains(node.0) {
                        // If removed
                        return;
                    }

                    let mut pins_rect = outputs_rect;

                    // Show body if there's one.
                    if viewer.has_body(&self.nodes.get(node.0).unwrap().value) {
                        let body_rect = payload_rect.intersect(Rect::everything_below(next_y));

                        let r = self.draw_body(
                            viewer,
                            node,
                            &inputs,
                            &outputs,
                            ui,
                            body_rect,
                            payload_clip_rect,
                            viewport,
                            snarl_state,
                        );

                        let body_rect = r.final_rect;

                        new_pins_size.x = f32::max(new_pins_size.x, body_rect.width());
                        new_pins_size.y += body_rect.height() + ui.spacing().item_spacing.y;

                        if !self.nodes.contains(node.0) {
                            // If removed
                            return;
                        }

                        pins_rect = pins_rect.union(body_rect);
                        next_y = body_rect.bottom() + ui.spacing().item_spacing.y;
                    }

                    // Show output pins.

                    let inputs_rect = payload_rect.intersect(Rect::everything_below(next_y));

                    let r = self.draw_inputs(
                        viewer,
                        node,
                        &inputs,
                        pin_size,
                        style,
                        ui,
                        inputs_rect,
                        payload_clip_rect,
                        viewport,
                        input_x,
                        min_pin_y,
                        input_spacing,
                        snarl_state,
                        input,
                        input_positions,
                    );

                    drag_released |= r.drag_released;

                    if r.pin_hovered.is_some() {
                        pin_hovered = r.pin_hovered;
                    }

                    let inputs_rect = r.final_rect;

                    if !self.nodes.contains(node.0) {
                        // If removed
                        return;
                    }

                    new_pins_size.x = f32::max(new_pins_size.x, inputs_rect.width());
                    new_pins_size.y += inputs_rect.height() + ui.spacing().item_spacing.y;

                    pins_rect = pins_rect.union(inputs_rect);

                    pins_rect
                }
            };

            if viewer.has_footer(&self.nodes[node.0].value) {
                let footer_left = node_rect.left();
                let footer_right = node_rect.right();
                let footer_top = pins_rect.bottom() + ui.spacing().item_spacing.y;
                let footer_bottom = node_rect.bottom();

                let footer_rect = Rect::from_min_max(
                    pos2(footer_left, footer_top),
                    pos2(footer_right, footer_bottom),
                );

                let mut footer_ui = ui.new_child(
                    UiBuilder::new()
                        .max_rect(footer_rect)
                        .layout(Layout::left_to_right(Align::Min))
                        .id_salt("footer"),
                );
                footer_ui.set_clip_rect(payload_clip_rect.intersect(viewport));

                viewer.show_footer(
                    node,
                    &inputs,
                    &outputs,
                    &mut footer_ui,
                    snarl_state.scale(),
                    self,
                );

                let final_rect = footer_ui.min_rect();
                ui.expand_to_include_rect(final_rect.intersect(payload_clip_rect));
                let footer_size = final_rect.size();

                new_pins_size.x = f32::max(new_pins_size.x, footer_size.x);
                new_pins_size.y += footer_size.y + ui.spacing().item_spacing.y;

                if !self.nodes.contains(node.0) {
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
                    .max_rect(node_rect + header_frame.total_margin())
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
                            self.open_node(node, !open);
                        }
                    }

                    ui.allocate_exact_size(header_drag_space, Sense::hover());

                    viewer.show_header(node, &inputs, &outputs, ui, snarl_state.scale(), self);

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

        if !self.nodes.contains(node.0) {
            ui.ctx().request_repaint();
            node_state.clear(ui.ctx());
            // If removed
            return None;
        }

        let final_rect = snarl_state.screen_rect_to_graph(r.response.rect, viewport);
        viewer.final_node_rect(
            node,
            r.response.rect,
            final_rect,
            ui,
            snarl_state.scale(),
            self,
        );

        node_state.store(ui.ctx());
        ui.ctx().request_repaint();
        Some(DrawNodeResponse {
            node_moved,
            node_to_top,
            drag_released,
            pin_hovered,
            final_rect,
        })
    }
}

fn mix_colors(a: Color32, b: Color32) -> Color32 {
    Color32::from_rgba_premultiplied(
        ((a.r() as u32 + b.r() as u32) / 2) as u8,
        ((a.g() as u32 + b.g() as u32) / 2) as u8,
        ((a.b() as u32 + b.b() as u32) / 2) as u8,
        ((a.a() as u32 + b.a() as u32) / 2) as u8,
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

#[test]
fn snarl_style_is_send_sync() {
    fn is_send_sync<T: Send + Sync>() {}
    is_send_sync::<SnarlStyle>();
}
