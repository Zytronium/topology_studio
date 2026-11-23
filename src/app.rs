use egui::*;
use crate::canvas::{canvas_ui, CanvasTransform, CanvasInteraction};
use crate::model::TopologyProject;
use crate::theme::{ThemeMode, AppTheme, AppSettings, save_settings, load_settings_public};
use crate::palette::{PaletteState, palette_ui, links_ui};
use crate::icons;
use crate::commands::UndoRedoStack;
use crate::clipboard::Clipboard;
use crate::export::{self, ExportOptions};

/// Action pending after the user confirms they want to discard or save changes.
#[derive(Clone, Copy)]
enum PendingAction {
    New,
    Open,
    Close,
}

pub struct TopologyApp {
    pub project: TopologyProject,
    pub transform: CanvasTransform,
    pub theme_mode: ThemeMode,
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
    /// App settings (snap to grid, autosave, etc.)
    pub settings: AppSettings,
    /// Whether to show the settings dialog
    pub show_settings: bool,
    /// Snapshot of project state before a nudge session started (for undo batching)
    pub nudge_start_project: Option<TopologyProject>,
    /// Timer for autosave (seconds remaining until next save)
    pub autosave_timer: f64,
    /// Whether to show the "Save Changes?" dialog
    pub show_save_prompt: bool,
    /// Action to perform after the user confirms the save prompt
    pub pending_action: Option<PendingAction>,
}

impl TopologyApp {
    pub fn new(_cc: &eframe::CreationContext) -> Self {
        // Load saved theme mode and settings
        let theme_mode = ThemeMode::load();
        let settings = load_settings_public();
        Self {
            project: TopologyProject::new(),
            transform: CanvasTransform::default(),
            theme_mode,
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
            settings,
            show_settings: false,
            nudge_start_project: None,
            autosave_timer: 0.0,
            show_save_prompt: false,
            pending_action: None,
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

    /// Show the settings dialog.
    fn show_settings_dialog(&mut self, ctx: &egui::Context) {
        egui::Window::new("Settings")
            .collapsible(false)
            .resizable(false)
            .default_size([280.0, 180.0])
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                ui.heading("Settings");
                ui.separator();
                ui.add_space(4.0);

                // Theme selection
                ui.label("Theme:");
                ui.horizontal(|ui| {
                    let mut current = match self.theme_mode {
                        ThemeMode::System => 0,
                        ThemeMode::Light => 1,
                        ThemeMode::Dark => 2,
                    };
                    if ui.radio_value(&mut current, 0, "System").clicked() {
                        self.theme_mode = ThemeMode::System;
                        self.theme_mode.save();
                    }
                    if ui.radio_value(&mut current, 1, "Light").clicked() {
                        self.theme_mode = ThemeMode::Light;
                        self.theme_mode.save();
                    }
                    if ui.radio_value(&mut current, 2, "Dark").clicked() {
                        self.theme_mode = ThemeMode::Dark;
                        self.theme_mode.save();
                    }
                });

                ui.add_space(8.0);

                // Snap to grid
                ui.checkbox(&mut self.settings.snap_to_grid, "Snap to grid (20px)");

                ui.add_space(8.0);

                // Autosave
                ui.checkbox(&mut self.settings.autosave, "Autosave");
                if self.settings.autosave {
                    ui.label(egui::RichText::new("Saves every 30 seconds when a file is open").size(11.0).weak());
                }

                ui.add_space(12.0);
                if ui.button("Close").clicked() {
                    self.show_settings = false;
                    save_settings(&self.settings);
                }
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

    /// Show the "Save Changes?" dialog.
    fn show_save_prompt_dialog(&mut self, ctx: &egui::Context) {
        egui::Window::new("Save Changes?")
            .collapsible(false)
            .resizable(false)
            .default_size([300.0, 100.0])
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                ui.label("Do you want to save your changes before continuing?");
                ui.add_space(12.0);
                ui.horizontal(|ui| {
                    if ui.button("Save").clicked() {
                        // Save first, then perform the pending action
                        self.save_project();
                        // Don't close the dialog yet — let the action happen next frame
                        // if save succeeded (unsaved_changes cleared)
                        if !self.unsaved_changes {
                            self.show_save_prompt = false;
                            if let Some(action) = self.pending_action.take() {
                                match action {
                                    PendingAction::New => self.do_new(),
                                    PendingAction::Open => self.open_project(),
                                    PendingAction::Close => ctx.send_viewport_cmd(egui::ViewportCommand::Close),
                                }
                            }
                        }
                    }
                    if ui.button("Discard").clicked() {
                        self.show_save_prompt = false;
                        if let Some(action) = self.pending_action.take() {
                            match action {
                                PendingAction::New => self.do_new(),
                                PendingAction::Open => self.open_project(),
                                PendingAction::Close => ctx.send_viewport_cmd(egui::ViewportCommand::Close),
                            }
                        }
                    }
                    if ui.button("Cancel").clicked() {
                        self.show_save_prompt = false;
                        self.pending_action = None;
                    }
                });
            });
    }

    /// Prompt the user to save changes if there are any, then perform the action.
    /// Returns true if the action was deferred (dialog shown), false if executed immediately.
    fn prompt_save_if_needed(&mut self, action: PendingAction) -> bool {
        if self.unsaved_changes {
            self.show_save_prompt = true;
            self.pending_action = Some(action);
            true
        } else {
            false
        }
    }

    /// Reset the project to a new, empty state.
    fn do_new(&mut self) {
        self.project = TopologyProject::new();
        self.transform = CanvasTransform::default();
        self.canvas_interaction = CanvasInteraction::default();
        self.undo_redo.clear();
        self.current_file = None;
        self.unsaved_changes = false;
        self.autosave_timer = 0.0;
        self.drag_start_project = None;
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
                self.autosave_timer = 0.0;
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
                self.autosave_timer = 0.0;
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
                        self.autosave_timer = 0.0;
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
        // Set theme based on theme mode
        let resolved_theme = self.theme_mode.resolve();
        match resolved_theme {
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

        // === HANDLE WINDOW CLOSE ===
        // Intercept close request to prompt for saving unsaved changes
        let close_requested = ctx.input(|i| i.raw.viewport().close_requested());
        if close_requested {
            if self.prompt_save_if_needed(PendingAction::Close) {
                // Dialog will be shown; prevent the window from closing
                ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            }
        }

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
            self.nudge_start_project = None;
        }
        // Ctrl+Shift+Z or Ctrl+Y: Redo
        if ctrl && (key_z && shift || key_y) {
            self.undo_redo.redo(&mut self.project);
            self.unsaved_changes = true;
            self.nudge_start_project = None;
        }
        // Ctrl+C: Copy (use Event::Copy because egui intercepts Ctrl+C for TextEdit)
        if event_copy {
            let ids: Vec<uuid::Uuid> = self.canvas_interaction.selected_nodes.iter().copied().collect();
            if !ids.is_empty() {
                self.clipboard.copy_from(&self.project, &ids);
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
                // Select all pasted nodes
                for id in &new_ids {
                    self.canvas_interaction.selected_nodes.insert(*id);
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
            if !self.prompt_save_if_needed(PendingAction::Open) {
                self.open_project();
            }
        }
        // Ctrl+0: Reset zoom to 100% (check raw text input for "0")
        if ctrl {
            let has_zero = ctx.input(|i| i.events.iter().any(|ev| matches!(ev, egui::Event::Text(t) if t == "0")));
            if has_zero {
                self.transform.reset_zoom();
            }
        }

        // === MARK UNSAVED CHANGES ===
        // Track when the project is modified so we can show * in the title

        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("New").clicked() {
                        if !self.prompt_save_if_needed(PendingAction::New) {
                            self.do_new();
                        }
                    }
                    if ui.button("Open…").clicked() {
                        if !self.prompt_save_if_needed(PendingAction::Open) {
                            self.open_project();
                        }
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
                        self.export_options.dark_mode = matches!(resolved_theme, AppTheme::Dark);
                        self.show_export_dialog = true;
                    }
                    ui.separator();
                    if ui.button("Settings").clicked() {
                        self.show_settings = true;
                    }
                });
                ui.menu_button("Edit", |ui| {
                    let undo_enabled = self.undo_redo.can_undo();
                    if ui.add_enabled(undo_enabled, egui::Button::new("Undo")).clicked() {
                        self.undo_redo.undo(&mut self.project);
                        self.unsaved_changes = true;
                    }
                    let redo_enabled = self.undo_redo.can_redo();
                    if ui.add_enabled(redo_enabled, egui::Button::new("Redo")).clicked() {
                        self.undo_redo.redo(&mut self.project);
                        self.unsaved_changes = true;
                    }
                    ui.separator();
                    let copy_enabled = !self.canvas_interaction.selected_nodes.is_empty();
                    if ui.add_enabled(copy_enabled, egui::Button::new("Copy")).clicked() {
                        let ids: Vec<uuid::Uuid> = self.canvas_interaction.selected_nodes.iter().copied().collect();
                        if !ids.is_empty() {
                            self.clipboard.copy_from(&self.project, &ids);
                        }
                    }
                    let paste_enabled = !self.clipboard.is_empty();
                    if ui.add_enabled(paste_enabled, egui::Button::new("Paste")).clicked() {
                        let (paste_x, paste_y) = self.paste_position(ctx);
                        let before = self.project.clone();
                        let new_ids = self.clipboard.paste_into(&mut self.project, paste_x, paste_y);
                        self.undo_redo.execute(crate::commands::Command::new(before, self.project.clone()));
                        for id in &new_ids {
                            self.canvas_interaction.selected_nodes.insert(*id);
                        }
                    }
                    let delete_enabled = !self.canvas_interaction.selected_nodes.is_empty()
                        || self.canvas_interaction.selected_link.is_some();
                    if ui.add_enabled(delete_enabled, egui::Button::new("Delete")).clicked() {
                        let before = self.project.clone();
                        if let Some(link_id) = self.canvas_interaction.selected_link {
                            self.project.remove_link(link_id);
                            self.canvas_interaction.selected_link = None;
                        } else {
                            let ids_to_remove: Vec<uuid::Uuid> = self.canvas_interaction.selected_nodes.iter().copied().collect();
                            for node_id in &ids_to_remove {
                                self.project.remove_node(*node_id);
                            }
                            self.canvas_interaction.selected_nodes.clear();
                            self.canvas_interaction.editing_node = None;
                        }
                        self.undo_redo.execute(crate::commands::Command::new(before, self.project.clone()));
                    }
                });
                ui.menu_button("View", |ui| {
                    if ui.button("Zoom to Fit").clicked() {
                        self.transform.zoom_to_fit(&self.project, 50.0);
                    }
                    if ui.button("Reset Zoom").clicked() {
                        self.transform.reset_zoom();
                    }
                    ui.separator();
                    ui.checkbox(&mut self.settings.snap_to_grid, "Snap to grid");
                });

                ui.add_space(ui.available_width() * 0.3);
                ui.label("Network Topology Studio");
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
                self.settings.snap_to_grid,
            );

            // If dragging just started, save the pre-drag state
            if !was_dragging && self.canvas_interaction.dragging_node.is_some() {
                self.drag_start_project = Some(before_project.clone());
            }

            // Record undo/redo for discrete canvas actions
            if result.node_added || result.node_deleted || result.node_renamed
                || result.link_added || result.link_deleted {
                self.undo_redo.execute(crate::commands::Command::new(before_project.clone(), self.project.clone()));
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
            // For node nudges (Ctrl+Arrow), batch into a single undo entry
            if result.nodes_nudged {
                // If this is the first nudge frame, save the pre-nudge state
                if self.nudge_start_project.is_none() {
                    self.nudge_start_project = Some(before_project.clone());
                }
                // Don't record undo yet — wait for the nudge to stop
            } else if self.nudge_start_project.is_some() {
                // Nudge just stopped — record a single undo entry for the entire session
                if let Some(nudge_start) = self.nudge_start_project.take() {
                    self.undo_redo.execute(crate::commands::Command::new(nudge_start, self.project.clone()));
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

        // === SETTINGS DIALOG ===
        if self.show_settings {
            self.show_settings_dialog(ctx);
        }

        // === SAVE CHANGES? DIALOG ===
        if self.show_save_prompt {
            self.show_save_prompt_dialog(ctx);
        }

        // === AUTOSAVE TIMER ===
        if self.settings.autosave && self.current_file.is_some() && self.unsaved_changes {
            let dt = ctx.input(|i| i.stable_dt) as f64;
            self.autosave_timer += dt;
            // Autosave every 30 seconds
            if self.autosave_timer >= 30.0 {
                self.autosave_timer = 0.0;
                self.save_project();
            }
        }
    }
}
