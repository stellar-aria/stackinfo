use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::path::PathBuf;

use bimap::{hash, BiMap, Overwritten};

use cpp_demangle::Symbol;

use pest::Parser;
use petgraph::graph::{EdgeIndex, Edges, NodeIndex};
use petgraph::graphmap::EdgesDirected;
use petgraph::prelude::{DiGraphMap, GraphMap};
use petgraph::{Directed, Direction, Graph};

use from_pest::FromPest;

mod ci {
    #[derive(pest_derive::Parser)]
    #[grammar = "callgraph_info/callgraph_info.pest"]
    pub struct Parser;
}

pub mod ast;

fn get_object_map(items: Vec<ast::Item>) -> HashMap<String, String> {
    items
        .into_iter()
        .filter_map(|i| match i {
            ast::Item::Field(ast::Field { key, value }) => Some((key, value)),
            ast::Item::Object(..) => None,
        })
        .collect()
}

fn maybe_demangle(string: &str) -> String {
    let mut split_string: Vec<&str> = string.split(":").collect();
    split_string.reverse();

    let maybe_demangled = match Symbol::new(split_string[0]) {
        Ok(demangled) => demangled.to_string(),
        Err(_) => return string.to_string(),
    };

    split_string[0] = &maybe_demangled;
    split_string.reverse();
    split_string.join(":")
}

pub struct CallGraph {
    pub graph: DiGraphMap<u64, String>,   // name reference, location
    pub functions: BiMap<String, String>, // name, location
    function_idx: BiMap<String, u64>,     // name, graph_idx
    hasher: DefaultHasher,
}

impl CallGraph {
    pub fn new() -> CallGraph {
        CallGraph {
            graph: DiGraphMap::<u64, String>::new(),
            functions: BiMap::new(),
            function_idx: BiMap::new(),
            hasher: DefaultHasher::new(),
        }
    }

    pub fn add_function(&mut self, name: &str, location: &str) {
        let name_string = name.to_string();
        self.functions
            .insert(name.to_string(), location.to_string());

        name_string.hash(&mut self.hasher);
        let hash_value = self.hasher.finish();
        self.function_idx.insert(name_string, hash_value);

        self.graph.add_node(hash_value);
    }

    pub fn add_call(&mut self, from: &str, to: &str, location: &str) -> Result<(), String> {
        let from_idx = self
            .function_idx
            .get_by_left(from)
            .ok_or(format!("No such source found: '{}'", from))?;

        let to_idx = self
            .function_idx
            .get_by_left(to)
            .ok_or(format!("No such target found: '{}'", to))?;

        self.graph
            .add_edge(*from_idx, *to_idx, location.to_owned());

        Ok(())
    }

    pub fn get_location(&self, function: &str) -> &str {
        self.functions.get_by_left(function).unwrap()
    }

    pub fn get_name(&self, node: &str) -> String {
        (*self.functions.get_by_right(node).unwrap()).to_string()
    }

    pub fn get_calls(&self, function: &str) -> EdgesDirected<'_, u64, String, Directed> {
        let node_idx = self.function_idx.get_by_left(function).unwrap();
        self.graph.edges_directed(*node_idx, Direction::Outgoing)
    }

    pub fn get_callers(&self, function: &str) -> EdgesDirected<'_, u64, String, Directed> {
        let node_idx = self.function_idx.get_by_left(function).unwrap();
        self.graph.edges_directed(*node_idx, Direction::Incoming)
    }

    pub fn parse_file(&mut self, path: &PathBuf) {
        let data = std::fs::read_to_string(path.clone()).expect("Unable to read file");
        let mut pairs =
            ci::Parser::parse(ci::Rule::object, &data).unwrap_or_else(|e| panic!("{}", e));
        let syntax_tree = ast::Object::from_pest(&mut pairs).unwrap_or_else(|e| panic!("{}", e));

        let ast::Object {
            kind: graph_kind,
            items: graph_items,
        } = syntax_tree;

        if graph_kind.as_str() != "graph" {
            panic!(
                "Error: file does not contain a graph at the toplevel {}",
                path.display()
            )
        }

        let nodes: Vec<ast::Object> = graph_items
            .clone()
            .into_iter()
            .filter_map(|item| match item {
                ast::Item::Object(o) if o.kind.as_str() == "node" => Some(o),
                _ => None,
            })
            .collect();

        let edges: Vec<ast::Object> = graph_items
            .into_iter()
            .filter_map(|item| match item {
                ast::Item::Object(o) if o.kind.as_str() == "edge" => Some(o),
                _ => None,
            })
            .collect();

        for node in nodes {
            let obj_map = get_object_map(node.items);
            let labels: Vec<&str> = obj_map["label"].split("\\n").collect();
            let label = match labels.get(1) {
                Some(s) => s,
                None => labels[0],
            };
            let name = maybe_demangle(&obj_map["title"]);
            self.add_function(&name, label)
        }

        for edge in edges {
            let obj_map = get_object_map(edge.items);
            let label = match obj_map.get("label") {
                Some(l) => l,
                None => "intrinsic",
            };
            let source = maybe_demangle(&obj_map["sourcename"]);
            let target = maybe_demangle(&obj_map["targetname"]);
            let add = self.add_call(&source, &target, label);
            match add {
                Ok(_) => (),
                Err(_e) => (), //println!("Error at file {}: {}", path.display(), e),
            }
        }
    }
}
