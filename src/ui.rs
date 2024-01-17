use std::{collections::HashMap, hash::Hash};

use egui::{
    collapsing_header::paint_default_icon, epaint::Shadow, pos2, vec2, Align, Color32, Frame, Id,
    Layout, Modifiers, PointerButton, Pos2, Rect, Sense, Shape, Stroke, Style, Ui, Vec2,
};

use crate::{InPin, InPinId, Node, OutPin, OutPinId, Snarl};

mod background_pattern;
mod pin;
mod state;
mod viewer;
mod wire;
mod zoom;

use self::{
    background_pattern::BackgroundPattern,
    pin::draw_pin,
    state::{NewWires, NodeState, SnarlState},
    wire::{draw_wire, hit_wire, mix_colors},
};

pub use self::{
    pin::{AnyPin, PinInfo, PinShape},
    viewer::SnarlViewer,
    wire::WireLayer,
    zoom::Zoom,
};

/// Style for rendering Snarl.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SnarlStyle {
    /// Size of pins.
    pub pin_size: Option<f32>,

    /// Width of wires.
    pub wire_width: Option<f32>,

    /// Size of wire frame which controls curvature of wires.
    pub wire_frame_size: Option<f32>,

    /// Whether to downscale wire frame when nodes are close.
    pub downscale_wire_frame: bool,

    /// Weather to upscale wire frame when nodes are close.
    pub upscale_wire_frame: bool,

    pub wire_layer: WireLayer,
    pub header_drag_space: Option<Vec2>,
    pub input_output_spacing: Option<f32>,
    pub collapsible: bool,

    pub bg_fill: Option<Color32>,
    pub bg_pattern: BackgroundPattern,
    pub background_pattern_stroke: Option<Stroke>,

    pub min_scale: f32,
    pub max_scale: f32,
    pub scale_velocity: f32,
}

impl SnarlStyle {
    pub const fn new() -> Self {
        SnarlStyle {
            pin_size: None,
            wire_width: None,
            wire_frame_size: None,
            downscale_wire_frame: false,
            upscale_wire_frame: true,
            wire_layer: WireLayer::BehindNodes,
            header_drag_space: None,
            input_output_spacing: None,
            collapsible: true,

            bg_fill: None,
            bg_pattern: BackgroundPattern::new(),
            background_pattern_stroke: None,

            min_scale: 0.1,
            max_scale: 2.0,
            scale_velocity: 0.005,
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
    node_moved: Option<(usize, Vec2)>,
    node_idx_to_top: Option<usize>,
    drag_released: bool,
    pin_hovered: Option<AnyPin>,
}

impl<T> Snarl<T> {
    fn draw_background(
        &mut self,
        style: &SnarlStyle,
        snarl_state: &SnarlState,
        viewport: &Rect,
        ui: &mut Ui,
    ) {
        style.bg_pattern.draw(style, snarl_state, viewport, ui);
    }

    /// Render [`Snarl`] using given viewer and style into the [`Ui`].
    pub fn show<V>(&mut self, viewer: &mut V, style: &SnarlStyle, id_source: impl Hash, ui: &mut Ui)
    where
        V: SnarlViewer<T>,
    {
        let mut node_moved = None;
        let mut node_idx_to_top = None;

        let snarl_id = ui.make_persistent_id(id_source);

        // Draw background pattern.
        let bg_fill = style
            .bg_fill
            .unwrap_or_else(|| ui.visuals().widgets.noninteractive.bg_fill);

        let bg_stroke = style
            .background_pattern_stroke
            .unwrap_or_else(|| ui.visuals().widgets.noninteractive.bg_stroke);

        let input = ui.ctx().input(|i| Input {
            scroll_delta: i.scroll_delta.y,
            hover_pos: i.pointer.hover_pos(),
            modifiers: i.modifiers,
            // primary_pressed: i.pointer.primary_pressed(),
            secondary_pressed: i.pointer.secondary_pressed(),
        });

        Frame::none()
            .fill(bg_fill)
            .stroke(bg_stroke)
            .show(ui, |ui| {
                let viewport = ui.max_rect();
                ui.set_clip_rect(viewport);

                let pivot = input.hover_pos.unwrap_or_else(|| viewport.center());

                let mut snarl_state =
                    SnarlState::load(ui.ctx(), snarl_id, pivot, viewport, self, style);

                let mut node_style: Style = (**ui.style()).clone();
                node_style.zoom(snarl_state.scale());

                //Draw background
                self.draw_background(style, &snarl_state, &viewport, ui);

                let pin_size = style
                    .pin_size
                    .unwrap_or(node_style.spacing.interact_size.y * 0.5);

                let wire_frame_size = style.wire_frame_size.unwrap_or(pin_size * 5.0);
                let wire_width = style.wire_width.unwrap_or(pin_size * 0.2);

                let node_frame = Frame::window(&node_style);
                let header_frame = node_frame.shadow(Shadow::NONE);

                let wire_shape_idx = match style.wire_layer {
                    WireLayer::BehindNodes => Some(ui.painter().add(Shape::Noop)),
                    WireLayer::AboveNodes => None,
                };

                let mut bg_r = ui.allocate_rect(viewport, Sense::click_and_drag());

                // Zooming
                match input.hover_pos {
                    Some(hover_pos) if bg_r.rect.contains(hover_pos) => {
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

                let draw_order = self.draw_order.clone();
                let mut drag_released = false;

                for node_idx in draw_order {
                    // show_node(node_idx);
                    let response = self.draw_node(
                        ui,
                        node_idx,
                        viewer,
                        &mut snarl_state,
                        style,
                        &viewport,
                        snarl_id,
                        &node_style,
                        &node_frame,
                        &header_frame,
                        &mut input_info,
                        &input,
                        &mut output_info,
                    );
                    if let Some(v) = response.node_idx_to_top {
                        node_idx_to_top = Some(v);
                    }
                    if let Some(v) = response.node_moved {
                        node_moved = Some(v);
                    }
                    if let Some(v) = response.pin_hovered {
                        pin_hovered = Some(v);
                    }
                    drag_released |= response.drag_released;
                }

                let mut hovered_wire = None;
                let mut hovered_wire_disconnect = false;
                let mut wire_shapes = Vec::new();

                for wire in self.wires.iter() {
                    let (from, color_from) = output_info[&wire.out_pin];
                    let (to, color_to) = input_info[&wire.in_pin];

                    if !snarl_state.has_new_wires() && bg_r.hovered() {
                        // Try to find hovered wire
                        // If not draggin new wire
                        // And not hovering over item above.

                        if let Some(hover_pos) = input.hover_pos {
                            let hit = hit_wire(
                                hover_pos,
                                wire_frame_size,
                                style.upscale_wire_frame,
                                style.downscale_wire_frame,
                                from,
                                to,
                                wire_width.max(1.5),
                            );

                            if hit {
                                hovered_wire = Some(wire);

                                //Remove hovered wire by second click
                                hovered_wire_disconnect |=
                                    bg_r.clicked_by(PointerButton::Secondary);

                                // Background is not hovered then.
                                bg_r.hovered = false;
                                bg_r.clicked = [false; egui::NUM_POINTER_BUTTONS];
                                bg_r.double_clicked = [false; egui::NUM_POINTER_BUTTONS];
                                bg_r.triple_clicked = [false; egui::NUM_POINTER_BUTTONS];
                            }
                        }
                    }

                    let color =
                        mix_colors(color_from, color_to);

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
                        from,
                        to,
                        Stroke::new(draw_width, color),
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

                if bg_r.hovered() && bg_r.dragged_by(PointerButton::Primary) {
                    snarl_state.pan(-bg_r.drag_delta());
                }
                bg_r.context_menu(|ui| {
                    viewer.graph_menu(
                        snarl_state.screen_pos_to_graph(ui.cursor().min, viewport),
                        ui,
                        snarl_state.scale(),
                        self,
                    );
                });

                match snarl_state.new_wires() {
                    None => {}
                    Some(NewWires::In(pins)) => {
                        for pin in pins {
                            let from = input.hover_pos.unwrap_or(Pos2::ZERO);
                            let (to, color) = input_info[&pin];

                            draw_wire(
                                ui,
                                &mut wire_shapes,
                                wire_frame_size,
                                style.upscale_wire_frame,
                                style.downscale_wire_frame,
                                from,
                                to,
                                Stroke::new(wire_width, color),
                            );
                        }
                    }
                    Some(NewWires::Out(pins)) => {
                        for pin in pins {
                            let (from, color) = output_info[&pin];
                            let to = input.hover_pos.unwrap_or(Pos2::ZERO);

                            draw_wire(
                                ui,
                                &mut wire_shapes,
                                wire_frame_size,
                                style.upscale_wire_frame,
                                style.downscale_wire_frame,
                                from,
                                to,
                                Stroke::new(wire_width, color),
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
                        _ => {}
                    }
                }

                ui.advance_cursor_after_rect(Rect::from_min_size(viewport.min, Vec2::ZERO));

                snarl_state.store(ui.ctx());
            });

        if let Some((node_idx, delta)) = node_moved {
            ui.ctx().request_repaint();
            let node = &mut self.nodes[node_idx];
            node.pos += delta;
        }

        if let Some(node_idx) = node_idx_to_top {
            ui.ctx().request_repaint();
            if let Some(order) = self.draw_order.iter().position(|idx| *idx == node_idx) {
                self.draw_order.remove(order);
                self.draw_order.push(node_idx);
            }
        }
    }

    //First step for split big function to parts
    /// Draw one node. Return Pins info
    #[inline(always)]
    fn draw_node<V>(
        &mut self,
        ui: &mut Ui,
        node_idx: usize,
        viewer: &mut V,
        snarl_state: &mut SnarlState,
        style: &SnarlStyle,
        viewport: &Rect,
        snarl_id: Id,
        node_style: &Style,
        node_frame: &Frame,
        header_frame: &Frame,
        input_positions: &mut HashMap<InPinId, (Pos2, Color32)>,
        input: &Input,
        output_positions: &mut HashMap<OutPinId, (Pos2, Color32)>,
    ) -> DrawNodeResponse
    where
        V: SnarlViewer<T>,
    {
        let Node {
            pos,
            open,
            ref value,
        } = self.nodes[node_idx];

        let mut response = DrawNodeResponse {
            node_idx_to_top: None,
            node_moved: None,
            drag_released: false,
            pin_hovered: None,
        };

        // Collect pins
        let inputs_count = viewer.inputs(value);
        let outputs_count = viewer.outputs(value);

        let node_pos = snarl_state.graph_pos_to_screen(pos, *viewport);

        // Generate persistent id for the node.
        let node_id = snarl_id.with(("snarl-node", node_idx));

        let openness = ui.ctx().animate_bool(node_id, open);

        let mut node_state = NodeState::load(ui.ctx(), node_id, &node_style.spacing);

        let node_rect = node_state.node_rect(node_pos);

        let pin_size = style
            .pin_size
            .unwrap_or(node_style.spacing.interact_size.y * 0.5);

        let header_drag_space = style
            .header_drag_space
            .unwrap_or_else(|| vec2(node_style.spacing.icon_width, node_style.spacing.icon_width));

        // let header_rect = node_state.header_rect(&node_style.spacing, node_pos);
        // let pins_rect =
        //     node_state.pins_rect(&header_frame, &node_style.spacing, openness, node_pos);

        // Interact with node frame.
        let r = ui.interact(node_rect, node_id, Sense::click_and_drag());

        if r.dragged_by(PointerButton::Primary) {
            response.node_moved = Some((node_idx, snarl_state.screen_vec_to_graph(r.drag_delta())));
        }
        if r.clicked() || r.dragged() {
            response.node_idx_to_top = Some(node_idx);
        }

        let inputs = (0..inputs_count)
            .map(|idx| {
                InPin::new(
                    self,
                    InPinId {
                        node: node_idx,
                        input: idx,
                    },
                )
            })
            .collect::<Vec<_>>();

        let outputs = (0..outputs_count)
            .map(|idx| {
                OutPin::new(
                    self,
                    OutPinId {
                        node: node_idx,
                        output: idx,
                    },
                )
            })
            .collect::<Vec<_>>();

        r.context_menu(|ui| {
            viewer.node_menu(node_idx, &inputs, &outputs, ui, snarl_state.scale(), self);
        });

        if !self.nodes.contains(node_idx) {
            node_state.clear(ui.ctx());
            // If removed
            return response;
        }

        // Rect for node + frame margin.
        let node_frame_rect = node_frame.total_margin().expand_rect(node_rect);

        let node_ui = &mut ui.child_ui_with_id_source(
            node_frame_rect,
            Layout::top_down(Align::Center),
            node_id,
        );
        node_ui.set_style(node_style.clone());

        node_frame.show(node_ui, |ui| {
            // Render header frame.
            let mut header_rect = node_rect;

            let mut header_frame_rect = header_frame.total_margin().expand_rect(header_rect);

            // Show node's header
            let header_ui = &mut ui.child_ui_with_id_source(
                header_frame_rect,
                Layout::top_down(Align::Center),
                node_id,
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
                            self.open_node(node_idx, !open);
                        }
                    }

                    ui.allocate_exact_size(header_drag_space, Sense::hover());

                    viewer.show_header(node_idx, &inputs, &outputs, ui, snarl_state.scale(), self);

                    header_rect.max = ui.min_rect().max;
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
            if !self.nodes.contains(node_idx) {
                node_state.clear(ui.ctx());
                // If removed
                return;
            }

            let min_pin_y = header_rect.center().y;

            let input_x = header_rect.left() + header_frame.total_margin().left + pin_size * 0.5;

            let output_x = f32::max(header_rect.right(), node_rect.right())
                - header_frame.total_margin().right
                - pin_size * 0.5;

            if true {
                if (openness < 1.0 && open) || (openness > 0.0 && !open) {
                    ui.ctx().request_repaint();
                }

                // Show input pins.

                // Inputs are placed under the header and must not go outside of the header frame.

                let pins_rect = Rect::from_min_max(
                    pos2(
                        header_rect.min.x,
                        header_frame_rect.max.y
                            + node_style.spacing.item_spacing.y
                            + (node_frame_rect.height()) * (openness - 1.0),
                    ),
                    pos2(f32::max(node_rect.max.x, header_rect.max.x), f32::INFINITY),
                );

                let pins_clip_rect = Rect::from_min_max(
                    pos2(header_rect.min.x, header_frame_rect.max.y),
                    pos2(f32::max(node_rect.max.x, header_rect.max.x), f32::INFINITY),
                );

                let inputs_ui = &mut ui.child_ui_with_id_source(
                    pins_rect,
                    Layout::top_down(Align::Center),
                    (node_id, "inputs"),
                );

                inputs_ui.set_clip_rect(pins_clip_rect.intersect(*viewport));

                // Input pins on the left.
                let r = inputs_ui.with_layout(Layout::top_down(Align::Min), |ui| {
                    // let r = Grid::new((node_id, "inputs")).min_col_width(0.0).show(ui, |ui| {
                    for in_pin in inputs {
                        // Show input pin.
                        ui.with_layout(Layout::left_to_right(Align::Min), |ui| {
                            // Allocate space for pin shape.
                            let (pin_id, _) = ui.allocate_space(vec2(pin_size, pin_size));

                            let y0 = ui.cursor().min.y;

                            // Show input content
                            let pin_info =
                                viewer.show_input(&in_pin, ui, snarl_state.scale(), self);
                            if !self.nodes.contains(node_idx) {
                                // If removed
                                return;
                            }

                            let y1 = ui.min_rect().max.y;

                            // ui.end_row();

                            // Centered vertically.
                            let y = min_pin_y.max((y0 + y1) * 0.5);

                            let pin_pos = pos2(input_x, y);

                            input_positions.insert(in_pin.id, (pin_pos, pin_info.fill));

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
                                    viewer.drop_inputs(&in_pin, self);
                                }
                            }
                            if r.drag_started_by(PointerButton::Primary) {
                                if input.modifiers.command {
                                    snarl_state.start_new_wires_out(&in_pin.remotes);
                                    if !input.modifiers.shift {
                                        self.drop_inputs(in_pin.id);
                                    }
                                } else {
                                    snarl_state.start_new_wire_in(in_pin.id);
                                }
                            }
                            if r.drag_released() {
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

                            draw_pin(ui.painter(), pin_info, pin_pos, pin_size);
                        });
                    }
                });
                let inputs_rect = r.response.rect;

                if !self.nodes.contains(node_idx) {
                    node_state.clear(ui.ctx());
                    // If removed
                    return;
                }

                // Show output pins.

                // Outputs are placed under the header and must not go outside of the header frame.

                let outputs_ui = &mut ui.child_ui_with_id_source(
                    pins_rect,
                    Layout::top_down(Align::Center),
                    (node_id, "outputs"),
                );

                outputs_ui.set_clip_rect(pins_clip_rect.intersect(*viewport));

                // Output pins on the right.
                let r = outputs_ui.with_layout(Layout::top_down(Align::Max), |ui| {
                    // let r = Grid::new((node_id, "outputs")).min_col_width(0.0).show(ui, |ui| {

                    for out_pin in outputs {
                        // Show output pin.
                        ui.with_layout(Layout::right_to_left(Align::Min), |ui| {
                            // Allocate space for pin shape.

                            let (pin_id, _) = ui.allocate_space(vec2(pin_size, pin_size));

                            let y0 = ui.cursor().min.y;

                            // Show output content
                            let pin_info =
                                viewer.show_output(&out_pin, ui, snarl_state.scale(), self);
                            if !self.nodes.contains(node_idx) {
                                // If removed
                                return;
                            }

                            let y1 = ui.min_rect().max.y;

                            // ui.end_row();

                            // Centered vertically.
                            let y = min_pin_y.max((y0 + y1) * 0.5);

                            let pin_pos = pos2(output_x, y);

                            output_positions.insert(out_pin.id, (pin_pos, pin_info.fill));

                            let r = ui.interact(
                                Rect::from_center_size(pin_pos, vec2(pin_size, pin_size)),
                                pin_id,
                                Sense::click_and_drag(),
                            );

                            if r.clicked_by(PointerButton::Secondary) {
                                if snarl_state.has_new_wires() {
                                    snarl_state.remove_new_wire_out(out_pin.id);
                                } else {
                                    viewer.drop_outputs(&out_pin, self);
                                }
                            }
                            if r.drag_started_by(PointerButton::Primary) {
                                if input.modifiers.command {
                                    snarl_state.start_new_wires_in(&out_pin.remotes);

                                    if !input.modifiers.shift {
                                        self.drop_outputs(out_pin.id);
                                    }
                                } else {
                                    snarl_state.start_new_wire_out(out_pin.id);
                                }
                            }
                            if r.drag_released() {
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
                            draw_pin(ui.painter(), pin_info, pin_pos, pin_size);
                        });
                    }
                });
                let outputs_rect = r.response.rect;

                if !self.nodes.contains(node_idx) {
                    node_state.clear(ui.ctx());
                    // If removed
                    return;
                }

                ui.expand_to_include_rect(header_rect);
                ui.expand_to_include_rect(inputs_rect.intersect(pins_clip_rect));
                ui.expand_to_include_rect(outputs_rect.intersect(pins_clip_rect));

                let inputs_size = inputs_rect.size();
                let outputs_size = outputs_rect.size();
                let header_size = header_rect.size();

                node_state.set_size(vec2(
                    f32::max(
                        header_size.x,
                        inputs_size.x + outputs_size.x + node_style.spacing.item_spacing.x,
                    ),
                    header_size.y
                        + header_frame.total_margin().bottom
                        + node_style.spacing.item_spacing.y
                        + f32::max(inputs_size.y, outputs_size.y),
                ));

                // ui.painter()
                //     .debug_rect(header_frame_rect, Color32::GREEN, "header");
                // ui.painter()
                //     .debug_rect(pins_clip_rect, Color32::RED, "pins_clip");
            } else {
                for in_pin in inputs {
                    let pin_pos = pos2(input_x, min_pin_y);
                    input_positions.insert(in_pin.id, (pin_pos, viewer.input_color(&in_pin, node_style, self)));

                    if !self.nodes.contains(node_idx) {
                        node_state.clear(ui.ctx());
                        // If removed
                        return;
                    }
                }
                for out_pin in outputs {
                    let pin_pos = pos2(output_x, min_pin_y);
                    output_positions.insert(out_pin.id, (pin_pos, viewer.output_color(&out_pin, node_style, self)));

                    if !self.nodes.contains(node_idx) {
                        node_state.clear(ui.ctx());
                        // If removed
                        return;
                    }
                }
            }
        });

        node_state.store(ui.ctx());
        ui.ctx().request_repaint();
        response
    }
}
