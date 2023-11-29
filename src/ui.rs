use std::cell::RefCell;

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

    fn size_hint(&self, node: &T) -> Vec2;

    fn title<'a>(&'a mut self, node: &'a T) -> &'a str;

    fn outputs(&mut self, node: &T) -> usize;

    fn inputs(&mut self, node: &T) -> usize;

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
            node.pos += delta;
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
                let wire_frame_size = base_size * 5.0;

                let max_rect = ui.max_rect();

                let mut input_positions = HashMap::with_hasher(egui::ahash::RandomState::new());
                let mut output_positions = HashMap::with_hasher(egui::ahash::RandomState::new());

                let mut input_colors = HashMap::with_hasher(egui::ahash::RandomState::new());
                let mut output_colors = HashMap::with_hasher(egui::ahash::RandomState::new());

                let mut part_wire_drag_released = false;
                let mut pin_hovered = None;

                for (node_idx, node) in &self.nodes {
                    let node_rect = Rect::from_min_size(
                        node.pos + vec2(max_rect.min.x, max_rect.min.y),
                        viewer.size_hint(&node.value.borrow()),
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
                                        input_colors.insert(in_pin, pin_color);
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
                                        output_colors.insert(out_pin, pin_color);
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

                    let [or, og, ob, oa] = output_colors[&wire.out_pin].to_array();
                    let [ir, ig, ib, ia] = input_colors[&wire.in_pin].to_array();

                    let color = Color32::from_rgba_premultiplied(
                        or / 2 + ir / 2,
                        og / 2 + ig / 2,
                        ob / 2 + ib / 2,
                        oa / 2 + ia / 2,
                    );

                    draw_wire(&painter, wire_frame_size, from, to, Stroke::new(1.0, color));
                }

                match get_part_wire(ui, snarl_id) {
                    None => {}
                    Some(AnyPin::In(pin)) => {
                        let from = ui.input(|i| i.pointer.latest_pos().unwrap_or(Pos2::ZERO));
                        let to = input_positions[&pin];

                        let color = input_colors[&pin];

                        draw_wire(&painter, wire_frame_size, from, to, Stroke::new(1.0, color));
                    }
                    Some(AnyPin::Out(pin)) => {
                        let from: Pos2 = output_positions[&pin];
                        let to = ui.input(|i| i.pointer.latest_pos().unwrap_or(Pos2::ZERO));

                        let color = output_colors[&pin];

                        draw_wire(&painter, wire_frame_size, from, to, Stroke::new(1.0, color));
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

fn draw_wire(painter: &Painter, frame_size: f32, from: Pos2, to: Pos2, stroke: Stroke) {
    let from_norm_x = frame_size;
    let from_2 = pos2(from.x + from_norm_x, from.y);
    let to_norm_x = -from_norm_x;
    let to_2 = pos2(to.x + to_norm_x, to.y);

    let between = (from_2 - to_2).length();

    if from_2.x <= to_2.x && between >= frame_size * 2.0 {
        let middle_1 = from_2 + (to_2 - from_2).normalized() * frame_size;
        let middle_2 = to_2 + (from_2 - to_2).normalized() * frame_size;

        draw_bezier(
            painter,
            &[from, from_2, middle_1, middle_2, to_2, to],
            stroke,
        );
    } else if from_2.x <= to_2.x {
        // let t = 1.0 - ((to_2.y - from_2.y).abs() - between) / between;
        let t = (between - frame_size * 2.0) / ((to_2.y - from_2.y).abs() - frame_size * 2.0);

        let middle_1 = to_2.lerp(pos2(from_2.x, from_2.y + frame_size), t);
        let middle_1 = from_2 + (middle_1 - from_2).normalized() * frame_size;

        let middle_2 = from_2.lerp(pos2(to_2.x, to_2.y + frame_size), t);
        let middle_2 = to_2 + (middle_2 - to_2).normalized() * frame_size;

        draw_bezier(
            painter,
            &[from, from_2, middle_1, middle_2, to_2, to],
            stroke,
        );
    } else {
        let middle_1 = pos2(from_2.x, from_2.y + frame_size);
        let middle_2 = pos2(to_2.x, to_2.y + frame_size);

        draw_bezier(
            painter,
            &[from, from_2, middle_1, middle_2, to_2, to],
            stroke,
        );
    }
}

fn draw_bezier(painter: &Painter, points: &[Pos2], stroke: Stroke) {
    assert!(points.len() > 0);

    let total_length = points[1..]
        .iter()
        .scan(points[0], |last, point| {
            let l = (*point - *last).length();
            *last = *point;
            Some(l)
        })
        .sum::<f32>();

    let samples = total_length.ceil() as usize;

    let mut path = Vec::new();

    for i in 0..samples {
        let t = i as f32 / (samples - 1) as f32;
        path.push(sample_bezier(points, t));
    }

    painter.add(Shape::Path(epaint::PathShape {
        points: path,
        closed: false,
        fill: Color32::TRANSPARENT,
        stroke,
    }));
}

fn sample_bezier(points: &[Pos2], t: f32) -> Pos2 {
    assert!(points.len() > 0);

    match points {
        [] => panic!("Empty bezier curve"),
        [p] => *p,
        [p1, p2] => p1.lerp(*p2, t),
        [p1, p2, p3] => {
            let x1 = p1.lerp(*p2, t);
            let x2 = p2.lerp(*p3, t);
            x1.lerp(x2, t)
        }
        [p1, p2, p3, p4] => {
            let x1 = p1.lerp(*p2, t);
            let x2 = p2.lerp(*p3, t);
            let x3 = p3.lerp(*p4, t);
            let y1 = x1.lerp(x2, t);
            let y2 = x2.lerp(x3, t);
            y1.lerp(y2, t)
        }
        [p1, p2, p3, p4, p5] => {
            let x1 = p1.lerp(*p2, t);
            let x2 = p2.lerp(*p3, t);
            let x3 = p3.lerp(*p4, t);
            let x4 = p4.lerp(*p5, t);
            let y1 = x1.lerp(x2, t);
            let y2 = x2.lerp(x3, t);
            let y3 = x3.lerp(x4, t);
            let z1 = y1.lerp(y2, t);
            let z2 = y2.lerp(y3, t);
            z1.lerp(z2, t)
        }
        [p1, p2, p3, p4, p5, p6] => {
            let x1 = p1.lerp(*p2, t);
            let x2 = p2.lerp(*p3, t);
            let x3 = p3.lerp(*p4, t);
            let x4 = p4.lerp(*p5, t);
            let x5 = p5.lerp(*p6, t);
            let y1 = x1.lerp(x2, t);
            let y2 = x2.lerp(x3, t);
            let y3 = x3.lerp(x4, t);
            let y4 = x4.lerp(x5, t);
            let z1 = y1.lerp(y2, t);
            let z2 = y2.lerp(y3, t);
            let z3 = y3.lerp(y4, t);
            let w1 = z1.lerp(z2, t);
            let w2 = z2.lerp(z3, t);
            w1.lerp(w2, t)
        }
        [p1, p2, p3, p4, p5, p6, p7] => {
            let x1 = p1.lerp(*p2, t);
            let x2 = p2.lerp(*p3, t);
            let x3 = p3.lerp(*p4, t);
            let x4 = p4.lerp(*p5, t);
            let x5 = p5.lerp(*p6, t);
            let x6 = p6.lerp(*p7, t);
            let y1 = x1.lerp(x2, t);
            let y2 = x2.lerp(x3, t);
            let y3 = x3.lerp(x4, t);
            let y4 = x4.lerp(x5, t);
            let y5 = x5.lerp(x6, t);
            let z1 = y1.lerp(y2, t);
            let z2 = y2.lerp(y3, t);
            let z3 = y3.lerp(y4, t);
            let z4 = y4.lerp(y5, t);
            let w1 = z1.lerp(z2, t);
            let w2 = z2.lerp(z3, t);
            let w3 = z3.lerp(z4, t);
            let u1 = w1.lerp(w2, t);
            let u2 = w2.lerp(w3, t);
            u1.lerp(u2, t)
        }
        many => {
            let a = sample_bezier(&many[..many.len() - 1], t);
            let b = sample_bezier(&many[1..], t);
            a.lerp(b, t)
        }
    }
}
