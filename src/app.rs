use egui::*;

use crate::canvas::{canvas_ui, CanvasTransform};
use crate::model::TopologyProject;
use crate::theme::AppTheme;

pub struct TopologyApp {
    pub project: TopologyProject,
    pub transform: CanvasTransform,
    pub theme: AppTheme,
}

impl TopologyApp {
    pub fn new(_cc: &eframe::CreationContext) -> Self {
        Self {
            project: TopologyProject::new(),
            transform: CanvasTransform::default(),
            theme: AppTheme::Light,
        }
    }
}

impl eframe::App for TopologyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Set theme
        match self.theme {
            AppTheme::Light => ctx.set_theme(egui::Theme::Light),
            AppTheme::Dark => ctx.set_theme(egui::Theme::Dark),
        }

        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("New").clicked() {
                        self.project = TopologyProject::new();
                        self.transform = CanvasTransform::default();
                    }
                    if ui.button("Open…").clicked() {
                        // TODO: file dialog
                    }
                    if ui.button("Save").clicked() {
                        // TODO: file dialog
                    }
                    ui.separator();
                    if ui.button("Export SVG").clicked() {
                        // TODO: SVG export
                    }
                });
                ui.menu_button("Edit", |ui| {
                    ui.label("Undo");
                    ui.label("Redo");
                });
                ui.add_space(ui.available_width() * 0.3);
                ui.label("Network Topology Studio");
                ui.add_space(8.0);

                // Theme toggle
                let theme_label = match self.theme {
                    AppTheme::Light => "Light Mode",
                    AppTheme::Dark => "Dark Mode",
                };
                if ui.button(theme_label).clicked() {
                    self.theme = match self.theme {
                        AppTheme::Light => AppTheme::Dark,
                        AppTheme::Dark => AppTheme::Light,
                    };
                }
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            canvas_ui(ui, &mut self.project, &mut self.transform);
        });
    }
}
