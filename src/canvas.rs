use egui::*;

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

/// Draw the canvas with grid, pan, and zoom
pub fn canvas_ui(ui: &mut Ui, _project: &mut crate::model::TopologyProject, transform: &mut CanvasTransform) {
    let (response, painter) = ui.allocate_painter(ui.available_size(), Sense::click_and_drag());

    let rect = response.rect;
    let visuals = ui.visuals();
    let bg_color = visuals.extreme_bg_color;

    // Draw background
    painter.rect_filled(rect, 0.0, bg_color);

    // Draw grid with a faint but visible color
    // Detect dark mode by checking if the background is dark
    let bg_luminance = bg_color.to_array()[0] as f32 * 0.299
        + bg_color.to_array()[1] as f32 * 0.587
        + bg_color.to_array()[2] as f32 * 0.114;
    let grid_color = if bg_luminance < 128.0 {
        // Dark background: faint light grid lines
        Color32::from_rgba_unmultiplied(180, 180, 180, 20)
    } else {
        // Light background: faint dark grid lines
        Color32::from_rgba_unmultiplied(100, 100, 100, 40)
    };

    let grid_spacing = 40.0 * transform.zoom;
    if grid_spacing > 5.0 {
        let grid_stroke = Stroke::new(1.0, grid_color);

        // Calculate visible area in world coordinates
        let world_min = transform.screen_to_world(rect.min);
        let world_max = transform.screen_to_world(rect.max);

        let start_x = (world_min.x / 40.0).floor() * 40.0;
        let start_y = (world_min.y / 40.0).floor() * 40.0;
        let end_x = (world_max.x / 40.0).ceil() * 40.0;
        let end_y = (world_max.y / 40.0).ceil() * 40.0;

        // Draw vertical grid lines
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

        // Draw horizontal grid lines
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

    // Handle pan (left-click drag on empty space)
    if response.dragged() && response.drag_delta().length() > 0.0 {
        transform.offset += response.drag_delta();
    }

    // Handle zoom (scroll wheel)
    let zoom_delta = ui.input(|i| i.zoom_delta());
    if zoom_delta != 1.0 {
        let center = rect.center();

        let old_zoom = transform.zoom;
        transform.zoom = (transform.zoom * zoom_delta).clamp(CanvasTransform::MIN_ZOOM, CanvasTransform::MAX_ZOOM);
        let actual_factor = transform.zoom / old_zoom;

        // Adjust offset to zoom toward cursor
        let mouse_pos = ui.input(|i| i.pointer.hover_pos()).unwrap_or(center);
        transform.offset = mouse_pos - (mouse_pos - transform.offset) * actual_factor;
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
}
