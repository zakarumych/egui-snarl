use std::{borrow::Cow, cell::RefCell};

use egui::{ahash::HashMap, *};

use crate::{wire_pins, AnyPin, InPin, OutPin, Snarl};

/// Error returned from methods where `Viewer` forbids the operation.
pub struct Forbidden;

pub enum Effect<T> {
    /// Adds connection between two nodes.
    Connect {
        from: OutPin,
        to: InPin,
    },

    /// Removes connection between two nodes.
    Disconnect {
        from: OutPin,
        to: InPin,
    },

    DropInputs(InPin),

    DropOutputs(OutPin),

    /// Executes a closure with mutable reference to the Snarl.
    Closure(Box<dyn FnOnce(&mut Snarl<T>)>),
}

pub struct Effects<T> {
    effects: Vec<Effect<T>>,
}

impl<T> Default for Effects<T> {
    #[inline]
    fn default() -> Self {
        Effects {
            effects: Default::default(),
        }
    }
}

impl<T> Effects<T> {
    pub fn new() -> Self {
        Effects {
            effects: Vec::new(),
        }
    }

    pub fn connect(&mut self, from: OutPin, to: InPin) {
        self.effects.push(Effect::Connect { from, to });
    }

    pub fn disconnect(&mut self, from: OutPin, to: InPin) {
        self.effects.push(Effect::Disconnect { from, to });
    }

    pub fn drop_inputs(&mut self, input: InPin) {
        self.effects.push(Effect::DropInputs(input));
    }

    pub fn drop_outputs(&mut self, output: OutPin) {
        self.effects.push(Effect::DropOutputs(output));
    }
}

pub struct Remote<'a, T> {
    pub node_idx: usize,
    pub pin_idx: usize,
    pub node: &'a RefCell<T>,
}

/// Node's pin that contains local idx, remove idx and remove node reference.
pub struct Pin<'a, T> {
    pub pin_idx: usize,
    pub remotes: Vec<Remote<'a, T>>,
}

impl<'a, T> Pin<'a, T> {
    pub fn output(snarl: &'a Snarl<T>, pin: OutPin) -> Self {
        Pin {
            pin_idx: pin.output,
            remotes: snarl
                .wires
                .wired_inputs(pin)
                .map(|pin| Remote {
                    node: &snarl.nodes[pin.node].value,
                    node_idx: pin.node,
                    pin_idx: pin.input,
                })
                .collect(),
        }
    }

    pub fn input(snarl: &'a Snarl<T>, pin: InPin) -> Self {
        Pin {
            pin_idx: pin.input,
            remotes: snarl
                .wires
                .wired_outputs(pin)
                .map(|pin| Remote {
                    node: &snarl.nodes[pin.node].value,
                    node_idx: pin.node,
                    pin_idx: pin.output,
                })
                .collect(),
        }
    }
}

/// Node and its output pin.
pub struct NodeOutPin<'a, T> {
    pub out_pin: OutPin,
    pub node: &'a RefCell<T>,
    pub remotes: Vec<Remote<'a, T>>,
}

/// Node and its output pin.
pub struct NodeInPin<'a, T> {
    pub in_pin: InPin,
    pub node: &'a RefCell<T>,
    pub remotes: Vec<Remote<'a, T>>,
}

impl<'a, T> NodeOutPin<'a, T> {
    pub fn new(snarl: &'a Snarl<T>, pin: OutPin) -> Self {
        NodeOutPin {
            out_pin: pin,
            node: &snarl.nodes[pin.node].value,
            remotes: snarl
                .wires
                .wired_inputs(pin)
                .map(|pin| Remote {
                    node: &snarl.nodes[pin.node].value,
                    node_idx: pin.node,
                    pin_idx: pin.input,
                })
                .collect(),
        }
    }
}

impl<'a, T> NodeInPin<'a, T> {
    pub fn input(snarl: &'a Snarl<T>, pin: InPin) -> Self {
        NodeInPin {
            in_pin: pin,
            node: &snarl.nodes[pin.node].value,
            remotes: snarl
                .wires
                .wired_outputs(pin)
                .map(|pin| Remote {
                    node: &snarl.nodes[pin.node].value,
                    node_idx: pin.node,
                    pin_idx: pin.output,
                })
                .collect(),
        }
    }
}

/// SnarlViewer is a trait for viewing a Snarl.
///
/// It can extract necessary data from the nodes and controls their
/// response to certain events.
pub trait SnarlViewer<T> {
    /// Called to create new node in the Snarl.
    ///
    /// Returns response with effects to be applied to the Snarl after the node is added.
    ///
    /// # Errors
    ///
    /// Returns `Forbidden` error if the node cannot be added.
    #[inline]
    fn add_node(
        &mut self,
        idx: usize,
        node: &T,
        effects: &mut Effects<T>,
    ) -> Result<(), Forbidden> {
        let _ = (idx, node, effects);
        Ok(())
    }

    #[inline]
    fn connect(
        &mut self,
        from: NodeOutPin<T>,
        to: NodeInPin<T>,
        effects: &mut Effects<T>,
    ) -> Result<(), Forbidden> {
        let _ = (from, to, effects);
        Ok(())
    }

    /// Called when a node is about to be removed.
    ///
    /// # Arguments
    ///
    /// * `node` - Node that is about to be removed.
    /// * `inputs` - Array of input pins connected to the node.
    /// * `outputs` - Array of output pins connected to the node.
    ///
    /// Returns response with effects to be applied to the Snarl after the node is removed.
    ///
    /// # Errors
    ///
    /// Returns `Forbidden` error if the node cannot be removed.
    #[inline]
    fn remove_node(
        &mut self,
        idx: usize,
        node: &RefCell<T>,
        inputs: &[Pin<T>],
        outputs: &[Pin<T>],
        effects: &mut Effects<T>,
    ) -> Result<(), Forbidden> {
        let _ = (idx, node, inputs, outputs, effects);
        Ok(())
    }

    fn node_picker(&mut self, ui: &mut Ui) -> egui::InnerResponse<Option<T>>;

    fn inputs(&mut self, node: &T) -> usize;

    fn outputs(&mut self, node: &T) -> usize;

    fn title(&mut self, node: &T) -> Cow<'static, str>;

    fn show_input(
        &mut self,
        node: &RefCell<T>,
        pin: Pin<T>,
        ui: &mut Ui,
        effects: &mut Effects<T>,
    ) -> egui::InnerResponse<Color32>;

    fn show_output(
        &mut self,
        node: &RefCell<T>,
        pin: Pin<T>,
        ui: &mut Ui,
        effects: &mut Effects<T>,
    ) -> egui::InnerResponse<Color32>;
}

impl<T> Snarl<T> {
    fn apply_effects(&mut self, response: Effects<T>) {
        for effect in response.effects {
            self.apply_effect(effect);
        }
    }

    fn apply_effect(&mut self, effect: Effect<T>) {
        match effect {
            Effect::Connect { from, to } => {
                self.wires.insert(wire_pins(from, to));
            }
            Effect::Disconnect { from, to } => {
                self.wires.remove(&wire_pins(from, to));
            }
            Effect::DropInputs(pin) => {
                self.wires.drop_inputs(pin);
            }
            Effect::DropOutputs(pin) => {
                self.wires.drop_outputs(pin);
            }
            Effect::Closure(f) => f(self),
        }
    }

    pub fn show<V>(&mut self, viewer: &mut V, id: Id, ui: &mut Ui)
    where
        V: SnarlViewer<T>,
    {
        let mut effects = Effects::new();
        let mut nodes_moved = Vec::new();
        self._show(viewer, id, ui, &mut effects, &mut nodes_moved);
        self.apply_effects(effects);

        for (node_idx, delta) in nodes_moved {
            let node = &mut self.nodes[node_idx];
            node.rect.min += delta;
            node.rect.max += delta;
        }
    }

    fn _show<V>(
        &self,
        viewer: &mut V,
        snarl_id: Id,
        ui: &mut Ui,
        effects: &mut Effects<T>,
        nodes_moved: &mut Vec<(usize, Vec2)>,
    ) where
        V: SnarlViewer<T>,
    {
        Frame::none()
            .fill(Color32::DARK_GRAY)
            .stroke(Stroke::new(1.0, Color32::GRAY))
            .show(ui, |ui| {
                let base_size = ui.style().spacing.interact_size.y * 0.5;

                let max_rect = ui.max_rect();

                let mut input_positions = HashMap::with_hasher(egui::ahash::RandomState::new());
                let mut output_positions = HashMap::with_hasher(egui::ahash::RandomState::new());

                let mut part_wire_drag_released = false;
                let mut pin_hovered = None;

                for (node_idx, node) in &self.nodes {
                    let node_rect = Rect::from_min_size(
                        node.rect.min + vec2(max_rect.min.x, max_rect.min.y),
                        node.rect.size(),
                    );

                    let ref mut ui = ui.child_ui_with_id_source(
                        node_rect,
                        Layout::top_down(Align::Center),
                        "snarl",
                    );

                    Frame::window(ui.style()).show(ui, |ui| {
                        let r = ui.scope(|ui| {
                            ui.label(viewer.title(&node.value.borrow()));
                            ui.separator();
                        });

                        let r = ui.interact(
                            r.response.rect,
                            r.response.id.with("drag").with(node_idx),
                            Sense::drag(),
                        );

                        if r.dragged() {
                            nodes_moved.push((node_idx, r.drag_delta()));
                        }

                        ui.horizontal(|ui| {
                            ui.with_layout(Layout::top_down(Align::Min), |ui| {
                                let inputs = viewer.inputs(&node.value.borrow());

                                for input_idx in 0..inputs {
                                    let in_pin = InPin {
                                        node: node_idx,
                                        input: input_idx,
                                    };

                                    ui.horizontal(|ui| {
                                        let pin = Pin::input(&self, in_pin);
                                        ui.allocate_space(vec2(base_size, base_size));

                                        let r = viewer.show_input(&node.value, pin, ui, effects);
                                        let pin_color = r.inner;

                                        let x = r.response.rect.left()
                                            - base_size / 2.0
                                            - ui.style().spacing.item_spacing.x;

                                        let y = (r.response.rect.top() + r.response.rect.bottom())
                                            / 2.0;

                                        let r = ui.allocate_rect(
                                            Rect::from_center_size(
                                                pos2(x, y),
                                                vec2(base_size, base_size),
                                            ),
                                            Sense::click_and_drag(),
                                        );

                                        ui.painter().circle(
                                            r.rect.center(),
                                            base_size / 2.0,
                                            pin_color,
                                            Stroke::new(1.0, Color32::BLACK),
                                        );

                                        if r.double_clicked() {
                                            effects.drop_inputs(in_pin);
                                        }
                                        if r.drag_started() {
                                            set_part_wire(ui, snarl_id, AnyPin::In(in_pin));
                                        }
                                        if r.drag_released() {
                                            part_wire_drag_released = true;
                                        }
                                        if r.hovered() {
                                            pin_hovered = Some(AnyPin::In(in_pin));
                                        }

                                        input_positions.insert(in_pin, r.rect.center());
                                    });
                                }
                            });

                            ui.with_layout(Layout::top_down(Align::Max), |ui| {
                                let outputs = viewer.outputs(&node.value.borrow());

                                for output_idx in 0..outputs {
                                    let out_pin = OutPin {
                                        node: node_idx,
                                        output: output_idx,
                                    };
                                    let pin = Pin::output(self, out_pin);

                                    ui.horizontal(|ui| {
                                        let r = viewer.show_output(&node.value, pin, ui, effects);
                                        let pin_color = r.inner;

                                        ui.allocate_space(vec2(base_size, base_size));

                                        let x = r.response.rect.right()
                                            + base_size / 2.0
                                            + ui.style().spacing.item_spacing.x;
                                        let y = (r.response.rect.top() + r.response.rect.bottom())
                                            / 2.0;

                                        let r = ui.allocate_rect(
                                            Rect::from_center_size(
                                                pos2(x, y),
                                                vec2(base_size, base_size),
                                            ),
                                            Sense::click_and_drag(),
                                        );

                                        ui.painter().circle(
                                            r.rect.center(),
                                            base_size / 2.0,
                                            pin_color,
                                            Stroke::new(1.0, Color32::BLACK),
                                        );

                                        if r.double_clicked() {
                                            effects.drop_outputs(out_pin);
                                        }
                                        if r.drag_started() {
                                            set_part_wire(ui, snarl_id, AnyPin::Out(out_pin));
                                        }
                                        if r.drag_released() {
                                            part_wire_drag_released = true;
                                        }
                                        if r.hovered() {
                                            pin_hovered = Some(AnyPin::Out(out_pin));
                                        }

                                        output_positions.insert(out_pin, r.rect.center());
                                    });
                                }
                            });
                        });
                    });
                }

                let leftover = ui.available_size();
                ui.allocate_exact_size(leftover, Sense::hover());

                let id = Id::new(("wires", ui.layer_id()));
                let ref mut ui = Ui::new(
                    ui.ctx().clone(),
                    LayerId::new(Order::Middle, id),
                    id,
                    max_rect,
                    max_rect,
                );
                let mut painter = ui.painter().clone();
                painter.set_layer_id(LayerId::new(Order::Middle, Id::new("wires")));
                for wire in self.wires.iter() {
                    let from = output_positions[&wire.out_pin];
                    let to = input_positions[&wire.in_pin];

                    draw_wire(
                        &painter,
                        base_size,
                        from,
                        to,
                        Stroke::new(1.0, Color32::BLACK),
                    );
                }

                match get_part_wire(ui, snarl_id) {
                    None => {}
                    Some(AnyPin::In(pin)) => {
                        let from = ui.input(|i| i.pointer.latest_pos().unwrap_or(Pos2::ZERO));
                        let to = input_positions[&pin];

                        // painter.line_segment([from, to], Stroke::new(1.0, Color32::BLACK));

                        draw_wire(
                            &painter,
                            base_size,
                            from,
                            to,
                            Stroke::new(1.0, Color32::BLACK),
                        );
                    }
                    Some(AnyPin::Out(pin)) => {
                        let from: Pos2 = output_positions[&pin];
                        let to = ui.input(|i| i.pointer.latest_pos().unwrap_or(Pos2::ZERO));

                        // painter.line_segment([from, to], Stroke::new(1.0, Color32::BLACK));

                        draw_wire(
                            &painter,
                            base_size,
                            from,
                            to,
                            Stroke::new(1.0, Color32::BLACK),
                        );
                    }
                }

                if part_wire_drag_released {
                    match (take_part_wire(ui, snarl_id), pin_hovered) {
                        (Some(AnyPin::In(in_pin)), Some(AnyPin::Out(out_pin)))
                        | (Some(AnyPin::Out(out_pin)), Some(AnyPin::In(in_pin))) => {
                            let res = viewer.connect(
                                NodeOutPin::new(self, out_pin),
                                NodeInPin::input(self, in_pin),
                                effects,
                            );

                            match res {
                                Ok(()) => effects.connect(out_pin, in_pin),
                                Err(Forbidden) => {}
                            }
                        }
                        _ => {}
                    }
                }
            });
    }
}

#[derive(Clone, Copy)]
struct PartWire(AnyPin);

fn get_part_wire(ui: &Ui, id: Id) -> Option<AnyPin> {
    match ui.memory(|m| m.data.get_temp::<PartWire>(id)) {
        Some(PartWire(pin)) => Some(pin),
        None => None,
    }
}

fn set_part_wire(ui: &Ui, id: Id, pin: AnyPin) {
    ui.memory_mut(|m| m.data.insert_temp(id, PartWire(pin)));
}

fn take_part_wire(ui: &Ui, id: Id) -> Option<AnyPin> {
    let part_wire = ui.memory_mut(|m| {
        let value = m.data.get_temp::<PartWire>(id);
        m.data.remove::<PartWire>(id);
        value
    });
    match part_wire {
        Some(PartWire(pin)) => Some(pin),
        None => None,
    }
}

fn draw_wire(painter: &Painter, base_size: f32, from: Pos2, to: Pos2, stroke: Stroke) {
    let from_norm_x = ((to.x - from.x) * 0.5).max(base_size);
    let to_norm_x = -from_norm_x;

    draw_cubic_bezier(
        painter,
        from,
        vec2(from_norm_x, 0.0),
        to,
        vec2(to_norm_x, 0.0),
        stroke,
    );
}

fn draw_cubic_bezier(
    painter: &Painter,
    from: Pos2,
    from_norm: Vec2,
    to: Pos2,
    to_nrom: Vec2,
    stroke: Stroke,
) {
    let curve = lyon_geom::cubic_bezier::CubicBezierSegment {
        from: lyon_geom::point(from.x, from.y),
        ctrl1: lyon_geom::point(from.x + from_norm.x, from.y + from_norm.y),
        ctrl2: lyon_geom::point(to.x + to_nrom.x, to.y + to_nrom.y),
        to: lyon_geom::point(to.x, to.y),
    };

    let bb = curve.bounding_box();

    if !painter.clip_rect().intersects(Rect::from_min_max(
        pos2(bb.min.x, bb.min.y),
        pos2(bb.max.x, bb.max.y),
    )) {
        return;
    }

    let mut points = Vec::new();

    points.push(from);

    for point in curve.flattened(0.2) {
        points.push(pos2(point.x, point.y));
    }

    painter.add(Shape::Path(epaint::PathShape {
        points,
        closed: false,
        fill: Color32::TRANSPARENT,
        stroke,
    }));
}
