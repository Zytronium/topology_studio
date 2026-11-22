use crate::model::{Node, Link, TopologyProject};
use uuid::Uuid;

/// Clipboard state for copy/paste operations.
pub struct Clipboard {
    /// Nodes that were copied (stored as full data for pasting)
    pub nodes: Vec<Node>,
    /// Links that were copied
    pub links: Vec<Link>,
}

impl Clipboard {
    pub fn new() -> Self {
        Self {
            nodes: Vec::new(),
            links: Vec::new(),
        }
    }

    /// Copy the selected nodes and their links from the project.
    pub fn copy_from(&mut self, project: &TopologyProject, selected_ids: &[Uuid]) {
        self.nodes.clear();
        self.links.clear();

        // Copy selected nodes
        for node in &project.nodes {
            if selected_ids.contains(&node.id) {
                self.nodes.push(node.clone());
            }
        }

        // Copy links between selected nodes
        for link in &project.links {
            if selected_ids.contains(&link.source) && selected_ids.contains(&link.target) {
                self.links.push(link.clone());
            }
        }
    }

    /// Paste the copied nodes and links into the project.
    /// Returns the IDs of the newly pasted nodes.
    /// The pasted nodes are placed at (paste_x, paste_y) as the reference point for the first node,
    /// preserving the relative layout of the copied group.
    pub fn paste_into(&self, project: &mut TopologyProject, paste_x: f64, paste_y: f64) -> Vec<Uuid> {
        let mut new_node_ids: Vec<Uuid> = Vec::new();
        let mut id_mapping: std::collections::HashMap<Uuid, Uuid> = std::collections::HashMap::new();

        // Calculate the bounding box of copied nodes to preserve relative layout
        let first_node_x = self.nodes.first().map(|n| n.x).unwrap_or(0.0);
        let first_node_y = self.nodes.first().map(|n| n.y).unwrap_or(0.0);

        // Create new nodes with new IDs, placing the first node at the paste position
        for node in &self.nodes {
            let mut new_node = node.clone();
            new_node.id = Uuid::new_v4();
            // Preserve relative offset from the first node, but place the first node at paste position
            new_node.x = paste_x + (node.x - first_node_x);
            new_node.y = paste_y + (node.y - first_node_y);
            id_mapping.insert(node.id, new_node.id);
            new_node_ids.push(new_node.id);
            project.add_node(new_node);
        }

        // Create new links with new IDs, remapping source/target
        for link in &self.links {
            if let (Some(new_source), Some(new_target)) = (
                id_mapping.get(&link.source),
                id_mapping.get(&link.target),
            ) {
                let mut new_link = link.clone();
                new_link.id = Uuid::new_v4();
                new_link.source = *new_source;
                new_link.target = *new_target;
                project.add_link(new_link);
            }
        }

        new_node_ids
    }

    /// Check if there is anything in the clipboard.
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }
}

impl Default for Clipboard {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{NodeType, LinkStyle};

    #[test]
    fn test_copy_paste_single_node() {
        let mut project = TopologyProject::new();
        let node = Node::new(NodeType::Router, 100.0, 200.0);
        let node_id = node.id;
        project.add_node(node);

        let mut clipboard = Clipboard::new();
        clipboard.copy_from(&project, &[node_id]);
        assert_eq!(clipboard.nodes.len(), 1);

        // Paste at (50.0, 50.0) - the pasted node should be placed at that position
        let new_ids = clipboard.paste_into(&mut project, 50.0, 50.0);
        assert_eq!(new_ids.len(), 1);
        assert_eq!(project.nodes.len(), 2);

        let pasted = project.get_node(new_ids[0]).unwrap();
        assert_eq!(pasted.x, 50.0);
        assert_eq!(pasted.y, 50.0);
    }

    #[test]
    fn test_copy_paste_with_links() {
        let mut project = TopologyProject::new();
        let n1 = Node::new(NodeType::Router, 0.0, 0.0);
        let n2 = Node::new(NodeType::Switch, 100.0, 0.0);
        project.add_node(n1.clone());
        project.add_node(n2.clone());
        project.add_link(Link::new(n1.id, n2.id, LinkStyle::Solid));

        let mut clipboard = Clipboard::new();
        clipboard.copy_from(&project, &[n1.id, n2.id]);
        assert_eq!(clipboard.nodes.len(), 2);
        assert_eq!(clipboard.links.len(), 1);

        // Paste at (50.0, 50.0) - first node goes there, second keeps relative offset
        let new_ids = clipboard.paste_into(&mut project, 50.0, 50.0);
        assert_eq!(new_ids.len(), 2);
        assert_eq!(project.nodes.len(), 4);
        assert_eq!(project.links.len(), 2);

        // Verify relative layout is preserved
        let pasted_n1 = project.get_node(new_ids[0]).unwrap();
        let pasted_n2 = project.get_node(new_ids[1]).unwrap();
        assert_eq!(pasted_n1.x, 50.0);
        assert_eq!(pasted_n1.y, 50.0);
        assert_eq!(pasted_n2.x, 150.0); // 50 + (100 - 0)
        assert_eq!(pasted_n2.y, 50.0);  // 50 + (0 - 0)
    }

    #[test]
    fn test_copy_preserves_node_data() {
        let mut project = TopologyProject::new();
        let node = Node::new(NodeType::Firewall, 100.0, 200.0);
        let node_id = node.id;
        project.add_node(node);

        let mut clipboard = Clipboard::new();
        clipboard.copy_from(&project, &[node_id]);

        let copied = &clipboard.nodes[0];
        assert_eq!(copied.node_type, NodeType::Firewall);
        assert_eq!(copied.label, "Firewall");
        assert_eq!(copied.x, 100.0);
        assert_eq!(copied.y, 200.0);
    }

    #[test]
    fn test_clipboard_is_empty() {
        let clipboard = Clipboard::new();
        assert!(clipboard.is_empty());
    }
}
