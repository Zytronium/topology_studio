use egui::*;
use uuid::Uuid;
use crate::model::{Node, NodeType, TopologyProject};
use crate::icons;

/// Pan and zoom state for the canvas
#[derive(Clone, Copy, Debug)]
pub struct CanvasTransform {
    pub offset: Pos2,
    pub zoom: f32,
}

impl Default for CanvasTransform {
    fn default() -> Self {
        Self {
            offset: Pos2::ZERO,
            zoom: 1.0,
        }
    }
}

impl CanvasTransform {
    const MIN_ZOOM: f32 = 0.1;
    const MAX_ZOOM: f32 = 8.0;

    /// Convert screen coordinates to world coordinates
    pub fn screen_to_world(&self, screen: Pos2) -> Pos2 {
        let v: Vec2 = (screen - self.offset) / self.zoom;
        Pos2::new(v.x, v.y)
    }

    /// Convert world coordinates to screen coordinates
    pub fn world_to_screen(&self, world: Pos2) -> Pos2 {
        let v: Vec2 = world.to_vec2() * self.zoom + self.offset.to_vec2();
        Pos2::new(v.x, v.y)
    }
}

/// Interaction state for the canvas
pub struct CanvasInteraction {
    pub selected_node: Option<Uuid>,
    pub dragging_node: Option<Uuid>,
    pub is_panning: bool,
    pub pan_start: Pos2,
    pub pan_offset_start: Pos2,
    /// Node being edited via double-click; stores (node_id, current_text)
    pub editing_node: Option<(Uuid, String)>,
}

impl Default for CanvasInteraction {
    fn default() -> Self {
        Self {
            selected_node: None,
            dragging_node: None,
            is_panning: false,
            pan_start: Pos2::ZERO,
            pan_offset_start: Pos2::ZERO,
            editing_node: None,
        }
    }
}

impl CanvasInteraction {
    /// Find the node at the given world position, returning the closest one within hit range
    fn hit_test_node(project: &TopologyProject, world_pos: Pos2, transform: &CanvasTransform) -> Option<Uuid> {
        let hit_radius = icons::NODE_HIT_SIZE / (2.0 * transform.zoom);
        let mut closest: Option<(Uuid, f32)> = None;
        
        for node in &project.nodes {
            let node_center = pos2(node.x as f32, node.y as f32);
            let dist = (world_pos - node_center).length();
            if dist <= hit_radius {
                match closest {
                    Some((_, best_dist)) if dist >= best_dist => {},
                    _ => { closest = Some((node.id, dist)); }
                }
            }
        }
        
        closest.map(|(id, _)| id)
    }
}

/// Result of canvas interaction processing
pub struct CanvasResult {
    pub node_added: bool,
    pub node_deleted: bool,
    pub node_moved: bool,
    pub node_renamed: bool,
}

/// Draw the canvas with grid, nodes, pan, and zoom
pub fn canvas_ui(
    ui: &mut Ui, 
    project: &mut TopologyProject, 
    transform: &mut CanvasTransform,
    interaction: &mut CanvasInteraction,
    palette_selected_type: Option<NodeType>,
    palette_drag_type: Option<NodeType>,
    palette_drag_ended: bool,
) -> CanvasResult {
    let mut result = CanvasResult {
        node_added: false,
        node_deleted: false,
        node_moved: false,
        node_renamed: false,
    };

    let (response, painter) = ui.allocate_painter(ui.available_size(), Sense::click_and_drag());
    let rect = response.rect;
    let visuals = ui.visuals();
    let bg_color = visuals.extreme_bg_color;

    // Draw background
    painter.rect_filled(rect, 0.0, bg_color);

    // Draw grid
    draw_grid(&painter, rect, transform, bg_color);

    // === NODE RENDERING ===
    for node in &project.nodes {
        let center = transform.world_to_screen(pos2(node.x as f32, node.y as f32));
        let icon_size = icons::ICON_SIZE * transform.zoom;
        let selected = interaction.selected_node == Some(node.id);
        let editing = interaction.editing_node.as_ref().map(|(id, _)| *id) == Some(node.id);
        icons::draw_node_icon(&painter, node.node_type, center, icon_size, selected || editing, visuals);

        // Draw label below the node (skip if this node is being edited)
        if !editing {
            let label = &node.label;
            let label_font = FontId::proportional(12.0 * transform.zoom);
            let galley = painter.layout_no_wrap(label.clone(), label_font, visuals.text_color());
            let label_pos = center + vec2(-galley.size().x / 2.0, icon_size / 2.0 + 4.0 * transform.zoom);
            painter.galley(label_pos, galley, Color32::TRANSPARENT);
        }
    }

    // === INLINE LABEL EDITOR ===
    // Use an Area that captures input focus so clicks inside it don't reach the canvas
    // IMPORTANT: We must persist edit_text back to interaction.editing_node each frame,
    // otherwise the TextEdit widget resets to the cloned value every frame.
    let editing_info = interaction.editing_node.clone();
    let mut editor_rect: Option<Rect> = None;
    if let Some((edit_id, edit_text)) = editing_info {
        if project.get_node(edit_id).is_some() {
            let node = project.get_node(edit_id).unwrap();
            let center = transform.world_to_screen(pos2(node.x as f32, node.y as f32));
            let icon_size = icons::ICON_SIZE * transform.zoom;
            let label_pos = center + vec2(0.0, icon_size / 2.0 + 4.0 * transform.zoom);
            let text_field_rect = Rect::from_center_size(label_pos, vec2(120.0, 20.0 * transform.zoom));

            let window_id = egui::Id::new(edit_id).with("label_edit");
            let ctx = ui.ctx().clone();
            let mut close_editor = false;
            let mut saved_text = None;
            let mut edited_text = edit_text;

            // Area with Sense::click() captures pointer events so they don't fall through to canvas
            let area_response = egui::Area::new(window_id)
                .order(egui::Order::Foreground)
                .constrain(false)
                .fixed_pos(text_field_rect.min)
                .show(&ctx, |ui| {
                    let frame = egui::Frame::none()
                        .fill(visuals.extreme_bg_color)
                        .stroke(Stroke::new(1.5, visuals.selection.stroke.color))
                        .rounding(3.0);
                    frame.show(ui, |ui| {
                        ui.add_space(1.0);
                        let response = ui.add(
                            egui::TextEdit::singleline(&mut edited_text)
                                .desired_width(100.0)
                                .hint_text("Label")
                        );
                        // Auto-focus the text field
                        response.request_focus();
                        if ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                            close_editor = true;
                            saved_text = Some(edited_text.clone());
                        }
                        if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                            close_editor = true;
                            saved_text = None;
                        }
                    });
                });

            // Track the editor's screen rect for click-outside detection
            editor_rect = Some(area_response.response.rect);

            // Persist the edited text back to interaction state so it survives the next frame
            interaction.editing_node = Some((edit_id, edited_text));

            if close_editor {
                interaction.editing_node = None;
                if let Some(text) = saved_text {
                    if let Some(node) = project.get_node_mut(edit_id) {
                        node.label = text;
                        result.node_renamed = true;
                    }
                }
            }
        } else {
            interaction.editing_node = None;
        }
    }

    // === INPUT HANDLING ===
    // Read raw input state once to avoid multiple borrows
    let input = ui.input(|i| i.pointer.clone());
    let space_held = ui.input(|i| i.key_down(egui::Key::Space));

    // When the label editor is open, only handle clicks outside the editor to close it
    let editor_open = interaction.editing_node.is_some();

    // If the editor is open, check for clicks outside the editor area to close it
    if editor_open {
        if let Some(editor_r) = editor_rect {
            if let Some(click_pos) = input.hover_pos() {
                if input.primary_clicked() && !editor_r.contains(click_pos) {
                    // Clicked outside the editor — close and save
                    if let Some((edit_id, edited_text)) = interaction.editing_node.take() {
                        if !edited_text.is_empty() {
                            if let Some(node) = project.get_node_mut(edit_id) {
                                node.label = edited_text;
                                result.node_renamed = true;
                            }
                        }
                    }
                }
            }
        }
    }

    // Handle palette drag placement
    if palette_drag_ended {
        if let Some(node_type) = palette_drag_type {
            if let Some(pointer_pos) = ui.input(|i| i.pointer.hover_pos()) {
                if rect.contains(pointer_pos) {
                    let world_pos = transform.screen_to_world(pointer_pos);
                    let node = Node::new(node_type, world_pos.x as f64, world_pos.y as f64);
                    project.add_node(node);
                    result.node_added = true;
                }
            }
        }
    }

    // Use hover_pos for global pointer position (works even outside canvas rect)
    let global_pointer = input.hover_pos();
    let in_canvas = global_pointer.map(|p| rect.contains(p)).unwrap_or(false);

    // --- PRIMARY BUTTON CLICK: double-click to rename, single-click to select/place ---
    // Use primary_clicked() (edge-triggered) for click events so double-click is detected
    // before the drag logic kicks in. Drag is started only after a short delay to allow
    // the double-click to be recognized.
    if in_canvas && !editor_open {
        let pointer_pos = input.interact_pos().unwrap_or(rect.center());

        // Middle mouse button down - start panning
        if input.middle_down() {
            if !interaction.is_panning {
                interaction.is_panning = true;
                interaction.pan_start = pointer_pos;
                interaction.pan_offset_start = transform.offset;
                interaction.dragging_node = None;
            }
        }

        // Primary button clicked (edge-triggered, fires once per click)
        if input.primary_clicked() {
            // Check if we're on a node
            if let Some(node_id) = CanvasInteraction::hit_test_node(project, transform.screen_to_world(pointer_pos), transform) {
                if response.double_clicked() {
                    // Double-click: start editing the node's label
                    if let Some(node) = project.get_node(node_id) {
                        interaction.editing_node = Some((node_id, node.label.clone()));
                        interaction.selected_node = Some(node_id);
                        interaction.dragging_node = None;
                    }
                } else {
                    // Single click: select the node; drag will start after a delay
                    interaction.selected_node = Some(node_id);
                    // Don't start dragging yet — wait for drag delta to confirm it's not a double-click
                }
            } else if space_held {
                // Space+left click: start panning
                interaction.is_panning = true;
                interaction.pan_start = pointer_pos;
                interaction.pan_offset_start = transform.offset;
            } else {
                // Clicked on empty space - deselect
                interaction.selected_node = None;
                interaction.editing_node = None;

                // Place node from palette selection (single click placement)
                if let Some(node_type) = palette_selected_type {
                    let world_pos = transform.screen_to_world(pointer_pos);
                    let node = Node::new(node_type, world_pos.x as f64, world_pos.y as f64);
                    let node_id = node.id;
                    project.add_node(node);
                    interaction.selected_node = Some(node_id);
                    result.node_added = true;
                }
            }
        }

        // Primary button held down with drag delta - start dragging a node or panning
        let is_palette_dragging = palette_drag_type.is_some();
        if input.primary_down()
            && !space_held
            && !is_palette_dragging
            && !interaction.is_panning
        {
            if let Some(delta) = input.total_drag_delta() {
                if delta.length() > 4.0 {
                    // Drag detected: check if we're on any node (selected or not)
                    if interaction.dragging_node.is_none() {
                        let world_pos = transform.screen_to_world(pointer_pos);
                        if let Some(node_id) = CanvasInteraction::hit_test_node(project, world_pos, transform) {
                            // Dragging a node — select it and start dragging
                            interaction.selected_node = Some(node_id);
                            interaction.dragging_node = Some(node_id);
                        } else {
                            // Dragging on empty space - start panning
                            interaction.is_panning = true;
                            interaction.pan_start = pointer_pos;
                            interaction.pan_offset_start = transform.offset;
                        }
                    }
                }
            }
        }

        // Primary button released - stop dragging/panning
        if input.primary_released() {
            interaction.dragging_node = None;
            interaction.is_panning = false;
        }

        // Middle button released - stop panning
        if input.button_released(egui::PointerButton::Middle) {
            interaction.is_panning = false;
        }
    }

    // --- DRAG NODE: works even when pointer leaves canvas ---
    if let Some(dragging_id) = interaction.dragging_node {
        if let Some(global_pos) = global_pointer {
            let world_pos = transform.screen_to_world(global_pos);
            if let Some(node) = project.get_node_mut(dragging_id) {
                node.x = world_pos.x as f64;
                node.y = world_pos.y as f64;
                result.node_moved = true;
            }
        }
    }

    // --- PAN: works even when pointer leaves canvas ---
    if interaction.is_panning {
        if let Some(global_pos) = global_pointer {
            let delta = global_pos - interaction.pan_start;
            transform.offset = interaction.pan_offset_start + delta;
        }
    }

    // Clear dragging/panning if pointer button is released (even outside canvas)
    if input.primary_released() {
        interaction.dragging_node = None;
        interaction.is_panning = false;
    }
    if input.button_released(egui::PointerButton::Middle) {
        interaction.is_panning = false;
    }

    // Handle zoom (scroll wheel)
    let zoom_delta = ui.input(|i| i.zoom_delta());
    if zoom_delta != 1.0 {
        let center = rect.center();
        let old_zoom = transform.zoom;
        transform.zoom = (transform.zoom * zoom_delta).clamp(CanvasTransform::MIN_ZOOM, CanvasTransform::MAX_ZOOM);
        let actual_factor = transform.zoom / old_zoom;
        let mouse_pos = ui.input(|i| i.pointer.hover_pos()).unwrap_or(center);
        transform.offset = mouse_pos - (mouse_pos - transform.offset) * actual_factor;
    }

    // Handle keyboard - Delete selected node
    if ui.input(|i| i.key_pressed(egui::Key::Delete)) {
        if let Some(node_id) = interaction.selected_node {
            project.remove_node(node_id);
            interaction.selected_node = None;
            interaction.editing_node = None;
            result.node_deleted = true;
        }
    }

    // Draw zoom indicator
    let zoom_text = format!("Zoom: {:.0}%", transform.zoom * 100.0);
    let galley = painter.layout_no_wrap(
        zoom_text,
        FontId::monospace(12.0),
        ui.visuals().text_color(),
    );
    let galley_size = galley.size();
    painter.galley(
        rect.right_top() + vec2(-galley_size.x - 4.0, 4.0),
        galley,
        Color32::TRANSPARENT,
    );

    result
}

/// Draw the background grid
fn draw_grid(painter: &egui::Painter, rect: Rect, transform: &CanvasTransform, bg_color: Color32) {
    let bg_luminance = bg_color.to_array()[0] as f32 * 0.299
        + bg_color.to_array()[1] as f32 * 0.587
        + bg_color.to_array()[2] as f32 * 0.114;
    let grid_color = if bg_luminance < 128.0 {
        Color32::from_rgba_unmultiplied(180, 180, 180, 20)
    } else {
        Color32::from_rgba_unmultiplied(100, 100, 100, 40)
    };

    let grid_spacing = 40.0 * transform.zoom;
    if grid_spacing > 5.0 {
        let grid_stroke = Stroke::new(1.0, grid_color);

        let world_min = transform.screen_to_world(rect.min);
        let world_max = transform.screen_to_world(rect.max);

        let start_x = (world_min.x / 40.0).floor() * 40.0;
        let start_y = (world_min.y / 40.0).floor() * 40.0;
        let end_x = (world_max.x / 40.0).ceil() * 40.0;
        let end_y = (world_max.y / 40.0).ceil() * 40.0;

        let mut x = start_x;
        while x <= end_x {
            let screen_x = transform.world_to_screen(pos2(x, 0.0)).x;
            if screen_x >= rect.min.x && screen_x <= rect.max.x {
                painter.line_segment(
                    [pos2(screen_x, rect.min.y), pos2(screen_x, rect.max.y)],
                    grid_stroke,
                );
            }
            x += 40.0;
        }

        let mut y = start_y;
        while y <= end_y {
            let screen_y = transform.world_to_screen(pos2(0.0, y)).y;
            if screen_y >= rect.min.y && screen_y <= rect.max.y {
                painter.line_segment(
                    [pos2(rect.min.x, screen_y), pos2(rect.max.x, screen_y)],
                    grid_stroke,
                );
            }
            y += 40.0;
        }
    }
}
