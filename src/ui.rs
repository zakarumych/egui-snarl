//! This module provides functionality for showing [`Snarl`] graph in [`Ui`].

use std::{collections::HashMap, hash::Hash};

use egui::{
    collapsing_header::paint_default_icon, epaint::Shadow, pos2, vec2, Align, Color32, Frame, Id,
    Layout, Modifiers, PointerButton, Pos2, Rect, Sense, Shape, Stroke, Style, Ui, Vec2,
};

use crate::{InPin, InPinId, Node, NodeId, OutPin, OutPinId, Snarl};

mod background_pattern;
mod pin;
mod state;
mod viewer;
mod wire;
mod zoom;

use self::{
    pin::{default_pin_fill, default_pin_stroke, draw_pin, AnyPin},
    state::{NewWires, NodeState, SnarlState},
    wire::{draw_wire, hit_wire, pick_wire_style},
    zoom::Zoom,
};

pub use self::{
    background_pattern::{BackgroundPattern, CustomBackground, Grid, Viewport},
    pin::{AnyPins, CustomPinShape, DefaultPinShape, PinInfo, PinShape},
    viewer::SnarlViewer,
    wire::{WireLayer, WireStyle},
};

/// Style for rendering Snarl.
#[derive(Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "egui-probe", derive(egui_probe::EguiProbe))]
pub struct SnarlStyle {
    /// Size of pins.
    #[cfg_attr(feature = "egui-probe", egui_probe(range = 0.0..))]
    pub pin_size: Option<f32>,

    /// Default fill color for pins.
    pub pin_fill: Option<Color32>,

    /// Default stroke for pins.
    pub pin_stroke: Option<Stroke>,

    /// Shape of pins.
    pub pin_shape: DefaultPinShape,

    /// Width of wires.
    #[cfg_attr(feature = "egui-probe", egui_probe(range = 0.0..))]
    pub wire_width: Option<f32>,

    /// Size of wire frame which controls curvature of wires.
    pub wire_frame_size: Option<f32>,

    /// Whether to downscale wire frame when nodes are close.
    pub downscale_wire_frame: bool,

    /// Weather to upscale wire frame when nodes are far.
    pub upscale_wire_frame: bool,

    /// Controls default style of wires.
    pub wire_style: WireStyle,

    /// Layer where wires are rendered.
    pub wire_layer: WireLayer,

    /// Additional blank space for dragging node by header.
    pub header_drag_space: Option<Vec2>,

    /// Whether nodes can be collapsed.
    /// If true, headers will have collapsing button.
    /// When collapsed, node will not show its pins, body and footer.
    pub collapsible: bool,

    /// Background fill color.
    /// Defaults to `ui.visuals().widgets.noninteractive.bg_fill`.
    pub bg_fill: Option<Color32>,

    /// Background pattern.
    /// Defaults to [`BackgroundPattern::Grid`].
    pub bg_pattern: BackgroundPattern,

    /// Stroke for background pattern.
    /// Defaults to `ui.visuals().widgets.noninteractive.bg_stroke`.
    pub background_pattern_stroke: Option<Stroke>,

    /// Minimum scale that can be set.
    #[cfg_attr(feature = "egui-probe", egui_probe(range = 0.0..=1.0))]
    pub min_scale: f32,

    /// Maximum scale that can be set.
    #[cfg_attr(feature = "egui-probe", egui_probe(range = 1.0..))]
    pub max_scale: f32,

    /// Scale velocity when scaling with mouse wheel.
    #[cfg_attr(feature = "egui-probe", egui_probe(range = 0.0..))]
    pub scale_velocity: f32,

    /// Frame used to draw nodes.
    /// Defaults to [`Frame::window`] constructed from current ui's style.
    #[cfg_attr(feature = "serde", serde(with = "serde_frame_option"))]
    pub node_frame: Option<Frame>,

    /// Frame used to draw node headers.
    /// Defaults to [`node_frame`] without shadow and transparent fill.
    ///
    /// If set, it should not have shadow and fill should be either opaque of fully transparent
    /// unless layering of header fill color with node fill color is desired.
    #[cfg_attr(feature = "serde", serde(with = "serde_frame_option"))]
    pub header_frame: Option<Frame>,

    #[doc(hidden)]
    #[cfg_attr(feature = "egui-probe", egui_probe(skip))]
    /// Do not access other than with .., here to emulate `#[non_exhaustive(pub)]`
    pub _non_exhaustive: (),

    /// Enable centering by double click on background
    pub centering: bool,
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
            pin_size: None,
            pin_fill: None,
            pin_stroke: None,
            pin_shape: DefaultPinShape::Circle,
            wire_width: None,
            wire_frame_size: None,
            downscale_wire_frame: false,
            upscale_wire_frame: true,
            wire_style: WireStyle::Bezier5,
            wire_layer: WireLayer::BehindNodes,
            header_drag_space: None,
            collapsible: true,

            bg_fill: None,
            bg_pattern: background_pattern::BackgroundPattern::new(),
            background_pattern_stroke: None,

            min_scale: 0.1,
            max_scale: 2.0,
            scale_velocity: 0.005,
            node_frame: None,
            header_frame: None,

            _non_exhaustive: (),
            centering: true,
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
    center: Pos2,
}

struct PinResponse {
    pos: Pos2,
    pin_fill: Color32,
    wire_style: Option<WireStyle>,
}

impl<T> Snarl<T> {
    fn draw_background(style: &SnarlStyle, snarl_state: &SnarlState, viewport: &Rect, ui: &mut Ui) {
        let viewport = Viewport {
            rect: *viewport,
            scale: snarl_state.scale(),
            offset: snarl_state.offset(),
        };

        style.bg_pattern.draw(style, &viewport, ui);
    }

    /// Render [`Snarl`] using given viewer and style into the [`Ui`].

    pub fn show<V>(&mut self, viewer: &mut V, style: &SnarlStyle, id_source: impl Hash, ui: &mut Ui)
    where
        V: SnarlViewer<T>,
    {
        #![allow(clippy::too_many_lines)]

        let snarl_id = ui.make_persistent_id(id_source);

        // Draw background pattern.
        let bg_fill = style
            .bg_fill
            .unwrap_or_else(|| ui.visuals().widgets.noninteractive.bg_fill);

        let bg_stroke = style
            .background_pattern_stroke
            .unwrap_or_else(|| ui.visuals().widgets.noninteractive.bg_stroke);

        let input = ui.ctx().input(|i| Input {
            scroll_delta: i.raw_scroll_delta.y,
            hover_pos: i.pointer.hover_pos(),
            modifiers: i.modifiers,
            // primary_pressed: i.pointer.primary_pressed(),
            secondary_pressed: i.pointer.secondary_pressed(),
        });

        Frame::none()
            .fill(bg_fill)
            .stroke(bg_stroke)
            .show(ui, |ui| {
                let mut node_moved = None;
                let mut node_to_top = None;

                let mut bg_r = ui.allocate_rect(ui.max_rect(), Sense::click_and_drag());
                let viewport = bg_r.rect;
                ui.set_clip_rect(viewport);

                let pivot = input.hover_pos.unwrap_or_else(|| viewport.center());

                let mut snarl_state =
                    SnarlState::load(ui.ctx(), snarl_id, pivot, viewport, self, style);

                let mut node_style: Style = (**ui.style()).clone();
                node_style.zoom(snarl_state.scale());

                //Draw background
                Self::draw_background(style, &snarl_state, &viewport, ui);

                let pin_size = style
                    .pin_size
                    .zoomed(snarl_state.scale())
                    .unwrap_or(node_style.spacing.interact_size.y * 0.5);

                let wire_frame_size = style
                    .wire_frame_size
                    .zoomed(snarl_state.scale())
                    .unwrap_or(pin_size * 5.0);
                let wire_width = style
                    .wire_width
                    .zoomed(snarl_state.scale())
                    .unwrap_or(pin_size * 0.2);

                let node_frame = style
                    .node_frame
                    .zoomed(snarl_state.scale())
                    .unwrap_or_else(|| Frame::window(&node_style));

                let header_frame = style
                    .header_frame
                    .zoomed(snarl_state.scale())
                    .unwrap_or_else(|| node_frame.shadow(Shadow::NONE).fill(Color32::TRANSPARENT));

                let wire_shape_idx = match style.wire_layer {
                    WireLayer::BehindNodes => Some(ui.painter().add(Shape::Noop)),
                    WireLayer::AboveNodes => None,
                };

                // Zooming
                match input.hover_pos {
                    Some(hover_pos) if viewport.contains(hover_pos) => {
                        if input.scroll_delta != 0.0 {
                            let new_scale = (snarl_state.scale()
                                * (1.0 + input.scroll_delta * style.scale_velocity))
                                .clamp(style.min_scale, style.max_scale);

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
                let centers_weight = draw_order.len();

                for node_idx in draw_order {
                    // show_node(node_idx);
                    let response = self.draw_node(
                        ui,
                        node_idx,
                        viewer,
                        &mut snarl_state,
                        style,
                        snarl_id,
                        &node_style,
                        &node_frame,
                        &header_frame,
                        &mut input_info,
                        &input,
                        &mut output_info,
                    );
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
                    centers_sum += response.center.to_vec2();
                }

                let mut hovered_wire = None;
                let mut hovered_wire_disconnect = false;
                let mut wire_shapes = Vec::new();
                let mut wire_hit = false;

                for wire in self.wires.iter() {
                    let from_r = &output_info[&wire.out_pin];
                    let to_r = &input_info[&wire.in_pin];

                    if !wire_hit
                        && !snarl_state.has_new_wires()
                        && bg_r.hovered()
                        && !bg_r.dragged()
                    {
                        // Try to find hovered wire
                        // If not draggin new wire
                        // And not hovering over item above.

                        if let Some(hover_pos) = input.hover_pos {
                            wire_hit = hit_wire(
                                hover_pos,
                                wire_frame_size,
                                style.upscale_wire_frame,
                                style.downscale_wire_frame,
                                from_r.pos,
                                to_r.pos,
                                wire_width.max(1.5),
                                pick_wire_style(
                                    style.wire_style,
                                    from_r.wire_style,
                                    to_r.wire_style,
                                )
                                .zoomed(snarl_state.scale()),
                            );

                            if wire_hit {
                                hovered_wire = Some(wire);

                                //Remove hovered wire by second click
                                hovered_wire_disconnect |=
                                    bg_r.clicked_by(PointerButton::Secondary);

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
                        style.upscale_wire_frame,
                        style.downscale_wire_frame,
                        from_r.pos,
                        to_r.pos,
                        Stroke::new(draw_width, color),
                        pick_wire_style(
                            style.wire_style.zoomed(snarl_state.scale()),
                            from_r.wire_style,
                            to_r.wire_style,
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

                if bg_r.dragged_by(PointerButton::Primary) {
                    snarl_state.pan(-bg_r.drag_delta());
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

                //Do centering
                if style.centering && bg_r.double_clicked() {
                    centers_sum /= centers_weight as f32;
                    snarl_state.set_offset(centers_sum * snarl_state.scale());
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
                        if snarl_state.is_link_menu_open()
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
                                style.upscale_wire_frame,
                                style.downscale_wire_frame,
                                from_pos,
                                to_r.pos,
                                Stroke::new(wire_width, to_r.pin_fill),
                                to_r.wire_style
                                    .unwrap_or(style.wire_style)
                                    .zoomed(snarl_state.scale()),
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
                                style.upscale_wire_frame,
                                style.downscale_wire_frame,
                                from_r.pos,
                                to_pos,
                                Stroke::new(wire_width, from_r.pin_fill),
                                from_r
                                    .wire_style
                                    .unwrap_or(style.wire_style)
                                    .zoomed(snarl_state.scale()),
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
                snarl_state.store(ui.ctx());

                if let Some((node, delta)) = node_moved {
                    ui.ctx().request_repaint();
                    let node = &mut self.nodes[node.0];
                    node.pos += delta;
                }
            });
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
        node_style: &Style,
        node_frame: &Frame,
        header_frame: &Frame,
        input_positions: &mut HashMap<InPinId, PinResponse>,
        input: &Input,
        output_positions: &mut HashMap<OutPinId, PinResponse>,
    ) -> DrawNodeResponse
    where
        V: SnarlViewer<T>,
    {
        let Node {
            pos,
            open,
            ref value,
        } = self.nodes[node.0];

        let mut response = DrawNodeResponse {
            node_to_top: None,
            node_moved: None,
            drag_released: false,
            pin_hovered: None,
            center: pos,
        };

        let viewport = ui.max_rect();

        // Collect pins
        let inputs_count = viewer.inputs(value);
        let outputs_count = viewer.outputs(value);

        let node_pos = snarl_state.graph_pos_to_screen(pos, viewport);

        // Generate persistent id for the node.
        let node_id = snarl_id.with(("snarl-node", node));

        let openness = ui.ctx().animate_bool(node_id, open);

        let mut node_state =
            NodeState::load(ui.ctx(), node_id, &node_style.spacing, snarl_state.scale());

        let node_rect = node_state.node_rect(node_pos, openness);

        // Rect for node + frame margin.
        let node_frame_rect = node_frame.total_margin().expand_rect(node_rect);

        let pin_size = style
            .pin_size
            .zoomed(snarl_state.scale())
            .unwrap_or(node_style.spacing.interact_size.y * 0.5);

        let header_drag_space = style
            .header_drag_space
            .zoomed(snarl_state.scale())
            .unwrap_or_else(|| vec2(node_style.spacing.icon_width, node_style.spacing.icon_width));

        let inputs = (0..inputs_count)
            .map(|idx| InPin::new(self, InPinId { node, input: idx }))
            .collect::<Vec<_>>();

        let outputs = (0..outputs_count)
            .map(|idx| OutPin::new(self, OutPinId { node, output: idx }))
            .collect::<Vec<_>>();

        // Interact with node frame.
        let r = ui.interact(node_frame_rect, node_id, Sense::click_and_drag());

        if r.dragged_by(PointerButton::Primary) {
            response.node_moved = Some((node, snarl_state.screen_vec_to_graph(r.drag_delta())));
        }
        if r.clicked() || r.dragged() {
            response.node_to_top = Some(node);
        }

        if viewer.has_node_menu(&self.nodes[node.0].value) {
            r.context_menu(|ui| {
                viewer.show_node_menu(node, &inputs, &outputs, ui, snarl_state.scale(), self);
            });
        }

        if !self.nodes.contains(node.0) {
            node_state.clear(ui.ctx());
            // If removed
            return response;
        }

        if viewer.has_on_hover_popup(&self.nodes[node.0].value) {
            r.on_hover_ui_at_pointer(|ui| {
                viewer.show_on_hover_popup(node, &inputs, &outputs, ui, snarl_state.scale(), self);
            });
        }

        if !self.nodes.contains(node.0) {
            node_state.clear(ui.ctx());
            // If removed
            return response;
        }

        let node_ui = &mut ui.child_ui_with_id_source(
            node_frame_rect,
            Layout::top_down(Align::Center),
            ("node", node_id),
        );
        node_ui.set_style(node_style.clone());

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
                    if style.collapsible {
                        let (_, r) = ui.allocate_exact_size(
                            vec2(node_style.spacing.icon_width, node_style.spacing.icon_width),
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
                    header_frame_rect.max.y + node_style.spacing.item_spacing.y
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
                        response.drag_released = true;
                    }

                    let mut pin_size = pin_size;

                    match input.hover_pos {
                        Some(hover_pos) if r.rect.contains(hover_pos) => {
                            if input.modifiers.shift {
                                snarl_state.add_new_wire_in(in_pin.id);
                            } else if input.secondary_pressed {
                                snarl_state.remove_new_wire_in(in_pin.id);
                            }
                            response.pin_hovered = Some(AnyPin::In(in_pin.id));
                            pin_size *= 1.2;
                        }
                        _ => {}
                    }

                    let pin_fill = pin_info
                        .fill
                        .unwrap_or(default_pin_fill(style, &node_style.visuals));

                    draw_pin(
                        ui.painter(),
                        pin_info.shape.as_ref().unwrap_or(&style.pin_shape.into()),
                        pin_fill,
                        pin_info
                            .stroke
                            .unwrap_or(default_pin_stroke(style, &node_style.visuals)),
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
                        response.drag_released = true;
                    }

                    let mut pin_size = pin_size;
                    match input.hover_pos {
                        Some(hover_pos) if r.rect.contains(hover_pos) => {
                            if input.modifiers.shift {
                                snarl_state.add_new_wire_out(out_pin.id);
                            } else if input.secondary_pressed {
                                snarl_state.remove_new_wire_out(out_pin.id);
                            }
                            response.pin_hovered = Some(AnyPin::Out(out_pin.id));
                            pin_size *= 1.2;
                        }
                        _ => {}
                    }

                    let pin_fill = pin_info
                        .fill
                        .unwrap_or(default_pin_fill(style, &node_style.visuals));

                    draw_pin(
                        ui.painter(),
                        pin_info.shape.as_ref().unwrap_or(&style.pin_shape.into()),
                        pin_fill,
                        pin_info
                            .stroke
                            .unwrap_or(default_pin_stroke(style, &node_style.visuals)),
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
                inputs_size.x + outputs_size.x + node_style.spacing.item_spacing.x,
                f32::max(inputs_size.y, outputs_size.y),
            );

            let mut pins_bottom = f32::max(inputs_rect.bottom(), outputs_rect.bottom());

            // Show body if there's one.
            if viewer.has_body(&self.nodes.get(node.0).unwrap().value) {
                let body_left = inputs_rect.right() + node_style.spacing.item_spacing.x;
                let body_right = outputs_rect.left() - node_style.spacing.item_spacing.x;
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

                new_pins_size.x += body_size.x + node_style.spacing.item_spacing.x;
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
                let footer_top = pins_bottom + node_style.spacing.item_spacing.y;

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
                new_pins_size.y += footer_size.y + node_style.spacing.item_spacing.y;

                if !self.nodes.contains(node.0) {
                    // If removed
                    return;
                }
            }

            node_state.set_size(vec2(
                f32::max(header_size.x, new_pins_size.x),
                header_size.y
                    + header_frame.total_margin().bottom
                    + node_style.spacing.item_spacing.y
                    + new_pins_size.y,
            ));
        });

        let node_graph_rect = snarl_state.screen_rect_to_graph(r.response.rect, viewport);
        viewer.final_node_rect(node, node_graph_rect, self);
        response.center = node_graph_rect.center();

        if !self.nodes.contains(node.0) {
            ui.ctx().request_repaint();
            node_state.clear(ui.ctx());
            // If removed
            return response;
        }

        node_state.store(ui.ctx());
        ui.ctx().request_repaint();
        response
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
