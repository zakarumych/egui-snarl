//!
//! # egui-snarl
//!
//! Provides a node-graph container for egui.
//!
//!

pub mod ui;

use egui::{ahash::HashSet, Pos2};
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
    value: T,

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
            value: node,
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
            value: node,
            pos,
            open: false,
        });
        self.draw_order.push(idx);
        idx
    }

    /// Opens or collapses a node.
    #[track_caller]
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
    #[track_caller]
    pub fn remove_node(&mut self, idx: usize) -> T {
        let value = self.nodes.remove(idx).value;
        self.wires.drop_node(idx);
        let order = self.draw_order.iter().position(|&i| i == idx).unwrap();
        self.draw_order.remove(order);
        value
    }

    /// Connects two nodes.
    /// Returns true if the connection was successful.
    /// Returns false if the connection already exists.
    #[track_caller]
    pub fn connect(&mut self, from: OutPinId, to: InPinId) -> bool {
        debug_assert!(self.nodes.contains(from.node));
        debug_assert!(self.nodes.contains(to.node));

        let wire = Wire {
            out_pin: from,
            in_pin: to,
        };
        self.wires.insert(wire)
    }

    #[track_caller]
    pub fn disconnect(&mut self, from: OutPinId, to: InPinId) {
        debug_assert!(self.nodes.contains(from.node));
        debug_assert!(self.nodes.contains(to.node));

        let wire = Wire {
            out_pin: from,
            in_pin: to,
        };
        self.wires.remove(&wire);
    }

    #[track_caller]
    pub fn drop_inputs(&mut self, pin: InPinId) {
        debug_assert!(self.nodes.contains(pin.node));

        self.wires.drop_inputs(pin);
    }

    #[track_caller]
    pub fn drop_outputs(&mut self, pin: OutPinId) {
        debug_assert!(self.nodes.contains(pin.node));

        self.wires.drop_outputs(pin);
    }

    /// Returns reference to the node.
    #[track_caller]
    pub fn get_node(&self, idx: usize) -> &T {
        &self.nodes[idx].value
    }

    /// Returns mutable reference to the node.
    #[track_caller]
    pub fn get_node_mut(&mut self, idx: usize) -> &mut T {
        &mut self.nodes[idx].value
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

    pub fn nodes_pos(&self) -> NodesPosIter<'_, T> {
        NodesPosIter {
            nodes: self.nodes.iter(),
        }
    }

    pub fn nodes_pos_mut(&mut self) -> NodesPosIterMut<'_, T> {
        NodesPosIterMut {
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

    pub fn nodes_pos_indices(&self) -> NodesPosIndicesIter<'_, T> {
        NodesPosIndicesIter {
            nodes: self.nodes.iter(),
        }
    }

    pub fn nodes_pos_indices_mut(&mut self) -> NodesPosIndicesIterMut<'_, T> {
        NodesPosIndicesIterMut {
            nodes: self.nodes.iter_mut(),
        }
    }

    pub fn in_pin(&self, pin: InPinId) -> InPin {
        InPin::new(self, pin)
    }

    pub fn out_pin(&self, pin: OutPinId) -> OutPin {
        OutPin::new(self, pin)
    }
}

pub struct NodesIter<'a, T> {
    nodes: slab::Iter<'a, Node<T>>,
}

impl<'a, T> Iterator for NodesIter<'a, T> {
    type Item = &'a T;

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.nodes.size_hint()
    }

    fn next(&mut self) -> Option<&'a T> {
        let (_, node) = self.nodes.next()?;
        Some(&node.value)
    }

    fn nth(&mut self, n: usize) -> Option<&'a T> {
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
        Some(&mut node.value)
    }

    fn nth(&mut self, n: usize) -> Option<&'a mut T> {
        let (_, node) = self.nodes.nth(n)?;
        Some(&mut node.value)
    }
}

pub struct NodesPosIter<'a, T> {
    nodes: slab::Iter<'a, Node<T>>,
}

impl<'a, T> Iterator for NodesPosIter<'a, T> {
    type Item = (Pos2, &'a T);

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.nodes.size_hint()
    }

    fn next(&mut self) -> Option<(Pos2, &'a T)> {
        let (_, node) = self.nodes.next()?;
        Some((node.pos, &node.value))
    }

    fn nth(&mut self, n: usize) -> Option<(Pos2, &'a T)> {
        let (_, node) = self.nodes.nth(n)?;
        Some((node.pos, &node.value))
    }
}

pub struct NodesPosIterMut<'a, T> {
    nodes: slab::IterMut<'a, Node<T>>,
}

impl<'a, T> Iterator for NodesPosIterMut<'a, T> {
    type Item = (Pos2, &'a mut T);

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.nodes.size_hint()
    }

    fn next(&mut self) -> Option<(Pos2, &'a mut T)> {
        let (_, node) = self.nodes.next()?;
        Some((node.pos, &mut node.value))
    }

    fn nth(&mut self, n: usize) -> Option<(Pos2, &'a mut T)> {
        let (_, node) = self.nodes.nth(n)?;
        Some((node.pos, &mut node.value))
    }
}

pub struct NodesIndicesIter<'a, T> {
    nodes: slab::Iter<'a, Node<T>>,
}

impl<'a, T> Iterator for NodesIndicesIter<'a, T> {
    type Item = (usize, &'a T);

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.nodes.size_hint()
    }

    fn next(&mut self) -> Option<(usize, &'a T)> {
        let (idx, node) = self.nodes.next()?;
        Some((idx, &node.value))
    }

    fn nth(&mut self, n: usize) -> Option<(usize, &'a T)> {
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
        Some((idx, &mut node.value))
    }

    fn nth(&mut self, n: usize) -> Option<(usize, &'a mut T)> {
        let (idx, node) = self.nodes.nth(n)?;
        Some((idx, &mut node.value))
    }
}

pub struct NodesPosIndicesIter<'a, T> {
    nodes: slab::Iter<'a, Node<T>>,
}

impl<'a, T> Iterator for NodesPosIndicesIter<'a, T> {
    type Item = (usize, Pos2, &'a T);

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.nodes.size_hint()
    }

    fn next(&mut self) -> Option<(usize, Pos2, &'a T)> {
        let (idx, node) = self.nodes.next()?;
        Some((idx, node.pos, &node.value))
    }

    fn nth(&mut self, n: usize) -> Option<(usize, Pos2, &'a T)> {
        let (idx, node) = self.nodes.nth(n)?;
        Some((idx, node.pos, &node.value))
    }
}

pub struct NodesPosIndicesIterMut<'a, T> {
    nodes: slab::IterMut<'a, Node<T>>,
}

impl<'a, T> Iterator for NodesPosIndicesIterMut<'a, T> {
    type Item = (usize, Pos2, &'a mut T);

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.nodes.size_hint()
    }

    fn next(&mut self) -> Option<(usize, Pos2, &'a mut T)> {
        let (idx, node) = self.nodes.next()?;
        Some((idx, node.pos, &mut node.value))
    }

    fn nth(&mut self, n: usize) -> Option<(usize, Pos2, &'a mut T)> {
        let (idx, node) = self.nodes.nth(n)?;
        Some((idx, node.pos, &mut node.value))
    }
}

/// Node and its output pin.
#[derive(Clone, Debug)]
pub struct OutPin {
    pub id: OutPinId,
    pub remotes: Vec<InPinId>,
}

/// Node and its output pin.
#[derive(Clone, Debug)]
pub struct InPin {
    pub id: InPinId,
    pub remotes: Vec<OutPinId>,
}

impl OutPin {
    fn new<T>(snarl: &Snarl<T>, pin: OutPinId) -> Self {
        OutPin {
            id: pin,
            remotes: snarl.wires.wired_inputs(pin).collect(),
        }
    }
}

impl InPin {
    fn new<T>(snarl: &Snarl<T>, pin: InPinId) -> Self {
        InPin {
            id: pin,
            remotes: snarl.wires.wired_outputs(pin).collect(),
        }
    }
}
