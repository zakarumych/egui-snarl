use std::cell::RefCell;

use eframe::App;
use egui::{pos2, vec2, InnerResponse, Ui};
use egui_snarl::{Effects, Pin, Snarl, SnarlViewer};

#[derive(Clone, Copy, serde::Deserialize, serde::Serialize)]
enum DemoNode {
    /// Node with single input.
    /// Displays the value of the input.
    Sink,

    /// Value node with a single output.
    /// The value is editable in UI.
    Integer(u32),
}

struct DemoViewer;

impl SnarlViewer<DemoNode> for DemoViewer {
    fn node_picker(&mut self, _ui: &mut Ui) -> egui::InnerResponse<Option<DemoNode>> {
        todo!()
    }

    fn title(&mut self, node: &DemoNode) -> std::borrow::Cow<'static, str> {
        match node {
            DemoNode::Sink => "Sink".into(),
            DemoNode::Integer(_) => "Integer".into(),
        }
    }

    fn inputs(&mut self, node: &DemoNode) -> usize {
        match node {
            DemoNode::Sink => 1,
            DemoNode::Integer(_) => 0,
        }
    }

    fn outputs(&mut self, node: &DemoNode) -> usize {
        match node {
            DemoNode::Sink => 0,
            DemoNode::Integer(_) => 1,
        }
    }

    fn show_input(
        &mut self,
        node: &RefCell<DemoNode>,
        pin: Pin<DemoNode>,
        ui: &mut Ui,
    ) -> egui::InnerResponse<Effects<DemoNode>> {
        match *node.borrow() {
            DemoNode::Sink => {
                assert_eq!(pin.local, 0, "Sink node has only one input");

                match &*pin.remote {
                    [] => {
                        let r = ui.label("None");
                        InnerResponse::new(Effects::default(), r)
                    }
                    [remote] => match *remote.node.borrow() {
                        DemoNode::Sink => unreachable!("Sink node has no outputs"),
                        DemoNode::Integer(value) => {
                            assert_eq!(remote.idx, 0, "Integer node has only one output");
                            let r = ui.label(format!("{}", value));
                            InnerResponse::new(Effects::default(), r)
                        }
                    },
                    _ => unreachable!("Sink input has only one wire"),
                }
            }
            DemoNode::Integer(_) => {
                unreachable!("Integer node has no inputs")
            }
        }
    }

    fn show_output(
        &mut self,
        node: &RefCell<DemoNode>,
        pin: Pin<DemoNode>,
        ui: &mut Ui,
    ) -> egui::InnerResponse<Effects<DemoNode>> {
        match &mut *node.borrow_mut() {
            DemoNode::Sink => {
                unreachable!("Sink node has no outputs")
            }
            DemoNode::Integer(value) => {
                assert_eq!(pin.local, 0, "Integer node has only one output");
                let r = ui.add(egui::DragValue::new(value));
                InnerResponse::new(Effects::default(), r)
            }
        }
    }
}

#[derive(Default, serde::Deserialize, serde::Serialize)]
#[serde(default)] // if we add new fields, give them default values when deserializing old state
pub struct DemoApp {
    snarl: Snarl<DemoNode>,
}

impl DemoApp {
    pub fn new() -> Self {
        let mut snarl = Snarl::new();

        snarl.add_node(
            DemoNode::Integer(42),
            egui::Rect::from_min_size(pos2(10.0, 20.0), vec2(100.0, 50.0)),
        );

        snarl.add_node(
            DemoNode::Sink,
            egui::Rect::from_min_size(pos2(40.0, 200.0), vec2(100.0, 50.0)),
        );

        DemoApp { snarl }
    }
}

impl App for DemoApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            // The top panel is often a good place for a menu bar:

            egui::menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("Quit").clicked() {
                        _frame.close();
                    }
                });
                ui.add_space(16.0);

                egui::widgets::global_dark_light_mode_switch(ui);
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            self.snarl.show(&mut DemoViewer, ui);
        });
    }
}

fn main() -> eframe::Result<()> {
    let native_options = eframe::NativeOptions {
        initial_window_size: Some([400.0, 300.0].into()),
        min_window_size: Some([300.0, 220.0].into()),
        ..Default::default()
    };

    eframe::run_native(
        "eframe template",
        native_options,
        Box::new(|_| Box::new(DemoApp::new())),
    )
}
