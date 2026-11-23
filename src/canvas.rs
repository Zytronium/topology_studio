use egui::*;
use std::collections::HashSet;
use uuid::Uuid;
use crate::model::{Node, NodeType, TopologyProject, Link, LinkStyle};
use crate::icons;

/// Pan and zoom state for the canvas
#[derive(Clone, Copy, Debug)]
pub struct CanvasTransform {
    pub offset: Pos2,
    pub zoom: f32,
    /// The screen rectangle of the canvas area (set each frame from the UI layout).
    /// All screen-to-world conversions are relative to this rect's min corner.
    pub canvas_rect: Rect,
}

impl Default for CanvasTransform {
    fn default() -> Self {
        Self {
            offset: Pos2::ZERO,
            zoom: 1.0,
            canvas_rect: Rect::ZERO,
        }
    }
}

impl CanvasTransform {
    const MIN_ZOOM: f32 = 0.1;
    const MAX_ZOOM: f32 = 8.0;

    /// Initialize the transform so that world (0,0) is at the center of the visible canvas.
    pub fn init_with_canvas_rect(&mut self, canvas_rect: Rect) {
        self.canvas_rect = canvas_rect;
        // world_to_screen(0,0) = offset + canvas_rect.min, so offset = center - min
        // makes world (0,0) appear at the center of the canvas rect
        let v: Vec2 = canvas_rect.center().to_vec2() - canvas_rect.min.to_vec2();
        self.offset = Pos2::new(v.x, v.y);
    }

    /// Convert screen coordinates to world coordinates.
    /// Screen coordinates are relative to the entire window; the canvas_rect.min is subtracted
    /// so that (0,0) world is relative to the canvas area, not the window origin.
    pub fn screen_to_world(&self, screen: Pos2) -> Pos2 {
        let v: Vec2 = (screen.to_vec2() - self.canvas_rect.min.to_vec2() - self.offset.to_vec2()) / self.zoom;
        Pos2::new(v.x, v.y)
    }

    /// Convert world coordinates to screen coordinates.
    pub fn world_to_screen(&self, world: Pos2) -> Pos2 {
        let v: Vec2 = world.to_vec2() * self.zoom + self.offset.to_vec2() + self.canvas_rect.min.to_vec2();
        Pos2::new(v.x, v.y)
    }
}

/// Interaction state for the canvas
pub struct CanvasInteraction {
    pub selected_nodes: HashSet<Uuid>,
    pub selected_link: Option<Uuid>,
    /// The node being dragged (the one the user clicked on). All selected nodes move together.
    pub dragging_node: Option<Uuid>,
    /// Original world positions of all selected nodes at drag start (for multi-drag)
    pub drag_offsets: Vec<(Uuid, Pos2)>,
    pub is_panning: bool,
    pub pan_start: Pos2,
    pub pan_offset_start: Pos2,
    /// Node being edited via double-click; stores (node_id, current_text)
    pub editing_node: Option<(Uuid, String)>,
    /// Source node for link creation (first click in link mode)
    pub link_source: Option<Uuid>,
    /// Node ID for context menu (right-click target)
    pub context_menu_node: Option<Uuid>,
    /// Position of the context menu on screen
    pub context_menu_pos: Pos2,
    /// Whether the Add Link submenu is open (persists across frames)
    pub context_submenu_open: bool,
}

impl Default for CanvasInteraction {
    fn default() -> Self {
        Self {
            selected_nodes: HashSet::new(),
            selected_link: None,
            dragging_node: None,
            drag_offsets: Vec::new(),
            is_panning: false,
            pan_start: Pos2::ZERO,
            pan_offset_start: Pos2::ZERO,
            editing_node: None,
            link_source: None,
            context_menu_node: None,
            context_menu_pos: Pos2::ZERO,
            context_submenu_open: false,
        }
    }
}

impl CanvasInteraction {
    /// Set a single selected node, clearing others
    pub fn set_selected_node(&mut self, id: Option<Uuid>) {
        self.selected_nodes.clear();
        if let Some(node_id) = id {
            self.selected_nodes.insert(node_id);
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

    /// Find the link closest to the given screen position, returning the link ID if within hit range
    fn hit_test_link(project: &TopologyProject, screen_pos: Pos2, transform: &CanvasTransform) -> Option<Uuid> {
        let hit_radius = 8.0 / transform.zoom; // 8 pixels in screen space
        let mut closest: Option<(Uuid, f32)> = None;
        
        for link in &project.links {
            let source_node = project.get_node(link.source);
            let target_node = project.get_node(link.target);
            if let (Some(src), Some(tgt)) = (source_node, target_node) {
                let src_center = pos2(src.x as f32, src.y as f32);
                let tgt_center = pos2(tgt.x as f32, tgt.y as f32);
                
                let src_edge = node_edge_point(src, src_center, tgt_center);
                let tgt_edge = node_edge_point(tgt, tgt_center, src_center);
                
                let src_screen = transform.world_to_screen(src_edge);
                let tgt_screen = transform.world_to_screen(tgt_edge);
                
                let dist = distance_point_to_segment(screen_pos, src_screen, tgt_screen);
                if dist <= hit_radius {
                    match closest {
                        Some((_, best_dist)) if dist >= best_dist => {},
                        _ => { closest = Some((link.id, dist)); }
                    }
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
    /// True if a node was moved during the current drag session (persists after drag ends)
    pub node_moved_in_drag: bool,
    pub node_renamed: bool,
    pub link_added: bool,
    pub link_deleted: bool,
    /// Link style requested from context menu (activates link mode in app.rs)
    pub context_link_style: Option<LinkStyle>,
    /// True if nodes were nudged by keyboard (for undo batching)
    pub nodes_nudged: bool,
}

/// Context menu action selected by the user
enum ContextMenuAction {
    Rename,
    AddLink(LinkStyle),
    Delete,
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
    link_mode: bool,
    link_style: Option<LinkStyle>,
    snap_to_grid: bool,
) -> CanvasResult {
    let mut result = CanvasResult {
        node_added: false,
        node_deleted: false,
        node_moved: false,
        node_moved_in_drag: false,
        node_renamed: false,
        link_added: false,
        link_deleted: false,
        context_link_style: None,
        nodes_nudged: false,
    };

    let (response, painter) = ui.allocate_painter(ui.available_size(), Sense::click_and_drag());
    let rect = response.rect;
    let visuals = ui.visuals();
    let bg_color = visuals.extreme_bg_color;

    // Initialize transform with canvas rect on first frame (when canvas_rect is still Rect::ZERO)
    // On resize, adjust the offset so the visual center stays locked in place
    let old_canvas_rect = transform.canvas_rect;
    if transform.canvas_rect == Rect::ZERO {
        transform.init_with_canvas_rect(rect);
    } else {
        // Window resized: adjust offset so the center of the visible canvas stays fixed
        let center_delta = rect.center() - old_canvas_rect.center();
        transform.offset = transform.offset + center_delta;
        transform.canvas_rect = rect;
    }

    // Draw background
    painter.rect_filled(rect, 0.0, bg_color);

    // Draw grid
    draw_grid(&painter, rect, transform, bg_color);

    // === LINK RENDERING === (before nodes so node backgrounds cover link endpoints)
    draw_links(&painter, project, transform, interaction, visuals, link_mode);

    // === NODE RENDERING ===
    for node in &project.nodes {
        let center = transform.world_to_screen(pos2(node.x as f32, node.y as f32));
        let icon_size = icons::ICON_SIZE * transform.zoom;
        let selected = interaction.selected_nodes.contains(&node.id);
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
                    let frame = egui::Frame::NONE
                        .fill(visuals.extreme_bg_color)
                        .stroke(Stroke::new(1.5_f32, visuals.selection.stroke.color))
                        .corner_radius(3.0);
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

    // === RIGHT-CLICK CONTEXT MENU ===
    let mut context_action: Option<(Uuid, ContextMenuAction)> = None;
    if let Some(menu_node_id) = interaction.context_menu_node {
        let ctx = ui.ctx().clone();
        let menu_pos = interaction.context_menu_pos;
        let mut action: Option<ContextMenuAction> = None;
        let mut addlink_rect: Option<Rect> = None;
        let mut submenu_rect: Option<Rect> = None;

        // Menu styling
        let menu_bg = visuals.extreme_bg_color;
        let menu_text = visuals.text_color();
        // Compute a hover color that contrasts with the background
        let is_dark = menu_bg.to_array().iter().map(|c| *c as u16).sum::<u16>() < 384;
        let menu_hover_bg = if is_dark {
            Color32::from_rgba_premultiplied(80, 80, 100, 100)
        } else {
            Color32::from_rgba_premultiplied(60, 60, 80, 60)
        };
        let area_response = egui::Area::new(egui::Id::new("context_menu"))
            .order(egui::Order::Foreground)
            .constrain(true)
            .fixed_pos(menu_pos)
            .show(&ctx, |ui| {
                let frame = egui::Frame::NONE
                    .fill(menu_bg)
                    .stroke(Stroke::new(1.0_f32, menu_text))
                    .corner_radius(4.0)
                    .inner_margin(2.0);
                frame.show(ui, |ui| {
                    // Helper: create a transparent menu item that fits its text
                    // `extra_width` adds padding (e.g. for the chevron on "Add Link")
                    // Returns (rect, clicked, hovered)
                    let menu_item = |ui: &mut egui::Ui, text: &str, extra_width: f32| -> (Rect, bool, bool) {
                        let galley = ui.painter().layout_no_wrap(
                            text.to_string(),
                            FontId::proportional(14.0),
                            menu_text,
                        );
                        let height = galley.size().y + 8.0;
                        let width = galley.size().x + 8.0 + extra_width;
                        let (rect, response) = ui.allocate_at_least(
                            vec2(width, height),
                            egui::Sense::click(),
                        );
                        ui.painter().galley(rect.min + vec2(4.0, (height - galley.size().y) / 2.0), galley, Color32::TRANSPARENT);
                        (rect, response.clicked(), response.hovered())
                    };

                    // Rename
                    let (rename_rect, rename_clicked, rename_hovered) = menu_item(ui, "Rename", 0.0);
                    if rename_clicked {
                        action = Some(ContextMenuAction::Rename);
                    }
                    if rename_hovered {
                        ui.painter().rect_filled(rename_rect, 2.0, menu_hover_bg);
                    }

                    // Add Link with submenu on hover (extra padding for chevron)
                    let (addlink_rect_local, _addlink_clicked, addlink_hovered) = menu_item(ui, "Add Link", 24.0);
                    addlink_rect = Some(addlink_rect_local);

                    // Draw chevron arrow on the right side of "Add Link"
                    {
                        let painter = ui.painter();
                        let chevron_x = addlink_rect_local.right() - 10.0;
                        let chevron_y = addlink_rect_local.center().y;
                        let size = 4.0;
                        painter.line(
                            vec![
                                pos2(chevron_x - size, chevron_y - size),
                                pos2(chevron_x, chevron_y),
                                pos2(chevron_x - size, chevron_y + size),
                            ],
                            Stroke::new(1.0_f32, menu_text),
                        );
                    }

                    if addlink_hovered {
                        ui.painter().rect_filled(addlink_rect_local, 2.0, menu_hover_bg);
                    }

                    // Show submenu if hovering over "Add Link" or the submenu itself
                    let show_submenu = addlink_hovered || interaction.context_submenu_open;
                    if show_submenu {
                        let submenu_pos = addlink_rect_local.right_top();
                        let submenu_id = egui::Id::new("link_submenu");
                        let submenu_response = egui::Area::new(submenu_id)
                            .order(egui::Order::Foreground)
                            .constrain(true)
                            .fixed_pos(submenu_pos)
                            .show(&ctx, |ui| {
                                let frame2 = egui::Frame::NONE
                                    .fill(menu_bg)
                                    .stroke(Stroke::new(1.0_f32, menu_text))
                                    .corner_radius(4.0)
                                    .inner_margin(2.0);
                                frame2.show(ui, |ui| {
                                    let (eth_rect, eth_clicked, eth_hovered) = menu_item(ui, "Ethernet", 0.0);
                                    if eth_clicked {
                                        action = Some(ContextMenuAction::AddLink(LinkStyle::Solid));
                                    }
                                    if eth_hovered {
                                        ui.painter().rect_filled(eth_rect, 2.0, menu_hover_bg);
                                    }

                                    let (wire_rect, wire_clicked, wire_hovered) = menu_item(ui, "Wireless", 0.0);
                                    if wire_clicked {
                                        action = Some(ContextMenuAction::AddLink(LinkStyle::Dashed));
                                    }
                                    if wire_hovered {
                                        ui.painter().rect_filled(wire_rect, 2.0, menu_hover_bg);
                                    }
                                });
                            });
                        submenu_rect = Some(submenu_response.response.rect);
                    }

                    // Delete
                    let (delete_rect, delete_clicked, delete_hovered) = menu_item(ui, "Delete", 0.0);
                    if delete_clicked {
                        action = Some(ContextMenuAction::Delete);
                    }
                    if delete_hovered {
                        ui.painter().rect_filled(delete_rect, 2.0, menu_hover_bg);
                    }
                });
            });

        // Update submenu open state based on hover over "Add Link" or the submenu
        let pointer_hover = ui.ctx().input(|i| i.pointer.hover_pos());
        let hovering_addlink = addlink_rect
            .and_then(|r| pointer_hover.map(|p| r.contains(p)))
            .unwrap_or(false);
        let hovering_submenu = submenu_rect
            .and_then(|r| pointer_hover.map(|p| r.contains(p)))
            .unwrap_or(false);
        interaction.context_submenu_open = hovering_addlink || hovering_submenu;

        // Close menu if Escape is pressed
        if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
            interaction.context_menu_node = None;
            interaction.context_submenu_open = false;
        }

        // Close menu if clicking outside the menu area (including submenu)
        if input.primary_clicked() {
            if let Some(click_pos) = input.hover_pos() {
                let in_main = area_response.response.rect.contains(click_pos);
                let in_submenu = submenu_rect.map_or(false, |r| r.contains(click_pos));
                if !in_main && !in_submenu {
                    interaction.context_menu_node = None;
                    interaction.context_submenu_open = false;
                }
            }
        }

        if let Some(a) = action {
            context_action = Some((menu_node_id, a));
        }
    }

    // Handle context menu actions
    if let Some((node_id, action)) = context_action {
        interaction.context_menu_node = None;
        match action {
            ContextMenuAction::Rename => {
                if let Some(node) = project.get_node(node_id) {
                    interaction.editing_node = Some((node_id, node.label.clone()));
                    interaction.set_selected_node(Some(node_id));
                }
            }
            ContextMenuAction::AddLink(style) => {
                interaction.set_selected_node(Some(node_id));
                interaction.link_source = Some(node_id);
                result.context_link_style = Some(style);
            }
            ContextMenuAction::Delete => {
                project.remove_node(node_id);
                interaction.selected_nodes.remove(&node_id);
                interaction.editing_node = None;
                result.node_deleted = true;
            }
        }
    }

    // Handle palette drag placement
    if palette_drag_ended {
        if let Some(node_type) = palette_drag_type {
            if let Some(pointer_pos) = ui.input(|i| i.pointer.hover_pos()) {
                if rect.contains(pointer_pos) {
                    let world_pos = transform.screen_to_world(pointer_pos);
                    let (snap_x, snap_y) = if snap_to_grid {
                        snap_to_grid_pos(world_pos.x as f64, world_pos.y as f64)
                    } else {
                        (world_pos.x as f64, world_pos.y as f64)
                    };
                    let node = Node::new(node_type, snap_x, snap_y);
                    let node_id = node.id;
                    project.add_node(node);
                    interaction.set_selected_node(Some(node_id));
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

        // Secondary (right) button clicked - show context menu on node
        if input.secondary_clicked() {
            let world_pos = transform.screen_to_world(pointer_pos);
            if let Some(node_id) = CanvasInteraction::hit_test_node(project, world_pos, transform) {
                interaction.context_menu_node = Some(node_id);
                interaction.context_menu_pos = pointer_pos;
                interaction.set_selected_node(Some(node_id));
            } else {
                interaction.context_menu_node = None;
            }
        }

        // Primary button clicked (edge-triggered, fires once per click)
        if input.primary_clicked() {
            let ctrl_held = ui.input(|i| i.modifiers.ctrl);
            // Check if we're on a node
            if let Some(node_id) = CanvasInteraction::hit_test_node(project, transform.screen_to_world(pointer_pos), transform) {
                if link_mode {
                    // Link creation mode
                    if interaction.link_source.is_none() {
                        // First click: set source
                        interaction.link_source = Some(node_id);
                        interaction.set_selected_node(Some(node_id));
                    } else if interaction.link_source != Some(node_id) {
                        // Second click: create link
                        let source_id = interaction.link_source.unwrap();
                        let style = link_style.unwrap_or(LinkStyle::Solid);
                        // Check if link already exists
                        let already_exists = project.links.iter().any(|l|
                            (l.source == source_id && l.target == node_id) ||
                            (l.source == node_id && l.target == source_id)
                        );
                        if !already_exists {
                            let link = Link::new(source_id, node_id, style);
                            let link_id = link.id;
                            project.add_link(link);
                            interaction.selected_link = Some(link_id);
                            result.link_added = true;
                            // Reset link source after successful link creation
                            interaction.link_source = None;
                        }
                        // If link already exists, stay in link mode so the user can click a different target
                    }
                    // If clicking the same node as source, do nothing
                } else if response.double_clicked() {
                    // Double-click: start editing the node's label
                    if let Some(node) = project.get_node(node_id) {
                        interaction.editing_node = Some((node_id, node.label.clone()));
                        interaction.set_selected_node(Some(node_id));
                        interaction.selected_link = None;
                        interaction.dragging_node = None;
                    }
                } else if ctrl_held {
                    // Ctrl+click: toggle this node in the selection set
                    if interaction.selected_nodes.contains(&node_id) {
                        interaction.selected_nodes.remove(&node_id);
                    } else {
                        interaction.selected_nodes.insert(node_id);
                    }
                    interaction.selected_link = None;
                } else {
                    // Single click: select only this node
                    interaction.set_selected_node(Some(node_id));
                    interaction.selected_link = None;
                    // Don't start dragging yet — wait for drag delta to confirm it's not a double-click
                }
            } else if space_held {
                // Space+left click: start panning
                interaction.is_panning = true;
                interaction.pan_start = pointer_pos;
                interaction.pan_offset_start = transform.offset;
            } else {
                // Clicked on empty space - check for link hit first
                if let Some(link_id) = CanvasInteraction::hit_test_link(project, pointer_pos, transform) {
                    if link_mode {
                        // In link mode, don't select links, just deselect
                        interaction.selected_link = None;
                    } else {
                        interaction.selected_link = Some(link_id);
                        interaction.selected_nodes.clear();
                    }
                } else {
                    interaction.selected_nodes.clear();
                    interaction.selected_link = None;
                    interaction.editing_node = None;

                    // Place node from palette selection (single click placement)
                    if let Some(node_type) = palette_selected_type {
                        let world_pos = transform.screen_to_world(pointer_pos);
                        let (snap_x, snap_y) = if snap_to_grid {
                            snap_to_grid_pos(world_pos.x as f64, world_pos.y as f64)
                        } else {
                            (world_pos.x as f64, world_pos.y as f64)
                        };
                        let node = Node::new(node_type, snap_x, snap_y);
                        let node_id = node.id;
                        project.add_node(node);
                        interaction.set_selected_node(Some(node_id));
                        result.node_added = true;
                    }
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
                            // If clicking a node that's not in the selection, select it alone
                            if !interaction.selected_nodes.contains(&node_id) {
                                interaction.set_selected_node(Some(node_id));
                            }
                            // Record original positions of ALL selected nodes for multi-drag
                            interaction.drag_offsets = interaction.selected_nodes.iter().map(|id| {
                                if let Some(n) = project.get_node(*id) {
                                    (*id, pos2(n.x as f32, n.y as f32))
                                } else {
                                    (*id, Pos2::ZERO)
                                }
                            }).collect();
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
            interaction.is_panning = false;
        }

        // Middle button released - stop panning
        if input.button_released(egui::PointerButton::Middle) {
            interaction.is_panning = false;
        }
    }

    // --- DRAG NODE: works even when pointer leaves canvas ---
    // Move ALL selected nodes together, not just the one clicked
    if let Some(_dragging_id) = interaction.dragging_node {
        if let Some(global_pos) = global_pointer {
            let world_pos = transform.screen_to_world(global_pos);
            // Calculate the delta from the dragged node's original position
            let dragged_node_id = _dragging_id;
            let original_pos = interaction.drag_offsets.iter()
                .find(|(id, _)| *id == dragged_node_id)
                .map(|(_, p)| *p)
                .unwrap_or(world_pos);
            let delta = world_pos - original_pos;

            // Move all selected nodes by the same delta
            for (node_id, orig_pos) in &interaction.drag_offsets {
                if let Some(node) = project.get_node_mut(*node_id) {
                    let new_x = orig_pos.x + delta.x;
                    let new_y = orig_pos.y + delta.y;
                    let (x, y) = if snap_to_grid {
                        snap_to_grid_pos(new_x as f64, new_y as f64)
                    } else {
                        (new_x as f64, new_y as f64)
                    };
                    node.x = x;
                    node.y = y;
                }
            }
            result.node_moved = true;
            result.node_moved_in_drag = true;
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
    // This must come AFTER the drag block so the drag block runs one last time
    if input.primary_released() {
        interaction.dragging_node = None;
        interaction.drag_offsets.clear();
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

    // Handle keyboard - Delete selected node(s) or link
    if ui.input(|i| i.key_pressed(egui::Key::Delete)) {
        if link_mode {
            // Exit link mode on Delete
            interaction.link_source = None;
        } else if let Some(link_id) = interaction.selected_link {
            project.remove_link(link_id);
            interaction.selected_link = None;
            result.link_deleted = true;
        } else if !interaction.selected_nodes.is_empty() {
            // Delete all selected nodes
            let ids_to_remove: Vec<Uuid> = interaction.selected_nodes.iter().copied().collect();
            for node_id in &ids_to_remove {
                project.remove_node(*node_id);
            }
            interaction.selected_nodes.clear();
            interaction.editing_node = None;
            result.node_deleted = true;
        }
    }

    // Handle keyboard - Escape to cancel link mode or selection
    if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
        if link_mode {
            interaction.link_source = None;
        } else {
            interaction.selected_nodes.clear();
            interaction.selected_link = None;
        }
    }

    // Handle keyboard - Arrow keys for panning and Ctrl+Arrow for nudging
    let dt = ui.input(|i| i.stable_dt);
    let pan_speed = 200.0 * dt; // pixels per second
    let ctrl_held_kb = ui.input(|i| i.modifiers.ctrl);
    let shift_held_kb = ui.input(|i| i.modifiers.shift);
    let nudge_amount = if shift_held_kb { 10.0 } else { 1.0 };

    // Check arrow key states
    let arrow_left = ui.input(|i| i.key_down(egui::Key::ArrowLeft));
    let arrow_right = ui.input(|i| i.key_down(egui::Key::ArrowRight));
    let arrow_up = ui.input(|i| i.key_down(egui::Key::ArrowUp));
    let arrow_down = ui.input(|i| i.key_down(egui::Key::ArrowDown));

    if ctrl_held_kb && !interaction.selected_nodes.is_empty() {
        // Ctrl+Arrow: nudge selected nodes
        if arrow_left || arrow_right || arrow_up || arrow_down {
            let dx = if arrow_right { nudge_amount } else if arrow_left { -nudge_amount } else { 0.0 };
            let dy = if arrow_down { nudge_amount } else if arrow_up { -nudge_amount } else { 0.0 };
            for node_id in &interaction.selected_nodes {
                if let Some(node) = project.get_node_mut(*node_id) {
                    node.x += dx;
                    node.y += dy;
                }
            }
            result.nodes_nudged = true;
        }
    } else if !ctrl_held_kb {
        // Arrow keys without Ctrl: pan the canvas
        if arrow_left || arrow_right || arrow_up || arrow_down {
            let pan_dx = if arrow_right { -pan_speed } else if arrow_left { pan_speed } else { 0.0 };
            let pan_dy = if arrow_down { -pan_speed } else if arrow_up { pan_speed } else { 0.0 };
            transform.offset = transform.offset + vec2(pan_dx, pan_dy);
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

    // Draw link mode indicator
    if link_mode {
        let mode_text = if interaction.link_source.is_some() {
            "Click another node to complete the link"
        } else {
            "Link mode: Click a node to start"
        };
        let galley = painter.layout_no_wrap(
            mode_text.to_string(),
            FontId::proportional(13.0),
            ui.visuals().text_color(),
        );
        let galley_size = galley.size();
        painter.galley(
            rect.left_top() + vec2(8.0, 8.0 + galley_size.y),
            galley,
            Color32::TRANSPARENT,
        );
    }

    result
}

/// Draw all links on the canvas
fn draw_links(
    painter: &egui::Painter,
    project: &TopologyProject,
    transform: &CanvasTransform,
    interaction: &CanvasInteraction,
    visuals: &egui::Visuals,
    link_mode: bool,
) {
    // Determine link colors based on theme
    let bg_luminance = visuals.extreme_bg_color.to_array()[0] as f32 * 0.299
        + visuals.extreme_bg_color.to_array()[1] as f32 * 0.587
        + visuals.extreme_bg_color.to_array()[2] as f32 * 0.114;
    let is_dark = bg_luminance < 128.0;
    let link_color = if is_dark {
        Color32::from_rgba_unmultiplied(180, 180, 200, 255)
    } else {
        Color32::from_rgba_unmultiplied(80, 80, 100, 255)
    };
    let selected_link_color = visuals.selection.stroke.color;

    for link in &project.links {
        let source_node = project.get_node(link.source);
        let target_node = project.get_node(link.target);
        if let (Some(src), Some(tgt)) = (source_node, target_node) {
            // Calculate edge points for both nodes
            let src_center = pos2(src.x as f32, src.y as f32);
            let tgt_center = pos2(tgt.x as f32, tgt.y as f32);
            
            let src_edge = node_edge_point(src, src_center, tgt_center);
            let tgt_edge = node_edge_point(tgt, tgt_center, src_center);
            
            // Transform to screen coordinates
            let src_screen = transform.world_to_screen(src_edge);
            let tgt_screen = transform.world_to_screen(tgt_edge);
            
            let is_selected = interaction.selected_link == Some(link.id);
            let stroke_color = if is_selected { selected_link_color } else { link_color };
            let stroke_width = if is_selected { 2.5_f32 } else { 1.5_f32 };
            
            match link.style {
                LinkStyle::Solid => {
                    painter.line_segment(
                        [src_screen, tgt_screen],
                        Stroke::new(stroke_width, stroke_color),
                    );
                }
                LinkStyle::Dashed => {
                    // Draw dashed line
                    let direction = tgt_screen - src_screen;
                    let length = direction.length();
                    if length > 0.0 {
                        let dir = direction / length;
                        let dash_length = 8.0 * transform.zoom;
                        let gap_length = 4.0 * transform.zoom;
                        let mut pos = src_screen;
                        let mut in_dash = true;
                        let mut seg_start = pos;
                        
                        while (pos - src_screen).length() < length {
                            let step = if in_dash { dash_length } else { gap_length };
                            let next = pos + dir * step;
                            // Check if we've gone past the target
                            let t = (next - src_screen).length() / length;
                            if t > 1.0 {
                                // Draw final segment to target
                                if in_dash {
                                    painter.line_segment(
                                        [seg_start, tgt_screen],
                                        Stroke::new(stroke_width, stroke_color),
                                    );
                                }
                                break;
                            }
                            if in_dash {
                                painter.line_segment(
                                    [seg_start, next],
                                    Stroke::new(stroke_width, stroke_color),
                                );
                            }
                            seg_start = next;
                            pos = next;
                            in_dash = !in_dash;
                        }
                    }
                }
            }
        }
    }

    // Draw preview line while creating a link (from source to cursor)
    if link_mode && interaction.link_source.is_some() {
        if let Some(source_id) = interaction.link_source {
            if let Some(source_node) = project.get_node(source_id) {
                let source_center = pos2(source_node.x as f32, source_node.y as f32);
                if let Some(pointer_pos) = painter.ctx().input(|i| i.pointer.hover_pos()) {
                    let world_pos = transform.screen_to_world(pointer_pos);
                    let src_edge = node_edge_point(source_node, source_center, world_pos);
                    let src_screen = transform.world_to_screen(src_edge);
                    
                    painter.line_segment(
                        [src_screen, pointer_pos],
                        Stroke::new(1.5_f32, Color32::from_rgba_unmultiplied(100, 150, 255, 180)),
                    );
                }
            }
        }
    }
}

/// Calculate the distance from a point to a line segment
fn distance_point_to_segment(p: Pos2, a: Pos2, b: Pos2) -> f32 {
    let ab = b - a;
    let ap = p - a;
    let len_sq = ab.dot(ab);
    if len_sq < 0.0001 {
        return ap.length();
    }
    let t = (ap.dot(ab) / len_sq).clamp(0.0, 1.0);
    let projection = a + ab * t;
    (p - projection).length()
}

/// Calculate the point on a node's edge where a line from the node center toward
/// the target direction intersects the node's bounding box.
fn node_edge_point(node: &Node, center: Pos2, toward: Pos2) -> Pos2 {
    let (node_w, node_h) = node.size();
    let half_w = (node_w / 2.0) as f32;
    let half_h = (node_h / 2.0) as f32;
    
    let direction = toward - center;
    
    // Handle edge case: same position
    if direction.length() < 0.001 {
        return center;
    }
    
    let dx = direction.x;
    let dy = direction.y;
    
    // Calculate intersection with the bounding box edges
    let mut t = f32::MAX;
    
    if dx.abs() > 0.001 {
        let t_x = if dx > 0.0 { half_w / dx } else { -half_w / dx };
        if t_x > 0.0 && t_x < t {
            t = t_x;
        }
    }
    if dy.abs() > 0.001 {
        let t_y = if dy > 0.0 { half_h / dy } else { -half_h / dy };
        if t_y > 0.0 && t_y < t {
            t = t_y;
        }
    }
    
    if t == f32::MAX {
        center
    } else {
        center + direction.normalized() * t
    }
}

/// Snap a world position to the nearest grid intersection.
/// Grid spacing is 20 pixels.
fn snap_to_grid_pos(x: f64, y: f64) -> (f64, f64) {
    let grid = 20.0;
    let sx = (x / grid).round() * grid;
    let sy = (y / grid).round() * grid;
    (sx, sy)
}

impl CanvasTransform {
    /// Reset zoom to 100% and center the view on (0,0).
    pub fn reset_zoom(&mut self) {
        self.zoom = 1.0;
        let v: Vec2 = self.canvas_rect.center().to_vec2() - self.canvas_rect.min.to_vec2();
        self.offset = Pos2::new(v.x, v.y);
    }

    /// Zoom to fit all nodes with the given padding.
    pub fn zoom_to_fit(&mut self, project: &TopologyProject, padding: f32) {
        if project.nodes.is_empty() {
            self.reset_zoom();
            return;
        }

        let mut min_x = f64::MAX;
        let mut min_y = f64::MAX;
        let mut max_x = f64::MIN;
        let mut max_y = f64::MIN;

        for node in &project.nodes {
            let (w, h) = node.size();
            min_x = min_x.min(node.x - w / 2.0);
            min_y = min_y.min(node.y - h / 2.0);
            max_x = max_x.max(node.x + w / 2.0);
            max_y = max_y.max(node.y + h / 2.0);
        }

        let content_w = (max_x - min_x) as f32 + padding * 2.0;
        let content_h = (max_y - min_y) as f32 + padding * 2.0;
        let canvas_w = self.canvas_rect.width();
        let canvas_h = self.canvas_rect.height();

        let zoom_x = canvas_w / content_w;
        let zoom_y = canvas_h / content_h;
        self.zoom = (zoom_x.min(zoom_y)).clamp(Self::MIN_ZOOM, Self::MAX_ZOOM);

        let center_x = ((min_x + max_x) / 2.0) as f32;
        let center_y = ((min_y + max_y) / 2.0) as f32;
        let screen_center = self.canvas_rect.center();
        // offset + canvas_rect.min = screen_center - center * zoom
        let v: Vec2 = screen_center.to_vec2() - self.canvas_rect.min.to_vec2() - vec2(center_x, center_y) * self.zoom;
        self.offset = Pos2::new(v.x, v.y);
    }
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
        let grid_stroke = Stroke::new(1.0_f32, grid_color);

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
