use std::collections::HashMap;

use eframe::{App, CreationContext};
use egui::{Color32, Ui};
use egui_snarl::{
    ui::{PinInfo, SnarlStyle, SnarlViewer},
    InPin, InPinId, NodeId, OutPin, Snarl,
};

#[derive(Clone, serde::Serialize, serde::Deserialize)]
enum DemoNode {
    /// Node with single input.
    /// Displays the value of the input.
    Sink,

    /// Value node with a single output.
    /// The value is editable in UI.
    Integer(f64),

    /// Value node with a single output.
    String(String),

    /// Converts URI to Image
    ShowImage(String),

    /// Expression node with a single output.
    /// It has number of inputs equal to number of variables in the expression.
    ExprNode(ExprNode),
}

struct DemoViewer;

impl SnarlViewer<DemoNode> for DemoViewer {
    #[inline]
    fn connect(&mut self, from: &OutPin, to: &InPin, snarl: &mut Snarl<DemoNode>) {
        // Validate connection
        match (&snarl[from.id.node], &snarl[to.id.node]) {
            (DemoNode::Sink, _) => {
                unreachable!("Sink node has no outputs")
            }
            (_, DemoNode::Sink) => {}
            (_, DemoNode::Integer(_)) => {
                unreachable!("Integer node has no inputs")
            }
            (_, DemoNode::String(_)) => {
                unreachable!("String node has no inputs")
            }
            (DemoNode::Integer(_), DemoNode::ShowImage(_)) => {
                return;
            }
            (DemoNode::ShowImage(_), DemoNode::ShowImage(_)) => {
                return;
            }
            (DemoNode::String(_), DemoNode::ShowImage(_)) => {}
            (DemoNode::ExprNode(_), DemoNode::ExprNode(_)) => {}
            (DemoNode::Integer(_), DemoNode::ExprNode(_)) => {}
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
            DemoNode::Integer(_) => "Integer".to_owned(),
            DemoNode::String(_) => "String".to_owned(),
            DemoNode::ShowImage(_) => "Show image".to_owned(),
            DemoNode::ExprNode(_) => "Expr".to_owned(),
        }
    }

    fn show_header(
        &mut self,
        node: NodeId,
        inputs: &[InPin],
        _outputs: &[OutPin],
        ui: &mut Ui,
        _scale: f32,
        snarl: &mut Snarl<DemoNode>,
    ) {
        match &mut snarl[node] {
            DemoNode::Sink => {
                ui.label("Sink");
            }
            DemoNode::Integer(value) => {
                ui.add(egui::DragValue::new(value));
            }
            DemoNode::String(value) => {
                let edit = egui::TextEdit::singleline(value)
                    .clip_text(false)
                    .desired_width(0.0)
                    .margin(ui.spacing().item_spacing);
                ui.add(edit);
            }
            DemoNode::ShowImage(_) => {
                ui.label("Show image");
            }
            DemoNode::ExprNode(expr_node) => {
                let edit = egui::TextEdit::singleline(&mut expr_node.text)
                    .clip_text(false)
                    .desired_width(0.0)
                    .margin(ui.spacing().item_spacing);

                let r = ui.add(edit);
                if r.changed() {
                    match syn::parse_str(&expr_node.text) {
                        Ok(expr) => {
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

                            for (idx, name) in old_bindings.iter().enumerate() {
                                let new_idx =
                                    new_bindings.iter().position(|new_name| *new_name == *name);

                                match new_idx {
                                    None => {
                                        snarl.drop_inputs(inputs[idx].id);
                                    }
                                    Some(new_idx) if new_idx != idx => {
                                        let new_in_pin = InPinId {
                                            node: node,
                                            input: new_idx,
                                        };
                                        for &remote in &inputs[idx].remotes {
                                            snarl.disconnect(remote, inputs[idx].id);
                                            snarl.connect(remote, new_in_pin);
                                        }
                                    }
                                    _ => {}
                                }
                            }
                        }
                        Err(_) => {}
                    }
                }
            }
        }
    }

    fn inputs(&mut self, node: &DemoNode) -> usize {
        match node {
            DemoNode::Sink => 1,
            DemoNode::Integer(_) => 0,
            DemoNode::String(_) => 0,
            // DemoNode::Add(values) => values.len() + 1,
            DemoNode::ShowImage(_) => 1,
            DemoNode::ExprNode(expr_node) => expr_node.bindings.len(),
        }
    }

    fn outputs(&mut self, node: &DemoNode) -> usize {
        match node {
            DemoNode::Sink => 0,
            DemoNode::Integer(_) => 1,
            DemoNode::String(_) => 1,
            // DemoNode::Add(_) => 1,
            DemoNode::ShowImage(_) => 1,
            DemoNode::ExprNode(_) => 1,
        }
    }

    fn show_input(
        &mut self,
        pin: &InPin,
        ui: &mut Ui,
        scale: f32,
        snarl: &mut Snarl<DemoNode>,
    ) -> PinInfo {
        match snarl[pin.id.node] {
            DemoNode::Sink => {
                assert_eq!(pin.id.input, 0, "Sink node has only one input");

                match &*pin.remotes {
                    [] => {
                        ui.label("None");
                        PinInfo::circle().with_fill(Color32::GRAY)
                    }
                    [remote] => match snarl[remote.node] {
                        DemoNode::Sink => unreachable!("Sink node has no outputs"),
                        DemoNode::Integer(value) => {
                            assert_eq!(remote.output, 0, "Integer node has only one output");
                            ui.label(format_float(value));
                            PinInfo::square().with_fill(Color32::RED)
                        }
                        DemoNode::String(ref value) => {
                            assert_eq!(remote.output, 0, "String node has only one output");
                            ui.label(format!("{:?}", value));
                            PinInfo::triangle().with_fill(Color32::GREEN)
                        }
                        DemoNode::ExprNode(ref expr) => {
                            assert_eq!(remote.output, 0, "Expr node has only one output");
                            ui.label(format_float(expr.eval()));
                            PinInfo::square().with_fill(Color32::RED)
                        }
                        DemoNode::ShowImage(ref uri) => {
                            assert_eq!(remote.output, 0, "ShowImage node has only one output");

                            let image = egui::Image::new(uri)
                                .fit_to_original_size(scale)
                                .show_loading_spinner(true);
                            ui.add(image);

                            PinInfo::circle().with_fill(Color32::GOLD)
                        }
                    },
                    _ => unreachable!("Sink input has only one wire"),
                }
            }
            DemoNode::Integer(_) => {
                unreachable!("Integer node has no inputs")
            }
            DemoNode::String(_) => {
                unreachable!("String node has no inputs")
            }
            DemoNode::ShowImage(_) => match &*pin.remotes {
                [] => match &mut snarl[pin.id.node] {
                    DemoNode::ShowImage(uri) => {
                        let edit = egui::TextEdit::singleline(uri)
                            .clip_text(false)
                            .desired_width(0.0)
                            .margin(ui.spacing().item_spacing);
                        ui.add(edit);
                        PinInfo::triangle().with_fill(Color32::GREEN)
                    }
                    _ => unreachable!(),
                },
                [remote] => match snarl[remote.node] {
                    DemoNode::Sink => unreachable!("Sink node has no outputs"),
                    DemoNode::ShowImage(_) => {
                        unreachable!("ShowImage node has no outputs")
                    }
                    DemoNode::Integer(_) | DemoNode::ExprNode(_) => {
                        unreachable!("Invalid connection")
                    }
                    DemoNode::String(ref value) => {
                        let value = value.clone();
                        match &mut snarl[pin.id.node] {
                            DemoNode::ShowImage(uri) => {
                                *uri = value;
                                ui.label(&*uri);
                                PinInfo::triangle().with_fill(Color32::GREEN)
                            }
                            _ => unreachable!(),
                        }
                    }
                },
                _ => unreachable!("Sink input has only one wire"),
            },
            DemoNode::ExprNode(ref expr_node) => {
                if pin.id.input < expr_node.bindings.len() {
                    match &*pin.remotes {
                        [] => match &mut snarl[pin.id.node] {
                            DemoNode::ExprNode(expr_node) => {
                                ui.label(&expr_node.bindings[pin.id.input]);
                                ui.add(egui::DragValue::new(&mut expr_node.values[pin.id.input]));
                                PinInfo::square().with_fill(Color32::RED)
                            }
                            _ => unreachable!(),
                        },
                        [remote] => {
                            ui.label(&expr_node.bindings[pin.id.input]);
                            let pin_info = match snarl[remote.node] {
                                DemoNode::Sink => unreachable!("Sink node has no outputs"),
                                DemoNode::Integer(value) => {
                                    assert_eq!(
                                        remote.output, 0,
                                        "Integer node has only one output"
                                    );
                                    match &mut snarl[pin.id.node] {
                                        DemoNode::ExprNode(expr_node) => {
                                            expr_node.values[pin.id.input] = value as f64;
                                        }
                                        _ => unreachable!(),
                                    }
                                    PinInfo::square().with_fill(Color32::RED)
                                }
                                DemoNode::ExprNode(ref expr_node) => {
                                    let value = expr_node.eval();

                                    assert_eq!(remote.output, 0, "Expr node has only one output");
                                    match &mut snarl[pin.id.node] {
                                        DemoNode::ExprNode(expr_node) => {
                                            expr_node.values[pin.id.input] = value;
                                        }
                                        _ => unreachable!(),
                                    }
                                    PinInfo::square().with_fill(Color32::RED)
                                }
                                DemoNode::ShowImage(_) => {
                                    unreachable!("ShowImage node has no outputs")
                                }
                                DemoNode::String(_) => {
                                    unreachable!("Invalid connection")
                                }
                            };

                            pin_info
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

    fn show_output(
        &mut self,
        pin: &OutPin,
        ui: &mut Ui,
        _scale: f32,
        snarl: &mut Snarl<DemoNode>,
    ) -> PinInfo {
        match snarl[pin.id.node] {
            DemoNode::Sink => {
                unreachable!("Sink node has no outputs")
            }
            DemoNode::Integer(value) => {
                assert_eq!(pin.id.output, 0, "Integer node has only one output");
                // ui.add(egui::DragValue::new(value));
                ui.label(format_float(value));
                PinInfo::square().with_fill(Color32::RED)
            }
            DemoNode::String(ref value) => {
                assert_eq!(pin.id.output, 0, "String node has only one output");
                // ui.text_edit_singleline(value);
                ui.label(value);
                PinInfo::triangle().with_fill(Color32::GREEN)
            }
            DemoNode::ExprNode(ref expr_node) => {
                let value = expr_node.eval();
                assert_eq!(pin.id.output, 0, "Expr node has only one output");
                ui.label(format_float(value));
                PinInfo::square().with_fill(Color32::RED)
            }
            DemoNode::ShowImage(_) => {
                ui.allocate_at_least(egui::Vec2::ZERO, egui::Sense::hover());
                PinInfo::circle().with_fill(Color32::GOLD)
            }
        }
    }

    fn input_color(
        &mut self,
        pin: &InPin,
        _style: &egui::Style,
        snarl: &mut Snarl<DemoNode>,
    ) -> Color32 {
        match snarl[pin.id.node] {
            DemoNode::Sink => {
                assert_eq!(pin.id.input, 0, "Sink node has only one input");
                match &*pin.remotes {
                    [] => Color32::GRAY,
                    [remote] => match snarl[remote.node] {
                        DemoNode::Sink => unreachable!("Sink node has no outputs"),
                        DemoNode::Integer(_) => Color32::RED,
                        DemoNode::String(_) => Color32::GREEN,
                        DemoNode::ExprNode(_) => Color32::RED,
                        DemoNode::ShowImage(_) => Color32::GOLD,
                    },
                    _ => unreachable!("Sink input has only one wire"),
                }
            }
            DemoNode::Integer(_) => {
                unreachable!("Integer node has no inputs")
            }
            DemoNode::String(_) => {
                unreachable!("String node has no inputs")
            }
            DemoNode::ShowImage(_) => Color32::GREEN,
            DemoNode::ExprNode(_) => Color32::RED,
        }
    }

    fn output_color(
        &mut self,
        pin: &OutPin,
        _style: &egui::Style,
        snarl: &mut Snarl<DemoNode>,
    ) -> Color32 {
        match snarl[pin.id.node] {
            DemoNode::Sink => {
                unreachable!("Sink node has no outputs")
            }
            DemoNode::Integer(_) => Color32::RED,
            DemoNode::String(_) => Color32::GREEN,
            DemoNode::ShowImage(_) => Color32::GOLD,
            DemoNode::ExprNode(_) => Color32::RED,
        }
    }

    fn graph_menu(
        &mut self,
        pos: egui::Pos2,
        ui: &mut Ui,
        _scale: f32,
        snarl: &mut Snarl<DemoNode>,
    ) {
        ui.label("Add node");
        if ui.button("Integer").clicked() {
            snarl.insert_node(pos, DemoNode::Integer(0.0));
            ui.close_menu();
        }
        if ui.button("Expr").clicked() {
            snarl.insert_node(pos, DemoNode::ExprNode(ExprNode::new()));
            ui.close_menu();
        }
        if ui.button("String").clicked() {
            snarl.insert_node(pos, DemoNode::String("".to_owned()));
            ui.close_menu();
        }
        if ui.button("Show image").clicked() {
            snarl.insert_node(pos, DemoNode::ShowImage("".to_owned()));
            ui.close_menu();
        }
        if ui.button("Sink").clicked() {
            snarl.insert_node(pos, DemoNode::Sink);
            ui.close_menu();
        }
    }

    fn node_menu(
        &mut self,
        node: NodeId,
        _inputs: &[InPin],
        _outputs: &[OutPin],
        ui: &mut Ui,
        _scale: f32,
        snarl: &mut Snarl<DemoNode>,
    ) {
        ui.label("Node menu");
        if ui.button("Remove").clicked() {
            snarl.remove_node(node);
            ui.close_menu();
        }
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
        _scale: f32,
        snarl: &mut Snarl<DemoNode>,
    ) {
        match snarl[node] {
            DemoNode::Sink => {
                ui.label("Displays anything connected to it");
            }
            DemoNode::Integer(_) => {
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
            text: format!("0"),
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
            Expr::Var(ref name) => args[binding_index(name)],
            Expr::Val(value) => *value,
            Expr::UnOp { op, ref expr } => match op {
                UnOp::Pos => expr.eval(bindings, args),
                UnOp::Neg => -expr.eval(bindings, args),
            },
            Expr::BinOp {
                ref lhs,
                op,
                ref rhs,
            } => match op {
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

        match (op, next_op) {
            (BinOp::Add | BinOp::Sub, BinOp::Mul | BinOp::Div) => {
                let rhs = Self::parse_binop(rhs, next_op, input)?;
                Ok(Expr::BinOp {
                    lhs,
                    op,
                    rhs: Box::new(rhs),
                })
            }
            _ => {
                let lhs = Expr::BinOp { lhs, op, rhs };
                Self::parse_binop(Box::new(lhs), next_op, input)
            }
        }
    }
}

pub struct DemoApp {
    snarl: Snarl<DemoNode>,
}

impl DemoApp {
    pub fn new(cx: &CreationContext) -> Self {
        let snarl = match cx.storage {
            None => Snarl::new(),
            Some(storage) => {
                let snarl = storage
                    .get_string("snarl")
                    .and_then(|snarl| serde_json::from_str(&snarl).ok())
                    .unwrap_or_else(Snarl::new);

                snarl
            }
        };

        DemoApp { snarl }
    }
}

impl App for DemoApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui_extras::install_image_loaders(ctx);

        ctx.style_mut(|style| style.animation_time = 5.0);

        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            // The top panel is often a good place for a menu bar:

            egui::menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("Quit").clicked() {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close)
                    }
                });
                ui.add_space(16.0);

                egui::widgets::global_dark_light_mode_switch(ui);
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            self.snarl.show(
                &mut DemoViewer,
                &SnarlStyle {
                    collapsible: true,
                    wire_frame_size: Some(100.0),
                    ..Default::default()
                },
                egui::Id::new("snarl"),
                ui,
            );
        });
    }

    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        let snarl = serde_json::to_string(&self.snarl).unwrap();
        storage.set_string("snarl", snarl);
    }
}

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
        Box::new(|cx| Box::new(DemoApp::new(cx))),
    )
}

fn format_float(v: f64) -> String {
    let v = (v * 1000.0).round() / 1000.0;
    format!("{}", v)
}
