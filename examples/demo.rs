#![allow(clippy::use_self)]

use std::collections::HashMap;

use eframe::{App, CreationContext};
use egui::{Color32, Id, Key, KeyboardShortcut, Modifiers, Ui};
use egui_snarl::{
    InPin, InPinId, NodeId, OutPin, OutPinId, Snarl,
    ui::{
        AnyPins, NodeLayout, PinInfo, PinPlacement, SnarlStyle, SnarlViewer, SnarlWidget,
        WireStyle, get_selected_nodes,
    },
};

const STRING_COLOR: Color32 = Color32::from_rgb(0x00, 0xb0, 0x00);
const NUMBER_COLOR: Color32 = Color32::from_rgb(0xb0, 0x00, 0x00);
const IMAGE_COLOR: Color32 = Color32::from_rgb(0xb0, 0x00, 0xb0);
const UNTYPED_COLOR: Color32 = Color32::from_rgb(0xb0, 0xb0, 0xb0);
const COPY_SHORTCUT: KeyboardShortcut = KeyboardShortcut::new(Modifiers::COMMAND, Key::C);
const CUT_SHORTCUT: KeyboardShortcut = KeyboardShortcut::new(Modifiers::COMMAND, Key::X);
const PASTE_SHORTCUT: KeyboardShortcut = KeyboardShortcut::new(Modifiers::COMMAND, Key::V);
const DUPLICATE_SHORTCUT: KeyboardShortcut = KeyboardShortcut::new(Modifiers::COMMAND, Key::D);
const SELECT_ALL_SHORTCUT: KeyboardShortcut = KeyboardShortcut::new(Modifiers::COMMAND, Key::A);
const DELETE_SHORTCUT: KeyboardShortcut = KeyboardShortcut::new(Modifiers::NONE, Key::Delete);
const DUPLICATE_GAP: f32 = 200.0;

fn clipboard_event(ui: &Ui) -> Option<egui::Event> {
    ui.input(|input| {
        input
            .events
            .iter()
            .find(|event| {
                matches!(
                    event,
                    egui::Event::Copy | egui::Event::Cut | egui::Event::Paste(_)
                )
            })
            .cloned()
    })
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
enum DemoNode {
    /// Node with single input.
    /// Displays the value of the input.
    Sink,

    /// Value node with a single output.
    /// The value is editable in UI.
    Number(f64),

    /// Value node with a single output.
    String(String),

    /// Converts URI to Image
    ShowImage(String),

    /// Expression node with a single output.
    /// It has number of inputs equal to number of variables in the expression.
    ExprNode(ExprNode),
}

impl DemoNode {
    const fn name(&self) -> &str {
        match self {
            DemoNode::Sink => "Sink",
            DemoNode::Number(_) => "Number",
            DemoNode::String(_) => "String",
            DemoNode::ShowImage(_) => "ShowImage",
            DemoNode::ExprNode(_) => "ExprNode",
        }
    }

    fn number_out(&self) -> f64 {
        match self {
            DemoNode::Number(value) => *value,
            DemoNode::ExprNode(expr_node) => expr_node.eval(),
            _ => unreachable!(),
        }
    }

    fn number_in(&mut self, idx: usize) -> &mut f64 {
        match self {
            DemoNode::ExprNode(expr_node) => &mut expr_node.values[idx - 1],
            _ => unreachable!(),
        }
    }

    fn label_in(&mut self, idx: usize) -> &str {
        match self {
            DemoNode::ShowImage(_) if idx == 0 => "URL",
            DemoNode::ExprNode(expr_node) => &expr_node.bindings[idx - 1],
            _ => unreachable!(),
        }
    }

    fn string_out(&self) -> &str {
        match self {
            DemoNode::String(value) => value,
            _ => unreachable!(),
        }
    }

    fn string_in(&mut self) -> &mut String {
        match self {
            DemoNode::ShowImage(uri) => uri,
            DemoNode::ExprNode(expr_node) => &mut expr_node.text,
            _ => unreachable!(),
        }
    }

    fn expr_node(&mut self) -> &mut ExprNode {
        match self {
            DemoNode::ExprNode(expr_node) => expr_node,
            _ => unreachable!(),
        }
    }
}

#[derive(Default)]
struct DemoViewer {
    clipboard: Option<Snarl<DemoNode>>,
    selected_nodes: Option<Vec<NodeId>>,
    center_view: bool,
}

impl DemoViewer {
    fn copy_selection(
        selected_nodes: &[NodeId],
        snarl: &Snarl<DemoNode>,
    ) -> Option<Snarl<DemoNode>> {
        let mut copied = Snarl::new();
        let mut copied_ids = HashMap::new();

        for &node_id in selected_nodes {
            if copied_ids.contains_key(&node_id) {
                continue;
            }

            let Some(node) = snarl.get_node_info(node_id) else {
                continue;
            };

            let copied_id = if node.open {
                copied.insert_node(node.pos, node.value.clone())
            } else {
                copied.insert_node_collapsed(node.pos, node.value.clone())
            };
            copied_ids.insert(node_id, copied_id);
        }

        if copied_ids.is_empty() {
            return None;
        }

        for (from, to) in snarl.wires() {
            if let (Some(&from_node), Some(&to_node)) =
                (copied_ids.get(&from.node), copied_ids.get(&to.node))
            {
                copied.connect(
                    OutPinId {
                        node: from_node,
                        output: from.output,
                    },
                    InPinId {
                        node: to_node,
                        input: to.input,
                    },
                );
            }
        }

        Some(copied)
    }

    fn top_left(snarl: &Snarl<DemoNode>) -> Option<egui::Pos2> {
        snarl
            .nodes_info()
            .map(|node| node.pos)
            .reduce(|a, b| egui::pos2(a.x.min(b.x), a.y.min(b.y)))
    }

    fn insert_copy(
        copied: &Snarl<DemoNode>,
        pos: egui::Pos2,
        snarl: &mut Snarl<DemoNode>,
    ) -> Vec<NodeId> {
        let Some(top_left) = Self::top_left(copied) else {
            return Vec::new();
        };
        let offset = pos - top_left;
        let mut pasted_ids = HashMap::new();
        let mut selected_nodes = Vec::new();

        for (node_id, node) in copied.nodes_ids_data() {
            let pasted_id = if node.open {
                snarl.insert_node(node.pos + offset, node.value.clone())
            } else {
                snarl.insert_node_collapsed(node.pos + offset, node.value.clone())
            };
            pasted_ids.insert(node_id, pasted_id);
            selected_nodes.push(pasted_id);
        }

        for (from, to) in copied.wires() {
            snarl.connect(
                OutPinId {
                    node: pasted_ids[&from.node],
                    output: from.output,
                },
                InPinId {
                    node: pasted_ids[&to.node],
                    input: to.input,
                },
            );
        }

        selected_nodes
    }

    fn set_clipboard(&mut self, copied: Snarl<DemoNode>, ui: &Ui) {
        ui.ctx().copy_text(serde_json::to_string(&copied).unwrap());
        self.clipboard = Some(copied);
    }

    fn copy_nodes(&mut self, selected_nodes: &[NodeId], snarl: &Snarl<DemoNode>, ui: &Ui) {
        if let Some(copied) = Self::copy_selection(selected_nodes, snarl) {
            self.set_clipboard(copied, ui);
        }
    }

    fn cut_nodes(&mut self, selected_nodes: &[NodeId], snarl: &mut Snarl<DemoNode>, ui: &Ui) {
        let Some(copied) = Self::copy_selection(selected_nodes, snarl) else {
            return;
        };
        self.set_clipboard(copied, ui);

        self.remove_nodes(selected_nodes, snarl);
    }

    fn remove_nodes(&mut self, selected_nodes: &[NodeId], snarl: &mut Snarl<DemoNode>) {
        for &node_id in selected_nodes {
            if snarl.get_node(node_id).is_some() {
                snarl.remove_node(node_id);
            }
        }
        self.selected_nodes = Some(Vec::new());
    }

    fn paste_nodes(&mut self, pos: egui::Pos2, snarl: &mut Snarl<DemoNode>) {
        let Some(clipboard) = &self.clipboard else {
            return;
        };
        self.selected_nodes = Some(Self::insert_copy(clipboard, pos, snarl));
    }

    fn paste_text(&mut self, text: &str, pos: egui::Pos2, snarl: &mut Snarl<DemoNode>) {
        let Ok(clipboard) = serde_json::from_str(text) else {
            return;
        };
        self.selected_nodes = Some(Self::insert_copy(&clipboard, pos, snarl));
        self.clipboard = Some(clipboard);
    }

    fn duplicate_nodes(&mut self, selected_nodes: &[NodeId], snarl: &mut Snarl<DemoNode>) {
        let Some(copied) = Self::copy_selection(selected_nodes, snarl) else {
            return;
        };
        let Some(copied_top_left) = Self::top_left(&copied) else {
            return;
        };
        let right = snarl
            .nodes_info()
            .map(|node| node.pos.x)
            .reduce(f32::max)
            .unwrap_or_default();
        let pos = egui::pos2(right + DUPLICATE_GAP, copied_top_left.y);

        self.selected_nodes = Some(Self::insert_copy(&copied, pos, snarl));
        self.center_view = true;
    }

    fn show_edit_menu(
        &mut self,
        paste_pos: egui::Pos2,
        selected_nodes: &[NodeId],
        ui: &mut Ui,
        snarl: &mut Snarl<DemoNode>,
    ) {
        let has_selection = !selected_nodes.is_empty();
        let copy =
            egui::Button::new("Copy").shortcut_text(ui.ctx().format_shortcut(&COPY_SHORTCUT));
        if ui.add_enabled(has_selection, copy).clicked() {
            self.copy_nodes(selected_nodes, snarl, ui);
            ui.close();
        }

        let cut = egui::Button::new("Cut").shortcut_text(ui.ctx().format_shortcut(&CUT_SHORTCUT));
        if ui.add_enabled(has_selection, cut).clicked() {
            self.cut_nodes(selected_nodes, snarl, ui);
            ui.close();
        }

        let paste =
            egui::Button::new("Paste").shortcut_text(ui.ctx().format_shortcut(&PASTE_SHORTCUT));
        if ui.add_enabled(self.clipboard.is_some(), paste).clicked() {
            self.paste_nodes(paste_pos, snarl);
            ui.close();
        }

        let duplicate = egui::Button::new("Duplicate")
            .shortcut_text(ui.ctx().format_shortcut(&DUPLICATE_SHORTCUT));
        if ui.add_enabled(has_selection, duplicate).clicked() {
            self.duplicate_nodes(selected_nodes, snarl);
            ui.close();
        }

        if has_selection {
            let remove = egui::Button::new("Remove")
                .shortcut_text(ui.ctx().format_shortcut(&DELETE_SHORTCUT));
            if ui.add(remove).clicked() {
                self.remove_nodes(selected_nodes, snarl);
                ui.close();
            }
        }
    }
}

impl SnarlViewer<DemoNode> for DemoViewer {
    #[inline]
    fn connect(&mut self, from: &OutPin, to: &InPin, snarl: &mut Snarl<DemoNode>) {
        // Validate connection
        #[allow(clippy::match_same_arms)] // For match clarity
        match (&snarl[from.id.node], &snarl[to.id.node]) {
            (DemoNode::Sink, _) => {
                unreachable!("Sink node has no outputs")
            }
            (_, DemoNode::Sink) => {}
            (_, DemoNode::Number(_)) => {
                unreachable!("Number node has no inputs")
            }
            (_, DemoNode::String(_)) => {
                unreachable!("String node has no inputs")
            }
            (DemoNode::Number(_), DemoNode::ShowImage(_)) => {
                return;
            }
            (DemoNode::ShowImage(_), DemoNode::ShowImage(_)) => {
                return;
            }
            (DemoNode::String(_), DemoNode::ShowImage(_)) => {}
            (DemoNode::ExprNode(_), DemoNode::ExprNode(_)) if to.id.input == 0 => {
                return;
            }
            (DemoNode::ExprNode(_), DemoNode::ExprNode(_)) => {}
            (DemoNode::Number(_), DemoNode::ExprNode(_)) if to.id.input == 0 => {
                return;
            }
            (DemoNode::Number(_), DemoNode::ExprNode(_)) => {}
            (DemoNode::String(_), DemoNode::ExprNode(_)) if to.id.input == 0 => {}
            (DemoNode::String(_), DemoNode::ExprNode(_)) => {
                return;
            }
            (DemoNode::ShowImage(_), DemoNode::ExprNode(_)) => {
                return;
            }
            (DemoNode::ExprNode(_), DemoNode::ShowImage(_)) => {
                return;
            }
        }

        for &remote in &to.remotes {
            snarl.disconnect(remote, to.id);
        }

        snarl.connect(from.id, to.id);
    }

    fn title(&mut self, node: &DemoNode) -> String {
        match node {
            DemoNode::Sink => "Sink".to_owned(),
            DemoNode::Number(_) => "Number".to_owned(),
            DemoNode::String(_) => "String".to_owned(),
            DemoNode::ShowImage(_) => "Show image".to_owned(),
            DemoNode::ExprNode(_) => "Expr".to_owned(),
        }
    }

    fn inputs(&mut self, node: &DemoNode) -> usize {
        match node {
            DemoNode::Sink | DemoNode::ShowImage(_) => 1,
            DemoNode::Number(_) | DemoNode::String(_) => 0,
            DemoNode::ExprNode(expr_node) => 1 + expr_node.bindings.len(),
        }
    }

    fn outputs(&mut self, node: &DemoNode) -> usize {
        match node {
            DemoNode::Sink => 0,
            DemoNode::Number(_)
            | DemoNode::String(_)
            | DemoNode::ShowImage(_)
            | DemoNode::ExprNode(_) => 1,
        }
    }

    #[allow(clippy::too_many_lines)]
    #[allow(refining_impl_trait)]
    fn show_input(&mut self, pin: &InPin, ui: &mut Ui, snarl: &mut Snarl<DemoNode>) -> PinInfo {
        match snarl[pin.id.node] {
            DemoNode::Sink => {
                assert_eq!(pin.id.input, 0, "Sink node has only one input");

                match &*pin.remotes {
                    [] => {
                        ui.label("None");
                        PinInfo::circle().with_fill(UNTYPED_COLOR)
                    }
                    [remote] => match snarl[remote.node] {
                        DemoNode::Sink => unreachable!("Sink node has no outputs"),
                        DemoNode::Number(value) => {
                            assert_eq!(remote.output, 0, "Number node has only one output");
                            ui.label(format_float(value));
                            PinInfo::circle().with_fill(NUMBER_COLOR)
                        }
                        DemoNode::String(ref value) => {
                            assert_eq!(remote.output, 0, "String node has only one output");
                            ui.label(format!("{value:?}"));

                            PinInfo::circle().with_fill(STRING_COLOR).with_wire_style(
                                WireStyle::AxisAligned {
                                    corner_radius: 10.0,
                                },
                            )
                        }
                        DemoNode::ExprNode(ref expr) => {
                            assert_eq!(remote.output, 0, "Expr node has only one output");
                            ui.label(format_float(expr.eval()));
                            PinInfo::circle().with_fill(NUMBER_COLOR)
                        }
                        DemoNode::ShowImage(ref uri) => {
                            assert_eq!(remote.output, 0, "ShowImage node has only one output");

                            let image = egui::Image::new(uri).show_loading_spinner(true);
                            ui.add(image);

                            PinInfo::circle().with_fill(IMAGE_COLOR)
                        }
                    },
                    _ => unreachable!("Sink input has only one wire"),
                }
            }
            DemoNode::Number(_) => {
                unreachable!("Number node has no inputs")
            }
            DemoNode::String(_) => {
                unreachable!("String node has no inputs")
            }
            DemoNode::ShowImage(_) => match &*pin.remotes {
                [] => {
                    let input = snarl[pin.id.node].string_in();
                    egui::TextEdit::singleline(input)
                        .clip_text(false)
                        .desired_width(0.0)
                        .margin(ui.spacing().item_spacing)
                        .show(ui);
                    PinInfo::circle().with_fill(STRING_COLOR).with_wire_style(
                        WireStyle::AxisAligned {
                            corner_radius: 10.0,
                        },
                    )
                }
                [remote] => {
                    let new_value = snarl[remote.node].string_out().to_owned();

                    egui::TextEdit::singleline(&mut &*new_value)
                        .clip_text(false)
                        .desired_width(0.0)
                        .margin(ui.spacing().item_spacing)
                        .show(ui);

                    let input = snarl[pin.id.node].string_in();
                    *input = new_value;

                    PinInfo::circle().with_fill(STRING_COLOR).with_wire_style(
                        WireStyle::AxisAligned {
                            corner_radius: 10.0,
                        },
                    )
                }
                _ => unreachable!("Sink input has only one wire"),
            },
            DemoNode::ExprNode(_) if pin.id.input == 0 => {
                let changed = match &*pin.remotes {
                    [] => {
                        let input = snarl[pin.id.node].string_in();
                        let r = egui::TextEdit::singleline(input)
                            .clip_text(false)
                            .desired_width(0.0)
                            .margin(ui.spacing().item_spacing)
                            .show(ui)
                            .response;

                        r.changed()
                    }
                    [remote] => {
                        let new_string = snarl[remote.node].string_out().to_owned();

                        egui::TextEdit::singleline(&mut &*new_string)
                            .clip_text(false)
                            .desired_width(0.0)
                            .margin(ui.spacing().item_spacing)
                            .show(ui);

                        let input = snarl[pin.id.node].string_in();
                        if new_string == *input {
                            false
                        } else {
                            *input = new_string;
                            true
                        }
                    }
                    _ => unreachable!("Expr pins has only one wire"),
                };

                if changed {
                    let expr_node = snarl[pin.id.node].expr_node();

                    if let Ok(expr) = syn::parse_str(&expr_node.text) {
                        expr_node.expr = expr;

                        let values = Iterator::zip(
                            expr_node.bindings.iter().map(String::clone),
                            expr_node.values.iter().copied(),
                        )
                        .collect::<HashMap<String, f64>>();

                        let mut new_bindings = Vec::new();
                        expr_node.expr.extend_bindings(&mut new_bindings);

                        let old_bindings =
                            std::mem::replace(&mut expr_node.bindings, new_bindings.clone());

                        let new_values = new_bindings
                            .iter()
                            .map(|name| values.get(&**name).copied().unwrap_or(0.0))
                            .collect::<Vec<_>>();

                        expr_node.values = new_values;

                        let old_inputs = (0..old_bindings.len())
                            .map(|idx| {
                                snarl.in_pin(InPinId {
                                    node: pin.id.node,
                                    input: idx + 1,
                                })
                            })
                            .collect::<Vec<_>>();

                        for (idx, name) in old_bindings.iter().enumerate() {
                            let new_idx =
                                new_bindings.iter().position(|new_name| *new_name == *name);

                            match new_idx {
                                None => {
                                    snarl.drop_inputs(old_inputs[idx].id);
                                }
                                Some(new_idx) if new_idx != idx => {
                                    let new_in_pin = InPinId {
                                        node: pin.id.node,
                                        input: new_idx,
                                    };
                                    for &remote in &old_inputs[idx].remotes {
                                        snarl.disconnect(remote, old_inputs[idx].id);
                                        snarl.connect(remote, new_in_pin);
                                    }
                                }
                                _ => {}
                            }
                        }
                    }
                }
                PinInfo::circle()
                    .with_fill(STRING_COLOR)
                    .with_wire_style(WireStyle::AxisAligned {
                        corner_radius: 10.0,
                    })
            }
            DemoNode::ExprNode(ref expr_node) => {
                if pin.id.input <= expr_node.bindings.len() {
                    match &*pin.remotes {
                        [] => {
                            let node = &mut snarl[pin.id.node];
                            ui.label(node.label_in(pin.id.input));
                            ui.add(egui::DragValue::new(node.number_in(pin.id.input)));
                            PinInfo::circle().with_fill(NUMBER_COLOR)
                        }
                        [remote] => {
                            let new_value = snarl[remote.node].number_out();
                            let node = &mut snarl[pin.id.node];
                            ui.label(node.label_in(pin.id.input));
                            ui.label(format_float(new_value));
                            *node.number_in(pin.id.input) = new_value;
                            PinInfo::circle().with_fill(NUMBER_COLOR)
                        }
                        _ => unreachable!("Expr pins has only one wire"),
                    }
                } else {
                    ui.label("Removed");
                    PinInfo::circle().with_fill(Color32::BLACK)
                }
            }
        }
    }

    #[allow(refining_impl_trait)]
    fn show_output(&mut self, pin: &OutPin, ui: &mut Ui, snarl: &mut Snarl<DemoNode>) -> PinInfo {
        match snarl[pin.id.node] {
            DemoNode::Sink => {
                unreachable!("Sink node has no outputs")
            }
            DemoNode::Number(ref mut value) => {
                assert_eq!(pin.id.output, 0, "Number node has only one output");
                ui.add(egui::DragValue::new(value));
                PinInfo::circle().with_fill(NUMBER_COLOR)
            }
            DemoNode::String(ref mut value) => {
                assert_eq!(pin.id.output, 0, "String node has only one output");
                let edit = egui::TextEdit::singleline(value)
                    .clip_text(false)
                    .desired_width(0.0)
                    .margin(ui.spacing().item_spacing);
                ui.add(edit);
                PinInfo::circle()
                    .with_fill(STRING_COLOR)
                    .with_wire_style(WireStyle::AxisAligned {
                        corner_radius: 10.0,
                    })
            }
            DemoNode::ExprNode(ref expr_node) => {
                let value = expr_node.eval();
                assert_eq!(pin.id.output, 0, "Expr node has only one output");
                ui.label(format_float(value));
                PinInfo::circle().with_fill(NUMBER_COLOR)
            }
            DemoNode::ShowImage(_) => {
                ui.allocate_at_least(egui::Vec2::ZERO, egui::Sense::hover());
                PinInfo::circle().with_fill(IMAGE_COLOR)
            }
        }
    }

    fn has_graph_menu(&mut self, _pos: egui::Pos2, _snarl: &mut Snarl<DemoNode>) -> bool {
        true
    }

    fn show_graph_menu(
        &mut self,
        pos: egui::Pos2,
        selected_nodes: &[NodeId],
        ui: &mut Ui,
        snarl: &mut Snarl<DemoNode>,
    ) {
        ui.label("Add node");
        if ui.button("Number").clicked() {
            snarl.insert_node(pos, DemoNode::Number(0.0));
            ui.close();
        }
        if ui.button("Expr").clicked() {
            snarl.insert_node(pos, DemoNode::ExprNode(ExprNode::new()));
            ui.close();
        }
        if ui.button("String").clicked() {
            snarl.insert_node(pos, DemoNode::String(String::new()));
            ui.close();
        }
        if ui.button("Show image").clicked() {
            snarl.insert_node(pos, DemoNode::ShowImage(String::new()));
            ui.close();
        }
        if ui.button("Sink").clicked() {
            snarl.insert_node(pos, DemoNode::Sink);
            ui.close();
        }

        ui.separator();
        self.show_edit_menu(pos, selected_nodes, ui, snarl);
    }

    fn has_dropped_wire_menu(&mut self, _src_pins: AnyPins, _snarl: &mut Snarl<DemoNode>) -> bool {
        true
    }

    fn show_dropped_wire_menu(
        &mut self,
        pos: egui::Pos2,
        ui: &mut Ui,
        src_pins: AnyPins,
        snarl: &mut Snarl<DemoNode>,
    ) {
        // In this demo, we create a context-aware node graph menu, and connect a wire
        // dropped on the fly based on user input to a new node created.
        //
        // In your implementation, you may want to define specifications for each node's
        // pin inputs and outputs and compatibility to make this easier.

        type PinCompat = usize;
        const PIN_NUM: PinCompat = 1;
        const PIN_STR: PinCompat = 2;
        const PIN_IMG: PinCompat = 4;
        const PIN_SINK: PinCompat = PIN_NUM | PIN_STR | PIN_IMG;

        const fn pin_out_compat(node: &DemoNode) -> PinCompat {
            match node {
                DemoNode::Sink => 0,
                DemoNode::String(_) => PIN_STR,
                DemoNode::ShowImage(_) => PIN_IMG,
                DemoNode::Number(_) | DemoNode::ExprNode(_) => PIN_NUM,
            }
        }

        const fn pin_in_compat(node: &DemoNode, pin: usize) -> PinCompat {
            match node {
                DemoNode::Sink => PIN_SINK,
                DemoNode::Number(_) | DemoNode::String(_) => 0,
                DemoNode::ShowImage(_) => PIN_STR,
                DemoNode::ExprNode(_) => {
                    if pin == 0 {
                        PIN_STR
                    } else {
                        PIN_NUM
                    }
                }
            }
        }

        ui.label("Add node");

        match src_pins {
            AnyPins::Out(src_pins) => {
                if src_pins.len() != 1 {
                    ui.label("Multiple output pins are not supported in this demo");
                    return;
                }

                let src_pin = src_pins[0];
                let src_out_ty = pin_out_compat(snarl.get_node(src_pin.node).unwrap());
                let dst_in_candidates = [
                    ("Sink", (|| DemoNode::Sink) as fn() -> DemoNode, PIN_SINK),
                    ("Show Image", || DemoNode::ShowImage(String::new()), PIN_STR),
                    ("Expr", || DemoNode::ExprNode(ExprNode::new()), PIN_STR),
                ];

                for (name, ctor, in_ty) in dst_in_candidates {
                    if src_out_ty & in_ty != 0 && ui.button(name).clicked() {
                        // Create new node.
                        let new_node = snarl.insert_node(pos, ctor());
                        let dst_pin = InPinId {
                            node: new_node,
                            input: 0,
                        };

                        // Connect the wire.
                        snarl.connect(src_pin, dst_pin);
                        ui.close();
                    }
                }
            }
            AnyPins::In(pins) => {
                let all_src_types = pins.iter().fold(0, |acc, pin| {
                    acc | pin_in_compat(snarl.get_node(pin.node).unwrap(), pin.input)
                });

                let dst_out_candidates = [
                    (
                        "Number",
                        (|| DemoNode::Number(0.)) as fn() -> DemoNode,
                        PIN_NUM,
                    ),
                    ("String", || DemoNode::String(String::new()), PIN_STR),
                    ("Expr", || DemoNode::ExprNode(ExprNode::new()), PIN_NUM),
                    ("Show Image", || DemoNode::ShowImage(String::new()), PIN_IMG),
                ];

                for (name, ctor, out_ty) in dst_out_candidates {
                    if all_src_types & out_ty != 0 && ui.button(name).clicked() {
                        // Create new node.
                        let new_node = ctor();
                        let dst_ty = pin_out_compat(&new_node);

                        let new_node = snarl.insert_node(pos, new_node);
                        let dst_pin = OutPinId {
                            node: new_node,
                            output: 0,
                        };

                        // Connect the wire.
                        for src_pin in pins {
                            let src_ty =
                                pin_in_compat(snarl.get_node(src_pin.node).unwrap(), src_pin.input);
                            if src_ty & dst_ty != 0 {
                                // In this demo, input pin MUST be unique ...
                                // Therefore here we drop inputs of source input pin.
                                snarl.drop_inputs(*src_pin);
                                snarl.connect(dst_pin, *src_pin);
                                ui.close();
                            }
                        }
                    }
                }
            }
        }
    }

    fn has_node_menu(&mut self, _node: &DemoNode) -> bool {
        true
    }

    fn show_node_menu(
        &mut self,
        node: NodeId,
        _inputs: &[InPin],
        _outputs: &[OutPin],
        selected_nodes: &[NodeId],
        ui: &mut Ui,
        snarl: &mut Snarl<DemoNode>,
    ) {
        ui.label("Node menu");
        let paste_pos = snarl.get_node_info(node).unwrap().pos;
        let selected_nodes = if selected_nodes.contains(&node) {
            selected_nodes
        } else {
            if !selected_nodes.is_empty() {
                self.selected_nodes = Some(vec![node]);
            }
            std::slice::from_ref(&node)
        };

        self.show_edit_menu(paste_pos, selected_nodes, ui, snarl);
    }

    fn center_view(&mut self, _snarl: &mut Snarl<DemoNode>) -> bool {
        std::mem::take(&mut self.center_view)
    }

    fn selected_nodes(
        &mut self,
        currently_selected: &[NodeId],
        ui: &mut Ui,
        snarl: &mut Snarl<DemoNode>,
    ) -> Option<Vec<NodeId>> {
        match clipboard_event(ui) {
            Some(egui::Event::Copy) => self.copy_nodes(currently_selected, snarl, ui),
            Some(egui::Event::Cut) => self.cut_nodes(currently_selected, snarl, ui),
            Some(egui::Event::Paste(text)) => {
                self.paste_text(&text, ui.clip_rect().center(), snarl);
            }
            None if ui.input_mut(|input| input.consume_shortcut(&SELECT_ALL_SHORTCUT)) => {
                self.selected_nodes = Some(snarl.node_ids().map(|(node_id, _)| node_id).collect());
            }
            None if ui.input_mut(|input| input.consume_shortcut(&DELETE_SHORTCUT)) => {
                self.remove_nodes(currently_selected, snarl);
            }
            None if ui.input_mut(|input| input.consume_shortcut(&DUPLICATE_SHORTCUT)) => {
                self.duplicate_nodes(currently_selected, snarl);
            }
            _ => {}
        }

        self.selected_nodes.take()
    }

    fn has_on_hover_popup(&mut self, _: &DemoNode) -> bool {
        true
    }

    fn show_on_hover_popup(
        &mut self,
        node: NodeId,
        _inputs: &[InPin],
        _outputs: &[OutPin],
        ui: &mut Ui,
        snarl: &mut Snarl<DemoNode>,
    ) {
        match snarl[node] {
            DemoNode::Sink => {
                ui.label("Displays anything connected to it");
            }
            DemoNode::Number(_) => {
                ui.label("Outputs integer value");
            }
            DemoNode::String(_) => {
                ui.label("Outputs string value");
            }
            DemoNode::ShowImage(_) => {
                ui.label("Displays image from URL in input");
            }
            DemoNode::ExprNode(_) => {
                ui.label("Evaluates algebraic expression with input for each unique variable name");
            }
        }
    }

    fn header_frame(
        &mut self,
        frame: egui::Frame,
        node: NodeId,
        _inputs: &[InPin],
        _outputs: &[OutPin],
        snarl: &Snarl<DemoNode>,
    ) -> egui::Frame {
        match snarl[node] {
            DemoNode::Sink => frame.fill(egui::Color32::from_rgb(70, 70, 80)),
            DemoNode::Number(_) => frame.fill(egui::Color32::from_rgb(70, 40, 40)),
            DemoNode::String(_) => frame.fill(egui::Color32::from_rgb(40, 70, 40)),
            DemoNode::ShowImage(_) => frame.fill(egui::Color32::from_rgb(40, 40, 70)),
            DemoNode::ExprNode(_) => frame.fill(egui::Color32::from_rgb(70, 66, 40)),
        }
    }
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
struct ExprNode {
    text: String,
    bindings: Vec<String>,
    values: Vec<f64>,
    expr: Expr,
}

impl ExprNode {
    fn new() -> Self {
        ExprNode {
            text: "0".to_string(),
            bindings: Vec::new(),
            values: Vec::new(),
            expr: Expr::Val(0.0),
        }
    }

    fn eval(&self) -> f64 {
        self.expr.eval(&self.bindings, &self.values)
    }
}

#[derive(Clone, Copy, serde::Serialize, serde::Deserialize)]
enum UnOp {
    Pos,
    Neg,
}

#[derive(Clone, Copy, serde::Serialize, serde::Deserialize)]
enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
enum Expr {
    Var(String),
    Val(f64),
    UnOp {
        op: UnOp,
        expr: Box<Expr>,
    },
    BinOp {
        lhs: Box<Expr>,
        op: BinOp,
        rhs: Box<Expr>,
    },
}

impl Expr {
    fn eval(&self, bindings: &[String], args: &[f64]) -> f64 {
        let binding_index =
            |name: &str| bindings.iter().position(|binding| binding == name).unwrap();

        match self {
            Expr::Var(name) => args[binding_index(name)],
            Expr::Val(value) => *value,
            Expr::UnOp { op, expr } => match op {
                UnOp::Pos => expr.eval(bindings, args),
                UnOp::Neg => -expr.eval(bindings, args),
            },
            Expr::BinOp { lhs, op, rhs } => match op {
                BinOp::Add => lhs.eval(bindings, args) + rhs.eval(bindings, args),
                BinOp::Sub => lhs.eval(bindings, args) - rhs.eval(bindings, args),
                BinOp::Mul => lhs.eval(bindings, args) * rhs.eval(bindings, args),
                BinOp::Div => lhs.eval(bindings, args) / rhs.eval(bindings, args),
            },
        }
    }

    fn extend_bindings(&self, bindings: &mut Vec<String>) {
        match self {
            Expr::Var(name) => {
                if !bindings.contains(name) {
                    bindings.push(name.clone());
                }
            }
            Expr::Val(_) => {}
            Expr::UnOp { expr, .. } => {
                expr.extend_bindings(bindings);
            }
            Expr::BinOp { lhs, rhs, .. } => {
                lhs.extend_bindings(bindings);
                rhs.extend_bindings(bindings);
            }
        }
    }
}

impl syn::parse::Parse for UnOp {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let lookahead = input.lookahead1();
        if lookahead.peek(syn::Token![+]) {
            input.parse::<syn::Token![+]>()?;
            Ok(UnOp::Pos)
        } else if lookahead.peek(syn::Token![-]) {
            input.parse::<syn::Token![-]>()?;
            Ok(UnOp::Neg)
        } else {
            Err(lookahead.error())
        }
    }
}

impl syn::parse::Parse for BinOp {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let lookahead = input.lookahead1();
        if lookahead.peek(syn::Token![+]) {
            input.parse::<syn::Token![+]>()?;
            Ok(BinOp::Add)
        } else if lookahead.peek(syn::Token![-]) {
            input.parse::<syn::Token![-]>()?;
            Ok(BinOp::Sub)
        } else if lookahead.peek(syn::Token![*]) {
            input.parse::<syn::Token![*]>()?;
            Ok(BinOp::Mul)
        } else if lookahead.peek(syn::Token![/]) {
            input.parse::<syn::Token![/]>()?;
            Ok(BinOp::Div)
        } else {
            Err(lookahead.error())
        }
    }
}

impl syn::parse::Parse for Expr {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let lookahead = input.lookahead1();

        let lhs;
        if lookahead.peek(syn::token::Paren) {
            let content;
            syn::parenthesized!(content in input);
            let expr = content.parse::<Expr>()?;
            if input.is_empty() {
                return Ok(expr);
            }
            lhs = expr;
        // } else if lookahead.peek(syn::LitFloat) {
        //     let lit = input.parse::<syn::LitFloat>()?;
        //     let value = lit.base10_parse::<f64>()?;
        //     let expr = Expr::Val(value);
        //     if input.is_empty() {
        //         return Ok(expr);
        //     }
        //     lhs = expr;
        } else if lookahead.peek(syn::LitInt) {
            let lit = input.parse::<syn::LitInt>()?;
            let value = lit.base10_parse::<f64>()?;
            let expr = Expr::Val(value);
            if input.is_empty() {
                return Ok(expr);
            }
            lhs = expr;
        } else if lookahead.peek(syn::Ident) {
            let ident = input.parse::<syn::Ident>()?;
            let expr = Expr::Var(ident.to_string());
            if input.is_empty() {
                return Ok(expr);
            }
            lhs = expr;
        } else {
            let unop = input.parse::<UnOp>()?;

            return Self::parse_with_unop(unop, input);
        }

        let binop = input.parse::<BinOp>()?;

        Self::parse_binop(Box::new(lhs), binop, input)
    }
}

impl Expr {
    fn parse_with_unop(op: UnOp, input: syn::parse::ParseStream) -> syn::Result<Self> {
        let lookahead = input.lookahead1();

        let lhs;
        if lookahead.peek(syn::token::Paren) {
            let content;
            syn::parenthesized!(content in input);
            let expr = Expr::UnOp {
                op,
                expr: Box::new(content.parse::<Expr>()?),
            };
            if input.is_empty() {
                return Ok(expr);
            }
            lhs = expr;
        } else if lookahead.peek(syn::LitFloat) {
            let lit = input.parse::<syn::LitFloat>()?;
            let value = lit.base10_parse::<f64>()?;
            let expr = Expr::UnOp {
                op,
                expr: Box::new(Expr::Val(value)),
            };
            if input.is_empty() {
                return Ok(expr);
            }
            lhs = expr;
        } else if lookahead.peek(syn::LitInt) {
            let lit = input.parse::<syn::LitInt>()?;
            let value = lit.base10_parse::<f64>()?;
            let expr = Expr::UnOp {
                op,
                expr: Box::new(Expr::Val(value)),
            };
            if input.is_empty() {
                return Ok(expr);
            }
            lhs = expr;
        } else if lookahead.peek(syn::Ident) {
            let ident = input.parse::<syn::Ident>()?;
            let expr = Expr::UnOp {
                op,
                expr: Box::new(Expr::Var(ident.to_string())),
            };
            if input.is_empty() {
                return Ok(expr);
            }
            lhs = expr;
        } else {
            return Err(lookahead.error());
        }

        let op = input.parse::<BinOp>()?;

        Self::parse_binop(Box::new(lhs), op, input)
    }

    fn parse_binop(lhs: Box<Expr>, op: BinOp, input: syn::parse::ParseStream) -> syn::Result<Self> {
        let lookahead = input.lookahead1();

        let rhs;
        if lookahead.peek(syn::token::Paren) {
            let content;
            syn::parenthesized!(content in input);
            rhs = Box::new(content.parse::<Expr>()?);
            if input.is_empty() {
                return Ok(Expr::BinOp { lhs, op, rhs });
            }
        } else if lookahead.peek(syn::LitFloat) {
            let lit = input.parse::<syn::LitFloat>()?;
            let value = lit.base10_parse::<f64>()?;
            rhs = Box::new(Expr::Val(value));
            if input.is_empty() {
                return Ok(Expr::BinOp { lhs, op, rhs });
            }
        } else if lookahead.peek(syn::LitInt) {
            let lit = input.parse::<syn::LitInt>()?;
            let value = lit.base10_parse::<f64>()?;
            rhs = Box::new(Expr::Val(value));
            if input.is_empty() {
                return Ok(Expr::BinOp { lhs, op, rhs });
            }
        } else if lookahead.peek(syn::Ident) {
            let ident = input.parse::<syn::Ident>()?;
            rhs = Box::new(Expr::Var(ident.to_string()));
            if input.is_empty() {
                return Ok(Expr::BinOp { lhs, op, rhs });
            }
        } else {
            return Err(lookahead.error());
        }

        let next_op = input.parse::<BinOp>()?;

        if let (BinOp::Add | BinOp::Sub, BinOp::Mul | BinOp::Div) = (op, next_op) {
            let rhs = Self::parse_binop(rhs, next_op, input)?;
            Ok(Self::BinOp {
                lhs,
                op,
                rhs: Box::new(rhs),
            })
        } else {
            let lhs = Self::BinOp { lhs, op, rhs };
            Self::parse_binop(Box::new(lhs), next_op, input)
        }
    }
}

pub struct DemoApp {
    snarl: Snarl<DemoNode>,
    style: SnarlStyle,
    viewer: DemoViewer,
}

const fn default_style() -> SnarlStyle {
    SnarlStyle {
        node_layout: Some(NodeLayout::coil()),
        pin_placement: Some(PinPlacement::Edge),
        pin_size: Some(7.0),
        node_frame: Some(egui::Frame {
            inner_margin: egui::Margin::same(8),
            outer_margin: egui::Margin {
                left: 0,
                right: 0,
                top: 0,
                bottom: 4,
            },
            corner_radius: egui::CornerRadius::same(8),
            fill: egui::Color32::from_gray(30),
            stroke: egui::Stroke::NONE,
            shadow: egui::Shadow::NONE,
        }),
        bg_frame: Some(egui::Frame {
            inner_margin: egui::Margin::ZERO,
            outer_margin: egui::Margin::same(2),
            corner_radius: egui::CornerRadius::ZERO,
            fill: egui::Color32::from_gray(40),
            stroke: egui::Stroke::NONE,
            shadow: egui::Shadow::NONE,
        }),
        ..SnarlStyle::new()
    }
}

impl DemoApp {
    pub fn new(cx: &CreationContext) -> Self {
        egui_extras::install_image_loaders(&cx.egui_ctx);

        cx.egui_ctx
            .global_style_mut(|style| style.animation_time *= 10.0);

        let snarl = cx.storage.map_or_else(Snarl::new, |storage| {
            storage
                .get_string("snarl")
                .and_then(|snarl| serde_json::from_str(&snarl).ok())
                .unwrap_or_default()
        });
        // let snarl = Snarl::new();

        let style = cx.storage.map_or_else(default_style, |storage| {
            storage
                .get_string("style")
                .and_then(|style| serde_json::from_str(&style).ok())
                .unwrap_or_else(default_style)
        });
        // let style = SnarlStyle::new();

        DemoApp {
            snarl,
            style,
            viewer: DemoViewer::default(),
        }
    }
}

impl App for DemoApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        egui::Panel::top("top_panel").show(ui, |ui| {
            // The top panel is often a good place for a menu bar:

            egui::MenuBar::new().ui(ui, |ui| {
                #[cfg(not(target_arch = "wasm32"))]
                {
                    ui.menu_button("File", |ui| {
                        if ui.button("Quit").clicked() {
                            ui.send_viewport_cmd(egui::ViewportCommand::Close);
                        }
                    });
                    ui.add_space(16.0);
                }

                egui::widgets::global_theme_preference_switch(ui);

                if ui.button("Clear All").clicked() {
                    self.snarl = Snarl::default();
                }
            });
        });

        egui::Panel::left("style").show(ui, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                egui_probe::Probe::new(&mut self.style).show(ui);
            });
        });

        egui::Panel::right("selected-list").show(ui, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.strong("Selected nodes");

                let selected = get_selected_nodes(Id::new("snarl-demo"), ui.ctx());

                let mut selected = selected
                    .into_iter()
                    .filter_map(|id| self.snarl.get_node(id).map(|node| (id, node)))
                    .collect::<Vec<_>>();

                selected.sort_by_key(|(id, _)| *id);

                let mut remove = None;

                for (id, node) in selected {
                    ui.horizontal(|ui| {
                        ui.label(format!("{id:?}"));
                        ui.label(node.name());
                        ui.add_space(ui.spacing().item_spacing.x);
                        if ui.button("Remove").clicked() {
                            remove = Some(id);
                        }
                    });
                }

                if let Some(id) = remove {
                    self.snarl.remove_node(id);
                }
            });
        });

        egui::CentralPanel::default().show(ui, |ui| {
            SnarlWidget::new()
                .id(Id::new("snarl-demo"))
                .style(self.style)
                .show(&mut self.snarl, &mut self.viewer, ui);
        });
    }

    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        let snarl = serde_json::to_string(&self.snarl).unwrap();
        storage.set_string("snarl", snarl);

        let style = serde_json::to_string(&self.style).unwrap();
        storage.set_string("style", style);
    }
}

// When compiling natively:
#[cfg(not(target_arch = "wasm32"))]
fn main() -> eframe::Result<()> {
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([400.0, 300.0])
            .with_min_inner_size([300.0, 220.0]),
        ..Default::default()
    };

    eframe::run_native(
        "egui-snarl demo",
        native_options,
        Box::new(|cx| Ok(Box::new(DemoApp::new(cx)))),
    )
}

#[cfg(target_arch = "wasm32")]
fn get_canvas_element() -> Option<web_sys::HtmlCanvasElement> {
    use eframe::wasm_bindgen::JsCast;

    let document = web_sys::window()?.document()?;
    let canvas = document.get_element_by_id("egui_snarl_demo")?;
    canvas.dyn_into::<web_sys::HtmlCanvasElement>().ok()
}

// When compiling to web using trunk:
#[cfg(target_arch = "wasm32")]
fn main() {
    let canvas = get_canvas_element().expect("Failed to find canvas with id 'egui_snarl_demo'");

    let web_options = eframe::WebOptions::default();

    wasm_bindgen_futures::spawn_local(async {
        eframe::WebRunner::new()
            .start(
                canvas,
                web_options,
                Box::new(|cx| Ok(Box::new(DemoApp::new(cx)))),
            )
            .await
            .expect("failed to start eframe");
    });
}

fn format_float(v: f64) -> String {
    let v = (v * 1000.0).round() / 1000.0;
    format!("{v}")
}
