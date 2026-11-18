use egui::*;
use crate::model::NodeType;
use crate::icons;

/// Icon size for palette buttons
const PALETTE_ICON_SIZE: f32 = 24.0;

/// State for the palette component
pub struct PaletteState {
    /// Currently selected node type for placement (click palette, then click canvas)
    pub selected_type: Option<NodeType>,
    /// Dragging from palette to canvas
    pub dragging_type: Option<NodeType>,
}

impl Default for PaletteState {
    fn default() -> Self {
        Self {
            selected_type: None,
            dragging_type: None,
        }
    }
}

/// Draw the device palette panel
pub fn palette_ui(ui: &mut Ui, state: &mut PaletteState) {
    ui.vertical_centered(|ui| {
        ui.heading("Devices");
        ui.separator();
        ui.add_space(4.0);
    });

    for node_type in icons::all_node_types() {
        let is_selected = state.selected_type == Some(*node_type);

        // Pre-compute the label color (copy the Color32 value, not a reference)
        let label_color = if is_selected {
            ui.visuals().selection.stroke.color
        } else {
            ui.visuals().text_color()
        };

        // Allocate a clickable row area with click_and_drag sense
        let row_height = 44.0;
        let available = ui.available_size();
        let (row_rect, response) = ui.allocate_at_least(vec2(available.x, row_height), Sense::click_and_drag());

        // Draw the row background if selected
        if is_selected {
            ui.painter().rect_filled(row_rect.shrink(1.0), 2.0, ui.visuals().selection.bg_fill);
        }

        // Draw icon and label inside the allocated area
        let icon_center = pos2(row_rect.min.x + 16.0, row_rect.center().y);
        icons::draw_node_icon(ui.painter(), *node_type, icon_center, PALETTE_ICON_SIZE, is_selected, ui.visuals());

        let label_x = row_rect.min.x + 16.0 + PALETTE_ICON_SIZE + 4.0;
        let label_y = row_rect.center().y - 8.0;
        ui.painter().text(
            pos2(label_x, label_y),
            egui::Align2::LEFT_TOP,
            node_type.label(),
            FontId::proportional(14.0),
            label_color,
        );

        if response.clicked() {
            state.selected_type = if is_selected { None } else { Some(*node_type) };
        }

        // Start drag from palette
        if response.drag_started() {
            state.dragging_type = Some(*node_type);
        }

        ui.add_space(2.0);
    }

    ui.add_space(8.0);
    ui.separator();
    ui.add_space(4.0);

    // Instructions
    ui.strong("Instructions:");
    ui.add_space(2.0);
    ui.label("1. Click a device, then click the canvas to place it.");
    ui.label("2. Or drag a device from here to the canvas.");
    ui.add_space(8.0);
    ui.label("Delete: Select a node and press Delete key.");
}

impl PaletteState {
    /// Check if we're currently dragging from the palette
    pub fn is_dragging(&self) -> bool {
        self.dragging_type.is_some()
    }

    /// Get the current drag type without consuming it
    pub fn drag_type(&self) -> Option<NodeType> {
        self.dragging_type
    }
}
