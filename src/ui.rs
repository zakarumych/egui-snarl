use std::hash::Hash;

use egui::{
    ahash::{HashMap, HashMapExt},
    collapsing_header::paint_default_icon,
    emath::Rot2,
    epaint::Shadow,
    pos2, vec2, Align, Color32, Frame, Layout, PointerButton, Pos2, Rect, Sense, Shape, Stroke,
    Style, Ui, Vec2,
};

use crate::{InPinId, OutPinId, Snarl};

mod effect;
mod pin;
mod state;
mod viewer;
mod wire;
mod zoom;

pub use self::{
    effect::{Effect, Effects, Forbidden},
    pin::{AnyPin, InPin, OutPin, PinInfo, PinShape, RemoteInPin, RemoteOutPin},
    viewer::SnarlViewer,
    wire::WireLayer,
    zoom::Zoom,
};
use self::{
    pin::draw_pin,
    state::{NodeState, SnarlState},
    wire::{draw_wire, get_part_wire, hit_wire, mix_colors, set_part_wire, take_part_wire},
};

/// Background pattern show beneath nodes and wires.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum BackgroundPattern {
    /// Linear grid.
    Grid { spacing: Vec2, angle: f32 },
}

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
    pub bg_pattern: Option<BackgroundPattern>,
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
            bg_pattern: Some(BackgroundPattern::Grid {
                spacing: vec2(5.0, 5.0),
                angle: 1.0,
            }),
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

impl<T> Snarl<T> {
    /// Render [`Snarl`] using given viewer and style into the [`Ui`].
    pub fn show<V>(&mut self, viewer: &mut V, style: &SnarlStyle, id_source: impl Hash, ui: &mut Ui)
    where
        V: SnarlViewer<T>,
    {
        let mut effects = Effects::new();
        let mut node_moved = None;
        let mut node_order_to_top = None;

        self._show(
            viewer,
            style,
            id_source,
            ui,
            &mut effects,
            &mut node_moved,
            &mut node_order_to_top,
        );

        if !effects.is_empty() {
            ui.ctx().request_repaint();
            self.apply_effects(effects);
        }

        if let Some((node_idx, delta)) = node_moved {
            ui.ctx().request_repaint();
            let node = &mut self.nodes[node_idx];
            node.pos += delta;
        }

        if let Some(order) = node_order_to_top {
            ui.ctx().request_repaint();
            let node_idx = self.draw_order.remove(order);
            self.draw_order.push(node_idx);
        }
    }

    fn _show<V>(
        &self,
        viewer: &mut V,
        style: &SnarlStyle,
        id_source: impl Hash,
        ui: &mut Ui,
        effects: &mut Effects<T>,
        node_moved: &mut Option<(usize, Vec2)>,
        node_order_to_top: &mut Option<usize>,
    ) where
        V: SnarlViewer<T>,
    {
        let snarl_id = ui.make_persistent_id(id_source);

        // Draw background pattern.
        let bg_fill = style
            .bg_fill
            .unwrap_or_else(|| ui.visuals().widgets.noninteractive.bg_fill);

        let bg_stroke = style
            .background_pattern_stroke
            .unwrap_or_else(|| ui.visuals().widgets.noninteractive.bg_stroke);

        Frame::none()
            .fill(bg_fill)
            .stroke(bg_stroke)
            .show(ui, |ui| {
                let viewport = ui.max_rect();
                ui.set_clip_rect(viewport);

                let (scroll_delta, hover_pos) = ui
                    .ctx()
                    .input(|i| (i.scroll_delta.y, i.pointer.hover_pos()));

                let pivot = hover_pos.unwrap_or_else(|| viewport.center());

                let mut snarl_state =
                    SnarlState::load(ui.ctx(), snarl_id, pivot, viewport, self, style);

                let mut node_style: Style = (**ui.style()).clone();
                node_style.zoom(snarl_state.scale());

                match style.bg_pattern {
                    None => {}
                    Some(BackgroundPattern::Grid { spacing, angle }) => {
                        let stroke = Stroke::new(
                            bg_stroke.width * snarl_state.scale().max(1.0),
                            bg_stroke.color.gamma_multiply(snarl_state.scale().min(1.0)),
                        );

                        let spacing = ui.spacing().icon_width * spacing;

                        let rot = Rot2::from_angle(angle);
                        let rot_inv = rot.inverse();

                        let graph_viewport = Rect::from_min_max(
                            snarl_state.screen_pos_to_graph(viewport.min, viewport),
                            snarl_state.screen_pos_to_graph(viewport.max, viewport),
                        );

                        let pattern_bounds = graph_viewport.rotate_bb(rot_inv);

                        let min_x = (pattern_bounds.min.x / spacing.x).ceil();
                        let max_x = (pattern_bounds.max.x / spacing.x).floor();

                        for x in 0..=(max_x - min_x) as i64 {
                            let x = (x as f32 + min_x) * spacing.x;

                            let top = (rot * vec2(x, pattern_bounds.min.y)).to_pos2();
                            let bottom = (rot * vec2(x, pattern_bounds.max.y)).to_pos2();

                            let top = snarl_state.graph_pos_to_screen(top, viewport);
                            let bottom = snarl_state.graph_pos_to_screen(bottom, viewport);

                            ui.painter().line_segment([top, bottom], stroke);
                        }

                        let min_y = (pattern_bounds.min.y / spacing.y).ceil();
                        let max_y = (pattern_bounds.max.y / spacing.y).floor();

                        for y in 0..=(max_y - min_y) as i64 {
                            let y = (y as f32 + min_y) * spacing.y;

                            let top = (rot * vec2(pattern_bounds.min.x, y)).to_pos2();
                            let bottom = (rot * vec2(pattern_bounds.max.x, y)).to_pos2();

                            let top = snarl_state.graph_pos_to_screen(top, viewport);
                            let bottom = snarl_state.graph_pos_to_screen(bottom, viewport);

                            ui.painter().line_segment([top, bottom], stroke);
                        }
                    }
                }

                let pin_size = style
                    .pin_size
                    .unwrap_or_else(|| node_style.spacing.interact_size.y * 0.5);

                let wire_frame_size = style.wire_frame_size.unwrap_or(pin_size * 5.0);
                let wire_width = style.wire_width.unwrap_or_else(|| pin_size * 0.2);
                let header_drag_space = style.header_drag_space.unwrap_or_else(|| {
                    vec2(node_style.spacing.icon_width, node_style.spacing.icon_width)
                });

                let collapsible = style.collapsible;

                let node_frame = Frame::window(&node_style);
                let header_frame = node_frame.shadow(Shadow::NONE);

                let wire_shape_idx = match style.wire_layer {
                    WireLayer::BehindNodes => Some(ui.painter().add(Shape::Noop)),
                    WireLayer::AboveNodes => None,
                };

                let mut bg_r = ui.allocate_rect(viewport, Sense::click_and_drag());

                // Zooming
                match hover_pos {
                    Some(hover_pos) if bg_r.rect.contains(hover_pos) => {
                        if scroll_delta != 0.0 {
                            dbg!(scroll_delta);
                            let new_scale = (snarl_state.scale()
                                * (1.0 + scroll_delta * style.scale_velocity))
                                .clamp(style.min_scale, style.max_scale);

                            snarl_state.set_scale(new_scale);
                        }
                    }
                    _ => {}
                }

                let mut input_positions = HashMap::new();
                let mut output_positions = HashMap::new();

                let mut input_colors = HashMap::new();
                let mut output_colors = HashMap::new();

                let mut part_wire_drag_released = false;
                let mut pin_hovered = None;

                for (order, &node_idx) in self.draw_order.iter().enumerate() {
                    let node = &self.nodes[node_idx];

                    let node_pos = snarl_state.graph_pos_to_screen(node.pos, viewport);

                    // Generate persistent id for the node.
                    let node_id = snarl_id.with(("snarl-node", node_idx));

                    let openness = ui.ctx().animate_bool(node_id, node.open);

                    let node_state = NodeState::load(ui.ctx(), node_id, &node_style.spacing);

                    let mut new_state = node_state;

                    let node_rect = node_state.node_rect(node_pos);

                    // let header_rect = node_state.header_rect(&node_style.spacing, node_pos);
                    // let pins_rect =
                    //     node_state.pins_rect(&header_frame, &node_style.spacing, openness, node_pos);

                    // Interact with node frame.
                    let r = ui.interact(node_rect, node_id, Sense::click_and_drag());

                    if r.dragged_by(PointerButton::Primary) {
                        *node_moved =
                            Some((node_idx, snarl_state.screen_vec_to_graph(r.drag_delta())));
                        *node_order_to_top = Some(order);
                    } else if r.clicked_by(PointerButton::Primary) {
                        *node_order_to_top = Some(order);
                    }

                    // Collect pins
                    let inputs_count = viewer.inputs(&node.value.borrow());
                    let outputs_count = viewer.outputs(&node.value.borrow());

                    let inputs = (0..inputs_count)
                        .map(|idx| {
                            InPin::input(
                                &self,
                                InPinId {
                                    node: node_idx,
                                    input: idx,
                                },
                            )
                        })
                        .collect::<Vec<_>>();

                    let outputs = (0..outputs_count)
                        .map(|idx| {
                            OutPin::output(
                                &self,
                                OutPinId {
                                    node: node_idx,
                                    output: idx,
                                },
                            )
                        })
                        .collect::<Vec<_>>();

                    r.context_menu(|ui| {
                        viewer.node_menu(
                            node_idx,
                            &node.value,
                            &inputs,
                            &outputs,
                            ui,
                            snarl_state.scale(),
                            effects,
                        );
                    });

                    // Rect for node + frame margin.
                    let node_frame_rect = node_frame.total_margin().expand_rect(node_rect);

                    let ref mut node_ui = ui.child_ui_with_id_source(
                        node_frame_rect,
                        Layout::top_down(Align::Center),
                        node_id,
                    );
                    node_ui.set_style(node_style.clone());

                    node_frame.show(node_ui, |ui| {
                        // Render header frame.
                        let mut header_rect = node_rect;

                        let mut header_frame_rect =
                            header_frame.total_margin().expand_rect(header_rect);

                        // Show node's header
                        let ref mut header_ui = ui.child_ui_with_id_source(
                            header_frame_rect,
                            Layout::top_down(Align::Center),
                            node_id,
                        );

                        header_frame.show(header_ui, |ui: &mut Ui| {
                            // ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                            ui.horizontal(|ui| {
                                if collapsible {
                                    let (_, r) = ui.allocate_exact_size(
                                        vec2(
                                            node_style.spacing.icon_width,
                                            node_style.spacing.icon_width,
                                        ),
                                        Sense::click(),
                                    );
                                    paint_default_icon(ui, openness, &r);

                                    if r.clicked_by(PointerButton::Primary) {
                                        // Toggle node's openness.
                                        effects.open_node(node_idx, !node.open);
                                    }
                                }

                                ui.allocate_exact_size(header_drag_space, Sense::hover());

                                let r = viewer.show_header(
                                    node_idx,
                                    &node.value,
                                    &inputs,
                                    &outputs,
                                    ui,
                                    snarl_state.scale(),
                                    effects,
                                );

                                // header_rect.extend_with(r.rect.max);
                                // dbg!(header_rect);

                                header_rect.max = r.rect.max;

                                header_frame_rect =
                                    header_frame.total_margin().expand_rect(header_rect);
                            });

                            ui.advance_cursor_after_rect(Rect::from_min_max(
                                header_rect.min,
                                pos2(
                                    f32::max(header_rect.max.x, node_rect.max.x),
                                    header_rect.min.y,
                                ),
                            ));
                        });

                        let min_pin_y = header_rect.center().y;

                        let input_x =
                            header_rect.left() + header_frame.total_margin().left + pin_size * 0.5;

                        let output_x = f32::max(header_rect.right(), node_rect.right())
                            - header_frame.total_margin().right
                            - pin_size * 0.5;

                        if true {
                            if (openness < 1.0 && node.open) || (openness > 0.0 && !node.open) {
                                ui.ctx().request_repaint();
                            }

                            // Show input pins.

                            // Inputs are placed under the header and must not go outside of the header frame.
                            let pins_clip_rect = Rect::from_min_max(
                                pos2(
                                    header_rect.min.x,
                                    header_rect.max.y
                                        + header_frame.total_margin().bottom
                                        + node_style.spacing.item_spacing.y,
                                ),
                                pos2(f32::max(node_rect.max.x, header_rect.max.x), f32::INFINITY),
                            );

                            let mut pins_rect = pins_clip_rect;

                            pins_rect.min.y += (node_frame_rect.height()
                                - header_frame_rect.height())
                                * (openness - 1.0);

                            let ref mut inputs_ui = ui.child_ui_with_id_source(
                                pins_rect,
                                Layout::top_down(Align::Center),
                                (node_id, "inputs"),
                            );

                            inputs_ui.set_clip_rect(pins_clip_rect.intersect(viewport));

                            let mut inputs_rect = pins_rect;

                            inputs_ui.horizontal(|ui| {
                                // Input pins on the left.
                                let r = ui.with_layout(Layout::top_down(Align::Min), |ui| {
                                    // let r = Grid::new((node_id, "inputs")).min_col_width(0.0).show(ui, |ui| {
                                    for in_pin in inputs {
                                        // Show input pin.
                                        ui.with_layout(
                                            Layout::left_to_right(Align::Center),
                                            |ui| {
                                                // Allocate space for pin shape.
                                                let (pin_id, _) =
                                                    ui.allocate_space(vec2(pin_size, pin_size));

                                                // Show input content
                                                let r = viewer.show_input(
                                                    &in_pin,
                                                    ui,
                                                    snarl_state.scale(),
                                                    effects,
                                                );
                                                let pin_info = r.inner;
                                                let r = r.response;
                                                ui.end_row();

                                                // Centered vertically.
                                                let y = min_pin_y
                                                    .max((r.rect.min.y + r.rect.max.y) * 0.5);

                                                let pin_pos = pos2(input_x, y);

                                                input_positions.insert(in_pin.id, pin_pos);
                                                input_colors.insert(in_pin.id, pin_info.fill);

                                                // Interact with pin shape.
                                                let r = ui.interact(
                                                    Rect::from_center_size(
                                                        pin_pos,
                                                        vec2(pin_size, pin_size),
                                                    ),
                                                    pin_id,
                                                    Sense::click_and_drag(),
                                                );

                                                let mut pin_size = pin_size;
                                                if r.hovered() {
                                                    pin_size *= 1.2;
                                                }

                                                draw_pin(ui.painter(), pin_info, pin_pos, pin_size);

                                                if r.clicked_by(PointerButton::Secondary) {
                                                    let _ = viewer.drop_inputs(&in_pin, effects);
                                                }
                                                if r.drag_started_by(PointerButton::Primary) {
                                                    set_part_wire(
                                                        ui,
                                                        snarl_id,
                                                        AnyPin::In(in_pin.id),
                                                    );
                                                }
                                                if r.drag_released_by(PointerButton::Primary) {
                                                    part_wire_drag_released = true;
                                                }
                                                if r.hovered() {
                                                    pin_hovered = Some(AnyPin::In(in_pin.id));
                                                }
                                            },
                                        );
                                    }
                                });
                                inputs_rect = r.response.rect;
                            });

                            // Show output pins.

                            // Outputs are placed under the header and must not go outside of the header frame.

                            let ref mut outputs_ui = ui.child_ui_with_id_source(
                                pins_rect,
                                Layout::top_down(Align::Center),
                                (node_id, "outputs"),
                            );

                            outputs_ui.set_clip_rect(pins_clip_rect.intersect(viewport));

                            let mut outputs_rect = pins_rect;

                            outputs_ui.horizontal(|ui| {
                                // Output pins on the right.
                                let r = ui.with_layout(Layout::top_down(Align::Max), |ui| {
                                    // let r = Grid::new((node_id, "outputs")).min_col_width(0.0).show(ui, |ui| {

                                    for out_pin in outputs {
                                        // Show output pin.
                                        ui.with_layout(
                                            Layout::right_to_left(Align::Center),
                                            |ui| {
                                                // Allocate space for pin shape.

                                                let (pin_id, _) = ui
                                                    .with_layout(
                                                        Layout::right_to_left(Align::Center),
                                                        |ui| {
                                                            ui.allocate_space(vec2(
                                                                pin_size, pin_size,
                                                            ))
                                                        },
                                                    )
                                                    .inner;

                                                // Show output content
                                                let r = viewer.show_output(
                                                    &out_pin,
                                                    ui,
                                                    snarl_state.scale(),
                                                    effects,
                                                );
                                                let pin_info = r.inner;
                                                let r = r.response;

                                                // Centered vertically.
                                                let y = min_pin_y
                                                    .max((r.rect.min.y + r.rect.max.y) * 0.5);

                                                let pin_pos = pos2(output_x, y);

                                                output_positions.insert(out_pin.id, pin_pos);
                                                output_colors.insert(out_pin.id, pin_info.fill);

                                                let r = ui.interact(
                                                    Rect::from_center_size(
                                                        pin_pos,
                                                        vec2(pin_size, pin_size),
                                                    ),
                                                    pin_id,
                                                    Sense::click_and_drag(),
                                                );

                                                let mut pin_size = pin_size;
                                                if r.hovered() {
                                                    pin_size *= 1.2;
                                                }

                                                draw_pin(ui.painter(), pin_info, pin_pos, pin_size);

                                                if r.clicked_by(PointerButton::Secondary) {
                                                    let _ = viewer.drop_outputs(&out_pin, effects);
                                                }
                                                if r.drag_started_by(PointerButton::Primary) {
                                                    set_part_wire(
                                                        ui,
                                                        snarl_id,
                                                        AnyPin::Out(out_pin.id),
                                                    );
                                                }
                                                if r.drag_released_by(PointerButton::Primary) {
                                                    part_wire_drag_released = true;
                                                }
                                                if r.hovered() {
                                                    pin_hovered = Some(AnyPin::Out(out_pin.id));
                                                }
                                            },
                                        );
                                    }
                                });

                                outputs_rect = r.response.rect;
                            });

                            // ui.allocate_space(ui.available_size());

                            ui.expand_to_include_rect(header_rect);
                            ui.expand_to_include_rect(inputs_rect.intersect(pins_clip_rect));
                            ui.expand_to_include_rect(outputs_rect.intersect(pins_clip_rect));

                            let inputs_size = inputs_rect.size();
                            let outputs_size = outputs_rect.size();
                            let header_size = header_rect.size();

                            new_state.size = vec2(
                                f32::max(
                                    header_size.x,
                                    inputs_size.x
                                        + outputs_size.x
                                        + node_style.spacing.item_spacing.x,
                                ),
                                header_size.y
                                    + header_frame.total_margin().bottom
                                    + node_style.spacing.item_spacing.y
                                    + f32::max(inputs_size.y, outputs_size.y),
                            );

                            // ui.painter()
                            //     .debug_rect(header_frame_rect, Color32::GREEN, "header");
                            // ui.painter()
                            //     .debug_rect(pins_clip_rect, Color32::RED, "pins_clip");
                        } else {
                            for in_pin in inputs {
                                let pin_pos = pos2(input_x, min_pin_y);
                                input_positions.insert(in_pin.id, pin_pos);
                                input_colors
                                    .insert(in_pin.id, viewer.input_color(&in_pin, &node_style));
                            }
                            for out_pin in outputs {
                                let pin_pos = pos2(output_x, min_pin_y);
                                output_positions.insert(out_pin.id, pin_pos);
                                output_colors
                                    .insert(out_pin.id, viewer.output_color(&out_pin, &node_style));
                            }
                        }
                    });

                    // ui.painter()
                    //     .debug_rect(header_rect, Color32::BLACK, "Title rect");
                    // ui.painter()
                    //     .debug_rect(inputs_rect, Color32::RED, "Inputs rect");
                    // ui.painter()
                    //     .debug_rect(outputs_rect, Color32::GREEN, "Outputs rect");

                    // ui.painter()
                    //     .debug_rect(node_rect, Color32::WHITE, "node_rect");
                    // ui.painter()
                    //     .debug_rect(header_rect, Color32::GREEN, "header_rect");
                    // ui.painter()
                    //     .debug_rect(pins_rect, Color32::RED, "pins_rect");

                    if new_state != node_state {
                        new_state.store(ui.ctx(), node_id);
                        ui.ctx().request_repaint();
                    }
                }

                let part_wire = get_part_wire(ui, snarl_id);
                let mut hovered_wire = None;

                for wire in self.wires.iter() {
                    let from = output_positions[&wire.out_pin];
                    let to = input_positions[&wire.in_pin];

                    if part_wire.is_none() && bg_r.hovered() {
                        // Try to find hovered wire
                        // If not draggin new wire
                        // And not hovering over item above.

                        if let Some(hover_pos) = hover_pos {
                            let hit = hit_wire(
                                hover_pos,
                                wire_frame_size,
                                style.upscale_wire_frame,
                                style.downscale_wire_frame,
                                from,
                                to,
                                wire_width * 1.5,
                            );

                            if hit {
                                hovered_wire = Some(wire);
                                break;
                            }
                        }
                    }
                }

                if let Some(wire) = hovered_wire {
                    if bg_r.clicked_by(PointerButton::Secondary) {
                        let out_pin = OutPin::output(&self, wire.out_pin);
                        let in_pin = InPin::input(&self, wire.in_pin);

                        let _ = viewer.disconnect(&out_pin, &in_pin, effects);
                    }

                    // Background is not hovered then.
                    bg_r.hovered = false;
                    bg_r.clicked = [false; egui::NUM_POINTER_BUTTONS];
                    bg_r.double_clicked = [false; egui::NUM_POINTER_BUTTONS];
                    bg_r.triple_clicked = [false; egui::NUM_POINTER_BUTTONS];
                }

                if bg_r.hovered() {
                    if bg_r.dragged_by(PointerButton::Primary) {
                        snarl_state.pan(-bg_r.drag_delta());
                    }
                }
                bg_r.context_menu(|ui| {
                    viewer.graph_menu(
                        snarl_state.screen_pos_to_graph(ui.cursor().min, viewport),
                        ui,
                        snarl_state.scale(),
                        effects,
                    );
                });

                let mut wire_shapes = Vec::new();

                for wire in self.wires.iter() {
                    let from = output_positions[&wire.out_pin];
                    let to = input_positions[&wire.in_pin];

                    let color =
                        mix_colors(output_colors[&wire.out_pin], input_colors[&wire.in_pin]);

                    let mut draw_width = wire_width;
                    if hovered_wire == Some(wire) {
                        draw_width *= 1.5;
                    }

                    draw_wire(
                        &mut wire_shapes,
                        wire_frame_size,
                        style.upscale_wire_frame,
                        style.downscale_wire_frame,
                        from,
                        to,
                        Stroke::new(draw_width, color),
                    );
                }

                match part_wire {
                    None => {}
                    Some(AnyPin::In(pin)) => {
                        let from = hover_pos.unwrap_or(Pos2::ZERO);
                        let to = input_positions[&pin];

                        let color = input_colors[&pin];

                        draw_wire(
                            &mut wire_shapes,
                            wire_frame_size,
                            style.upscale_wire_frame,
                            style.downscale_wire_frame,
                            from,
                            to,
                            Stroke::new(wire_width, color),
                        );
                    }
                    Some(AnyPin::Out(pin)) => {
                        let from: Pos2 = output_positions[&pin];
                        let to = hover_pos.unwrap_or(Pos2::ZERO);

                        let color = output_colors[&pin];

                        draw_wire(
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

                match wire_shape_idx {
                    None => {
                        ui.painter().add(Shape::Vec(wire_shapes));
                    }
                    Some(idx) => {
                        ui.painter().set(idx, Shape::Vec(wire_shapes));
                    }
                }

                if part_wire_drag_released {
                    let part_wire = take_part_wire(ui, snarl_id);
                    if part_wire.is_some() {
                        ui.ctx().request_repaint();
                    }
                    match (part_wire, pin_hovered) {
                        (Some(AnyPin::In(in_pin)), Some(AnyPin::Out(out_pin)))
                        | (Some(AnyPin::Out(out_pin)), Some(AnyPin::In(in_pin))) => {
                            let _ = viewer.connect(
                                &OutPin::output(self, out_pin),
                                &InPin::input(self, in_pin),
                                effects,
                            );
                        }
                        _ => {}
                    }
                }

                ui.advance_cursor_after_rect(Rect::from_min_size(viewport.min, Vec2::ZERO));

                snarl_state.store(ui.ctx(), snarl_id);
            });
    }
}
