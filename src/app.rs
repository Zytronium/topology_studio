use egui::*;
use crate::canvas::{canvas_ui, CanvasTransform, CanvasInteraction};
use crate::model::TopologyProject;
use crate::theme::AppTheme;
use crate::palette::{PaletteState, palette_ui};
use crate::icons;

pub struct TopologyApp {
    pub project: TopologyProject,
    pub transform: CanvasTransform,
    pub theme: AppTheme,
    pub palette_state: PaletteState,
    pub canvas_interaction: CanvasInteraction,
    pub palette_drag_ended: bool,
}

impl TopologyApp {
    pub fn new(_cc: &eframe::CreationContext) -> Self {
        Self {
            project: TopologyProject::new(),
            transform: CanvasTransform::default(),
            theme: AppTheme::Light,
            palette_state: PaletteState::default(),
            canvas_interaction: CanvasInteraction::default(),
            palette_drag_ended: false,
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

        // Check if palette drag just ended
        self.palette_drag_ended = false;
        if self.palette_state.is_dragging() {
            // Check if pointer was released
            if ctx.input(|i| i.pointer.primary_released()) {
                self.palette_drag_ended = true;
            }
        }

        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("New").clicked() {
                        self.project = TopologyProject::new();
                        self.transform = CanvasTransform::default();
                        self.canvas_interaction = CanvasInteraction::default();
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

        // Left palette panel
        egui::SidePanel::left("palette_panel")
            .default_width(160.0)
            .resizable(false)
            .show(ctx, |ui| {
                palette_ui(ui, &mut self.palette_state);
            });

        // Capture palette selection before canvas consumes it
        let place_type = self.palette_state.selected_type;

        // Central canvas panel
        egui::CentralPanel::default().show(ctx, |ui| {
            let result = canvas_ui(
                ui,
                &mut self.project,
                &mut self.transform,
                &mut self.canvas_interaction,
                place_type,
                self.palette_state.drag_type(),
                self.palette_drag_ended,
            );

            // Clear palette selection after placing a node (click-to-place)
            if result.node_added {
                self.palette_state.selected_type = None;
            }

            // Clear drag state after processing
            if self.palette_drag_ended {
                self.palette_state.dragging_type = None;
            }
        });

        // === PALETTE DRAG PREVIEW OVERLAY ===
        // Draw the drag preview icon on top of all panels (including the palette)
        if let Some(drag_node_type) = self.palette_state.drag_type() {
            if let Some(pointer_pos) = ctx.input(|i| i.pointer.hover_pos()) {
                let layer_id = LayerId::new(egui::Order::Foreground, Id::new("palette_drag_preview"));
                let painter = ctx.layer_painter(layer_id);
                let preview_size = icons::ICON_SIZE;
                painter.circle_filled(pointer_pos, preview_size / 2.0, Color32::from_rgba_unmultiplied(255, 255, 255, 80));
                painter.circle_stroke(pointer_pos, preview_size / 2.0, Stroke::new(2.0, Color32::from_rgba_unmultiplied(100, 150, 255, 180)));
                let visuals = ctx.style().visuals.clone();
                icons::draw_node_icon_preview(&painter, drag_node_type, pointer_pos, preview_size, &visuals);
            }
        }
    }
}
