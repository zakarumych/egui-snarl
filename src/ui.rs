use std::{cell::RefCell, hash::Hash};

use egui::{
    ahash::{HashMap, HashMapExt},
    collapsing_header::paint_default_icon,
    epaint::{PathShape, Shadow},
    pos2,
    style::{Interaction, ScrollStyle, Spacing, WidgetVisuals, Widgets},
    vec2, Align, Color32, Context, FontId, Frame, Grid, Id, Layout, Margin, Painter, PointerButton,
    Pos2, Rect, Response, Rounding, Sense, Shape, Stroke, Style, Ui, Vec2, Visuals,
};

use crate::{wire_pins, InPinId, OutPinId, Snarl};

mod effect;
mod pin;
mod state;
mod viewer;
mod wire;
mod zoom;

pub use self::{
    effect::{Effect, Effects, Forbidden},
    pin::{AnyPin, InPin, OutPin, PinInfo, PinShape, RemoteInPin, RemoteOutPin},
    state::{NodeState, SnarlState},
    viewer::SnarlViewer,
    wire::WireLayer,
    zoom::Zoom,
};
use self::{
    pin::draw_pin,
    wire::{draw_wire, get_part_wire, hit_wire, mix_colors, set_part_wire, take_part_wire},
};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SnarlStyle {
    pub pin_size: Option<f32>,
    pub wire_width: Option<f32>,
    pub wire_frame_size: Option<f32>,
    pub downscale_wire_frame: bool,
    pub upscale_wire_frame: bool,
    pub wire_layer: WireLayer,
    pub title_drag_space: Option<Vec2>,
    pub input_output_spacing: Option<f32>,
    pub collapsible: bool,
}

impl Default for SnarlStyle {
    #[inline]
    fn default() -> Self {
        SnarlStyle {
            pin_size: None,
            wire_width: None,
            wire_frame_size: None,
            downscale_wire_frame: false,
            upscale_wire_frame: true,
            wire_layer: WireLayer::default(),
            title_drag_space: None,
            input_output_spacing: None,
            collapsible: true,
        }
    }
}

impl SnarlStyle {
    pub fn upscale_wire_frame(mut self, upscale: bool) -> Self {
        self.upscale_wire_frame = upscale;
        self
    }

    pub fn downscale_wire_frame(mut self, downscale: bool) -> Self {
        self.downscale_wire_frame = downscale;
        self
    }
}

impl<T> Snarl<T> {
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
        self.apply_effects(effects, ui.ctx());

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

        let snarl_state = SnarlState::load(ui.ctx(), snarl_id).unwrap_or_default();
        let zoom = snarl_state.get_zoom(snarl_id, ui.ctx(), ui.style());
        let mut new_snarl_state = snarl_state;

        Frame::none()
            .fill(ui.visuals().widgets.inactive.bg_fill)
            .stroke(ui.visuals().widgets.inactive.bg_stroke)
            .show(ui, |ui| {
                let mut node_style: Style = (**ui.style()).clone();
                zoom.zoom_style(&mut node_style);

                let pin_size = style
                    .pin_size
                    .unwrap_or_else(|| node_style.spacing.interact_size.y * 0.5);

                let wire_frame_size = style.wire_frame_size.unwrap_or(pin_size * 5.0);
                let wire_width = style.wire_width.unwrap_or_else(|| pin_size * 0.2);
                let title_drag_space = style.title_drag_space.unwrap_or_else(|| {
                    vec2(node_style.spacing.icon_width, node_style.spacing.icon_width)
                });

                // let input_output_spacing = style
                //     .input_output_spacing
                //     .unwrap_or_else(|| node_style.spacing.item_spacing.x);

                let collapsible = style.collapsible;

                let node_frame = Frame::window(&node_style);
                let title_frame = node_frame.shadow(Shadow::NONE);

                let wire_shape_idx = match style.wire_layer {
                    WireLayer::BehindNodes => Some(ui.painter().add(Shape::Noop)),
                    WireLayer::AboveNodes => None,
                };

                let max_rect = ui.max_rect();
                ui.set_clip_rect(max_rect);

                let r = ui.allocate_rect(max_rect, Sense::click_and_drag());

                let mut input_positions = HashMap::new();
                let mut output_positions = HashMap::new();

                let mut input_colors = HashMap::new();
                let mut output_colors = HashMap::new();

                let mut part_wire_drag_released = false;
                let mut pin_hovered = None;

                for (order, &node_idx) in self.draw_order.iter().enumerate() {
                    let node = &self.nodes[node_idx];

                    let node_pos = zoom.graph_point_to_screen(node.pos, max_rect);

                    // Generate persistent id for the node.
                    let node_id = snarl_id.with(("snarl-node", node_idx));

                    let openness = ui.ctx().animate_bool(node_id, node.open);

                    let node_state = NodeState::load(ui.ctx(), node_id)
                        .unwrap_or_else(|| NodeState::initial(&node_style.spacing));

                    let mut new_state = node_state;

                    let node_rect =
                        node_state.node_rect(&title_frame, &node_style.spacing, node_pos);

                    let title_rect = node_state.title_rect(&node_style.spacing, node_pos);
                    let pins_rect =
                        node_state.pins_rect(&title_frame, &node_style.spacing, openness, node_pos);

                    let r = ui.interact(node_rect, node_id, Sense::click_and_drag());

                    if r.dragged_by(PointerButton::Primary) {
                        *node_moved = Some((node_idx, r.drag_delta()));
                        *node_order_to_top = Some(order);
                    } else if r.clicked_by(PointerButton::Primary) {
                        *node_order_to_top = Some(order);
                    }

                    // Rect for node + frame margin.
                    let node_frame_rect = node_frame.total_margin().expand_rect(node_rect);

                    // Rect for title + frame margin.
                    let title_frame_rect = title_frame.total_margin().expand_rect(title_rect);

                    let ref mut node_ui = ui.child_ui_with_id_source(
                        node_frame_rect,
                        Layout::top_down(Align::Center),
                        node_id,
                    );
                    node_ui.set_style(node_style.clone());

                    node_frame.show(node_ui, |ui| {
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

                        // Show node's title
                        let ref mut title_ui = ui.child_ui_with_id_source(
                            title_frame_rect,
                            Layout::top_down(Align::Center),
                            node_id,
                        );
                        title_frame.show(title_ui, |ui| {
                            let r = ui.horizontal(|ui| {
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

                                ui.allocate_exact_size(title_drag_space, Sense::hover());
                                viewer.show_title(
                                    node_idx,
                                    &node.value,
                                    &inputs,
                                    &outputs,
                                    ui,
                                    effects,
                                );
                            });
                            new_state.title_size = r.response.rect.size();
                            ui.advance_cursor_after_rect(title_rect);
                        });

                        let min_pin_y = title_frame_rect.center().y;
                        let input_x = title_frame_rect.left_center().x
                            + title_frame.total_margin().left
                            + pin_size * 0.5;
                        let output_x = title_frame_rect.right_center().x
                            - title_frame.total_margin().right
                            - pin_size * 0.5;

                        if true {
                            if (openness < 1.0 && node.open) || (openness > 0.0 && !node.open) {
                                ui.ctx().request_repaint();
                            }

                            // Show pins.
                            let ref mut pins_ui = ui.child_ui_with_id_source(
                                pins_rect,
                                Layout::top_down(Align::Center),
                                node_id,
                            );

                            let pins_clip_rect = pins_ui
                                .clip_rect()
                                .intersect(Rect::everything_below(title_frame_rect.max.y));

                            pins_ui.set_clip_rect(pins_clip_rect);

                            pins_ui.horizontal(|ui| {
                                // Input pins on the left.
                                let r = ui.with_layout(Layout::top_down(Align::Min), |ui| {
                                    Grid::new((node_id, "inputs")).show(ui, |ui| {
                                        for in_pin in inputs {
                                            // Show input pin.
                                            // ui.with_layout(
                                            //     Layout::left_to_right(Align::Center),
                                            //     |ui| {
                                            // Allocate space for pin shape.
                                            let (pin_id, _) =
                                                ui.allocate_space(vec2(pin_size, pin_size));

                                            // Show input content
                                            let r = viewer.show_input(&in_pin, ui, effects);
                                            ui.end_row();

                                            let pin_info = r.inner;

                                            // Centered vertically.
                                            let y = min_pin_y.max(
                                                (r.response.rect.top() + r.response.rect.bottom())
                                                    * 0.5,
                                            );

                                            let pin_pos = pos2(input_x, y);

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
                                                set_part_wire(ui, snarl_id, AnyPin::In(in_pin.id));
                                            }
                                            if r.drag_released_by(PointerButton::Primary) {
                                                part_wire_drag_released = true;
                                            }
                                            if r.hovered() {
                                                pin_hovered = Some(AnyPin::In(in_pin.id));
                                            }

                                            input_positions.insert(in_pin.id, pin_pos);
                                            input_colors.insert(in_pin.id, pin_info.fill);
                                            //     },
                                            // );
                                        }
                                    });
                                });

                                new_state.inputs_size = r.response.rect.size();

                                // Output pins on the right.
                                let r = ui.with_layout(Layout::top_down(Align::Max), |ui| {
                                    for out_pin in outputs {
                                        // Show output pin.
                                        ui.with_layout(
                                            Layout::right_to_left(Align::Center),
                                            |ui| {
                                                // Allocate space for pin shape.
                                                let (pin_id, _) =
                                                    ui.allocate_space(vec2(pin_size, pin_size));

                                                // Show output content
                                                let r = viewer.show_output(&out_pin, ui, effects);
                                                let pin_info = r.inner;

                                                // Centered vertically.
                                                let y = min_pin_y.max(
                                                    (r.response.rect.top()
                                                        + r.response.rect.bottom())
                                                        * 0.5,
                                                );

                                                let pin_pos = pos2(output_x, y);

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

                                                output_positions.insert(out_pin.id, pin_pos);
                                                output_colors.insert(out_pin.id, pin_info.fill);
                                            },
                                        );
                                    }
                                });

                                new_state.outputs_size = r.response.rect.size();
                                ui.allocate_space(ui.available_size());
                            });

                            let pins_rect = new_state.pins_rect(
                                &title_frame,
                                &node_style.spacing,
                                openness,
                                node_pos,
                            );

                            ui.allocate_rect(
                                pins_rect.intersect(Rect::everything_below(title_frame_rect.max.y)),
                                Sense::hover(),
                            );

                            // ui.painter()
                            //     .debug_rect(title_frame_rect, Color32::GREEN, "title");
                            // ui.painter()
                            //     .debug_rect(pins_clip_rect, Color32::RED, "pins_clip");
                        } else {
                            for in_pin in inputs {
                                let pin_pos = pos2(input_x, min_pin_y);
                                input_positions.insert(in_pin.id, pin_pos);
                                input_colors.insert(in_pin.id, viewer.input_color(&in_pin));
                            }
                            for out_pin in outputs {
                                let pin_pos = pos2(output_x, min_pin_y);
                                output_positions.insert(out_pin.id, pin_pos);
                                output_colors.insert(out_pin.id, viewer.output_color(&out_pin));
                            }
                        }
                    });

                    // ui.painter()
                    //     .debug_rect(title_rect, Color32::BLACK, "Title rect");
                    // ui.painter()
                    //     .debug_rect(inputs_rect, Color32::RED, "Inputs rect");
                    // ui.painter()
                    //     .debug_rect(outputs_rect, Color32::GREEN, "Outputs rect");

                    if new_state != node_state {
                        new_state.store(ui.ctx(), node_id);
                        ui.ctx().request_repaint();
                    }

                    // ui.painter()
                    //     .debug_rect(node_rect, Color32::WHITE, "node_rect");
                    // ui.painter()
                    //     .debug_rect(title_rect, Color32::GREEN, "title_rect");
                    // ui.painter()
                    //     .debug_rect(pins_rect, Color32::RED, "pins_rect");
                }

                let part_wire = get_part_wire(ui, snarl_id);
                let hover_pos = r.hover_pos();
                let mut hovered_wire = None;

                for wire in self.wires.iter() {
                    let from = output_positions[&wire.out_pin];
                    let to = input_positions[&wire.in_pin];

                    if part_wire.is_none() {
                        // Do not select wire if we are dragging a new wire.
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
                            }
                        }
                    }
                }

                if let Some(wire) = hovered_wire {
                    if r.clicked_by(PointerButton::Secondary) {
                        let out_pin = OutPin::output(&self, wire.out_pin);
                        let in_pin = InPin::input(&self, wire.in_pin);

                        let _ = viewer.disconnect(&out_pin, &in_pin, effects);
                    }
                }

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
                        let from = ui.input(|i| i.pointer.latest_pos().unwrap_or(Pos2::ZERO));
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
                        let to = ui.input(|i| i.pointer.latest_pos().unwrap_or(Pos2::ZERO));

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

                ui.advance_cursor_after_rect(Rect::from_min_size(max_rect.min, Vec2::ZERO));

                if ui.button("+").clicked() {
                    new_snarl_state.apply_scale_wrt_screen_point(1.1, max_rect.center(), max_rect);

                    new_snarl_state.store(ui.ctx(), snarl_id);
                }
                if ui.button("-").clicked() {
                    new_snarl_state.apply_scale_wrt_screen_point(
                        1.0 / 1.1,
                        max_rect.center(),
                        max_rect,
                    );

                    new_snarl_state.store(ui.ctx(), snarl_id);
                }
            });
    }
}
