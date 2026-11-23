use egui::*;
use crate::canvas::{canvas_ui, CanvasTransform, CanvasInteraction};
use crate::model::TopologyProject;
use crate::theme::AppTheme;
use crate::palette::{PaletteState, palette_ui, links_ui};
use crate::icons;
use crate::commands::UndoRedoStack;
use crate::clipboard::Clipboard;
use crate::export::{self, ExportOptions};

pub struct TopologyApp {
    pub project: TopologyProject,
    pub transform: CanvasTransform,
    pub theme: AppTheme,
    pub palette_state: PaletteState,
    pub canvas_interaction: CanvasInteraction,
    pub palette_drag_ended: bool,
    pub undo_redo: UndoRedoStack,
    pub clipboard: Clipboard,
    /// Current file path (for "Save" vs "Save As")
    pub current_file: Option<String>,
    /// Whether the project has unsaved changes since last save/open
    pub unsaved_changes: bool,
    /// Snapshot of project state before a node drag started (for undo batching)
    pub drag_start_project: Option<TopologyProject>,
    /// Whether the cursor was over the canvas last frame (for paste positioning)
    pub cursor_on_canvas: bool,
    /// Export options for SVG export dialog
    pub export_options: ExportOptions,
    /// Whether to show the export options dialog
    pub show_export_dialog: bool,
}

impl TopologyApp {
    pub fn new(_cc: &eframe::CreationContext) -> Self {
        // Load saved theme or detect system theme
        let theme = AppTheme::load();
        Self {
            project: TopologyProject::new(),
            transform: CanvasTransform::default(),
            theme,
            palette_state: PaletteState::default(),
            canvas_interaction: CanvasInteraction::default(),
            palette_drag_ended: false,
            undo_redo: UndoRedoStack::default(),
            clipboard: Clipboard::default(),
            current_file: None,
            unsaved_changes: false,
            drag_start_project: None,
            cursor_on_canvas: false,
            export_options: ExportOptions::default(),
            show_export_dialog: false,
        }
    }

    /// Show the export options dialog.
    fn show_export_dialog(&mut self, ctx: &egui::Context) {
        egui::Window::new("Export SVG")
            .collapsible(false)
            .resizable(false)
            .default_size([220.0, 140.0])
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                ui.checkbox(&mut self.export_options.dark_mode, "Dark mode");
                ui.checkbox(&mut self.export_options.show_grid, "Show grid lines");
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    if ui.button("Export").clicked() {
                        self.do_export_svg();
                        self.show_export_dialog = false;
                    }
                    if ui.button("Cancel").clicked() {
                        self.show_export_dialog = false;
                    }
                });
            });
    }

    /// Export the current project to an SVG file.
    fn do_export_svg(&mut self) {
        let result = rfd::FileDialog::new()
            .add_filter("SVG", &["svg"])
            .set_file_name("topology.svg")
            .save_file();
        let path = match result {
            Some(p) => p.to_string_lossy().to_string(),
            None => return,
        };
        let svg = export::export_to_svg(&self.project, &self.export_options);
        if let Err(e) = std::fs::write(&path, svg) {
            eprintln!("Failed to export SVG: {}", e);
        }
    }

    /// Save the current project to a JSON file (uses current_file path, or prompts for one).
    fn save_project(&mut self) {
        if self.current_file.is_none() {
            self.save_project_as();
            return;
        }
        let path = self.current_file.as_ref().unwrap().clone();
        if let Ok(json) = serde_json::to_string_pretty(&self.project) {
            if let Err(e) = std::fs::write(&path, json) {
                eprintln!("Failed to save project: {}", e);
            } else {
                self.unsaved_changes = false;
            }
        }
    }

    /// Save the current project to a new file path (always prompts for a file).
    fn save_project_as(&mut self) {
        let result = rfd::FileDialog::new()
            .add_filter("Topology", &["topo"])
            .set_file_name("untitled.topo")
            .save_file();
        let path = match result {
            Some(p) => p.to_string_lossy().to_string(),
            None => return,
        };
        if let Ok(json) = serde_json::to_string_pretty(&self.project) {
            if let Err(e) = std::fs::write(&path, json) {
                eprintln!("Failed to save project: {}", e);
            } else {
                self.current_file = Some(path);
                self.unsaved_changes = false;
            }
        }
    }

    /// Open a project from a JSON file.
    fn open_project(&mut self) {
        let result = rfd::FileDialog::new()
            .add_filter("Topology", &["topo"])
            .pick_file();
        match result {
            Some(path) => {
                let path_str = path.to_string_lossy().to_string();
                if let Ok(json) = std::fs::read_to_string(&path) {
                    if let Ok(project) = serde_json::from_str(&json) {
                        self.project = project;
                        self.current_file = Some(path_str);
                        self.unsaved_changes = false;
                        self.undo_redo.clear();
                        self.canvas_interaction = CanvasInteraction::default();
                    }
                }
            }
            None => {}
        }
    }

    /// Build the window title with filename and unsaved changes indicator.
    fn window_title(&self) -> String {
        let base = "Network Topology Studio";
        if let Some(ref path) = self.current_file {
            let filename = std::path::Path::new(path)
                .file_name()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_else(|| path.clone());
            if self.unsaved_changes {
                format!("{} - {} *", base, filename)
            } else {
                format!("{} - {}", base, filename)
            }
        } else if self.unsaved_changes {
            format!("{} *", base)
        } else {
            base.to_string()
        }
    }

    /// Calculate the paste position in world coordinates.
    /// If the cursor is over the canvas (tracked via cursor_on_canvas), use the cursor's world position.
    /// Otherwise, default to the center of the visible canvas area.
    fn paste_position(&self, ctx: &egui::Context) -> (f64, f64) {
        if self.cursor_on_canvas {
            if let Some(cursor_pos) = ctx.input(|i| i.pointer.hover_pos()) {
                let world = self.transform.screen_to_world(cursor_pos);
                return (world.x as f64, world.y as f64);
            }
        }
        // Default: paste at the center of the visible canvas area
        let screen_rect = ctx.available_rect();
        let screen_center = screen_rect.center();
        let world = self.transform.screen_to_world(screen_center);
        (world.x as f64, world.y as f64)
    }
}

impl eframe::App for TopologyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Set theme
        match self.theme {
            AppTheme::Light => ctx.set_theme(egui::Theme::Light),
            AppTheme::Dark => ctx.set_theme(egui::Theme::Dark),
        }

        // Update window title with filename and unsaved changes indicator
        ctx.send_viewport_cmd(egui::ViewportCommand::Title(self.window_title()));

        // Collect all input state in a single call to avoid nested RwLock acquisitions
        let mut input_state = None;
        ctx.input(|i| {
            input_state = Some((
                i.modifiers.ctrl,
                i.modifiers.shift,
                i.key_pressed(egui::Key::Z),
                i.key_pressed(egui::Key::Y),
                i.key_pressed(egui::Key::S),
                i.key_pressed(egui::Key::O),
                i.pointer.primary_released(),
                // egui intercepts Ctrl+C/V/X for its own TextEdit, so we check the raw event stream
                i.events.iter().any(|ev| matches!(ev, egui::Event::Copy)),
                i.events.iter().any(|ev| matches!(ev, egui::Event::Paste(_))),
            ));
        });
        let (ctrl, shift, key_z, key_y, key_s, key_o, pointer_released, event_copy, event_paste) =
            input_state.unwrap_or_default();

        // Check if palette drag just ended
        self.palette_drag_ended = false;
        if self.palette_state.is_dragging() && pointer_released {
            self.palette_drag_ended = true;
        }

        // === KEYBOARD SHORTCUTS ===
        // Ctrl+Z: Undo (not Ctrl+Shift+Z)
        if ctrl && key_z && !shift {
            self.undo_redo.undo(&mut self.project);
            self.unsaved_changes = true;
        }
        // Ctrl+Shift+Z or Ctrl+Y: Redo
        if ctrl && (key_z && shift || key_y) {
            self.undo_redo.redo(&mut self.project);
            self.unsaved_changes = true;
        }
        // Ctrl+C: Copy (use Event::Copy because egui intercepts Ctrl+C for TextEdit)
        if event_copy {
            if let Some(node_id) = self.canvas_interaction.selected_node {
                self.clipboard.copy_from(&self.project, &[node_id]);
            }
        }
        // Ctrl+V: Paste (use Event::Paste because egui intercepts Ctrl+V for TextEdit)
        // Paste at cursor position only if cursor is actually over the canvas panel
        if event_paste {
            if !self.clipboard.is_empty() {
                let (paste_x, paste_y) = self.paste_position(ctx);
                let before = self.project.clone();
                let new_ids = self.clipboard.paste_into(&mut self.project, paste_x, paste_y);
                self.undo_redo.execute(crate::commands::Command::new(before, self.project.clone()));
                self.unsaved_changes = true;
                // Select the first pasted node
                if let Some(id) = new_ids.first() {
                    self.canvas_interaction.selected_node = Some(*id);
                }
            }
        }
        // Ctrl+S: Save
        if ctrl && key_s && !shift {
            self.save_project();
        }
        // Ctrl+Shift+S: Save As
        if ctrl && key_s && shift {
            self.save_project_as();
        }
        // Ctrl+O: Open
        if ctrl && key_o {
            self.open_project();
        }

        // === MARK UNSAVED CHANGES ===
        // Track when the project is modified so we can show * in the title

        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("New").clicked() {
                        self.project = TopologyProject::new();
                        self.transform = CanvasTransform::default();
                        self.canvas_interaction = CanvasInteraction::default();
                        self.undo_redo.clear();
                        self.current_file = None;
                        self.unsaved_changes = false;
                        self.drag_start_project = None;
                    }
                    if ui.button("Open…").clicked() {
                        self.open_project();
                    }
                    if ui.button("Save").clicked() {
                        self.save_project();
                    }
                    if ui.button("Save As…").clicked() {
                        self.save_project_as();
                    }
                    ui.separator();
                    if ui.button("Export SVG").clicked() {
                        // Default dark_mode to match the current app theme
                        self.export_options.dark_mode = matches!(self.theme, AppTheme::Dark);
                        self.show_export_dialog = true;
                    }
                });
                ui.menu_button("Edit", |ui| {
                    let undo_enabled = self.undo_redo.can_undo();
                    if ui.add_enabled(undo_enabled, egui::Button::new("Undo")).clicked() {
                        self.undo_redo.undo(&mut self.project);
                    }
                    let redo_enabled = self.undo_redo.can_redo();
                    if ui.add_enabled(redo_enabled, egui::Button::new("Redo")).clicked() {
                        self.undo_redo.redo(&mut self.project);
                    }
                    ui.separator();
                    let copy_enabled = self.canvas_interaction.selected_node.is_some();
                    if ui.add_enabled(copy_enabled, egui::Button::new("Copy")).clicked() {
                        if let Some(node_id) = self.canvas_interaction.selected_node {
                            self.clipboard.copy_from(&self.project, &[node_id]);
                        }
                    }
                    let paste_enabled = !self.clipboard.is_empty();
                    if ui.add_enabled(paste_enabled, egui::Button::new("Paste")).clicked() {
                        // Menu paste goes to the center of the visible canvas area
                        let (paste_x, paste_y) = self.paste_position(ctx);
                        let before = self.project.clone();
                        let new_ids = self.clipboard.paste_into(&mut self.project, paste_x, paste_y);
                        self.undo_redo.execute(crate::commands::Command::new(before, self.project.clone()));
                        if let Some(id) = new_ids.first() {
                            self.canvas_interaction.selected_node = Some(*id);
                        }
                    }
                    let delete_enabled = self.canvas_interaction.selected_node.is_some()
                        || self.canvas_interaction.selected_link.is_some();
                    if ui.add_enabled(delete_enabled, egui::Button::new("Delete")).clicked() {
                        let before = self.project.clone();
                        if let Some(link_id) = self.canvas_interaction.selected_link {
                            self.project.remove_link(link_id);
                            self.canvas_interaction.selected_link = None;
                        } else if let Some(node_id) = self.canvas_interaction.selected_node {
                            self.project.remove_node(node_id);
                            self.canvas_interaction.selected_node = None;
                            self.canvas_interaction.editing_node = None;
                        }
                        self.undo_redo.execute(crate::commands::Command::new(before, self.project.clone()));
                    }
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
                    // Save the theme preference
                    self.theme.save();
                }
            });
        });

        // Left palette panel (scrollable when content exceeds window height)
        egui::SidePanel::left("palette_panel")
            .default_width(160.0)
            .resizable(false)
            .show(ctx, |ui| {
                egui::ScrollArea::vertical()
                    .auto_shrink(false)
                    .show(ui, |ui| {
                        palette_ui(ui, &mut self.palette_state);
                        ui.separator();
                        ui.add_space(4.0);
                        links_ui(ui, &mut self.palette_state.selected_link_style);
                    });
            });

        // Capture palette selection before canvas consumes it
        let place_type = self.palette_state.selected_type;
        let link_mode = self.palette_state.is_link_mode();
        let link_style = self.palette_state.link_style();

        // Central canvas panel
        egui::CentralPanel::default().show(ctx, |ui| {
            // Track whether cursor is over the canvas for paste positioning
            self.cursor_on_canvas = ui.rect_contains_pointer(ui.max_rect());

            // Track whether we were dragging before this frame
            let was_dragging = self.canvas_interaction.dragging_node.is_some();
            
            // Snapshot before canvas interaction for undo tracking
            let before_project = self.project.clone();

            let result = canvas_ui(
                ui,
                &mut self.project,
                &mut self.transform,
                &mut self.canvas_interaction,
                place_type,
                self.palette_state.drag_type(),
                self.palette_drag_ended,
                link_mode,
                link_style,
            );

            // If dragging just started, save the pre-drag state
            if !was_dragging && self.canvas_interaction.dragging_node.is_some() {
                self.drag_start_project = Some(before_project.clone());
            }

            // Record undo/redo for discrete canvas actions
            if result.node_added || result.node_deleted || result.node_renamed
                || result.link_added || result.link_deleted {
                self.undo_redo.execute(crate::commands::Command::new(before_project, self.project.clone()));
                self.unsaved_changes = true;
            }
            // For node moves, record undo only when dragging stops (not every frame)
            // Use node_moved_in_drag which persists across the drag session
            if was_dragging && self.canvas_interaction.dragging_node.is_none() && result.node_moved_in_drag {
                if let Some(drag_start) = self.drag_start_project.take() {
                    self.undo_redo.execute(crate::commands::Command::new(drag_start, self.project.clone()));
                    self.unsaved_changes = true;
                }
            }

            // Clear palette selection after placing a node (click-to-place)
            if result.node_added {
                self.palette_state.selected_type = None;
            }

            // Clear link mode after adding or deleting a link (single-link mode)
            if result.link_added || result.link_deleted {
                self.palette_state.selected_link_style = None;
                self.canvas_interaction.link_source = None;
            }

            // Handle context menu link style activation (after clear, so it takes precedence)
            if let Some(style) = result.context_link_style {
                self.palette_state.selected_link_style = Some(style);
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

        // === EXPORT OPTIONS DIALOG ===
        if self.show_export_dialog {
            self.show_export_dialog(ctx);
        }
    }
}
