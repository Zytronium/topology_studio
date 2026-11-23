/// Export the current topology project to an SVG string.
use crate::model::{TopologyProject, LinkStyle, NodeType};
use crate::icons;

const ICON_SIZE: f64 = 40.0;

/// Label font size used in the exported SVG
const LABEL_FONT_SIZE: f64 = 12.0;

/// Options for SVG export
#[derive(Clone, Copy)]
pub struct ExportOptions {
    /// Whether to use dark mode colors (dark background, light text/icons)
    pub dark_mode: bool,
    /// Whether to show the background grid
    pub show_grid: bool,
}

impl Default for ExportOptions {
    fn default() -> Self {
        Self { dark_mode: false, show_grid: false }
    }
}

// -------- id handling --------

/// Remove the first ` id="..."` attribute from an element string. Icons are inlined
/// once per node, so keeping ids would produce duplicate ids in the document.
fn strip_id_attr(elem: &str) -> String {
    if let Some(start) = elem.find(" id=\"") {
        let after = &elem[start + 5..];
        if let Some(end) = after.find('"') {
            return format!("{}{}", &elem[..start], &after[end + 1..]);
        }
    }
    elem.to_string()
}

/// Process an SVG string for export: replace fill colors, strip width/height,
/// and extract all drawable elements (path, circle, rect). Expands `<use>` tags
/// inline so all referenced elements are rendered.
fn process_svg_for_export(node_type: NodeType, fill_color: &str) -> (Vec<String>, f64) {
    let svg = icons::get_svg_string_for_export(node_type);

    // Extract viewBox size from the original SVG
    let viewbox_size = extract_viewbox_size(svg);

    // Check if the root <svg> tag has a fill attribute (inherited by child elements)
    let root_has_fill = has_root_fill(svg);

    // Replace fill colors, handling both root-level and element-level fills
    let modified = svg
        .replace("fill=\"#000000\"", &format!("fill=\"{}\"", fill_color))
        .replace("fill='#000000'", &format!("fill='{}'", fill_color));

    // Extract elements with id attributes (for <use> tag expansion)
    let mut id_elements: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    for line in modified.lines() {
        let trimmed = line.trim();
        if let Some(id_start) = trimmed.find("id=\"") {
            let after_id = &trimmed[id_start + 4..];
            if let Some(id_end) = after_id.find('"') {
                let elem_id = after_id[..id_end].to_string();
                id_elements.insert(elem_id, trimmed.to_string());
            }
        }
    }

    // Extract all drawable elements (path, circle, rect) and expand <use> tags
    let mut elements = Vec::new();
    for line in modified.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("<path") || trimmed.starts_with("<circle") || trimmed.starts_with("<rect") {
            // If the element doesn't have an explicit fill and the root had one,
            // inject the fill color so the icon renders with the correct color
            let elem = if root_has_fill && !trimmed.contains("fill=") {
                // Insert fill attribute right after the tag name
                let tag_name = if trimmed.starts_with("<path") { "<path" }
                else if trimmed.starts_with("<circle") { "<circle" }
                else { "<rect" };
                format!("{} fill=\"{}\" {}", tag_name, fill_color, &trimmed[tag_name.len()..])
            } else {
                trimmed.to_string()
            };
            elements.push(strip_id_attr(&elem));
        } else if trimmed.starts_with("<use") {
            // Expand <use href="#id" transform="..."/> into the referenced element
            if let Some(transform) = extract_use_transform(&trimmed) {
                if let Some(href) = extract_use_href(&trimmed) {
                    if let Some(ref_elem) = id_elements.get(&href) {
                        // Apply the transform to the referenced element
                        let expanded = apply_transform_to_element(ref_elem, &transform);
                        // The expanded element may not have a fill attribute (e.g., the arrow path
                        // in the multilayer switch). Inject the fill color so it renders correctly
                        // in both light and dark modes.
                        let with_fill = inject_fill_if_missing(&expanded, fill_color);
                        elements.push(strip_id_attr(&with_fill));
                    }
                }
            }
        }
    }

    (elements, viewbox_size)
}

/// Extract the href attribute from a <use> tag
fn extract_use_href(tag: &str) -> Option<String> {
    // Try href="..." first
    if let Some(start) = tag.find("href=\"") {
        let after = &tag[start + 6..];
        if let Some(end) = after.find('"') {
            let href = &after[..end];
            // Strip the "#" prefix
            if href.starts_with('#') {
                return Some(href[1..].to_string());
            }
            return Some(href.to_string());
        }
    }
    // Try href='...'
    if let Some(start) = tag.find("href='") {
        let after = &tag[start + 6..];
        if let Some(end) = after.find('\'') {
            let href = &after[..end];
            if href.starts_with('#') {
                return Some(href[1..].to_string());
            }
            return Some(href.to_string());
        }
    }
    None
}

/// Extract the transform attribute from a <use> tag
fn extract_use_transform(tag: &str) -> Option<String> {
    if let Some(start) = tag.find("transform=\"") {
        let after = &tag[start + 11..];
        if let Some(end) = after.find('"') {
            return Some(after[..end].to_string());
        }
    }
    if let Some(start) = tag.find("transform='") {
        let after = &tag[start + 11..];
        if let Some(end) = after.find('\'') {
            return Some(after[..end].to_string());
        }
    }
    None
}

/// Inject a fill attribute into an SVG element if it doesn't already have one.
/// This ensures expanded `<use>` tags render with the correct color in both
/// light and dark modes.
fn inject_fill_if_missing(elem: &str, fill_color: &str) -> String {
    if elem.contains("fill=") {
        elem.to_string()
    } else {
        // Find the opening tag and insert fill after the tag name
        if let Some(gt_pos) = elem.find('>') {
            let before_gt = &elem[..gt_pos];
            let after_gt = &elem[gt_pos..];
            // Extract the tag name (e.g., "<path")
            if let Some(tag_end) = before_gt.find(' ') {
                let tag_name = &before_gt[..tag_end];
                let rest = &before_gt[tag_end..];
                format!("{} fill=\"{}\"{}{}", tag_name, fill_color, rest, after_gt)
            } else if before_gt.ends_with("/>") {
                // Self-closing tag with no attributes
                format!("{} fill=\"{}\" />", before_gt, fill_color)
            } else {
                // Fallback: wrap in a group with fill
                format!("<g fill=\"{}\">{}</g>", fill_color, elem)
            }
        } else {
            elem.to_string()
        }
    }
}

/// Apply a transform attribute to an SVG element
fn apply_transform_to_element(elem: &str, transform: &str) -> String {
    // Handle self-closing tags like <path .../>
    if let Some(sl_pos) = elem.rfind("/>") {
        let before_sl = &elem[..sl_pos];
        // Insert transform="..." before the />
        format!("{} transform=\"{}\" />", before_sl, transform)
    } else if let Some(gt_pos) = elem.find('>') {
        let before_gt = &elem[..gt_pos];
        let after_gt = &elem[gt_pos..];
        format!("{} transform=\"{}\"{}", before_gt, transform, after_gt)
    } else {
        // Fallback: wrap in a group with transform
        format!("<g transform=\"{}\">{}</g>", transform, elem)
    }
}

/// Check if the root <svg> tag has a fill attribute (inherited by child elements).
fn has_root_fill(svg: &str) -> bool {
    // Find the opening <svg> tag
    let svg_tag_end = match svg.find('>') {
        Some(pos) => pos,
        None => return false,
    };
    let svg_tag = &svg[..svg_tag_end];

    // Check for fill attribute
    svg_tag.contains("fill=\"") || svg_tag.contains("fill='")
}

/// Calculate the point on a node's edge where a line from the node center toward
/// the target direction intersects the node's bounding box. Uses per-node dimensions
/// (width, height) so links connect at the actual node border, not a fixed square.
fn node_edge_point(
    node_x: f64, node_y: f64,
    node_w: f64, node_h: f64,
    target_x: f64, target_y: f64,
) -> (f64, f64) {
    let dx = target_x - node_x;
    let dy = target_y - node_y;

    // Handle edge case: same position
    if dx.abs() < 0.001 && dy.abs() < 0.001 {
        return (node_x, node_y - node_h / 2.0);
    }

    let half_w = node_w / 2.0;
    let half_h = node_h / 2.0;

    // Calculate intersection with the bounding box edges
    let mut t = f64::MAX;

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

    if t == f64::MAX {
        (node_x, node_y)
    } else {
        // t is a fraction along the direction vector (dx, dy)
        // Edge point = center + direction * t
        (node_x + dx * t, node_y + dy * t)
    }
}

pub fn export_to_svg(project: &TopologyProject, options: &ExportOptions) -> String {
    let mut min_x = f64::INFINITY;
    let mut min_y = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut max_y = f64::NEG_INFINITY;

    // -------- bounds --------
    for node in &project.nodes {
        // Use ICON_SIZE (40x40) for both background and link edges, matching the canvas
        let half = ICON_SIZE / 2.0;
        let label_bottom = node.y + half + 4.0 + 10.0;
        // Labels are centered on the node, so they may be wider than the icon
        let label_half_w = estimate_label_width(&node.label) / 2.0;
        let node_left = (node.x - half).min(node.x - label_half_w);
        let node_right = (node.x + half).max(node.x + label_half_w);
        let node_top = node.y - half;

        min_x = min_x.min(node_left);
        min_y = min_y.min(node_top);
        max_x = max_x.max(node_right);
        max_y = max_y.max(label_bottom);
    }

    // Handle empty project
    if project.nodes.is_empty() {
        return "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"800\" height=\"600\" viewBox=\"0 0 800 600\">\n</svg>\n".to_string();
    }

    // Padding around the content, 50px ensures all content fits within the viewBox
    let padding = 50.0;
    min_x -= padding;
    min_y -= padding;
    max_x += padding;
    max_y += padding;

    // Round the canvas size UP to whole pixels and use this one value everywhere
    // (width/height, viewBox, background, grid) so nothing is cropped by rounding
    let width = (max_x - min_x).ceil();
    let height = (max_y - min_y).ceil();

    // -------- theme colors --------
    let bg_color = if options.dark_mode { "#1e1e2e" } else { "#ffffff" };
    let grid_color = if options.dark_mode { "#333344" } else { "#cccccc" };
    let link_color = if options.dark_mode { "#888899" } else { "#333333" };
    let icon_color = if options.dark_mode { "#ccccdd" } else { "#333333" };
    let label_color = if options.dark_mode { "#dddddd" } else { "#333333" };
    let node_bg_color = if options.dark_mode { "#3c3c46" } else { "#f0f0f5" };
    let node_border_color = if options.dark_mode { "#787878" } else { "#646464" };

    // Offset all coordinates so viewBox starts at 0,0 for maximum viewer compatibility
    let offset_x = -min_x;
    let offset_y = -min_y;

    let mut svg = String::new();

    // -------- header --------
    // XML declaration for strict loaders (glycin/Gwenview)
    svg.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    // Explicit width and height matching the viewBox for glycin/Gwenview compatibility.
    // Without explicit dimensions, glycin uses a default size that cuts off tall content.
    // preserveAspectRatio keeps the full viewBox visible if a viewer rescales it.
    svg.push_str("<svg xmlns=\"http://www.w3.org/2000/svg\" ");
    svg.push_str(&format!(
        "width=\"{w:.0}\" height=\"{h:.0}\" viewBox=\"0 0 {w:.0} {h:.0}\" preserveAspectRatio=\"xMidYMid meet\">",
        w = width,
        h = height
    ));
    svg.push('\n');

    // -------- background --------
    svg.push_str(&format!(
        "  <rect width=\"{:.0}\" height=\"{:.0}\" fill=\"{}\"/>\n",
        width, height, bg_color
    ));

    // -------- grid --------
    if options.show_grid {
        let grid_spacing = 40.0;
        let start_x = ((min_x + offset_x) / grid_spacing).floor() * grid_spacing;
        let start_y = ((min_y + offset_y) / grid_spacing).floor() * grid_spacing;
        svg.push_str(&format!("  <g stroke=\"{}\" stroke-width=\"0.5\">\n", grid_color));
        for x in (start_x as i32..=(width as i32)).step_by(grid_spacing as usize) {
            svg.push_str(&format!("    <line x1=\"{:.0}\" y1=\"0\" x2=\"{:.0}\" y2=\"{:.0}\"/>\n", x as f64, x as f64, height));
        }
        for y in (start_y as i32..=(height as i32)).step_by(grid_spacing as usize) {
            svg.push_str(&format!("    <line x1=\"0\" y1=\"{:.0}\" x2=\"{:.0}\" y2=\"{:.0}\"/>\n", y as f64, width, y as f64));
        }
        svg.push_str("  </g>\n");
    }

    // -------- links --------
    for link in &project.links {
        let source_node = match project.get_node(link.source) {
            Some(n) => n,
            None => continue,
        };
        let target_node = match project.get_node(link.target) {
            Some(n) => n,
            None => continue,
        };

        let (sx, sy) = node_edge_point(
            source_node.x, source_node.y,
            ICON_SIZE, ICON_SIZE,
            target_node.x, target_node.y,
        );
        let (tx, ty) = node_edge_point(
            target_node.x, target_node.y,
            ICON_SIZE, ICON_SIZE,
            source_node.x, source_node.y,
        );

        let stroke_dash = match link.style {
            LinkStyle::Solid => String::new(),
            LinkStyle::Dashed => "stroke-dasharray=\"8,4\" ".to_string(),
        };

        svg.push_str(&format!(
            "  <line x1=\"{:.1}\" y1=\"{:.1}\" x2=\"{:.1}\" y2=\"{:.1}\" stroke=\"{}\" stroke-width=\"2\" {}/>\n",
            sx + offset_x, sy + offset_y, tx + offset_x, ty + offset_y, link_color, stroke_dash
        ));
    }

    // -------- nodes --------
    // Drawn after links so node backgrounds cover link endpoints
    for node in &project.nodes {
        let (elements, viewbox_size) = process_svg_for_export(node.node_type, icon_color);

        let scale = ICON_SIZE / viewbox_size;
        // Node background uses ICON_SIZE (40x40), matching the canvas rendering
        let half = ICON_SIZE / 2.0;
        let bg_x = (node.x - half) + offset_x;
        let bg_y = (node.y - half) + offset_y;

        // Node background (rounded rectangle)
        svg.push_str(&format!(
            "  <rect x=\"{:.1}\" y=\"{:.1}\" width=\"{:.0}\" height=\"{:.0}\" rx=\"4\" ry=\"4\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1.5\"/>\n",
            bg_x, bg_y, ICON_SIZE, ICON_SIZE, node_bg_color, node_border_color
        ));

        // Icon, centered on node position and scaled to ICON_SIZE within the background
        let icon_x = (node.x - ICON_SIZE / 2.0) + offset_x;
        let icon_y = (node.y - ICON_SIZE / 2.0) + offset_y;
        svg.push_str(&format!(
            "  <g transform=\"translate({:.1}, {:.1}) scale({:.4})\">\n",
            icon_x, icon_y, scale
        ));
        for elem in &elements {
            svg.push_str("    ");
            svg.push_str(elem);
            svg.push('\n');
        }
        svg.push_str("  </g>\n");

        // Label, positioned below the icon with proper spacing
        // Use baseline positioning (no dominant-baseline) for maximum viewer compatibility
        let label_y = (node.y + ICON_SIZE / 2.0 + 4.0 + 10.0) + offset_y;
        let label_x = node.x + offset_x;
        svg.push_str(&format!(
            "  <text x=\"{:.1}\" y=\"{:.1}\" text-anchor=\"middle\" font-family=\"sans-serif\" font-size=\"{:.0}\" fill=\"{}\">{}</text>\n",
            label_x, label_y, LABEL_FONT_SIZE, label_color, escape_xml(&node.label)
        ));
    }

    svg.push_str("</svg>\n");
    svg
}

/// Extract the viewBox size from an SVG string.
/// Parses the viewBox attribute value which is quote-delimited: viewBox="x y w h"
fn extract_viewbox_size(svg: &str) -> f64 {
    // Find the viewBox attribute
    if let Some(start) = svg.find("viewBox=") {
        let after_attr = &svg[start + 8..];
        // Skip whitespace and find the opening quote
        let trimmed = after_attr.trim_start();
        if let Some(first_quote) = trimmed.find(|c: char| c == '"' || c == '\'') {
            let quote_char = trimmed.as_bytes()[first_quote] as char;
            let value_start = first_quote + 1;
            // Find the closing quote
            if let Some(end) = trimmed[value_start..].find(quote_char) {
                let viewbox_str = &trimmed[value_start..value_start + end];
                let parts: Vec<&str> = viewbox_str.split_whitespace().collect();
                if parts.len() >= 4 {
                    if let (Ok(w), Ok(h)) = (parts[2].parse::<f64>(), parts[3].parse::<f64>()) {
                        if w > 0.0 && h > 0.0 {
                            return w.max(h);
                        }
                    }
                }
            }
        }
    }
    36.0 // Default fallback
}

// -------- label metrics --------

/// Conservative estimate of the rendered width of a label in sans-serif.
/// Average glyph width is roughly 0.6 em, rounded up a little so wide
/// characters do not get clipped.
fn estimate_label_width(label: &str) -> f64 {
    label.chars().count() as f64 * LABEL_FONT_SIZE * 0.62
}

fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Node, Link};

    #[test]
    fn test_export_empty_project() {
        let project = TopologyProject::new();
        let svg = export_to_svg(&project, &ExportOptions::default());
        assert!(svg.contains("<svg"));
        assert!(svg.ends_with("</svg>\n"));
    }

    #[test]
    fn test_export_single_node() {
        let mut project = TopologyProject::new();
        let node = Node::new(NodeType::Router, 0.0, 0.0);
        project.add_node(node);
        let svg = export_to_svg(&project, &ExportOptions::default());
        assert!(svg.contains("Router"));
        assert!(svg.contains("<path"));
    }

    #[test]
    fn test_export_with_link() {
        let mut project = TopologyProject::new();
        let n1 = Node::new(NodeType::Router, -100.0, 0.0);
        let n2 = Node::new(NodeType::PC, 100.0, 0.0);
        project.add_node(n1);
        project.add_node(n2);
        project.add_link(Link::new(
            project.nodes[0].id,
            project.nodes[1].id,
            LinkStyle::Solid,
        ));
        let svg = export_to_svg(&project, &ExportOptions::default());
        assert!(svg.contains("<line"));
    }

    #[test]
    fn test_export_dashed_link() {
        let mut project = TopologyProject::new();
        let n1 = Node::new(NodeType::Router, -100.0, 0.0);
        let n2 = Node::new(NodeType::Cloud, 100.0, 0.0);
        project.add_node(n1);
        project.add_node(n2);
        project.add_link(Link::new(
            project.nodes[0].id,
            project.nodes[1].id,
            LinkStyle::Dashed,
        ));
        let svg = export_to_svg(&project, &ExportOptions::default());
        assert!(svg.contains("stroke-dasharray"));
    }

    #[test]
    fn test_export_dark_mode() {
        let mut project = TopologyProject::new();
        let node = Node::new(NodeType::Router, 0.0, 0.0);
        project.add_node(node);
        let options = ExportOptions { dark_mode: true, show_grid: false };
        let svg = export_to_svg(&project, &options);
        assert!(svg.contains("#1e1e2e")); // dark background
    }

    #[test]
    fn test_export_with_grid() {
        let mut project = TopologyProject::new();
        let node = Node::new(NodeType::Router, 0.0, 0.0);
        project.add_node(node);
        let options = ExportOptions { dark_mode: false, show_grid: true };
        let svg = export_to_svg(&project, &options);
        assert!(svg.contains("stroke=\"#cccccc\"")); // grid lines
    }

    #[test]
    fn test_extract_viewbox_size_36() {
        let svg = r#"<svg viewBox="0 0 36 36">"#;
        assert!((extract_viewbox_size(svg) - 36.0).abs() < 0.01);
    }

    #[test]
    fn test_extract_viewbox_size_512() {
        let svg = r#"<svg viewBox="0 0 512 512">"#;
        assert!((extract_viewbox_size(svg) - 512.0).abs() < 0.01);
    }

    #[test]
    fn test_extract_viewbox_size_32() {
        let svg = r#"<svg viewBox="0 0 32 32">"#;
        assert!((extract_viewbox_size(svg) - 32.0).abs() < 0.01);
    }

    #[test]
    fn test_extract_viewbox_size_24() {
        let svg = r#"<svg viewBox="0 0 24 24">"#;
        assert!((extract_viewbox_size(svg) - 24.0).abs() < 0.01);
    }

    #[test]
    fn test_extract_viewbox_size_single_quotes() {
        let svg = "<svg viewBox='0 0 48 48'>";
        assert!((extract_viewbox_size(svg) - 48.0).abs() < 0.01);
    }

    #[test]
    fn test_extract_viewbox_size_no_viewbox() {
        let svg = "<svg>";
        assert!((extract_viewbox_size(svg) - 36.0).abs() < 0.01); // default fallback
    }

    #[test]
    fn test_node_edge_point() {
        // Square node (60x60) at origin, target to the right
        let (x, y) = node_edge_point(0.0, 0.0, 60.0, 60.0, 100.0, 0.0);
        assert!((x - 30.0).abs() < 0.01, "Expected x=30, got {}", x);
        assert!(y.abs() < 0.01);
        // Cloud node (80x60) at origin, target to the right, edge at x=40
        let (x2, y2) = node_edge_point(0.0, 0.0, 80.0, 60.0, 100.0, 0.0);
        assert!((x2 - 40.0).abs() < 0.01, "Expected x=40, got {}", x2);
        assert!(y2.abs() < 0.01);
    }

    #[test]
    fn test_escape_xml() {
        assert_eq!(escape_xml("a & b"), "a &amp; b");
        assert_eq!(escape_xml("a < b"), "a &lt; b");
        assert_eq!(escape_xml("a > b"), "a &gt; b");
    }

    #[test]
    fn test_export_firewall_includes_rects() {
        let mut project = TopologyProject::new();
        let node = Node::new(NodeType::Firewall, 0.0, 0.0);
        project.add_node(node);
        let svg = export_to_svg(&project, &ExportOptions::default());
        // Firewall icon has <rect> elements for the bricks, they must be included
        assert!(svg.contains("<rect") && svg.contains("width=\"178"), "Firewall rects should be exported");
    }

    #[test]
    fn test_export_viewbox_format() {
        let mut project = TopologyProject::new();
        let node = Node::new(NodeType::Router, 0.0, 0.0);
        project.add_node(node);
        let svg = export_to_svg(&project, &ExportOptions::default());
        // viewBox should have 4 values: x y width height
        assert!(svg.contains("viewBox="), "Should have viewBox attribute");
    }

    #[test]
    fn test_export_node_background() {
        let mut project = TopologyProject::new();
        let node = Node::new(NodeType::Router, 0.0, 0.0);
        project.add_node(node);
        let svg = export_to_svg(&project, &ExportOptions::default());
        // Node background should be a rounded rect with proper fill
        assert!(svg.contains("rx=\"4\""), "Node background should have rounded corners");
    }

    #[test]
    fn test_export_label_below_icon() {
        let mut project = TopologyProject::new();
        let node = Node::new(NodeType::Router, 0.0, 0.0);
        project.add_node(node);
        let svg = export_to_svg(&project, &ExportOptions::default());
        // ICON_SIZE=40, padding=50: node_top = -20, min_y = -70, offset_y = 70
        // Label y = node.y + ICON_SIZE/2 + 4 + 10 + offset_y = 0 + 20 + 4 + 10 + 70 = 104
        assert!(svg.contains("y=\"104.0\""), "Label should be below the icon, got: {}", svg);
        // Label should NOT use dominant-baseline for maximum viewer compatibility
        assert!(!svg.contains("dominant-baseline"), "Label should not use dominant-baseline");
    }

    #[test]
    fn test_export_dark_mode_colors() {
        let mut project = TopologyProject::new();
        let node = Node::new(NodeType::Router, 0.0, 0.0);
        project.add_node(node);
        let options = ExportOptions { dark_mode: true, show_grid: false };
        let svg = export_to_svg(&project, &options);
        assert!(svg.contains("#1e1e2e"), "Dark background");
        assert!(svg.contains("#3c3c46"), "Dark node background");
        // Icon color is applied via fill replacement on path elements
        assert!(svg.contains("fill=\"#ccccdd\""), "Light icon fill on paths");
    }

    #[test]
    fn test_export_all_node_types() {
        let mut project = TopologyProject::new();
        for node_type in icons::all_node_types() {
            let node = Node::new(*node_type, 0.0, 0.0);
            project.add_node(node);
        }
        let svg = export_to_svg(&project, &ExportOptions::default());
        // All 8 node types should produce output with paths or rects
        assert!(svg.contains("<path") || svg.contains("<rect"), "Should have drawable elements");
        assert!(svg.contains("Router"));
        assert!(svg.contains("Wireless AP"), "Wireless AP label should be present");
    }

    #[test]
    fn test_export_multilayer_switch_arrows() {
        let mut project = TopologyProject::new();
        let node = Node::new(NodeType::MultilayerSwitch, 0.0, 0.0);
        project.add_node(node);
        let svg = export_to_svg(&project, &ExportOptions::default());
        // Multilayer switch has 8 <use> tags referencing the arrow path.
        // After expansion: base arrow + 8 expanded copies = 9 path elements,
        // plus the hexagon path and circle = at least 11 drawable elements.
        let path_count = svg.matches("<path").count();
        assert!(path_count >= 9, "Multilayer switch should have at least 9 path elements (base arrow + 8 expanded), got {}", path_count);
    }

    #[test]
    fn test_export_multilayer_switch_dark_mode_fill() {
        let mut project = TopologyProject::new();
        let node = Node::new(NodeType::MultilayerSwitch, 0.0, 0.0);
        project.add_node(node);
        let options = ExportOptions { dark_mode: true, show_grid: false };
        let svg = export_to_svg(&project, &options);
        // All expanded arrow paths should have the fill color in dark mode
        // Count paths with fill="#ccccdd" (dark mode icon color)
        let filled_paths = svg.matches("fill=\"#ccccdd\"").count();
        // At least 9 paths (base arrow + 8 expanded) should have the fill
        assert!(filled_paths >= 9, "All multilayer switch arrows should have fill color in dark mode, got {} filled paths", filled_paths);
    }

    #[test]
    fn test_export_svg_is_valid_xml() {
        // Generate SVG from sample.topo and verify it's valid XML
        let json = include_str!("../sample.topo");
        let project: TopologyProject = serde_json::from_str(json).expect("parse sample.topo");
        let svg = export_to_svg(&project, &ExportOptions::default());

        // Write to temp file for external validation
        std::fs::write("/tmp/test_export_valid.svg", &svg).expect("write temp SVG");

        // Basic XML validation: check that all tags are properly closed
        // Check for common XML errors
        for (i, line) in svg.lines().enumerate() {
            // Check for unclosed tags (tags that start with < but don't end with > or />)
            let trimmed = line.trim();
            if trimmed.starts_with('<') && !trimmed.ends_with('>') && !trimmed.ends_with("/>") {
                // Multi-line tags are OK, but self-closing tags should end with />
                panic!("Line {} may have unclosed tag: {}", i + 1, line.trim());
            }
            // Check for malformed self-closing tags like <path .../ transform="...">
            if line.contains("/ rotate(") || line.contains("/ transform=") {
                panic!("Line {} has malformed self-closing tag: {}", i + 1, line.trim());
            }
        }

        // Verify the SVG starts and ends correctly
        assert!(svg.contains("<svg"), "SVG should contain <svg>");
        assert!(svg.trim_end().ends_with("</svg>"), "SVG should end with </svg>");

        // Count opening and closing tags
        let svg_path_count = svg.matches("<path").count();
        let svg_rect_count = svg.matches("<rect").count();
        let svg_circle_count = svg.matches("<circle").count();
        let svg_line_count = svg.matches("<line").count();
        let svg_text_count = svg.matches("<text").count();

        eprintln!("SVG elements: {} paths, {} rects, {} circles, {} lines, {} texts",
                  svg_path_count, svg_rect_count, svg_circle_count, svg_line_count, svg_text_count);

        // All 9 nodes should produce elements
        assert!(svg_path_count > 0 || svg_rect_count > 0, "SVG should have drawable elements");
    }

    #[test]
    fn test_export_node_background_size() {
        let mut project = TopologyProject::new();
        let node = Node::new(NodeType::Router, 0.0, 0.0);
        project.add_node(node);
        let svg = export_to_svg(&project, &ExportOptions::default());
        // Node background should be ICON_SIZE (40x40), matching the canvas rendering
        assert!(svg.contains("width=\"40\""), "Node background should be 40px wide, got: {}", svg);
        assert!(svg.contains("height=\"40\""), "Node background should be 40px tall, got: {}", svg);
    }

    #[test]
    fn test_export_cloud_node_background_size() {
        let mut project = TopologyProject::new();
        let node = Node::new(NodeType::Cloud, 0.0, 0.0);
        project.add_node(node);
        let svg = export_to_svg(&project, &ExportOptions::default());
        // All node backgrounds should be ICON_SIZE (40x40), matching the canvas
        assert!(svg.contains("width=\"40\""), "Cloud node background should be 40px wide");
        assert!(svg.contains("height=\"40\""), "Cloud node background should be 40px tall");
    }

    // -------- canvas size tests --------

    fn parse_dims(svg: &str) -> (f64, f64, f64, f64) {
        let grab = |attr: &str| -> String {
            let key = format!("{}=\"", attr);
            let start = svg.find(&key).expect("attr present") + key.len();
            let end = svg[start..].find('"').unwrap();
            svg[start..start + end].to_string()
        };
        let vb: Vec<f64> = grab("viewBox")
            .split_whitespace()
            .map(|v| v.parse().unwrap())
            .collect();
        (
            grab("width").parse().unwrap(),
            grab("height").parse().unwrap(),
            vb[2],
            vb[3],
        )
    }

    #[test]
    fn test_export_size_matches_viewbox_and_is_whole_pixels() {
        let mut project = TopologyProject::new();
        project.add_node(Node::new(NodeType::Router, 13.37, 7.91));
        project.add_node(Node::new(NodeType::PC, 1234.56, 987.65));
        let svg = export_to_svg(&project, &ExportOptions::default());
        let (w, h, vw, vh) = parse_dims(&svg);
        assert_eq!(w, vw);
        assert_eq!(h, vh);
        assert_eq!(w, w.trunc());
        assert_eq!(h, h.trunc());
    }

    #[test]
    fn test_export_long_label_fits_in_canvas() {
        let mut project = TopologyProject::new();
        let mut node = Node::new(NodeType::Router, 0.0, 0.0);
        node.label = "A very long router label that is wider than the icon".to_string();
        project.add_node(node);
        let svg = export_to_svg(&project, &ExportOptions::default());
        let (w, _, _, _) = parse_dims(&svg);
        let label_w = estimate_label_width("A very long router label that is wider than the icon");
        // Canvas must be at least label width plus padding on both sides
        assert!(w >= label_w + 100.0, "canvas {} too narrow for label {}", w, label_w);
    }

    #[test]
    fn test_export_has_no_duplicate_ids() {
        let mut project = TopologyProject::new();
        project.add_node(Node::new(NodeType::MultilayerSwitch, 0.0, 0.0));
        project.add_node(Node::new(NodeType::MultilayerSwitch, 100.0, 0.0));
        let svg = export_to_svg(&project, &ExportOptions::default());
        assert!(!svg.contains(" id=\""), "exported SVG should not contain id attributes");
    }
}
