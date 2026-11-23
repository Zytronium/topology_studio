use crate::model::NodeType;
use egui::*;
use std::collections::HashMap;
use std::sync::OnceLock;

/// Node icon size in world coordinates
pub const ICON_SIZE: f32 = 40.0;
/// Node bounding box size for hit testing (includes padding around icon)
pub const NODE_HIT_SIZE: f32 = 60.0;

// ---------------------------------------------------------------------------
// Inline SVG strings — copied exactly from the assets/ directory
// ---------------------------------------------------------------------------

const SVG_ROUTER: &str = r##"<svg fill="#000000" viewBox="0 0 36 36" xmlns="http://www.w3.org/2000/svg">
<path d="M18,1.67a16,16,0,1,0,16,16A16,16,0,0,0,18,1.67ZM13.86,9.92a.8.8,0,0,1,1.13,0l2.21,2.19V5.93a.8.8,0,0,1,1.6,0v6.18L21,9.92a.8.8,0,1,1,1.13,1.14L18,15.15l-4.14-4.1A.8.8,0,0,1,13.86,9.92ZM10.32,21.74a.8.8,0,0,1-1.13,0L5,17.67l4.19-4.09a.8.8,0,1,1,1.12,1.14l-2.2,2.14h6.27a.8.8,0,0,1,0,1.6H8.11l2.2,2.15A.8.8,0,0,1,10.32,21.74Zm11.82,3.67a.8.8,0,0,1-1.13,0L18.8,23.23V29.4a.8.8,0,0,1-1.6,0V23.23L15,25.42a.8.8,0,1,1-1.13-1.14L18,20.18l4.14,4.1A.8.8,0,0,1,22.14,25.41Zm4.67-3.66a.8.8,0,1,1-1.12-1.14l2.2-2.15H21.63a.8.8,0,0,1,0-1.6h6.27l-2.2-2.14a.8.8,0,1,1,1.12-1.14L31,17.67Z"/>
</svg>"##;

const SVG_SWITCH: &str = r##"<svg fill="#000000" viewBox="0 0 36 36" xmlns="http://www.w3.org/2000/svg">
<path d="M33.91,18.47,30.78,8.41A2,2,0,0,0,28.87,7H7.13A2,2,0,0,0,5.22,8.41L2.09,18.48a2,2,0,0,0-.09.59V27a2,2,0,0,0,2,2H32a2,2,0,0,0,2-2V19.06A2,2,0,0,0,33.91,18.47ZM8.92,25H7.12V22h1.8Zm5,0h-1.8V22h1.8Zm5,0h-1.8V22h1.8Zm5,0H22.1V22h1.8Zm5,0H27.1V22h1.8ZM31,19.4H5V18H31Z"/>
</svg>"##;

const SVG_MLS: &str = r##"<svg viewBox="0 0 36 36" xmlns="http://www.w3.org/2000/svg" fill="#000000">
<path d="M18 2 L32 9 V27 L18 34 L4 27 V9 Z M18 5 L7 10.5 V25.5 L18 31 L29 25.5 V10.5 Z" fill-rule="evenodd"/>
<circle cx="18" cy="18" r="3.2"/>
<path id="arrow" d="M17.3 15 V11.4 H15.9 L18 8.2 L20.1 11.4 H18.7 V15 Z"/>
<use href="#arrow" transform="rotate(0 18 18)"/>
<use href="#arrow" transform="rotate(45 18 18)"/>
<use href="#arrow" transform="rotate(90 18 18)"/>
<use href="#arrow" transform="rotate(135 18 18)"/>
<use href="#arrow" transform="rotate(180 18 18)"/>
<use href="#arrow" transform="rotate(225 18 18)"/>
<use href="#arrow" transform="rotate(270 18 18)"/>
<use href="#arrow" transform="rotate(315 18 18)"/>
</svg>"##;

const SVG_PC: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 512 512">
<path fill="#000000" d="M477.984,39.203H34.016C15.219,39.203,0,54.438,0,73.219v283.969c0,18.781,15.219,34.016,34.016,34.016H220 v7.578c0,18.781-15.219,34.016-34.016,34.016H136v40h73.188h93.625H376v-40h-49.984c-18.797,0-34.016-15.234-34.016-34.016v-7.578 h185.984c18.797,0,34.016-15.234,34.016-34.016V73.219C512,54.438,496.781,39.203,477.984,39.203z M464,315.859 c0,6.266-5.078,11.344-11.344,11.344H59.344c-6.266,0-11.344-5.078-11.344-11.344V98.547c0-6.266,5.078-11.344,11.344-11.344 h393.313c6.266,0,11.344,5.078,11.344,11.344V315.859z"/>
</svg>"##;

const SVG_SERVER: &str = r##"<svg fill="#000000" viewBox="0 0 36 36" xmlns="http://www.w3.org/2000/svg">
<path d="M26.5,2h-17C8.7,2,8,2.7,8,3.5V34h20V3.5C28,2.7,27.3,2,26.5,2z M18,30.5c-1.5,0-2.8-1.2-2.8-2.8S16.5,25,18,25s2.8,1.2,2.8,2.8S19.5,30.5,18,30.5z M23,22.6H13V21h10V22.6z M24,11.6H12V10h12V11.6z M24,7.6H12V6h12V7.6z"/>
<circle cx="18" cy="27.8" r="1.2"/>
</svg>"##;

const SVG_FIREWALL: &str = r##"<svg fill="#000000" viewBox="0 0 512 512" xmlns="http://www.w3.org/2000/svg">
<path d="M413.4,217.098c-52.807,22.982-43.402,74.765-41.898,114.632l-31.063-28.162c-38.767,17.12-69.482,91.808-33.808,150.306 c20.058,32.882,55.034,52.506,93.562,52.506c43.644,0,84.375-26.765,101.352-66.604C547.152,332.69,418.052,291.521,413.4,217.098 z"/>
<rect x="166.957" y="128.056" width="178.087" height="77.913"/>
<rect y="239.36" width="239.304" height="77.913"/>
<rect y="128.056" width="133.565" height="77.913"/>
<path d="M272.696,239.359v77.913h6.926c12.586-20.73,37.175-45.54,58.443-47.01c1.353-10.326,3.827-20.804,8.007-30.903H272.696z"/>
<path d="M456.213,239.359c13.436,21.106,36.265,41.914,55.787,68.336v-68.336H456.213z"/>
<path d="M264.535,350.663h-97.578v89.043h97.133C255.373,411.469,255.283,380.432,264.535,350.663z"/>
<path d="M0,350.663v72.348c0,9.217,7.479,16.696,16.696,16.696h116.87v-89.043H0z"/>
<path d="M378.435,128.053v70.567c9.15-6.552,23.677-14.915,34.967-14.915c13.738,0,26.416,8.368,31.352,22.261H512v-77.913 H378.435z"/>
<path d="M16.696,5.62C7.479,5.62,0,13.098,0,22.315v72.348h239.304V5.62H16.696z"/>
<path d="M495.304,5.62H272.696v89.044H512V22.315C512,13.098,504.521,5.62,495.304,5.62z"/>
</svg>"##;

const SVG_CLOUD: &str = r##"<svg fill="#000000" viewBox="0 0 32 32" xmlns="http://www.w3.org/2000/svg">
<path d="M24.972,12.288C24.608,7.657,20.723,4,16,4c-4.04,0-7.508,2.624-8.628,6.451C4.181,11.559,2,14.583,2,18 c0,4.411,3.589,8,8,8h13c3.859,0,7-3.14,7-7C30,15.851,27.93,13.148,24.972,12.288z"/>
</svg>"##;

const SVG_WIRELESS_AP: &str = r##"<svg viewBox="0 0 24 24" xmlns="http://www.w3.org/2000/svg" fill="#000000">
<path fill-rule="evenodd" d="M3.41 13.95 H20.59 A1.91 1.91 0 0 1 22.5 15.86 V19.68 A1.91 1.91 0 0 1 20.59 21.59 H3.41 A1.91 1.91 0 0 1 1.5 19.68 V15.86 A1.91 1.91 0 0 1 3.41 13.95 Z M5.32 16.82 A0.95 0.95 0 1 0 5.32 18.72 A0.95 0.95 0 1 0 5.32 16.82 Z M10.09 16.82 H12 V18.73 H10.09 Z M13.91 16.82 H15.82 V18.73 H13.91 Z M17.73 16.82 H19.64 V18.73 H17.73 Z"/>
<circle cx="12" cy="10.14" r="0.95"/>
<path d="M9.3 7.44 A3.82 3.82 0 0 1 14.7 7.44 L13.35 8.79 A1.91 1.91 0 0 0 10.65 8.79 Z"/>
<path d="M6.6 4.74 A7.62 7.62 0 0 1 17.4 4.74 L16.05 6.09 A5.71 5.71 0 0 0 7.95 6.09 Z"/>
<path d="M3.41 21.59 H5.32 V23.5 H3.41 Z M18.68 21.59 H20.59 V23.5 H18.68 Z"/>
</svg>"##;

fn get_svg_string(node_type: NodeType) -> &'static str {
    match node_type {
        NodeType::Router => SVG_ROUTER,
        NodeType::Switch => SVG_SWITCH,
        NodeType::MultilayerSwitch => SVG_MLS,
        NodeType::PC => SVG_PC,
        NodeType::Server => SVG_SERVER,
        NodeType::Firewall => SVG_FIREWALL,
        NodeType::Cloud => SVG_CLOUD,
        NodeType::WirelessAP => SVG_WIRELESS_AP,
    }
}

/// Public accessor for SVG strings, used by the export module.
pub fn get_svg_string_for_export(node_type: NodeType) -> &'static str {
    get_svg_string(node_type)
}

// ---------------------------------------------------------------------------
// Icon texture cache
// ---------------------------------------------------------------------------

struct IconCache {
    /// Textures for dark mode (white icons on dark backgrounds)
    dark_textures: HashMap<NodeType, TextureHandle>,
    /// Textures for light mode (dark icons on light backgrounds)
    light_textures: HashMap<NodeType, TextureHandle>,
}

impl IconCache {
    fn new() -> Self {
        Self {
            dark_textures: HashMap::new(),
            light_textures: HashMap::new(),
        }
    }

    fn get_or_insert(&mut self, ctx: &egui::Context, node_type: NodeType, is_dark_mode: bool) -> Option<TextureId> {
        let (textures, fill_color) = if is_dark_mode {
            (&mut self.dark_textures, "#ffffff")
        } else {
            (&mut self.light_textures, "#000000")
        };
        if !textures.contains_key(&node_type) {
            let svg = get_svg_string(node_type);
            if let Some(img) = render_svg_to_image(svg, fill_color) {
                let handle = ctx.load_texture(
                    format!("icon_{:?}_{}", node_type, if is_dark_mode { "dark" } else { "light" }),
                    img,
                    egui::TextureOptions::LINEAR,
                );
                textures.insert(node_type, handle);
            }
        }
        textures.get(&node_type).map(|h| h.id())
    }
}

static ICON_CACHE: OnceLock<std::sync::Mutex<IconCache>> = OnceLock::new();

fn get_cache() -> std::sync::MutexGuard<'static, IconCache> {
    ICON_CACHE.get_or_init(|| std::sync::Mutex::new(IconCache::new())).lock().unwrap()
}

// ---------------------------------------------------------------------------
// SVG rendering
// ---------------------------------------------------------------------------

/// Remove width and height attributes from the root <svg> element so usvg uses the viewBox size.
/// Only strips attributes from the opening <svg> tag, not from child elements like <rect>.
fn strip_svg_dimensions(svg: &str) -> String {
    // Find the opening <svg> tag and only strip width/height from it
    let svg_tag_end = match svg.find('>') {
        Some(pos) => pos,
        None => return svg.to_string(),
    };
    let svg_tag = &svg[..=svg_tag_end];
    let rest = &svg[svg_tag_end + 1..];
    
    let mut new_tag = svg_tag.to_string();
    for attr in ["width", "height"] {
        loop {
            let pat = format!(" {}=", attr);
            let pos = match new_tag.find(&pat) {
                Some(p) => p,
                None => break,
            };
            let after = pos + pat.len();
            if after < new_tag.len() {
                let ch = new_tag.as_bytes()[after];
                if ch == 34u8 || ch == 39u8 {
                    let end = match new_tag[after + 1..].find(ch as char) {
                        Some(e) => after + 1 + e + 1,
                        None => break,
                    };
                    new_tag.replace_range(pos..end, "");
                    continue;
                }
            }
            break;
        }
    }
    format!("{}{}", new_tag, rest)
}

/// Render an SVG string to an egui ColorImage with the given fill color.
/// The SVG's black fill is replaced with the specified color before rendering.
fn render_svg_to_image(svg_data: &str, fill_color: &str) -> Option<ColorImage> {
    // Replace black fill with the desired color
    let svg_modified = svg_data
        .replace("fill=\"#000000\"", &format!("fill=\"{}\"", fill_color))
        .replace("fill='#000000'", &format!("fill='{}'", fill_color));

    // Remove width/height so usvg uses viewBox
    let svg_modified = strip_svg_dimensions(&svg_modified);

    // Parse the SVG
    let options = resvg::usvg::Options::default();
    let rtree = match resvg::usvg::Tree::from_str(&svg_modified, &options) {
        Ok(tree) => tree,
        Err(e) => {
            eprintln!("Failed to parse SVG: {}", e);
            return None;
        }
    };

    let size = rtree.size();
    let scale = 2.0;
    let width = (size.width() * scale) as u32;
    let height = (size.height() * scale) as u32;

    if width == 0 || height == 0 {
        eprintln!("SVG has zero size: {}x{}", width, height);
        return None;
    }

    let mut pixmap = resvg::tiny_skia::Pixmap::new(width, height).expect("Failed to create pixmap");
    resvg::render(&rtree, resvg::tiny_skia::Transform::from_scale(scale, scale), &mut pixmap.as_mut());

    // Convert to egui ColorImage using unmultiplied alpha
    let mut img = ColorImage::new([width as usize, height as usize], vec![Color32::TRANSPARENT; (width * height) as usize]);
    for (i, pixel) in pixmap.pixels().iter().enumerate() {
        if i < img.pixels.len() {
            img.pixels[i] = Color32::from_rgba_unmultiplied(
                pixel.red(),
                pixel.green(),
                pixel.blue(),
                pixel.alpha(),
            );
        }
    }
    Some(img)
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Draw a node icon on the canvas painter at the given center position
pub fn draw_node_icon(painter: &egui::Painter, node_type: NodeType, center: Pos2, size: f32, selected: bool, visuals: &Visuals) {
    // Draw node background
    let bg = if selected {
        Color32::from_rgb(100, 150, 255)
    } else {
        let bg = visuals.extreme_bg_color;
        let lum = bg.to_array()[0] as f32 * 0.299
            + bg.to_array()[1] as f32 * 0.587
            + bg.to_array()[2] as f32 * 0.114;
        if lum < 128.0 {
            Color32::from_rgb(60, 60, 70)
        } else {
            Color32::from_rgb(240, 240, 245)
        }
    };

    let stroke_color = if selected {
        Color32::from_rgb(80, 130, 255)
    } else {
        let arr = visuals.extreme_bg_color.to_array();
        let is_dark = (arr[0] as f32) < 128.0;
        let c = if is_dark { 120u8 } else { 100u8 };
        Color32::from_rgb(c, c, c)
    };
    let stroke = Stroke::new(if selected { 2.0_f32 } else { 1.5_f32 }, stroke_color);

    let rect = Rect::from_center_size(center, vec2(size, size));
    painter.rect_filled(rect, 4.0, bg);
    painter.rect_stroke(rect, 4.0, stroke, StrokeKind::Outside);

    // Draw the SVG icon texture with the correct color for the theme
    // When selected, the background is blue, so always use white icons
    // In dark mode, use white icons on dark backgrounds
    // In light mode, use black icons on light backgrounds
    let is_dark_mode = selected || {
        let arr = visuals.extreme_bg_color.to_array();
        (arr[0] as f32) < 128.0
    };
    let ctx = painter.ctx().clone();
    if let Some(texture_id) = {
        let mut cache = get_cache();
        cache.get_or_insert(&ctx, node_type, is_dark_mode)
    } {
        // No tint needed — the texture is already rendered in the correct color
        painter.image(texture_id, rect, Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), Color32::WHITE);
    }
}

/// Draw a semi-transparent preview of a node icon (for palette drag preview)
pub fn draw_node_icon_preview(painter: &egui::Painter, node_type: NodeType, center: Pos2, size: f32, _visuals: &Visuals) {
    // Semi-transparent background
    let bg = Color32::from_rgba_unmultiplied(200, 200, 220, 100);
    let rect = Rect::from_center_size(center, vec2(size, size));
    painter.rect_filled(rect, 4.0, bg);
    painter.rect_stroke(rect, 4.0, Stroke::new(1.5_f32, Color32::from_rgba_unmultiplied(100, 150, 255, 180)), StrokeKind::Outside);

    // Draw the SVG icon — use dark mode texture for preview (always visible)
    let ctx = painter.ctx().clone();
    if let Some(texture_id) = {
        let mut cache = get_cache();
        cache.get_or_insert(&ctx, node_type, true)
    } {
        let tint = Color32::from_rgba_unmultiplied(200, 200, 255, 180);
        painter.image(texture_id, rect, Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), tint);
    }
}

/// Get all node types in palette order
pub fn all_node_types() -> &'static [NodeType] {
    &[
        NodeType::Router,
        NodeType::Switch,
        NodeType::MultilayerSwitch,
        NodeType::PC,
        NodeType::Server,
        NodeType::Firewall,
        NodeType::Cloud,
        NodeType::WirelessAP,
    ]
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_firewall_svg_rendering() {
        let svg = get_svg_string(NodeType::Firewall);
        let img = render_svg_to_image(svg, "#ffffff").expect("Failed to render firewall SVG");
        let w = img.width() as u32;
        let h = img.height() as u32;
        
        // Check the three rect areas (the "bricks" in the firewall icon)
        // At 2x scale: viewBox 512x512 -> 1024x1024
        // rect1: x=166.957, y=128.056, w=178.087, h=77.913 -> 2x: x=334..512, y=256..412
        // rect2: x=0, y=239.36, w=239.304, h=77.913 -> 2x: x=0..478, y=478..634
        // rect3: x=0, y=128.056, w=133.565, h=77.913 -> 2x: x=0..268, y=256..412
        let rects = [
            ("rect1 top-right", 334u32, 256u32, 512u32, 412u32),
            ("rect2 middle-left", 0u32, 478u32, 478u32, 634u32),
            ("rect3 top-left", 0u32, 256u32, 268u32, 412u32),
        ];
        for (name, x1, y1, x2, y2) in &rects {
            let mut f = 0u32;
            let mut t = 0u32;
            for y in *y1..*y2 {
                for x in *x1..*x2 {
                    if y < h && x < w {
                        t += 1;
                        let idx = (y * w + x) as usize;
                        if idx < img.pixels.len() && img.pixels[idx].a() > 128 {
                            f += 1;
                        }
                    }
                }
            }
            assert!(f as f64 / t as f64 > 0.9, "{} should be mostly filled (got {}/{} = {:.1}%)", name, f, t, f as f64 / t as f64 * 100.0);
        }
    }
}
