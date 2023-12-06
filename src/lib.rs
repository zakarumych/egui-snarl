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
    /// Node generic value.
    value: RefCell<T>,

    /// Position of the top-left corner of the node.
    /// This does not include frame margin.
    pos: egui::Pos2,

    /// Flag indicating that the node is open - not collapsed.
    open: bool,
}

/// Output pin identifier. Cosists of node index and pin index.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct OutPinId {
    /// Node index.
    pub node: usize,

    /// Output pin index.
    pub output: usize,
}

/// Input pin identifier. Cosists of node index and pin index.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct InPinId {
    /// Node index.
    pub node: usize,

    /// Input pin index.
    pub input: usize,
}

/// Connection between two nodes.
///
/// Nodes may support multiple connections to the same input or output.
/// But duplicate connections between same input and the same output are not allowed.
/// Attempt to insert existing connection will be ignored.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
struct Wire {
    out_pin: OutPinId,
    in_pin: InPinId,
}

fn wire_pins(out_pin: OutPinId, in_pin: InPinId) -> Wire {
    Wire { out_pin, in_pin }
}

#[derive(Clone, Debug)]
struct Wires {
    wires: HashSet<Wire>,
}

#[cfg(feature = "serde")]
impl serde::Serialize for Wires {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeSeq;

        let mut seq = serializer.serialize_seq(Some(self.wires.len()))?;
        for wire in &self.wires {
            seq.serialize_element(&wire)?;
        }
        seq.end()
    }
}

#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for Wires {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct Visitor;

        impl<'de> serde::de::Visitor<'de> for Visitor {
            type Value = HashSet<Wire>;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a sequence of wires")
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: serde::de::SeqAccess<'de>,
            {
                let mut wires = HashSet::with_hasher(egui::ahash::RandomState::new());
                while let Some(wire) = seq.next_element()? {
                    wires.insert(wire);
                }
                Ok(wires)
            }
        }

        let wires = deserializer.deserialize_seq(Visitor)?;
        Ok(Wires { wires })
    }
}

impl Wires {
    fn new() -> Self {
        Wires {
            wires: HashSet::with_hasher(egui::ahash::RandomState::new()),
        }
    }

    fn insert(&mut self, wire: Wire) -> bool {
        self.wires.insert(wire)
    }

    fn remove(&mut self, wire: &Wire) -> bool {
        self.wires.remove(wire)
    }

    fn drop_node(&mut self, node: usize) {
        self.wires
            .retain(|wire| wire.out_pin.node != node && wire.in_pin.node != node);
    }

    fn drop_inputs(&mut self, pin: InPinId) {
        self.wires.retain(|wire| wire.in_pin != pin);
    }

    fn drop_outputs(&mut self, pin: OutPinId) {
        self.wires.retain(|wire| wire.out_pin != pin);
    }

    fn wired_inputs(&self, out_pin: OutPinId) -> impl Iterator<Item = InPinId> + '_ {
        self.wires
            .iter()
            .filter(move |wire| wire.out_pin == out_pin)
            .map(|wire| (wire.in_pin))
    }

    fn wired_outputs(&self, in_pin: InPinId) -> impl Iterator<Item = OutPinId> + '_ {
        self.wires
            .iter()
            .filter(move |wire| wire.in_pin == in_pin)
            .map(|wire| (wire.out_pin))
    }

    fn iter(&self) -> impl Iterator<Item = Wire> + '_ {
        self.wires.iter().copied()
    }
}

/// Snarl is generic node-graph container.
///
/// It holds graph state - positioned nodes and wires between their pins.
/// It can be rendered using [`Snarl::show`].
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Snarl<T> {
    // #[cfg_attr(feature = "serde", serde(with = "serde_nodes"))]
    nodes: Slab<Node<T>>,
    draw_order: Vec<usize>,
    wires: Wires,
}

// #[cfg(feature = "serde")]
// mod serde_nodes {
//     use super::*;

//     fn serialize<S, T>(nodes: &Slab<Node<T>>, serializer: S) -> Result<S::Ok, S::Error>
//     where
//         S: serde::Serializer,
//         T: serde::Serialize,
//     {
//         use serde::ser::SerializeMap;

//         let mut map = serializer.serialize_map(Some(nodes.len()))?;
//         for (idx, node) in nodes.iter() {
//             map.serialize_entry(&idx, &node)?;
//         }
//         map.end()
//     }

//     fn deserialize<'de, D, T>(deserializer: D) -> Slab<Node<T>>
//     where
//         D: serde::Deserializer<'de>,
//         T: serde::Deserialize<'de>,
//     {
//         struct Visitor<T>(std::marker::PhantomData<T>);

//         impl<'de, T> serde::de::Visitor<'de> for Visitor<T>
//         where
//             T: serde::Deserialize<'de>,
//         {
//             type Value = Slab<Node<T>>;

//             fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
//                 formatter.write_str("a map of nodes")
//             }

//             fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
//             where
//                 A: serde::de::MapAccess<'de>,
//             {
//                 let mut nodes = Slab::new();
//                 while let Some((idx, node)) = map.next_entry()? {
//                     let next = nodes.insert()
//                 }
//                 Ok(nodes)
//             }
//         }

//         deserializer.deserialize_map(Visitor(std::marker::PhantomData))
//     }
// }

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
    /// snarl.insert_node(());
    /// ```
    pub fn insert_node(&mut self, pos: egui::Pos2, node: T) -> usize {
        let idx = self.nodes.insert(Node {
            value: RefCell::new(node),
            pos,
            open: true,
        });
        self.draw_order.push(idx);
        idx
    }

    /// Adds a node to the Snarl in collapsed state.
    /// Returns the index of the node.
    ///
    /// # Examples
    ///
    /// ```
    /// # use egui_snarl::Snarl;
    /// let mut snarl = Snarl::<()>::new();
    /// snarl.insert_node(());
    /// ```
    pub fn add_node_collapsed(&mut self, pos: egui::Pos2, node: T) -> usize {
        let idx = self.nodes.insert(Node {
            value: RefCell::new(node),
            pos,
            open: false,
        });
        self.draw_order.push(idx);
        idx
    }

    /// Opens or collapses a node.
    pub fn open_node(&mut self, node: usize, open: bool) {
        self.nodes[node].open = open;
    }

    /// Removes a node from the Snarl.
    /// Returns the node if it was removed.
    ///
    /// # Examples
    ///
    /// ```
    /// # use egui_snarl::Snarl;
    /// let mut snarl = Snarl::<()>::new();
    /// let node = snarl.insert_node(());
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
    pub fn connect(&mut self, from: OutPinId, to: InPinId) -> bool {
        debug_assert!(self.nodes.contains(from.node));
        debug_assert!(self.nodes.contains(to.node));

        let wire = Wire {
            out_pin: from,
            in_pin: to,
        };
        self.wires.insert(wire)
    }

    pub fn nodes(&self) -> NodesIter<'_, T> {
        NodesIter {
            nodes: self.nodes.iter(),
        }
    }

    pub fn nodes_mut(&mut self) -> NodesIterMut<'_, T> {
        NodesIterMut {
            nodes: self.nodes.iter_mut(),
        }
    }

    pub fn node_indices(&self) -> NodesIndicesIter<'_, T> {
        NodesIndicesIter {
            nodes: self.nodes.iter(),
        }
    }

    pub fn nodes_indices_mut(&mut self) -> NodesIndicesIterMut<'_, T> {
        NodesIndicesIterMut {
            nodes: self.nodes.iter_mut(),
        }
    }
}

pub struct NodesIter<'a, T> {
    nodes: slab::Iter<'a, Node<T>>,
}

impl<'a, T> Iterator for NodesIter<'a, T> {
    type Item = &'a RefCell<T>;

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.nodes.size_hint()
    }

    fn next(&mut self) -> Option<&'a RefCell<T>> {
        let (_, node) = self.nodes.next()?;
        Some(&node.value)
    }

    fn nth(&mut self, n: usize) -> Option<&'a RefCell<T>> {
        let (_, node) = self.nodes.nth(n)?;
        Some(&node.value)
    }
}

pub struct NodesIterMut<'a, T> {
    nodes: slab::IterMut<'a, Node<T>>,
}

impl<'a, T> Iterator for NodesIterMut<'a, T> {
    type Item = &'a mut T;

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.nodes.size_hint()
    }

    fn next(&mut self) -> Option<&'a mut T> {
        let (_, node) = self.nodes.next()?;
        Some(node.value.get_mut())
    }

    fn nth(&mut self, n: usize) -> Option<&'a mut T> {
        let (_, node) = self.nodes.nth(n)?;
        Some(node.value.get_mut())
    }
}

pub struct NodesIndicesIter<'a, T> {
    nodes: slab::Iter<'a, Node<T>>,
}

impl<'a, T> Iterator for NodesIndicesIter<'a, T> {
    type Item = (usize, &'a RefCell<T>);

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.nodes.size_hint()
    }

    fn next(&mut self) -> Option<(usize, &'a RefCell<T>)> {
        let (idx, node) = self.nodes.next()?;
        Some((idx, &node.value))
    }

    fn nth(&mut self, n: usize) -> Option<(usize, &'a RefCell<T>)> {
        let (idx, node) = self.nodes.nth(n)?;
        Some((idx, &node.value))
    }
}

pub struct NodesIndicesIterMut<'a, T> {
    nodes: slab::IterMut<'a, Node<T>>,
}

impl<'a, T> Iterator for NodesIndicesIterMut<'a, T> {
    type Item = (usize, &'a mut T);

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.nodes.size_hint()
    }

    fn next(&mut self) -> Option<(usize, &'a mut T)> {
        let (idx, node) = self.nodes.next()?;
        Some((idx, node.value.get_mut()))
    }

    fn nth(&mut self, n: usize) -> Option<(usize, &'a mut T)> {
        let (idx, node) = self.nodes.nth(n)?;
        Some((idx, node.value.get_mut()))
    }
}
