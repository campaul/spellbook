use crate::Handler;

use std::collections::HashMap;

#[derive(Clone)]
pub struct Node<S: Clone> {
    children: HashMap<String, usize>,
    wildcard: Option<(String, usize)>,
    handler: Option<Handler<S>>,
}

#[derive(Clone)]
pub struct Tree<S: Clone> {
    nodes: Vec<Node<S>>,
}

impl<S: Clone> Tree<S> {
    pub fn new() -> Tree<S> {
        let mut tree = Tree { nodes: vec![] };

        tree.node_new();

        tree
    }

    pub fn node_new(&mut self) -> usize {
        let next_index = self.nodes.len();

        self.nodes.push(Node {
            children: HashMap::new(),
            wildcard: None,
            handler: None,
        });

        next_index
    }

    pub fn node_add_child(&mut self, parent_id: usize, segment: String) -> usize {
        if let Some(_) = self.nodes[parent_id].wildcard {
            panic!("Literal conflicts with route params.");
        }

        match self.nodes[parent_id].children.get(&segment) {
            Some(&node_id) => node_id,
            None => {
                let node_id = self.node_new();
                self.nodes[parent_id].children.insert(segment, node_id);
                node_id
            }
        }
    }

    pub fn node_get_child(&self, node_id: usize, segment: String) -> Option<&usize> {
        self.nodes[node_id].children.get(&segment)
    }

    pub fn node_set_wildcard(&mut self, parent_id: usize, segment: String) -> usize {
        if self.nodes[parent_id].children.len() > 0 {
            panic!("Route param conflicts with literals.");
        }

        if let Some((ref s, _)) = self.nodes[parent_id].wildcard {
            if *s != segment {
                panic!("Ambiguous route param.");
            };
        }

        let node_id = match self.nodes[parent_id].wildcard {
            Some((_, node_id)) => node_id,
            None => {
                let node_id = self.node_new();
                self.nodes[parent_id].wildcard = Some((segment, node_id));
                node_id
            }
        };

        node_id
    }

    pub fn node_get_wildcard(&self, node_id: usize) -> Option<(String, usize)> {
        self.nodes[node_id].wildcard.clone()
    }

    pub fn node_set_handler(&mut self, node_id: usize, handler: Handler<S>) {
        if let Some(_) = self.nodes[node_id].handler {
            panic!("Handler already registered for route.");
        }

        self.nodes[node_id].handler = Some(handler);
    }

    pub fn node_get_handler(&self, node_id: usize) -> Option<Handler<S>> {
        self.nodes[node_id].handler
    }
}
