use egui::*;
use crate::model::{NodeType, LinkStyle};
use crate::icons;

/// Icon size for palette buttons
const PALETTE_ICON_SIZE: f32 = 24.0;

/// State for the palette component
pub struct PaletteState {
    /// Currently selected node type for placement (click palette, then click canvas)
    pub selected_type: Option<NodeType>,
    /// Dragging from palette to canvas
    pub dragging_type: Option<NodeType>,
    /// Currently selected link style for link creation (None = not in link mode)
    pub selected_link_style: Option<LinkStyle>,
}

impl Default for PaletteState {
    fn default() -> Self {
        Self {
            selected_type: None,
            dragging_type: None,
            selected_link_style: None,
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

/// Draw the links palette section (link creation tools)
pub fn links_ui(ui: &mut Ui, selected_style: &mut Option<LinkStyle>) {
    ui.vertical_centered(|ui| {
        ui.heading("Links");
        ui.separator();
        ui.add_space(4.0);
    });

    // Ethernet (Solid)
    let is_ethernet = *selected_style == Some(LinkStyle::Solid);
    let eth_color = if is_ethernet {
        ui.visuals().selection.stroke.color
    } else {
        ui.visuals().text_color()
    };
    let row_height = 40.0;
    let available = ui.available_size();
    let (row_rect, eth_response) = ui.allocate_at_least(vec2(available.x, row_height), Sense::click());

    if is_ethernet {
        ui.painter().rect_filled(row_rect.shrink(1.0), 2.0, ui.visuals().selection.bg_fill);
    }

    // Draw solid line icon
    let icon_x = row_rect.min.x + 16.0;
    let icon_y = row_rect.center().y;
    let line_len = 20.0;
    ui.painter().line_segment(
        [pos2(icon_x - line_len / 2.0, icon_y), pos2(icon_x + line_len / 2.0, icon_y)],
        Stroke::new(2.5, eth_color),
    );

    let label_x = row_rect.min.x + 16.0 + 16.0 + 4.0;
    let label_y = row_rect.center().y - 8.0;
    ui.painter().text(
        pos2(label_x, label_y),
        Align2::LEFT_TOP,
        "Ethernet",
        FontId::proportional(14.0),
        eth_color,
    );

    if eth_response.clicked() {
        *selected_style = if is_ethernet { None } else { Some(LinkStyle::Solid) };
    }

    ui.add_space(2.0);

    // Wireless (Dashed)
    let is_wireless = *selected_style == Some(LinkStyle::Dashed);
    let wl_color = if is_wireless {
        ui.visuals().selection.stroke.color
    } else {
        ui.visuals().text_color()
    };
    let (row_rect, wl_response) = ui.allocate_at_least(vec2(available.x, row_height), Sense::click());

    if is_wireless {
        ui.painter().rect_filled(row_rect.shrink(1.0), 2.0, ui.visuals().selection.bg_fill);
    }

    // Draw dashed line icon
    let icon_x = row_rect.min.x + 16.0;
    let icon_y = row_rect.center().y;
    let dash_len = 6.0;
    ui.painter().line_segment(
        [pos2(icon_x - 10.0, icon_y), pos2(icon_x - 4.0, icon_y)],
        Stroke::new(2.5, wl_color),
    );
    ui.painter().line_segment(
        [pos2(icon_x + 2.0, icon_y), pos2(icon_x + 8.0, icon_y)],
        Stroke::new(2.5, wl_color),
    );

    let label_x = row_rect.min.x + 16.0 + 16.0 + 4.0;
    let label_y = row_rect.center().y - 8.0;
    ui.painter().text(
        pos2(label_x, label_y),
        Align2::LEFT_TOP,
        "Wireless",
        FontId::proportional(14.0),
        wl_color,
    );

    if wl_response.clicked() {
        *selected_style = if is_wireless { None } else { Some(LinkStyle::Dashed) };
    }

    ui.add_space(2.0);
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

    /// Check if link mode is active
    pub fn is_link_mode(&self) -> bool {
        self.selected_link_style.is_some()
    }

    /// Get the current link style
    pub fn link_style(&self) -> Option<LinkStyle> {
        self.selected_link_style
    }
}
