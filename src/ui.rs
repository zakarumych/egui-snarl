//! This module provides functionality for showing [`Snarl`] graph in [`Ui`].

use std::{collections::HashMap, hash::Hash};

use egui::{
    collapsing_header::paint_default_icon, epaint::Shadow, pos2, vec2, Align, Color32, Frame, Id,
    InputState, Layout, Margin, Modifiers, PointerButton, Pos2, Rect, Rounding, Sense, Shape,
    Stroke, Ui, Vec2,
};

use crate::{InPin, InPinId, Node, NodeId, OutPin, OutPinId, Snarl};

mod background_pattern;
pub mod events;
mod pin;
mod state;
mod viewer;
mod wire;
mod zoom;

use self::{
    events::GraphEventsExtend,
    pin::{draw_pin, AnyPin},
    state::{NewWires, NodeState, SnarlState},
    wire::{draw_wire, hit_wire, pick_wire_style},
    zoom::Zoom,
};

pub use self::{
    background_pattern::{BackgroundPattern, Grid, Viewport},
    pin::{AnyPins, BasicPinShape, CustomPinShape, PinInfo, PinShape},
    viewer::SnarlViewer,
    wire::{WireLayer, WireStyle},
};

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

/// Style for rendering Snarl.
#[derive(Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "egui-probe", derive(egui_probe::EguiProbe))]
pub struct SnarlStyle<T: GraphEventsExtend> {
    /// Size of pins.
    #[cfg_attr(feature = "egui-probe", egui_probe(range = 0.0..))]
    #[cfg_attr(
        feature = "serde",
        serde(skip_serializing_if = "Option::is_none", default)
    )]
    pub _pin_size: Option<f32>,

    /// Default fill color for pins.
    #[cfg_attr(
        feature = "serde",
        serde(skip_serializing_if = "Option::is_none", default)
    )]
    pub _pin_fill: Option<Color32>,

    /// Default stroke for pins.
    #[cfg_attr(
        feature = "serde",
        serde(skip_serializing_if = "Option::is_none", default)
    )]
    pub _pin_stroke: Option<Stroke>,

    /// Shape of pins.
    #[cfg_attr(
        feature = "serde",
        serde(skip_serializing_if = "Option::is_none", default)
    )]
    pub _pin_shape: Option<BasicPinShape>,

    /// Width of wires.
    #[cfg_attr(feature = "egui-probe", egui_probe(range = 0.0..))]
    #[cfg_attr(
        feature = "serde",
        serde(skip_serializing_if = "Option::is_none", default)
    )]
    pub _wire_width: Option<f32>,

    /// Size of wire frame which controls curvature of wires.
    #[cfg_attr(feature = "egui-probe", egui_probe(range = 0.0..))]
    #[cfg_attr(
        feature = "serde",
        serde(skip_serializing_if = "Option::is_none", default)
    )]
    pub _wire_frame_size: Option<f32>,

    /// Whether to downscale wire frame when nodes are close.
    #[cfg_attr(
        feature = "serde",
        serde(skip_serializing_if = "Option::is_none", default)
    )]
    pub _downscale_wire_frame: Option<bool>,

    /// Weather to upscale wire frame when nodes are far.
    #[cfg_attr(
        feature = "serde",
        serde(skip_serializing_if = "Option::is_none", default)
    )]
    pub _upscale_wire_frame: Option<bool>,

    /// Controls default style of wires.
    #[cfg_attr(
        feature = "serde",
        serde(skip_serializing_if = "Option::is_none", default)
    )]
    pub _wire_style: Option<WireStyle>,

    /// Layer where wires are rendered.
    #[cfg_attr(
        feature = "serde",
        serde(skip_serializing_if = "Option::is_none", default)
    )]
    pub _wire_layer: Option<WireLayer>,

    /// Additional blank space for dragging node by header.
    #[cfg_attr(
        feature = "serde",
        serde(skip_serializing_if = "Option::is_none", default)
    )]
    pub _header_drag_space: Option<Vec2>,

    /// Whether nodes can be collapsed.
    /// If true, headers will have collapsing button.
    /// When collapsed, node will not show its pins, body and footer.
    #[cfg_attr(
        feature = "serde",
        serde(skip_serializing_if = "Option::is_none", default)
    )]
    pub _collapsible: Option<bool>,

    /// Frame used to draw background
    #[cfg_attr(
        feature = "serde",
        serde(
            skip_serializing_if = "Option::is_none",
            default,
            with = "serde_frame_option"
        )
    )]
    pub _bg_frame: Option<Frame>,

    /// Background pattern.
    /// Defaults to [`BackgroundPattern::Grid`].
    #[cfg_attr(
        feature = "serde",
        serde(skip_serializing_if = "Option::is_none", default, skip_deserializing)
    )]
    pub _bg_pattern: Option<BackgroundPattern<T>>,

    /// Stroke for background pattern.
    /// Defaults to `ui.visuals().widgets.noninteractive.bg_stroke`.
    #[cfg_attr(
        feature = "serde",
        serde(skip_serializing_if = "Option::is_none", default)
    )]
    pub _bg_pattern_stroke: Option<Stroke>,

    /// Minimum scale that can be set.
    #[cfg_attr(feature = "egui-probe", egui_probe(range = 0.0..=1.0))]
    #[cfg_attr(
        feature = "serde",
        serde(skip_serializing_if = "Option::is_none", default)
    )]
    pub _min_scale: Option<f32>,

    /// Maximum scale that can be set.
    #[cfg_attr(feature = "egui-probe", egui_probe(range = 1.0..))]
    #[cfg_attr(
        feature = "serde",
        serde(skip_serializing_if = "Option::is_none", default)
    )]
    pub _max_scale: Option<f32>,

    /// Scale velocity when scaling with mouse wheel.
    #[cfg_attr(feature = "egui-probe", egui_probe(range = 0.0..))]
    #[cfg_attr(
        feature = "serde",
        serde(skip_serializing_if = "Option::is_none", default)
    )]
    pub _scale_velocity: Option<f32>,

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
    pub _node_frame: Option<Frame>,

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
    pub _header_frame: Option<Frame>,

    /// Enable centering by double click on background
    #[cfg_attr(
        feature = "serde",
        serde(skip_serializing_if = "Option::is_none", default)
    )]
    pub _centering: Option<bool>,

    /// Stroke for selection.
    #[cfg_attr(
        feature = "serde",
        serde(skip_serializing_if = "Option::is_none", default)
    )]
    pub _select_stoke: Option<Stroke>,

    /// Fill for selection.
    #[cfg_attr(
        feature = "serde",
        serde(skip_serializing_if = "Option::is_none", default)
    )]
    pub _select_fill: Option<Color32>,

    /// Flag to control how rect selection works.
    /// If set to true, only nodes fully contained in selection rect will be selected.
    /// If set to false, nodes intersecting with selection rect will be selected.
    pub _select_rect_contained: Option<bool>,

    /// Style for node selection.
    #[cfg_attr(
        feature = "serde",
        serde(skip_serializing_if = "Option::is_none", default)
    )]
    pub _select_style: Option<SelectionStyle>,

    #[doc(hidden)]
    #[cfg_attr(feature = "egui-probe", egui_probe(skip))]
    #[cfg_attr(feature = "serde", serde(skip_serializing, default))]
    /// Do not access other than with .., here to emulate `#[non_exhaustive(pub)]`
    pub _non_exhaustive: (),

    #[cfg_attr(feature = "serde", serde(skip_deserializing))]
    /// User events. Do not save
    pub _graph_events: T,
}

impl<T: events::GraphEventsExtend> SnarlStyle<T> {
    fn pin_size(&self, scale: f32, ui: &Ui) -> f32 {
        self._pin_size
            .zoomed(scale)
            .unwrap_or_else(|| ui.spacing().interact_size.y * 0.5)
    }

    fn pin_fill(&self, ui: &Ui) -> Color32 {
        self._pin_fill
            .unwrap_or(ui.visuals().widgets.active.bg_fill)
    }

    fn pin_stoke(&self, scale: f32, ui: &Ui) -> Stroke {
        self._pin_stroke.zoomed(scale).unwrap_or(Stroke::new(
            ui.visuals().widgets.active.bg_stroke.width,
            ui.visuals().widgets.active.bg_stroke.color,
        ))
    }

    fn pin_shape(&self) -> PinShape {
        self._pin_shape.unwrap_or(BasicPinShape::Circle).into()
    }

    fn wire_width(&self, scale: f32, ui: &Ui) -> f32 {
        self._wire_width
            .zoomed(scale)
            .unwrap_or(self.pin_size(scale, ui) * 0.2)
    }

    fn wire_frame_size(&self, scale: f32, ui: &Ui) -> f32 {
        self._wire_frame_size
            .zoomed(scale)
            .unwrap_or(self.pin_size(scale, ui) * 5.0)
    }

    fn downscale_wire_frame(&self) -> bool {
        self._downscale_wire_frame.unwrap_or(true)
    }

    fn upscale_wire_frame(&self) -> bool {
        self._upscale_wire_frame.unwrap_or(false)
    }

    fn wire_style(&self, scale: f32) -> WireStyle {
        self._wire_style.zoomed(scale).unwrap_or(WireStyle::Bezier5)
    }

    fn wire_layer(&self) -> WireLayer {
        self._wire_layer.unwrap_or(WireLayer::BehindNodes)
    }

    fn header_drag_space(&self, scale: f32, ui: &Ui) -> Vec2 {
        self._header_drag_space
            .zoomed(scale)
            .unwrap_or(vec2(ui.spacing().icon_width, ui.spacing().icon_width))
    }

    fn collapsible(&self) -> bool {
        self._collapsible.unwrap_or(true)
    }

    fn bg_frame(&self, ui: &Ui) -> Frame {
        self._bg_frame.unwrap_or(Frame::canvas(ui.style()))
    }

    fn draw_bg_pattern(&self, style: &SnarlStyle<T>, viewport: &Viewport, ui: &mut Ui) {
        match &self._bg_pattern {
            None => BackgroundPattern::new().draw(style, viewport, ui),
            Some(pattern) => pattern.draw(style, viewport, ui),
        }
    }

    fn bg_pattern_stroke(&self, scale: f32, ui: &Ui) -> Stroke {
        self._bg_pattern_stroke
            .zoomed(scale)
            .unwrap_or(ui.visuals().widgets.noninteractive.bg_stroke)
    }

    fn min_scale(&self) -> f32 {
        self._min_scale.unwrap_or(0.2)
    }

    fn max_scale(&self) -> f32 {
        self._max_scale.unwrap_or(5.0)
    }

    fn scale_velocity(&self) -> f32 {
        self._scale_velocity.unwrap_or(0.005)
    }

    fn node_frame(&self, scale: f32, ui: &Ui) -> Frame {
        self._node_frame
            .zoomed(scale)
            .unwrap_or_else(|| Frame::window(ui.style()))
    }

    fn header_frame(&self, scale: f32, ui: &Ui) -> Frame {
        self._header_frame.zoomed(scale).unwrap_or_else(|| {
            self.node_frame(scale, ui)
                .shadow(Shadow::NONE)
                .fill(Color32::TRANSPARENT)
        })
    }

    fn centering(&self) -> bool {
        self._centering.unwrap_or(true)
    }

    fn select_stroke(&self, scale: f32, ui: &Ui) -> Stroke {
        self._select_stoke.zoomed(scale).unwrap_or(Stroke::new(
            ui.visuals().selection.stroke.width,
            ui.visuals().selection.stroke.color.gamma_multiply(0.5),
        ))
    }

    fn select_fill(&self, ui: &Ui) -> Color32 {
        self._select_fill
            .unwrap_or(ui.visuals().selection.bg_fill.gamma_multiply(0.3))
    }

    fn select_rect_contained(&self) -> bool {
        self._select_rect_contained.unwrap_or(false)
    }

    fn select_style(&self, scale: f32, ui: &Ui) -> SelectionStyle {
        self._select_style.zoomed(scale).unwrap_or(SelectionStyle {
            margin: ui.spacing().window_margin,
            rounding: ui.visuals().window_rounding,
            fill: self.select_fill(ui),
            stroke: self.select_stroke(scale, ui),
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

impl<T: events::GraphEventsExtend> SnarlStyle<T> {
    /// Creates new [`SnarlStyle`] filled with default values.
    #[must_use]
    pub const fn new(graph_events: T) -> Self {
        SnarlStyle {
            _pin_size: None,
            _pin_fill: None,
            _pin_stroke: None,
            _pin_shape: None,
            _wire_width: None,
            _wire_frame_size: None,
            _downscale_wire_frame: None,
            _upscale_wire_frame: None,
            _wire_style: None,
            _wire_layer: None,
            _header_drag_space: None,
            _collapsible: None,

            _bg_frame: None,
            _bg_pattern: None,
            _bg_pattern_stroke: None,

            _min_scale: None,
            _max_scale: None,
            _scale_velocity: None,
            _node_frame: None,
            _header_frame: None,
            _centering: None,
            _select_stoke: None,
            _select_fill: None,
            _select_rect_contained: None,
            _select_style: None,

            _non_exhaustive: (),
            _graph_events: graph_events,
        }
    }
}

impl Default for SnarlStyle<events::DefaultGraphEvents> {
    #[inline]
    fn default() -> Self {
        Self::new(events::DefaultGraphEvents::default())
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
    rect: Rect,
    response: egui::Response,
}

struct PinResponse {
    pos: Pos2,
    pin_fill: Color32,
    wire_style: Option<WireStyle>,
}

#[derive(Debug, Clone)]
/// Inner response
pub struct SnarlResponse {
    /// Event only on background
    pub event_on_background: Option<egui::Response>,
    /// Event on node
    pub event_on_node: Option<(NodeId, egui::Response)>,
    /// Event on wire
    pub event_on_wire: Option<((OutPinId, InPinId), egui::Response)>,
}

impl<T> Snarl<T> {
    fn draw_background<E: events::GraphEventsExtend>(
        style: &SnarlStyle<E>,
        snarl_state: &SnarlState,
        viewport: &Rect,
        ui: &mut Ui,
    ) {
        let viewport = Viewport {
            rect: *viewport,
            scale: snarl_state.scale(),
            offset: snarl_state.offset(),
        };

        style.draw_bg_pattern(style, &viewport, ui);
    }

    /// Render [`Snarl`] using given viewer and style into the [`Ui`].

    pub fn show<V, E: events::GraphEventsExtend>(
        &mut self,
        viewer: &mut V,
        style: &mut SnarlStyle<E>,
        id_source: impl Hash,
        ui: &mut Ui,
    ) -> SnarlResponse
    where
        V: SnarlViewer<T>,
    {
        #![allow(clippy::too_many_lines)]

        let snarl_id = ui.make_persistent_id(id_source);

        // Draw background pattern.
        let bg_frame = style.bg_frame(ui);

        let input = ui.ctx().input(|i| Input {
            scroll_delta: i.raw_scroll_delta.y,
            hover_pos: i.pointer.hover_pos(),
            interact_pos: i.pointer.interact_pos(),
            modifiers: i.modifiers,
            // primary_pressed: i.pointer.primary_pressed(),
            secondary_pressed: i.pointer.secondary_pressed(),
        });

        let input_state = ui.ctx().input(|is| is.clone());

        let bg_response = bg_frame.show(ui, |ui| {
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
            Self::draw_background(style, &snarl_state, &viewport, ui);

            let wire_frame_size = style.wire_frame_size(snarl_state.scale(), ui);
            let wire_width = style.wire_width(snarl_state.scale(), ui);
            let node_frame = style.node_frame(snarl_state.scale(), ui);
            let header_frame = style.header_frame(snarl_state.scale(), ui);

            let wire_shape_idx = match style.wire_layer() {
                WireLayer::BehindNodes => Some(ui.painter().add(Shape::Noop)),
                WireLayer::AboveNodes => None,
            };

            // Zooming
            match input.hover_pos {
                Some(hover_pos) if viewport.contains(hover_pos) => {
                    if input.scroll_delta != 0.0 {
                        let new_scale = (snarl_state.scale()
                            * (1.0 + input.scroll_delta * style.scale_velocity()))
                        .clamp(style.min_scale(), style.max_scale());

                        snarl_state.set_scale(new_scale);
                    }
                }
                _ => {}
            }

            let mut input_info = HashMap::new();
            let mut output_info = HashMap::new();

            let mut pin_hovered = None;

            let draw_order = snarl_state.update_draw_order(self);
            let mut drag_released = false; //TODO!: Remove drag to user event

            let mut centers_sum = vec2(0.0, 0.0);
            let mut centers_weight = 0;

            let mut node_rects = Vec::new();

            let mut node_response = None;

            for node_idx in draw_order {
                // show_node(node_idx);
                let response = self.draw_node(
                    ui,
                    node_idx,
                    viewer,
                    &mut snarl_state,
                    style,
                    snarl_id,
                    &node_frame,
                    &header_frame,
                    &mut input_info,
                    &input_state,
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

                    centers_sum += response.rect.center().to_vec2();
                    centers_weight += 1;

                    if snarl_state.is_rect_selection() {
                        node_rects.push((node_idx, response.rect));
                    }
                    // Only one node is hovered at the same time. Even if they overlap
                    if response.response.hovered() {
                        node_response = Some((node_idx, response.response));
                    }
                }
            }

            let mut hovered_wire = None;
            let mut hovered_wire_disconnect = false;
            let mut wire_shapes = Vec::new();
            let mut wire_hit = false;

            let mut wire_response = None;

            for wire in self.wires.iter() {
                let from_r = &output_info[&wire.out_pin];
                let to_r = &input_info[&wire.in_pin];

                //
                if !wire_hit && !snarl_state.has_new_wires() && bg_r.hovered() && !bg_r.dragged() {
                    // Try to find hovered wire
                    // If not draggin new wire
                    // And not hovering over item above.

                    if let Some(interact_pos) = input.interact_pos {
                        wire_hit = hit_wire(
                            interact_pos,
                            wire_frame_size,
                            style.upscale_wire_frame(),
                            style.downscale_wire_frame(),
                            from_r.pos,
                            to_r.pos,
                            wire_width.max(1.5),
                            pick_wire_style(
                                style.wire_style(snarl_state.scale()),
                                from_r.wire_style,
                                to_r.wire_style,
                            )
                            .zoomed(snarl_state.scale()),
                        );

                        if wire_hit {
                            hovered_wire = Some(wire);

                            //Remove hovered wire by second click
                            hovered_wire_disconnect |=
                                style._graph_events.remove_hovered_wire(&bg_r, &input_state);

                            wire_response = Some(((wire.out_pin, wire.in_pin), bg_r.clone()));
                            // Background is not hovered then.
                            bg_r.hovered = false;
                            bg_r.clicked = false;
                        }
                    }
                }

                let color = mix_colors(from_r.pin_fill, to_r.pin_fill);

                let mut draw_width = wire_width;
                if hovered_wire == Some(wire) {
                    draw_width *= 1.5;
                }

                draw_wire(
                    ui,
                    &mut wire_shapes,
                    wire_frame_size,
                    style.upscale_wire_frame(),
                    style.downscale_wire_frame(),
                    from_r.pos,
                    to_r.pos,
                    Stroke::new(draw_width, color),
                    pick_wire_style(
                        style.wire_style(snarl_state.scale()),
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

            // Start select area
            if style._graph_events.start_select_area(&bg_r, &input_state) {
                let screen_pos = input.interact_pos.unwrap_or(viewport.center());
                let graph_pos = snarl_state.screen_pos_to_graph(screen_pos, viewport);
                snarl_state.start_rect_selection(graph_pos);
            }

            // Move area
            if style._graph_events.move_area(&bg_r, &input_state) {
                if snarl_state.is_rect_selection() && input.hover_pos.is_some() {
                    let screen_pos = input.hover_pos.unwrap();
                    let graph_pos = snarl_state.screen_pos_to_graph(screen_pos, viewport);
                    snarl_state.update_rect_selection(graph_pos);
                } else {
                    snarl_state.pan(-bg_r.drag_delta());
                }
            }

            // Stop select area
            if style._graph_events.start_select_area(&bg_r, &input_state) {
                if let Some(select_rect) = snarl_state.rect_selection() {
                    let select_nodes = node_rects.into_iter().filter_map(|(id, rect)| {
                        let select = match style.select_rect_contained() {
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
                    style.select_fill(ui),
                    style.select_stroke(snarl_state.scale(), ui),
                );
            }

            // If right button is clicked while new wire is being dragged, cancel it.
            // This is to provide way to 'not open' the link graph node menu, but just
            // releasing the new wire to empty space.
            //
            // This uses `button_down` directly, instead of `clicked_by` to improve
            // responsiveness of the cancel action.
            // Cancel new wire
            if snarl_state.has_new_wires()
                && style._graph_events.cancel_new_wire(&bg_r, &input_state)
            {
                let _ = snarl_state.take_wires();
                bg_r.clicked = false;
            }

            //Do centering
            if style.centering() && style._graph_events.do_centering(&bg_r, &input_state) {
                centers_sum /= centers_weight as f32;
                snarl_state.set_offset(centers_sum * snarl_state.scale());
            }

            //Deselect all nodes
            if style._graph_events.deselect_all_nodes(&bg_r, &input_state) {
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
                } else {
                    if snarl_state.is_link_menu_open() || viewer.has_graph_menu(interact_pos, self)
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
                            style.upscale_wire_frame(),
                            style.downscale_wire_frame(),
                            from_pos,
                            to_r.pos,
                            Stroke::new(wire_width, to_r.pin_fill),
                            to_r.wire_style
                                .zoomed(snarl_state.scale())
                                .unwrap_or(style.wire_style(snarl_state.scale())),
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
                            style.upscale_wire_frame(),
                            style.downscale_wire_frame(),
                            from_r.pos,
                            to_pos,
                            Stroke::new(wire_width, from_r.pin_fill),
                            from_r
                                .wire_style
                                .zoomed(snarl_state.scale())
                                .unwrap_or(style.wire_style(snarl_state.scale())),
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
                ui.ctx().request_repaint();
                snarl_state.node_to_top(node);
            }

            if let Some((node, delta)) = node_moved {
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

            snarl_state.store(ui.ctx());
            SnarlResponse {
                event_on_background: if bg_r.hovered() { Some(bg_r) } else { None },
                event_on_node: node_response,
                event_on_wire: wire_response,
            }
        });
        bg_response.inner
    }

    //First step for split big function to parts
    /// Draw one node. Return Pins info
    #[inline]
    #[allow(clippy::too_many_lines)]
    #[allow(clippy::too_many_arguments)]
    fn draw_node<V, E: events::GraphEventsExtend>(
        &mut self,
        ui: &mut Ui,
        node: NodeId,
        viewer: &mut V,
        snarl_state: &mut SnarlState,
        style: &mut SnarlStyle<E>,
        snarl_id: Id,
        node_frame: &Frame,
        header_frame: &Frame,
        input_positions: &mut HashMap<InPinId, PinResponse>,
        input: &InputState,
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

        let node_pos = snarl_state.graph_pos_to_screen(pos, viewport);

        // Generate persistent id for the node.
        let node_id = snarl_id.with(("snarl-node", node));

        let openness = ui.ctx().animate_bool(node_id, open);

        let mut node_state = NodeState::load(ui.ctx(), node_id, ui.spacing(), snarl_state.scale());

        let node_rect = node_state.node_rect(node_pos, openness);

        let mut node_to_top = None;
        let mut node_moved = None;
        let mut drag_released = false; //TODO!: Remove to user event
        let mut pin_hovered = None;

        // Rect for node + frame margin.
        let node_frame_rect = node_frame.total_margin().expand_rect(node_rect);

        if snarl_state.selected_nodes().contains(&node) {
            let select_style = style.select_style(snarl_state.scale(), ui);

            let select_rect = select_style.margin.expand_rect(node_frame_rect);

            ui.painter().rect(
                select_rect,
                select_style.rounding,
                select_style.fill,
                select_style.stroke,
            );
        }

        let pin_size = style.pin_size(snarl_state.scale(), ui);

        let header_drag_space = style.header_drag_space(snarl_state.scale(), ui);

        let inputs = (0..inputs_count)
            .map(|idx| InPin::new(self, InPinId { node, input: idx }))
            .collect::<Vec<_>>();

        let outputs = (0..outputs_count)
            .map(|idx| OutPin::new(self, OutPinId { node, output: idx }))
            .collect::<Vec<_>>();

        // Interact with node frame.
        let r = ui.interact(node_frame_rect, node_id, Sense::click_and_drag());

        // Node move
        if style._graph_events.node_move(&r, input) {
            node_moved = Some((node, snarl_state.screen_vec_to_graph(r.drag_delta())));
        }

        //Select one node
        if style._graph_events.select_one_node(&r, input) {
            snarl_state.select_one_node(input.modifiers.command, node);
        } else if style._graph_events.deselect_one_node(&r, input) {
            //Deselect one node
            snarl_state.deselect_one_node(node);
        }

        // Node to top
        if style._graph_events.not_to_top(&r, input) {
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
        let node_response = r.clone();

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

        let node_ui = &mut ui.child_ui_with_id_source(
            node_frame_rect,
            Layout::top_down(Align::Center),
            ("node", node_id),
        );

        let r = node_frame.show(node_ui, |ui| {
            // Render header frame.
            let mut header_rect = node_rect;

            let mut header_frame_rect = header_frame.total_margin().expand_rect(header_rect);

            // Show node's header
            let header_ui = &mut ui.child_ui_with_id_source(
                header_frame_rect,
                Layout::top_down(Align::Center),
                "header",
            );

            header_frame.show(header_ui, |ui: &mut Ui| {
                ui.with_layout(Layout::left_to_right(Align::Min), |ui| {
                    if style.collapsible() {
                        //This is not need to bee customization - ???
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

                header_frame_rect = header_frame.total_margin().expand_rect(header_rect);

                ui.advance_cursor_after_rect(Rect::from_min_max(
                    header_rect.min,
                    pos2(
                        f32::max(header_rect.max.x, node_rect.max.x),
                        header_rect.min.y,
                    ),
                ));
            });
            let header_rect = header_rect;
            ui.expand_to_include_rect(header_rect);
            let header_size = header_rect.size();
            node_state.set_header_height(header_size.y);

            if !self.nodes.contains(node.0) {
                // If removed
                return;
            }

            let min_pin_y = header_rect.center().y;

            let input_x = node_frame_rect.left() + node_frame.inner_margin.left + pin_size;

            let output_x = node_frame_rect.right() - node_frame.inner_margin.right - pin_size;

            // Input/output pin block

            if (openness < 1.0 && open) || (openness > 0.0 && !open) {
                ui.ctx().request_repaint();
            }

            // Pins are placed under the header and must not go outside of the header frame.
            let payload_rect = Rect::from_min_max(
                pos2(
                    header_rect.min.x,
                    header_frame_rect.max.y + ui.spacing().item_spacing.y
                        - node_state.payload_offset(openness),
                ),
                pos2(f32::max(node_rect.max.x, header_rect.max.x), f32::INFINITY),
            );

            let payload_clip_rect = Rect::from_min_max(
                pos2(header_rect.min.x, header_frame_rect.max.y),
                pos2(f32::max(node_rect.max.x, header_rect.max.x), f32::INFINITY),
            );

            // Show input pins.

            // Input pins on the left.
            let inputs_ui = &mut ui.child_ui_with_id_source(
                payload_rect,
                Layout::top_down(Align::Min),
                "inputs",
            );

            inputs_ui.set_clip_rect(payload_clip_rect.intersect(viewport));

            for in_pin in &inputs {
                // Show input pin.
                inputs_ui.with_layout(Layout::left_to_right(Align::Min), |ui| {
                    let input = ui.input(|i| i.clone());
                    // Allocate space for pin shape.
                    let (pin_id, _) = ui.allocate_space(vec2(pin_size * 1.5, pin_size * 1.5));

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
                    let r = ui.interact(
                        Rect::from_center_size(pin_pos, vec2(pin_size, pin_size)),
                        pin_id,
                        Sense::click_and_drag(),
                    );

                    // Remove or drop new wire
                    if style._graph_events.remove_wire(&r, &input) {
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

                    // Start drag wire
                    if style._graph_events.start_drag_wire(&r, &input) {
                        todo!("Rewrite to grap event usage");
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
                    // Stop drag wire
                    if r.drag_stopped() {
                        drag_released = true;
                    }

                    let mut pin_size = pin_size;

                    match input.pointer.hover_pos() {
                        Some(hover_pos) if r.rect.contains(hover_pos) => {
                            if input.modifiers.shift {
                                snarl_state.add_new_wire_in(in_pin.id);
                            } else if input.pointer.secondary_clicked() {
                                snarl_state.remove_new_wire_in(in_pin.id);
                            }
                            pin_hovered = Some(AnyPin::In(in_pin.id));
                            pin_size *= 1.2;
                        }
                        _ => {}
                    }

                    let pin_fill = pin_info.fill.unwrap_or(style.pin_fill(ui));

                    draw_pin(
                        ui.painter(),
                        pin_info.shape.as_ref().unwrap_or(&style.pin_shape()),
                        pin_fill,
                        pin_info
                            .stroke
                            .zoomed(snarl_state.scale())
                            .unwrap_or(style.pin_stoke(snarl_state.scale(), ui)),
                        pin_pos,
                        pin_size,
                    );

                    input_positions.insert(
                        in_pin.id,
                        PinResponse {
                            pos: pin_pos,
                            pin_fill,
                            wire_style: pin_info.wire_style,
                        },
                    );
                });
            }
            let inputs_rect = inputs_ui.min_rect();
            ui.expand_to_include_rect(inputs_rect.intersect(payload_clip_rect));
            let inputs_size = inputs_rect.size();

            if !self.nodes.contains(node.0) {
                // If removed
                return;
            }

            // Show output pins.

            // Outputs are placed under the header and must not go outside of the header frame.

            let outputs_ui = &mut ui.child_ui_with_id_source(
                payload_rect,
                Layout::top_down(Align::Max),
                "outputs",
            );

            outputs_ui.set_clip_rect(payload_clip_rect.intersect(viewport));

            // Output pins on the right.
            for out_pin in &outputs {
                // Show output pin.
                outputs_ui.with_layout(Layout::right_to_left(Align::Min), |ui| {
                    // Allocate space for pin shape.

                    let input = ui.input(|i| i.clone());

                    let (pin_id, _) = ui.allocate_space(vec2(pin_size * 1.5, pin_size * 1.5));

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

                    let r = ui.interact(
                        Rect::from_center_size(pin_pos, vec2(pin_size, pin_size)),
                        pin_id,
                        Sense::click_and_drag(),
                    );
                    // Remove or drop new wire
                    if style._graph_events.remove_wire(&r, &input) {
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

                    // Start drag wire
                    if style._graph_events.start_drag_wire(&r, &input) {
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

                    // Stop drag wire
                    if style._graph_events.stop_drag_wire(&r, &input) {
                        drag_released = true;
                    }

                    let mut pin_size = pin_size;
                    match input.pointer.hover_pos() {
                        Some(hover_pos) if r.rect.contains(hover_pos) => {
                            if input.modifiers.shift {
                                snarl_state.add_new_wire_out(out_pin.id);
                            } else if input.pointer.secondary_pressed() {
                                snarl_state.remove_new_wire_out(out_pin.id);
                            }
                            pin_hovered = Some(AnyPin::Out(out_pin.id));
                            pin_size *= 1.2;
                        }
                        _ => {}
                    }

                    let pin_fill = pin_info.fill.unwrap_or(style.pin_fill(ui));

                    draw_pin(
                        ui.painter(),
                        pin_info.shape.as_ref().unwrap_or(&style.pin_shape()),
                        pin_fill,
                        pin_info
                            .stroke
                            .zoomed(snarl_state.scale())
                            .unwrap_or(style.pin_stoke(snarl_state.scale(), ui)),
                        pin_pos,
                        pin_size,
                    );

                    output_positions.insert(
                        out_pin.id,
                        PinResponse {
                            pos: pin_pos,
                            pin_fill,
                            wire_style: pin_info.wire_style,
                        },
                    );
                });
            }
            let outputs_rect = outputs_ui.min_rect();
            ui.expand_to_include_rect(outputs_rect.intersect(payload_clip_rect));
            let outputs_size = outputs_rect.size();

            if !self.nodes.contains(node.0) {
                // If removed
                return;
            }

            let mut new_pins_size = vec2(
                inputs_size.x + outputs_size.x + ui.spacing().item_spacing.x,
                f32::max(inputs_size.y, outputs_size.y),
            );

            let mut pins_bottom = f32::max(inputs_rect.bottom(), outputs_rect.bottom());

            // Show body if there's one.
            if viewer.has_body(&self.nodes.get(node.0).unwrap().value) {
                let body_left = inputs_rect.right() + ui.spacing().item_spacing.x;
                let body_right = outputs_rect.left() - ui.spacing().item_spacing.x;
                let body_top = payload_rect.top();

                let mut body_rect =
                    Rect::from_min_max(pos2(body_left, body_top), pos2(body_right, f32::INFINITY));
                body_rect = node_state.align_body(body_rect);

                let mut body_ui = ui.child_ui_with_id_source(
                    body_rect,
                    Layout::left_to_right(Align::Min),
                    "body",
                );
                body_ui.set_clip_rect(payload_clip_rect.intersect(viewport));

                viewer.show_body(
                    node,
                    &inputs,
                    &outputs,
                    &mut body_ui,
                    snarl_state.scale(),
                    self,
                );

                body_rect = body_ui.min_rect();
                ui.expand_to_include_rect(body_rect.intersect(payload_clip_rect));
                let body_size = body_rect.size();
                node_state.set_body_width(body_size.x);

                new_pins_size.x += body_size.x + ui.spacing().item_spacing.x;
                new_pins_size.y = f32::max(new_pins_size.y, body_size.y);

                pins_bottom = f32::max(pins_bottom, body_rect.bottom());

                if !self.nodes.contains(node.0) {
                    // If removed
                    return;
                }
            }

            if viewer.has_footer(&self.nodes[node.0].value) {
                let footer_left = node_rect.left();
                let footer_right = node_rect.right();
                let footer_top = pins_bottom + ui.spacing().item_spacing.y;

                let mut footer_rect = Rect::from_min_max(
                    pos2(footer_left, footer_top),
                    pos2(footer_right, f32::INFINITY),
                );

                footer_rect = node_state.align_footer(footer_rect);

                let mut footer_ui = ui.child_ui_with_id_source(
                    footer_rect,
                    Layout::left_to_right(Align::Min),
                    "footer",
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

                footer_rect = footer_ui.min_rect();
                ui.expand_to_include_rect(footer_rect.intersect(payload_clip_rect));
                let footer_size = footer_rect.size();
                node_state.set_footer_width(footer_size.x);

                new_pins_size.x = f32::max(new_pins_size.x, footer_size.x);
                new_pins_size.y += footer_size.y + ui.spacing().item_spacing.y;

                if !self.nodes.contains(node.0) {
                    // If removed
                    return;
                }
            }

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
            rect: final_rect,
            response: node_response,
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
