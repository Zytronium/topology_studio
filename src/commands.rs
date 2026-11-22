use crate::model::TopologyProject;

/// A command that can be executed and undone.
/// Uses snapshot-based approach: each command stores the project state
/// before and after the change for simple undo/redo.
#[derive(Clone, Debug)]
pub struct Command {
    pub before: TopologyProject,
    pub after: TopologyProject,
}

impl Command {
    pub fn new(before: TopologyProject, after: TopologyProject) -> Self {
        Self { before, after }
    }

    /// Execute this command, applying the "after" state to the project.
    pub fn execute(&self, project: &mut TopologyProject) {
        *project = self.after.clone();
    }

    /// Undo this command, restoring the "before" state.
    pub fn undo(&self, project: &mut TopologyProject) {
        *project = self.before.clone();
    }
}

/// Undo/redo stack that manages a history of commands.
pub struct UndoRedoStack {
    undo_stack: Vec<Command>,
    redo_stack: Vec<Command>,
    max_history: usize,
}

impl UndoRedoStack {
    pub fn new(max_history: usize) -> Self {
        Self {
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            max_history,
        }
    }

    /// Push a new command onto the undo stack.
    /// Clears the redo stack if the user has undone and then makes a new change.
    pub fn execute(&mut self, command: Command) {
        self.undo_stack.push(command);
        // Limit history size
        if self.undo_stack.len() > self.max_history {
            self.undo_stack.remove(0);
        }
        // Clear redo stack on new action
        self.redo_stack.clear();
    }

    /// Undo the last command, returning true if a command was undone.
    pub fn undo(&mut self, project: &mut TopologyProject) -> bool {
        if let Some(command) = self.undo_stack.pop() {
            command.undo(project);
            self.redo_stack.push(command);
            true
        } else {
            false
        }
    }

    /// Redo the last undone command, returning true if a command was redone.
    pub fn redo(&mut self, project: &mut TopologyProject) -> bool {
        if let Some(command) = self.redo_stack.pop() {
            command.execute(project);
            self.undo_stack.push(command);
            true
        } else {
            false
        }
    }

    /// Check if there are commands that can be undone.
    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }

    /// Check if there are commands that can be redone.
    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }

    /// Clear all history (e.g., when starting a new project).
    pub fn clear(&mut self) {
        self.undo_stack.clear();
        self.redo_stack.clear();
    }
}

impl Default for UndoRedoStack {
    fn default() -> Self {
        Self::new(50)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Node, NodeType, Link, LinkStyle};

    #[test]
    fn test_undo_add_node() {
        let mut project = TopologyProject::new();
        let mut stack = UndoRedoStack::new(10);

        let before = project.clone();
        let mut after = project.clone();
        after.add_node(Node::new(NodeType::Router, 100.0, 200.0));

        stack.execute(Command::new(before, after.clone()));
        assert_eq!(project.nodes.len(), 0);
        stack.undo(&mut project);
        assert_eq!(project.nodes.len(), 0);
    }

    #[test]
    fn test_redo_add_node() {
        let mut project = TopologyProject::new();
        let mut stack = UndoRedoStack::new(10);

        let before = project.clone();
        let mut after = project.clone();
        after.add_node(Node::new(NodeType::Router, 100.0, 200.0));

        stack.execute(Command::new(before, after.clone()));
        // Apply the command to the project
        stack.undo(&mut project);
        assert_eq!(project.nodes.len(), 0);
        stack.redo(&mut project);
        assert_eq!(project.nodes.len(), 1);
    }

    #[test]
    fn test_multiple_undo_redo() {
        let mut project = TopologyProject::new();
        let mut stack = UndoRedoStack::new(10);

        // Add two nodes
        let mut p1 = project.clone();
        p1.add_node(Node::new(NodeType::Router, 100.0, 0.0));
        stack.execute(Command::new(project.clone(), p1.clone()));

        let mut p2 = p1.clone();
        p2.add_node(Node::new(NodeType::Switch, 200.0, 0.0));
        stack.execute(Command::new(p1, p2));

        // Undo twice
        stack.undo(&mut project);
        assert_eq!(project.nodes.len(), 1);
        stack.undo(&mut project);
        assert_eq!(project.nodes.len(), 0);

        // Redo twice
        stack.redo(&mut project);
        assert_eq!(project.nodes.len(), 1);
        stack.redo(&mut project);
        assert_eq!(project.nodes.len(), 2);
    }

    #[test]
    fn test_redo_cleared_on_new_action() {
        let mut project = TopologyProject::new();
        let mut stack = UndoRedoStack::new(10);

        let mut p1 = project.clone();
        p1.add_node(Node::new(NodeType::Router, 100.0, 0.0));
        stack.execute(Command::new(project.clone(), p1.clone()));

        let mut p2 = p1.clone();
        p2.add_node(Node::new(NodeType::Switch, 200.0, 0.0));
        stack.execute(Command::new(p1, p2));

        // Undo one
        stack.undo(&mut project);
        assert!(stack.can_redo());

        // New action clears redo
        let mut p3 = project.clone();
        p3.add_node(Node::new(NodeType::PC, 300.0, 0.0));
        stack.execute(Command::new(project.clone(), p3));
        assert!(!stack.can_redo());
    }

    #[test]
    fn test_undo_remove_node() {
        let mut project = TopologyProject::new();
        let mut stack = UndoRedoStack::new(10);

        let node = Node::new(NodeType::Router, 100.0, 200.0);
        let mut before = TopologyProject::new();
        before.add_node(node);

        let after = TopologyProject::new();
        stack.execute(Command::new(before.clone(), after));

        // Undo restores the "before" state, which has the node
        stack.undo(&mut project);
        assert_eq!(project.nodes.len(), 1);

        // Redo applies the "after" state, which is empty
        stack.redo(&mut project);
        assert_eq!(project.nodes.len(), 0);
    }

    #[test]
    fn test_undo_link_addition() {
        let mut project = TopologyProject::new();
        let mut stack = UndoRedoStack::new(10);

        let n1 = Node::new(NodeType::Router, 0.0, 0.0);
        let n2 = Node::new(NodeType::Switch, 100.0, 0.0);

        let mut before = TopologyProject::new();
        before.add_node(n1.clone());
        before.add_node(n2.clone());

        let mut after = before.clone();
        after.add_link(Link::new(n1.id, n2.id, LinkStyle::Solid));

        stack.execute(Command::new(before, after));
        stack.undo(&mut project);
        assert_eq!(project.links.len(), 0);
        stack.redo(&mut project);
        assert_eq!(project.links.len(), 1);
    }

    #[test]
    fn test_can_undo_redo() {
        let mut stack = UndoRedoStack::new(10);
        assert!(!stack.can_undo());
        assert!(!stack.can_redo());

        let cmd = Command::new(TopologyProject::new(), TopologyProject::new());
        stack.execute(cmd);
        assert!(stack.can_undo());
        assert!(!stack.can_redo());

        let mut project = TopologyProject::new();
        stack.undo(&mut project);
        assert!(!stack.can_undo());
        assert!(stack.can_redo());
    }

    #[test]
    fn test_clear_history() {
        let mut stack = UndoRedoStack::new(10);
        let cmd = Command::new(TopologyProject::new(), TopologyProject::new());
        stack.execute(cmd);
        assert!(stack.can_undo());
        stack.clear();
        assert!(!stack.can_undo());
        assert!(!stack.can_redo());
    }

    #[test]
    fn test_max_history_limit() {
        let mut stack = UndoRedoStack::new(3);
        for i in 0..5 {
            let mut before = TopologyProject::new();
            if i > 0 {
                before.add_node(Node::new(NodeType::Router, i as f64, 0.0));
            }
            let mut after = before.clone();
            after.add_node(Node::new(NodeType::Router, (i + 1) as f64, 0.0));
            stack.execute(Command::new(before, after));
        }
        // Should only have 3 commands in the stack
        assert_eq!(stack.undo_stack.len(), 3);
    }
}
