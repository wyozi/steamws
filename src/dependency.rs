use std::collections::{HashMap, HashSet};
use std::fmt::Debug;
use petgraph::Graph;
use petgraph::graph::NodeIndex;
use petgraph::dot::{Dot, Config};

pub type NodeRef = NodeIndex<u32>;

pub struct DependencyGraph<Node, Edge> {
    root_ref: NodeRef,
    graph: Graph<Node, Edge>
}

impl <Node: Debug, Edge: Debug> DependencyGraph<Node, Edge> {
    pub fn new(root: Node) -> DependencyGraph<Node, Edge> {
        let mut g = Graph::new();
        DependencyGraph {
            root_ref: g.add_node(root),
            graph: g
        }
    }

    pub fn dot(&self) -> String {
        format!("{:?}", Dot::with_config(&self.graph, &[Config::EdgeNoLabel]))
    }

    pub fn flatten(self) -> Vec<Node> {
        self.graph.into_nodes_edges().0.into_iter().map(|n| n.weight).collect()
    }

    pub fn insert(&mut self, n: Node, e: Edge) -> NodeRef {
        let r = self.graph.add_node(n);
        self.graph.add_edge(self.root_ref, r, e);
        r
    }

    pub fn insert_sub(&mut self, parent: NodeRef, n: Node, e: Edge) -> NodeRef {
        let r = self.graph.add_node(n);
        self.graph.add_edge(parent, r, e);
        r
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn it_works() {
        let mut g: DependencyGraph<&str, ()> = DependencyGraph::new("Foo");
        g.insert("Bar", ());

        assert_eq!(2 + 2, 4);
    }
}