use std::cell::RefCell;

use egui::{
    ahash::{HashMap, HashMapExt},
    collapsing_header::paint_default_icon,
    epaint::{PathShape, Shadow},
    layers::ShapeIdx,
    style::Spacing,
    *,
};

use crate::{wire_pins, InPinId, OutPinId, Snarl};

/// Error returned from methods where `Viewer` forbids the operation.
pub struct Forbidden;

pub enum Effect<T> {
    /// Adds connection between two nodes.
    Connect { from: OutPinId, to: InPinId },

    /// Removes connection between two nodes.
    Disconnect { from: OutPinId, to: InPinId },

    /// Removes all connections from the output pin.
    DropOutputs { pin: OutPinId },

    /// Removes all connections to the input pin.
    DropInputs { pin: InPinId },

    /// Removes a node from snarl.
    RemoveNode { node: usize },

    /// Opens/closes a node.
    OpenNode { node: usize, open: bool },

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

    pub fn connect(&mut self, from: OutPinId, to: InPinId) {
        self.effects.push(Effect::Connect { from, to });
    }

    pub fn disconnect(&mut self, from: OutPinId, to: InPinId) {
        self.effects.push(Effect::Disconnect { from, to });
    }

    pub fn drop_inputs(&mut self, pin: InPinId) {
        self.effects.push(Effect::DropInputs { pin });
    }

    pub fn drop_outputs(&mut self, pin: OutPinId) {
        self.effects.push(Effect::DropOutputs { pin });
    }

    pub fn remove_node(&mut self, node: usize) {
        self.effects.push(Effect::RemoveNode { node });
    }

    pub fn open_node(&mut self, node: usize, open: bool) {
        self.effects.push(Effect::OpenNode { node, open });
    }
}

#[derive(Clone, Copy, Debug)]
pub struct RemoteOutPin<'a, T> {
    pub id: OutPinId,
    pub node: &'a RefCell<T>,
}

#[derive(Clone, Copy, Debug)]
pub struct RemoteInPin<'a, T> {
    pub id: InPinId,
    pub node: &'a RefCell<T>,
}

/// Node and its output pin.
#[derive(Clone, Debug)]
pub struct OutPin<'a, T> {
    pub id: OutPinId,
    pub node: &'a RefCell<T>,
    pub remotes: Vec<RemoteInPin<'a, T>>,
}

/// Node and its output pin.
#[derive(Clone, Debug)]
pub struct InPin<'a, T> {
    pub id: InPinId,
    pub node: &'a RefCell<T>,
    pub remotes: Vec<RemoteOutPin<'a, T>>,
}

impl<'a, T> OutPin<'a, T> {
    pub fn output(snarl: &'a Snarl<T>, pin: OutPinId) -> Self {
        OutPin {
            id: pin,
            node: &snarl.nodes[pin.node].value,
            remotes: snarl
                .wires
                .wired_inputs(pin)
                .map(|pin| RemoteInPin {
                    node: &snarl.nodes[pin.node].value,
                    id: pin,
                })
                .collect(),
        }
    }
}

impl<'a, T> InPin<'a, T> {
    pub fn input(snarl: &'a Snarl<T>, pin: InPinId) -> Self {
        InPin {
            id: pin,
            node: &snarl.nodes[pin.node].value,
            remotes: snarl
                .wires
                .wired_outputs(pin)
                .map(|pin| RemoteOutPin {
                    node: &snarl.nodes[pin.node].value,
                    id: pin,
                })
                .collect(),
        }
    }
}

/// Shape of a pin.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PinShape {
    Circle,
    Triangle,
    Square,
}

/// Information about a pin returned by `SnarlViewer::show_input` and `SnarlViewer::show_output`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PinInfo {
    pub shape: PinShape,
    pub size: f32,
    pub fill: Color32,
    pub stroke: Stroke,
}

impl Default for PinInfo {
    fn default() -> Self {
        PinInfo {
            shape: PinShape::Circle,
            size: 1.0,
            fill: Color32::GRAY,
            stroke: Stroke::new(1.0, Color32::BLACK),
        }
    }
}

impl PinInfo {
    pub fn with_shape(mut self, shape: PinShape) -> Self {
        self.shape = shape;
        self
    }

    pub fn with_size(mut self, size: f32) -> Self {
        self.size = size;
        self
    }

    pub fn with_fill(mut self, fill: Color32) -> Self {
        self.fill = fill;
        self
    }

    pub fn with_stroke(mut self, stroke: Stroke) -> Self {
        self.stroke = stroke;
        self
    }

    pub fn circle() -> Self {
        PinInfo {
            shape: PinShape::Circle,
            ..Default::default()
        }
    }

    pub fn triangle() -> Self {
        PinInfo {
            shape: PinShape::Triangle,
            ..Default::default()
        }
    }

    pub fn square() -> Self {
        PinInfo {
            shape: PinShape::Square,
            ..Default::default()
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

    /// Asks the viewer to connect two pins.
    ///
    /// This is usually happens when user drags a wire from one node's output pin to another node's input pin or vice versa.
    /// By default this method connects the pins and returns `Ok(())`.
    #[inline]
    fn connect(
        &mut self,
        from: &OutPin<T>,
        to: &InPin<T>,
        effects: &mut Effects<T>,
    ) -> Result<(), Forbidden> {
        effects.connect(from.id, to.id);
        Ok(())
    }

    /// Asks the viewer to disconnect two pins.
    #[inline]
    fn disconnect(
        &mut self,
        from: &OutPin<T>,
        to: &InPin<T>,
        effects: &mut Effects<T>,
    ) -> Result<(), Forbidden> {
        effects.disconnect(from.id, to.id);
        Ok(())
    }

    /// Asks the viewer to disconnect all wires from the output pin.
    ///
    /// This is usually happens when right-clicking on an output pin.
    /// By default this method disconnects the pins and returns `Ok(())`.
    #[inline]
    fn drop_outputs(&mut self, pin: &OutPin<T>, effects: &mut Effects<T>) -> Result<(), Forbidden> {
        effects.drop_outputs(pin.id);
        Ok(())
    }

    /// Asks the viewer to disconnect all wires from the input pin.
    ///
    /// This is usually happens when right-clicking on an input pin.
    /// By default this method disconnects the pins and returns `Ok(())`.
    #[inline]
    fn drop_inputs(&mut self, pin: &InPin<T>, effects: &mut Effects<T>) -> Result<(), Forbidden> {
        effects.drop_inputs(pin.id);
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
        inputs: &[InPin<T>],
        outputs: &[OutPin<T>],
        effects: &mut Effects<T>,
    ) -> Result<(), Forbidden> {
        let _ = (idx, node, inputs, outputs);
        effects.remove_node(idx);
        Ok(())
    }

    fn node_picker(&mut self, ui: &mut Ui) -> egui::InnerResponse<Option<T>>;

    fn title<'a>(&'a mut self, node: &'a T) -> &'a str;

    fn show_title(
        &mut self,
        idx: usize,
        node: &RefCell<T>,
        inputs: &[InPin<T>],
        outputs: &[OutPin<T>],
        ui: &mut Ui,
        effects: &mut Effects<T>,
    ) -> Response {
        let _ = (idx, node, inputs, outputs, effects);
        ui.label(self.title(&*node.borrow()))
    }

    fn outputs(&mut self, node: &T) -> usize;

    fn inputs(&mut self, node: &T) -> usize;

    fn show_input(
        &mut self,
        pin: &InPin<T>,
        ui: &mut Ui,
        effects: &mut Effects<T>,
    ) -> egui::InnerResponse<PinInfo>;

    fn show_output(
        &mut self,
        pin: &OutPin<T>,
        ui: &mut Ui,
        effects: &mut Effects<T>,
    ) -> egui::InnerResponse<PinInfo>;

    fn input_color(&mut self, pin: &InPin<T>) -> Color32;
    fn output_color(&mut self, pin: &OutPin<T>) -> Color32;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WireLayer {
    BehindNodes,
    AboveNodes,
}

impl Default for WireLayer {
    #[inline]
    fn default() -> Self {
        WireLayer::BehindNodes
    }
}

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

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum AnyPin {
    Out(OutPinId),
    In(InPinId),
}

/// Node UI state.
#[derive(Clone, Copy, PartialEq)]
struct NodeState {
    /// Size occupied by title.
    title_size: Vec2,

    /// Size occupied by inputs.
    inputs_size: Vec2,

    /// Size occupied by outputs.
    outputs_size: Vec2,
}

impl NodeState {
    fn load(cx: &Context, id: Id) -> Option<Self> {
        cx.data_mut(|d| d.get_temp(id))
    }

    fn store(&self, cx: &Context, id: Id) {
        cx.data_mut(|d| d.insert_temp(id, *self));
    }

    /// Finds node rect at specific position (excluding node frame margin).
    fn node_rect(&self, frame: &Frame, spacing: &Spacing, pos: Pos2) -> Rect {
        let width = self
            .title_size
            .x
            .max(self.inputs_size.x + spacing.item_spacing.x + self.outputs_size.x);

        let height = self.title_size.y
            + frame.total_margin().bottom
            + frame.total_margin().bottom
            + self.inputs_size.y.max(self.outputs_size.y);

        Rect::from_min_size(pos, vec2(width, height))
    }

    /// Finds title rect at specific position (excluding node frame margin).
    fn title_rect(&self, spacing: &Spacing, pos: Pos2) -> Rect {
        let width = self
            .title_size
            .x
            .max(self.inputs_size.x + spacing.item_spacing.x + self.outputs_size.x);

        let height = self.title_size.y;

        Rect::from_min_size(pos, vec2(width, height))
    }

    /// Finds pins rect at specific position (excluding node frame margin).
    fn pins_rect(&self, frame: &Frame, spacing: &Spacing, openness: f32, pos: Pos2) -> Rect {
        let height = self.inputs_size.y.max(self.outputs_size.y);
        let width = self
            .title_size
            .x
            .max(self.inputs_size.x + spacing.item_spacing.x + self.outputs_size.x);

        let moved =
            (height + frame.total_margin().bottom + frame.total_margin().bottom) * (openness - 1.0);

        let pos = pos
            + vec2(
                0.0,
                self.title_size.y
                    + frame.total_margin().bottom
                    + frame.total_margin().bottom
                    + moved,
            );

        Rect::from_min_size(pos, vec2(width, height))
    }

    fn initial(spacing: &Spacing) -> Self {
        NodeState {
            title_size: spacing.interact_size,
            inputs_size: spacing.interact_size,
            outputs_size: spacing.interact_size,
        }
    }
}

impl<T> Snarl<T> {
    fn apply_effects(&mut self, effects: Effects<T>, cx: &Context) {
        if effects.effects.is_empty() {
            return;
        }
        cx.request_repaint();
        for effect in effects.effects {
            self.apply_effect(effect);
        }
    }

    fn apply_effect(&mut self, effect: Effect<T>) {
        match effect {
            Effect::Connect { from, to } => {
                if self.nodes.contains(from.node) && self.nodes.contains(to.node) {
                    self.wires.insert(wire_pins(from, to));
                }
            }
            Effect::Disconnect { from, to } => {
                if self.nodes.contains(from.node) && self.nodes.contains(to.node) {
                    self.wires.remove(&wire_pins(from, to));
                }
            }
            Effect::DropOutputs { pin } => {
                if self.nodes.contains(pin.node) {
                    self.wires.drop_outputs(pin);
                }
            }
            Effect::DropInputs { pin } => {
                if self.nodes.contains(pin.node) {
                    self.wires.drop_inputs(pin);
                }
            }
            Effect::RemoveNode { node } => {
                if self.nodes.contains(node) {
                    self.remove_node(node);
                }
            }
            Effect::OpenNode { node, open } => {
                if self.nodes.contains(node) {
                    self.nodes[node].open = open;
                }
            }
            Effect::Closure(f) => f(self),
        }
    }

    pub fn show<V>(&mut self, viewer: &mut V, style: &SnarlStyle, snarl_id: Id, ui: &mut Ui)
    where
        V: SnarlViewer<T>,
    {
        let mut effects = Effects::new();
        let mut node_moved = None;
        let mut node_order_to_top = None;

        self._show(
            viewer,
            style,
            snarl_id,
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
        snarl_id: Id,
        ui: &mut Ui,
        effects: &mut Effects<T>,
        node_moved: &mut Option<(usize, Vec2)>,
        node_order_to_top: &mut Option<usize>,
    ) where
        V: SnarlViewer<T>,
    {
        Frame::none()
            .fill(ui.visuals().widgets.inactive.bg_fill)
            .stroke(ui.visuals().widgets.inactive.bg_stroke)
            .show(ui, |ui| {
                let pin_size = style
                    .pin_size
                    .unwrap_or_else(|| ui.spacing().interact_size.y * 0.5);

                let wire_frame_size = style.wire_frame_size.unwrap_or(pin_size * 5.0);
                let wire_width = style.wire_width.unwrap_or_else(|| pin_size * 0.2);
                let title_drag_space = style
                    .title_drag_space
                    .unwrap_or_else(|| vec2(ui.spacing().icon_width, ui.spacing().icon_width));
                // let input_output_spacing = style
                //     .input_output_spacing
                //     .unwrap_or_else(|| ui.spacing().item_spacing.x);

                let collapsible = style.collapsible;

                let node_frame = Frame::window(ui.style());
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

                    let node_pos = node.pos + vec2(max_rect.min.x, max_rect.min.y);

                    // Generate persistent id for the node.
                    let node_id = snarl_id.with(("snarl-node", node_idx));

                    let openness = ui.ctx().animate_bool(node_id, node.open);

                    let state = NodeState::load(ui.ctx(), node_id)
                        .unwrap_or_else(|| NodeState::initial(ui.spacing()));

                    let mut new_state = state;

                    let node_rect = state.node_rect(&title_frame, ui.spacing(), node_pos);

                    let title_rect = state.title_rect(ui.spacing(), node_pos);
                    let pins_rect = state.pins_rect(&title_frame, ui.spacing(), openness, node_pos);

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
                                        vec2(ui.spacing().icon_width, ui.spacing().icon_width),
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
                                    for in_pin in inputs {
                                        // Show input pin.
                                        ui.with_layout(
                                            Layout::left_to_right(Align::Center),
                                            |ui| {
                                                // Allocate space for pin shape.
                                                let (pin_id, _) =
                                                    ui.allocate_space(vec2(pin_size, pin_size));

                                                // Show input content
                                                let r = viewer.show_input(&in_pin, ui, effects);
                                                let pin_info = r.inner;

                                                // Place pin shape on the left from pin content.
                                                let x = r.response.rect.left()
                                                    - pin_size * 0.5
                                                    - ui.style().spacing.item_spacing.x;

                                                // Centered vertically.
                                                let y = min_pin_y.max(
                                                    (r.response.rect.top()
                                                        + r.response.rect.bottom())
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

                                                input_positions.insert(in_pin.id, pin_pos);
                                                input_colors.insert(in_pin.id, pin_info.fill);
                                            },
                                        );
                                    }
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

                                                // Place pin shape on the right from pin content.
                                                let x = r.response.rect.right()
                                                    + pin_size * 0.5
                                                    + ui.style().spacing.item_spacing.x;

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

                            let pins_rect =
                                new_state.pins_rect(&title_frame, ui.spacing(), openness, node_pos);

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

                    if new_state != state {
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

/// Returns 6th degree bezier curve for the wire
fn wire_bezier(
    mut frame_size: f32,
    upscale: bool,
    downscale: bool,
    from: Pos2,
    to: Pos2,
) -> [Pos2; 6] {
    if upscale {
        frame_size = frame_size.max((from - to).length() / 4.0);
    }
    if downscale {
        frame_size = frame_size.min((from - to).length() / 4.0);
    }

    let from_norm_x = frame_size;
    let from_2 = pos2(from.x + from_norm_x, from.y);
    let to_norm_x = -from_norm_x;
    let to_2 = pos2(to.x + to_norm_x, to.y);

    let between = (from_2 - to_2).length();

    if from_2.x <= to_2.x && between >= frame_size * 2.0 {
        let middle_1 = from_2 + (to_2 - from_2).normalized() * frame_size;
        let middle_2 = to_2 + (from_2 - to_2).normalized() * frame_size;

        [from, from_2, middle_1, middle_2, to_2, to]
    } else if from_2.x <= to_2.x {
        let t =
            (between - (to_2.y - from_2.y).abs()) / (frame_size * 2.0 - (to_2.y - from_2.y).abs());

        let mut middle_1 = from_2 + (to_2 - from_2).normalized() * frame_size;
        let mut middle_2 = to_2 + (from_2 - to_2).normalized() * frame_size;

        if from_2.y >= to_2.y + frame_size {
            let u = (from_2.y - to_2.y - frame_size) / frame_size;

            let t0_middle_1 = pos2(from_2.x + (1.0 - u) * frame_size, from_2.y - frame_size * u);
            let t0_middle_2 = pos2(to_2.x, to_2.y + frame_size);

            middle_1 = t0_middle_1.lerp(middle_1, t);
            middle_2 = t0_middle_2.lerp(middle_2, t);
        } else if from_2.y >= to_2.y {
            let u = (from_2.y - to_2.y) / frame_size;

            let t0_middle_1 = pos2(from_2.x + u * frame_size, from_2.y + frame_size * (1.0 - u));
            let t0_middle_2 = pos2(to_2.x, to_2.y + frame_size);

            middle_1 = t0_middle_1.lerp(middle_1, t);
            middle_2 = t0_middle_2.lerp(middle_2, t);
        } else if to_2.y >= from_2.y + frame_size {
            let u = (to_2.y - from_2.y - frame_size) / frame_size;

            let t0_middle_1 = pos2(from_2.x, from_2.y + frame_size);
            let t0_middle_2 = pos2(to_2.x - (1.0 - u) * frame_size, to_2.y - frame_size * u);

            middle_1 = t0_middle_1.lerp(middle_1, t);
            middle_2 = t0_middle_2.lerp(middle_2, t);
        } else if to_2.y >= from_2.y {
            let u = (to_2.y - from_2.y) / frame_size;

            let t0_middle_1 = pos2(from_2.x, from_2.y + frame_size);
            let t0_middle_2 = pos2(to_2.x - u * frame_size, to_2.y + frame_size * (1.0 - u));

            middle_1 = t0_middle_1.lerp(middle_1, t);
            middle_2 = t0_middle_2.lerp(middle_2, t);
        } else {
            unreachable!();
        }

        [from, from_2, middle_1, middle_2, to_2, to]
    } else if from_2.y >= to_2.y + frame_size * 2.0 {
        let middle_1 = pos2(from_2.x, from_2.y - frame_size);
        let middle_2 = pos2(to_2.x, to_2.y + frame_size);

        [from, from_2, middle_1, middle_2, to_2, to]
    } else if from_2.y >= to_2.y + frame_size {
        let t = (from_2.y - to_2.y - frame_size) / frame_size;

        let middle_1 = pos2(from_2.x + (1.0 - t) * frame_size, from_2.y - frame_size * t);
        let middle_2 = pos2(to_2.x, to_2.y + frame_size);

        [from, from_2, middle_1, middle_2, to_2, to]
    } else if from_2.y >= to_2.y {
        let t = (from_2.y - to_2.y) / frame_size;

        let middle_1 = pos2(from_2.x + t * frame_size, from_2.y + frame_size * (1.0 - t));
        let middle_2 = pos2(to_2.x, to_2.y + frame_size);

        [from, from_2, middle_1, middle_2, to_2, to]
    } else if to_2.y >= from_2.y + frame_size * 2.0 {
        let middle_1 = pos2(from_2.x, from_2.y + frame_size);
        let middle_2 = pos2(to_2.x, to_2.y - frame_size);

        [from, from_2, middle_1, middle_2, to_2, to]
    } else if to_2.y >= from_2.y + frame_size {
        let t = (to_2.y - from_2.y - frame_size) / frame_size;

        let middle_1 = pos2(from_2.x, from_2.y + frame_size);
        let middle_2 = pos2(to_2.x - (1.0 - t) * frame_size, to_2.y - frame_size * t);

        [from, from_2, middle_1, middle_2, to_2, to]
    } else if to_2.y >= from_2.y {
        let t = (to_2.y - from_2.y) / frame_size;

        let middle_1 = pos2(from_2.x, from_2.y + frame_size);
        let middle_2 = pos2(to_2.x - t * frame_size, to_2.y + frame_size * (1.0 - t));

        [from, from_2, middle_1, middle_2, to_2, to]
    } else {
        unreachable!();
    }
}

fn draw_wire(
    shapes: &mut Vec<Shape>,
    frame_size: f32,
    upscale: bool,
    downscale: bool,
    from: Pos2,
    to: Pos2,
    stroke: Stroke,
) {
    draw_bezier(
        shapes,
        &wire_bezier(frame_size, upscale, downscale, from, to),
        stroke,
    );
}

fn hit_wire(
    pos: Pos2,
    frame_size: f32,
    upscale: bool,
    downscale: bool,
    from: Pos2,
    to: Pos2,
    threshold: f32,
) -> bool {
    let points = wire_bezier(frame_size, upscale, downscale, from, to);
    hit_bezier(pos, &points, threshold)
}

fn bezier_reference_size(points: &[Pos2; 6]) -> f32 {
    let [p0, p1, p2, p3, p4, p5] = *points;

    (p1 - p0).length()
        + (p2 - p1).length()
        + (p3 - p2).length()
        + (p4 - p3).length()
        + (p5 - p4).length()
}

fn bezier_samples_number(points: &[Pos2; 6], threshold: f32) -> usize {
    let reference_size = bezier_reference_size(points);
    (reference_size / threshold).ceil() as usize
}

fn draw_bezier(shapes: &mut Vec<Shape>, points: &[Pos2; 6], stroke: Stroke) {
    assert!(points.len() > 0);

    let samples = bezier_samples_number(points, stroke.width);

    let mut path = Vec::new();

    for i in 0..samples {
        let t = i as f32 / (samples - 1) as f32;
        path.push(sample_bezier(points, t));
    }

    let shape = Shape::Path(epaint::PathShape {
        points: path,
        closed: false,
        fill: Color32::TRANSPARENT,
        stroke,
    });

    shapes.push(shape);
}

fn sample_bezier(points: &[Pos2; 6], t: f32) -> Pos2 {
    let [p0, p1, p2, p3, p4, p5] = *points;

    let p0_0 = p0;
    let p1_0 = p1;
    let p2_0 = p2;
    let p3_0 = p3;
    let p4_0 = p4;
    let p5_0 = p5;

    let p0_1 = p0_0.lerp(p1_0, t);
    let p1_1 = p1_0.lerp(p2_0, t);
    let p2_1 = p2_0.lerp(p3_0, t);
    let p3_1 = p3_0.lerp(p4_0, t);
    let p4_1 = p4_0.lerp(p5_0, t);

    let p0_2 = p0_1.lerp(p1_1, t);
    let p1_2 = p1_1.lerp(p2_1, t);
    let p2_2 = p2_1.lerp(p3_1, t);
    let p3_2 = p3_1.lerp(p4_1, t);

    let p0_3 = p0_2.lerp(p1_2, t);
    let p1_3 = p1_2.lerp(p2_2, t);
    let p2_3 = p2_2.lerp(p3_2, t);

    let p0_4 = p0_3.lerp(p1_3, t);
    let p1_4 = p1_3.lerp(p2_3, t);

    let p0_5 = p0_4.lerp(p1_4, t);

    p0_5
}

fn split_bezier(points: &[Pos2; 6], t: f32) -> [[Pos2; 6]; 2] {
    let [p0, p1, p2, p3, p4, p5] = *points;

    let p0_0 = p0;
    let p1_0 = p1;
    let p2_0 = p2;
    let p3_0 = p3;
    let p4_0 = p4;
    let p5_0 = p5;

    let p0_1 = p0_0.lerp(p1_0, t);
    let p1_1 = p1_0.lerp(p2_0, t);
    let p2_1 = p2_0.lerp(p3_0, t);
    let p3_1 = p3_0.lerp(p4_0, t);
    let p4_1 = p4_0.lerp(p5_0, t);

    let p0_2 = p0_1.lerp(p1_1, t);
    let p1_2 = p1_1.lerp(p2_1, t);
    let p2_2 = p2_1.lerp(p3_1, t);
    let p3_2 = p3_1.lerp(p4_1, t);

    let p0_3 = p0_2.lerp(p1_2, t);
    let p1_3 = p1_2.lerp(p2_2, t);
    let p2_3 = p2_2.lerp(p3_2, t);

    let p0_4 = p0_3.lerp(p1_3, t);
    let p1_4 = p1_3.lerp(p2_3, t);

    let p0_5 = p0_4.lerp(p1_4, t);

    [
        [p0_0, p0_1, p0_2, p0_3, p0_4, p0_5],
        [p0_5, p1_4, p2_3, p3_2, p4_1, p5_0],
    ]
}

fn hit_bezier(pos: Pos2, points: &[Pos2; 6], threshold: f32) -> bool {
    let aabb = Rect::from_points(points);

    if pos.x + threshold < aabb.left() {
        return false;
    }
    if pos.x - threshold > aabb.right() {
        return false;
    }
    if pos.y + threshold < aabb.top() {
        return false;
    }
    if pos.y - threshold > aabb.bottom() {
        return false;
    }

    let samples = bezier_samples_number(points, threshold);
    if samples > 16 {
        let [points1, points2] = split_bezier(points, 0.5);

        return hit_bezier(pos, &points1, threshold) || hit_bezier(pos, &points2, threshold);
    }

    for i in 0..samples {
        let t = i as f32 / (samples - 1) as f32;
        let p = sample_bezier(points, t);
        if (p - pos).length() < threshold {
            return true;
        }
    }

    false
}

fn draw_pin(painter: &Painter, pin: PinInfo, pos: Pos2, base_size: f32) {
    let size = base_size * pin.size;
    match pin.shape {
        PinShape::Circle => {
            painter.circle(pos, size * 0.5, pin.fill, pin.stroke);
        }
        PinShape::Triangle => {
            const A: Vec2 = vec2(-0.64951905283832895, 0.4875);
            const B: Vec2 = vec2(0.64951905283832895, 0.4875);
            const C: Vec2 = vec2(0.0, -0.6375);

            let points = vec![pos + A * size, pos + B * size, pos + C * size];

            painter.add(Shape::Path(PathShape {
                points,
                closed: true,
                fill: pin.fill,
                stroke: pin.stroke,
            }));
        }
        PinShape::Square => {
            let points = vec![
                pos + vec2(-0.5, -0.5) * size,
                pos + vec2(0.5, -0.5) * size,
                pos + vec2(0.5, 0.5) * size,
                pos + vec2(-0.5, 0.5) * size,
            ];

            painter.add(Shape::Path(PathShape {
                points,
                closed: true,
                fill: pin.fill,
                stroke: pin.stroke,
            }));
        }
    }
}

fn mix_colors(a: Color32, b: Color32) -> Color32 {
    let [or, og, ob, oa] = a.to_array();
    let [ir, ig, ib, ia] = b.to_array();

    Color32::from_rgba_premultiplied(
        or / 2 + ir / 2,
        og / 2 + ig / 2,
        ob / 2 + ib / 2,
        oa / 2 + ia / 2,
    )
}
