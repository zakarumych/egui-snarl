//!
//! # egui-snarl
//!
//! Provides a node-graph container for egui.
//!
//!

pub mod ui;

use std::cell::RefCell;

use egui::ahash::HashSet;
use slab::Slab;

impl<T> Default for Snarl<T> {
    fn default() -> Self {
        Snarl::new()
    }
}

#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
struct Node<T> {
    value: RefCell<T>,
    pos: egui::Pos2,
}

/// Connection between two nodes.
///
/// Nodes may support multiple connections to the same input or output.
/// But duplicate connections between same input and the same output are not allowed.
/// Attempt to insert existing connection will be ignored.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
struct Wire {
    #[cfg_attr(feature = "serde", serde(flatten))]
    out_pin: OutPin,

    #[cfg_attr(feature = "serde", serde(flatten))]
    in_pin: InPin,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct OutPin {
    pub node: usize,
    pub output: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct InPin {
    pub node: usize,
    pub input: usize,
}

fn wire_pins(out_pin: OutPin, in_pin: InPin) -> Wire {
    Wire { out_pin, in_pin }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
enum AnyPin {
    Out(OutPin),
    In(InPin),
}

#[derive(Clone, Debug)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(transparent)
)]
struct Wires {
    wires: HashSet<Wire>,
}

impl Wires {
    pub fn new() -> Self {
        Wires {
            wires: HashSet::with_hasher(egui::ahash::RandomState::new()),
        }
    }

    pub fn insert(&mut self, wire: Wire) -> bool {
        self.wires.insert(wire)
    }

    pub fn remove(&mut self, wire: &Wire) -> bool {
        self.wires.remove(wire)
    }

    pub fn drop_node(&mut self, node: usize) {
        self.wires
            .retain(|wire| wire.out_pin.node != node && wire.in_pin.node != node);
    }

    pub fn drop_inputs(&mut self, pin: InPin) {
        self.wires.retain(|wire| wire.in_pin != pin);
    }

    pub fn drop_outputs(&mut self, pin: OutPin) {
        self.wires.retain(|wire| wire.out_pin != pin);
    }

    pub fn wired_inputs(&self, out_pin: OutPin) -> impl Iterator<Item = InPin> + '_ {
        self.wires
            .iter()
            .filter(move |wire| wire.out_pin == out_pin)
            .map(|wire| (wire.in_pin))
    }

    pub fn wired_outputs(&self, in_pin: InPin) -> impl Iterator<Item = OutPin> + '_ {
        self.wires
            .iter()
            .filter(move |wire| wire.in_pin == in_pin)
            .map(|wire| (wire.out_pin))
    }

    pub fn iter(&self) -> impl Iterator<Item = Wire> + '_ {
        self.wires.iter().copied()
    }
}

/// Snarl is node-graph container.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Snarl<T> {
    nodes: Slab<Node<T>>,
    draw_order: Vec<usize>,
    wires: Wires,
}

impl<T> Snarl<T> {
    /// Create a new empty Snarl.
    ///
    /// # Examples
    ///
    /// ```
    /// # use egui_snarl::Snarl;
    /// let snarl = Snarl::<()>::new();
    /// ```
    pub fn new() -> Self {
        Snarl {
            nodes: Slab::new(),
            draw_order: Vec::new(),
            wires: Wires::new(),
        }
    }

    /// Adds a node to the Snarl.
    /// Returns the index of the node.
    ///
    /// # Examples
    ///
    /// ```
    /// # use egui_snarl::Snarl;
    /// let mut snarl = Snarl::<()>::new();
    /// snarl.add_node(());
    /// ```
    pub fn add_node(&mut self, node: T, pos: egui::Pos2) -> usize {
        let idx = self.nodes.insert(Node {
            value: RefCell::new(node),
            pos,
        });
        self.draw_order.push(idx);
        idx
    }

    /// Removes a node from the Snarl.
    /// Returns the node if it was removed.
    ///
    /// # Examples
    ///
    /// ```
    /// # use egui_snarl::Snarl;
    /// let mut snarl = Snarl::<()>::new();
    /// let node = snarl.add_node(());
    /// snarl.remove_node(node);
    /// ```
    pub fn remove_node(&mut self, idx: usize) -> T {
        let value = self.nodes.remove(idx).value.into_inner();
        self.wires.drop_node(idx);
        let order = self.draw_order.iter().position(|&i| i == idx).unwrap();
        self.draw_order.remove(order);
        value
    }

    /// Connects two nodes.
    /// Returns true if the connection was successful.
    /// Returns false if the connection already exists.
    pub fn connect(&mut self, from: OutPin, to: InPin) -> bool {
        debug_assert!(self.nodes.contains(from.node));
        debug_assert!(self.nodes.contains(to.node));

        let wire = Wire {
            out_pin: from,
            in_pin: to,
        };
        self.wires.insert(wire)
    }
}
