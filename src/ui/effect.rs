use std::cell::RefCell;

use egui::{Context, Pos2};

use crate::{wire_pins, InPinId, Node, OutPinId, Snarl};

/// Error returned from methods where `Viewer` forbids the operation.
pub struct Forbidden;

pub enum Effect<T> {
    /// Adds a new node to the Snarl.
    Insert { node: T, pos: Pos2 },

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

    pub fn insert(&mut self, node: T, pos: Pos2) {
        self.effects.push(Effect::Insert { node, pos });
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

impl<T> Snarl<T> {
    pub(crate) fn apply_effects(&mut self, effects: Effects<T>, cx: &Context) {
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
            Effect::Insert { node, pos } => {
                let idx = self.nodes.insert(Node {
                    value: RefCell::new(node),
                    pos,
                    open: true,
                });
                self.draw_order.push(idx);
            }
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
}
