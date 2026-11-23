use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Type of network device
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub enum NodeType {
    Router,
    Switch,
    MultilayerSwitch,
    PC,
    Server,
    Firewall,
    Cloud,
    WirelessAP,
}

impl NodeType {
    pub fn label(&self) -> &'static str {
        match self {
            NodeType::Router => "Router",
            NodeType::Switch => "Switch",
            NodeType::MultilayerSwitch => "Multilayer Switch",
            NodeType::PC => "PC",
            NodeType::Server => "Server",
            NodeType::Firewall => "Firewall",
            NodeType::Cloud => "Cloud",
            NodeType::WirelessAP => "Wireless AP",
        }
    }

    /// Default label for a new node of this type
    pub fn default_label(&self) -> String {
        self.label().to_string()
    }
}

/// Visual style of a link
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum LinkStyle {
    Solid,
    Dashed,
}

/// A node (device) on the canvas
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Node {
    pub id: Uuid,
    pub node_type: NodeType,
    pub label: String,
    pub x: f64,
    pub y: f64,
}

impl Node {
    pub fn new(node_type: NodeType, x: f64, y: f64) -> Self {
        Node {
            id: Uuid::new_v4(),
            node_type,
            label: node_type.default_label(),
            x,
            y,
        }
    }

    /// Bounding box size for this node type (width, height)
    pub fn size(&self) -> (f64, f64) {
        match self.node_type {
            NodeType::Cloud => (80.0, 60.0),
            _ => (60.0, 60.0),
        }
    }
}

/// A link (connection) between two nodes
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Link {
    pub id: Uuid,
    pub source: Uuid,
    pub target: Uuid,
    pub style: LinkStyle,
}

impl Link {
    pub fn new(source: Uuid, target: Uuid, style: LinkStyle) -> Self {
        Link {
            id: Uuid::new_v4(),
            source,
            target,
            style,
        }
    }
}

/// A complete topology project
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct TopologyProject {
    pub nodes: Vec<Node>,
    pub links: Vec<Link>,
}

impl TopologyProject {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_node(&mut self, node: Node) {
        self.nodes.push(node);
    }

    pub fn remove_node(&mut self, id: Uuid) {
        self.nodes.retain(|n| n.id != id);
        self.links.retain(|l| l.source != id && l.target != id);
    }

    pub fn get_node(&self, id: Uuid) -> Option<&Node> {
        self.nodes.iter().find(|n| n.id == id)
    }

    pub fn get_node_mut(&mut self, id: Uuid) -> Option<&mut Node> {
        self.nodes.iter_mut().find(|n| n.id == id)
    }

    pub fn add_link(&mut self, link: Link) {
        self.links.push(link);
    }

    pub fn remove_link(&mut self, id: Uuid) {
        self.links.retain(|l| l.id != id);
    }

}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_node() {
        let node = Node::new(NodeType::Router, 100.0, 200.0);
        assert_eq!(node.node_type, NodeType::Router);
        assert_eq!(node.label, "Router");
        assert_eq!(node.x, 100.0);
        assert_eq!(node.y, 200.0);
    }

    #[test]
    fn test_node_size() {
        let router = Node::new(NodeType::Router, 0.0, 0.0);
        assert_eq!(router.size(), (60.0, 60.0));

        let cloud = Node::new(NodeType::Cloud, 0.0, 0.0);
        assert_eq!(cloud.size(), (80.0, 60.0));
    }

    #[test]
    fn test_create_link() {
        let n1 = Node::new(NodeType::Router, 0.0, 0.0);
        let n2 = Node::new(NodeType::Switch, 100.0, 0.0);
        let link = Link::new(n1.id, n2.id, LinkStyle::Solid);
        assert_eq!(link.source, n1.id);
        assert_eq!(link.target, n2.id);
        assert_eq!(link.style, LinkStyle::Solid);
    }

    #[test]
    fn test_add_remove_node() {
        let mut project = TopologyProject::new();
        let node = Node::new(NodeType::PC, 50.0, 50.0);
        let node_id = node.id;
        project.add_node(node);
        assert_eq!(project.nodes.len(), 1);
        project.remove_node(node_id);
        assert_eq!(project.nodes.len(), 0);
    }

    #[test]
    fn test_add_remove_link() {
        let mut project = TopologyProject::new();
        let n1 = Node::new(NodeType::Router, 0.0, 0.0);
        let n2 = Node::new(NodeType::Switch, 100.0, 0.0);
        project.add_node(n1.clone());
        project.add_node(n2.clone());
        let link = Link::new(n1.id, n2.id, LinkStyle::Solid);
        let link_id = link.id;
        project.add_link(link);
        assert_eq!(project.links.len(), 1);
        project.remove_link(link_id);
        assert_eq!(project.links.len(), 0);
    }

    #[test]
    fn test_remove_node_cascades_to_links() {
        let mut project = TopologyProject::new();
        let n1 = Node::new(NodeType::Router, 0.0, 0.0);
        let n2 = Node::new(NodeType::Switch, 100.0, 0.0);
        project.add_node(n1.clone());
        project.add_node(n2.clone());
        project.add_link(Link::new(n1.id, n2.id, LinkStyle::Solid));
        assert_eq!(project.links.len(), 1);
        project.remove_node(n1.id);
        assert_eq!(project.nodes.len(), 1);
        assert_eq!(project.links.len(), 0);
    }

    #[test]
    fn test_get_node() {
        let mut project = TopologyProject::new();
        let node = Node::new(NodeType::Server, 0.0, 0.0);
        project.add_node(node.clone());
        assert!(project.get_node(node.id).is_some());
        assert_eq!(project.get_node(node.id).unwrap().node_type, NodeType::Server);
    }

    #[test]
    fn test_json_roundtrip() {
        let mut project = TopologyProject::new();
        let n1 = Node::new(NodeType::Router, 100.0, 200.0);
        let n2 = Node::new(NodeType::PC, 300.0, 400.0);
        project.add_node(n1.clone());
        project.add_node(n2.clone());
        project.add_link(Link::new(n1.id, n2.id, LinkStyle::Dashed));

        let json = serde_json::to_string(&project).unwrap();
        let restored: TopologyProject = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.nodes.len(), 2);
        assert_eq!(restored.links.len(), 1);
        assert_eq!(restored.nodes[0].x, 100.0);
        assert_eq!(restored.nodes[0].y, 200.0);
        assert_eq!(restored.links[0].style, LinkStyle::Dashed);
    }

    #[test]
    fn test_node_type_label() {
        assert_eq!(NodeType::Router.label(), "Router");
        assert_eq!(NodeType::Switch.label(), "Switch");
        assert_eq!(NodeType::MultilayerSwitch.label(), "Multilayer Switch");
        assert_eq!(NodeType::PC.label(), "PC");
        assert_eq!(NodeType::Server.label(), "Server");
        assert_eq!(NodeType::Firewall.label(), "Firewall");
        assert_eq!(NodeType::Cloud.label(), "Cloud");
        assert_eq!(NodeType::WirelessAP.label(), "Wireless AP");
    }

    #[test]
    fn test_link_style_solid() {
        let n1 = Node::new(NodeType::Router, 0.0, 0.0);
        let n2 = Node::new(NodeType::Switch, 100.0, 0.0);
        let link = Link::new(n1.id, n2.id, LinkStyle::Solid);
        assert_eq!(link.style, LinkStyle::Solid);
    }

    #[test]
    fn test_link_style_dashed() {
        let n1 = Node::new(NodeType::Router, 0.0, 0.0);
        let n2 = Node::new(NodeType::Cloud, 100.0, 0.0);
        let link = Link::new(n1.id, n2.id, LinkStyle::Dashed);
        assert_eq!(link.style, LinkStyle::Dashed);
    }

    #[test]
    fn test_multiple_links() {
        let mut project = TopologyProject::new();
        let n1 = Node::new(NodeType::Router, 0.0, 0.0);
        let n2 = Node::new(NodeType::Switch, 100.0, 0.0);
        let n3 = Node::new(NodeType::PC, 200.0, 0.0);
        project.add_node(n1.clone());
        project.add_node(n2.clone());
        project.add_node(n3.clone());
        project.add_link(Link::new(n1.id, n2.id, LinkStyle::Solid));
        project.add_link(Link::new(n2.id, n3.id, LinkStyle::Dashed));
        assert_eq!(project.links.len(), 2);
        assert_eq!(project.links[0].style, LinkStyle::Solid);
        assert_eq!(project.links[1].style, LinkStyle::Dashed);
    }

    #[test]
    fn test_remove_node_cascades_to_all_links() {
        let mut project = TopologyProject::new();
        let n1 = Node::new(NodeType::Router, 0.0, 0.0);
        let n2 = Node::new(NodeType::Switch, 100.0, 0.0);
        let n3 = Node::new(NodeType::PC, 200.0, 0.0);
        project.add_node(n1.clone());
        project.add_node(n2.clone());
        project.add_node(n3.clone());
        project.add_link(Link::new(n1.id, n2.id, LinkStyle::Solid));
        project.add_link(Link::new(n2.id, n3.id, LinkStyle::Solid));
        assert_eq!(project.links.len(), 2);
        // Remove the middle node — both links should be removed
        project.remove_node(n2.id);
        assert_eq!(project.nodes.len(), 2);
        assert_eq!(project.links.len(), 0);
    }

    #[test]
    fn test_link_json_roundtrip() {
        let mut project = TopologyProject::new();
        let n1 = Node::new(NodeType::Router, 100.0, 200.0);
        let n2 = Node::new(NodeType::PC, 300.0, 400.0);
        project.add_node(n1.clone());
        project.add_node(n2.clone());
        project.add_link(Link::new(n1.id, n2.id, LinkStyle::Dashed));

        let json = serde_json::to_string(&project).unwrap();
        let restored: TopologyProject = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.links.len(), 1);
        assert_eq!(restored.links[0].source, n1.id);
        assert_eq!(restored.links[0].target, n2.id);
        assert_eq!(restored.links[0].style, LinkStyle::Dashed);
    }
}
