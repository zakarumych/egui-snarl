use std::cell::RefCell;

use egui::Pos2;

use crate::{wire_pins, InPinId, Node, OutPinId, Snarl};

pub enum Effect<T> {
    /// Adds a new node to the Snarl.
    InsertNode { pos: Pos2, node: T },

    /// Removes a node from snarl.
    RemoveNode { node: NodeId },

    /// Opens/closes a node.
    OpenNode { node: NodeId, open: bool },

    /// Adds connection between two nodes.
    Connect { from: OutPinId, to: InPinId },

    /// Removes connection between two nodes.
    Disconnect { from: OutPinId, to: InPinId },

    /// Removes all connections from the output pin.
    DropOutputs { pin: OutPinId },

    /// Removes all connections to the input pin.
    DropInputs { pin: InPinId },

    /// Executes a closure with mutable reference to the Snarl.
    Closure(Box<dyn FnOnce(&mut Snarl<T>)>),
}

/// Contained for deferred execution of effects.
/// It is populated by [`SnarlViewer`] methods and then applied to the Snarl.
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
    #[inline(always)]
    #[doc(hidden)]
    pub fn new() -> Self {
        Effects {
            effects: Vec::new(),
        }
    }

    /// Returns `true` if there are no effects.
    /// Returns `false` otherwise.
    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.effects.is_empty()
    }

    /// Inserts a new node to the Snarl.
    #[inline(always)]
    pub fn insert_node(&mut self, pos: Pos2, node: T) {
        self.effects.push(Effect::InsertNode { node, pos });
    }

    /// Removes a node from the Snarl.
    #[inline(always)]
    pub fn remove_node(&mut self, node: NodeId) {
        self.effects.push(Effect::RemoveNode { node });
    }

    /// Opens/closes a node.
    #[inline(always)]
    pub fn open_node(&mut self, node: NodeId, open: bool) {
        self.effects.push(Effect::OpenNode { node, open });
    }

    /// Connects two nodes.
    #[inline(always)]
    pub fn connect(&mut self, from: OutPinId, to: InPinId) {
        self.effects.push(Effect::Connect { from, to });
    }

    /// Disconnects two nodes.
    #[inline(always)]
    pub fn disconnect(&mut self, from: OutPinId, to: InPinId) {
        self.effects.push(Effect::Disconnect { from, to });
    }

    /// Removes all connections from the output pin.
    #[inline(always)]
    pub fn drop_inputs(&mut self, pin: InPinId) {
        self.effects.push(Effect::DropInputs { pin });
    }

    /// Removes all connections to the input pin.
    #[inline(always)]
    pub fn drop_outputs(&mut self, pin: OutPinId) {
        self.effects.push(Effect::DropOutputs { pin });
    }
}

impl<T> Snarl<T> {
    pub fn apply_effects(&mut self, effects: Effects<T>) {
        if effects.effects.is_empty() {
            return;
        }
        for effect in effects.effects {
            self.apply_effect(effect);
        }
    }

    pub fn apply_effect(&mut self, effect: Effect<T>) {
        match effect {
            Effect::InsertNode { node, pos } => {
                let idx = self.nodes.insert(Node {
                    value: RefCell::new(node),
                    pos,
                    open: true,
                });
                self.draw_order.push(idx);
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
            Effect::Closure(f) => f(self),
        }
    }
}
