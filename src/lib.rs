//!
//! # egui-snarl
//!
//! Provides a node-graph container for egui.
//!
//!

mod ui;

use std::{borrow::Cow, cell::RefCell};

use egui::{ahash::HashSet, Ui};
use slab::Slab;

impl<T> Default for Snarl<T> {
    fn default() -> Self {
        Snarl::new()
    }
}

/// Node's pin that contains local idx, remove idx and remove node reference.
pub struct Pin<'a, T> {
    pub local: usize,
    pub remote: Vec<Remote<'a, T>>,
}

pub struct Remote<'a, T> {
    pub idx: usize,
    pub node: &'a RefCell<T>,
}

pub enum Effect<T> {
    /// Adds connection between two nodes.
    Connect {
        from_node: usize,
        from_output: usize,
        to_node: usize,
        to_input: usize,
    },

    /// Removes connection between two nodes.
    Disconnect {
        from_node: usize,
        from_output: usize,
        to_node: usize,
        to_input: usize,
    },

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
    fn add_node(&mut self, idx: usize, node: &T) -> Result<Effects<T>, Forbidden> {
        let _ = (idx, node);
        Ok(Effects::default())
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
    ) -> Result<Effects<T>, Forbidden> {
        let _ = (idx, node, inputs, outputs);
        Ok(Effects::default())
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
    ) -> egui::InnerResponse<Effects<T>>;

    fn show_output(
        &mut self,
        node: &RefCell<T>,
        pin: Pin<T>,
        ui: &mut Ui,
    ) -> egui::InnerResponse<Effects<T>>;
}

/// Error returned from methods where `Viewer` forbids the operation.
pub struct Forbidden;

#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
struct Node<T> {
    value: RefCell<T>,
    rect: egui::Rect,
}

/// Connection between two nodes.
///
/// Nodes may support multiple connections to the same input or output.
/// But duplicate connections between same input and the same output are not allowed.
/// Attempt to insert existing connection will be ignored.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
struct Wire {
    from_node: usize,
    from_output: usize,
    to_node: usize,
    to_input: usize,
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

    pub fn drop_with_node(&mut self, node: usize) {
        self.wires
            .retain(|wire| wire.from_node != node && wire.to_node != node);
    }

    pub fn wired_inputs(
        &self,
        node: usize,
        output: usize,
    ) -> impl Iterator<Item = (usize, usize)> + '_ {
        self.wires
            .iter()
            .filter(move |wire| wire.from_node == node && wire.from_output == output)
            .map(|wire| (wire.to_node, wire.to_input))
    }

    pub fn wired_outputs(
        &self,
        node: usize,
        input: usize,
    ) -> impl Iterator<Item = (usize, usize)> + '_ {
        self.wires
            .iter()
            .filter(move |wire| wire.to_node == node && wire.to_input == input)
            .map(|wire| (wire.from_node, wire.from_output))
    }
}

/// Snarl is node-graph container.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Snarl<T> {
    nodes: Slab<Node<T>>,
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
    pub fn add_node(&mut self, node: T, rect: egui::Rect) -> usize {
        self.nodes.insert(Node {
            value: RefCell::new(node),
            rect,
        })
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
        self.nodes.remove(idx).value.into_inner()
    }

    /// Connects two nodes.
    /// Returns true if the connection was successful.
    /// Returns false if the connection already exists.
    pub fn connect(
        &mut self,
        from_node: usize,
        from_output: usize,
        to_node: usize,
        to_input: usize,
    ) -> bool {
        debug_assert!(self.nodes.contains(from_node));
        debug_assert!(self.nodes.contains(to_node));

        let wire = Wire {
            from_node,
            from_output,
            to_node,
            to_input,
        };
        self.wires.insert(wire)
    }
}

struct SnarlView<'a, T, V> {
    snarl: &'a mut Snarl<T>,
    viewer: &'a mut V,
    pos: egui::Pos2,
    scale: f32,
}

/// Methods that use viewer hooks.
impl<T, V> SnarlView<'_, T, V>
where
    V: SnarlViewer<T>,
{
    fn apply_effects(&mut self, response: Effects<T>) {
        for effect in response.effects {
            self.apply_effect(effect);
        }
    }

    fn apply_effect(&mut self, effect: Effect<T>) {
        match effect {
            Effect::Connect {
                from_node,
                from_output,
                to_node,
                to_input,
            } => {
                let wire = Wire {
                    from_node,
                    from_output,
                    to_node,
                    to_input,
                };
                self.snarl.wires.insert(wire);
            }
            Effect::Disconnect {
                from_node,
                from_output,
                to_node,
                to_input,
            } => {
                let wire = Wire {
                    from_node,
                    from_output,
                    to_node,
                    to_input,
                };
                self.snarl.wires.remove(&wire);
            }
            Effect::Closure(f) => f(self.snarl),
        }
    }

    /// Creates new node using a viewer hook
    /// and places it in the Snarl.
    fn add_node(&mut self, node: T, rect: egui::Rect) -> Result<(), Forbidden> {
        let idx = self.snarl.nodes.vacant_key();
        let e = self.viewer.add_node(idx, &node)?;
        self.snarl.nodes.insert(Node {
            value: RefCell::new(node),
            rect,
        });
        self.apply_effects(e);
        Ok(())
    }
}
