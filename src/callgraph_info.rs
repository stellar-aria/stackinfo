use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::path::PathBuf;

use bimap::BiMap;

use cpp_demangle::Symbol;

use pest::Parser;

use petgraph::graphmap::EdgesDirected;
use petgraph::prelude::DiGraphMap;
use petgraph::{Directed, Direction};

use from_pest::FromPest;

use crate::location::Location;

mod ci {
    #[derive(pest_derive::Parser)]
    #[grammar = "callgraph_info/callgraph_info.pest"]
    pub struct Parser;
}

mod ast;

fn get_object_map(items: Vec<ast::Item>) -> HashMap<&str, &str> {
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

type GraphIdx = u64;

pub struct CallGraph {
    pub graph: DiGraphMap<GraphIdx, Option<Location>>, // name reference, location
    pub locations: BiMap<GraphIdx, Option<Location>>, // graph_idx, location
    pub functions: BiMap<GraphIdx, String>,   // graph_idx, name
    hasher: DefaultHasher,
}

impl CallGraph {
    pub fn new() -> CallGraph {
        CallGraph {
            graph: DiGraphMap::<GraphIdx, Option<Location>>::new(),
            locations: BiMap::new(),
            functions: BiMap::new(),
            hasher: DefaultHasher::new(),
        }
    }

    pub fn add_function(&mut self, name: &str, location: &str) {
        let name_string = name.to_string();
        name_string.hash(&mut self.hasher);
        let location = match Location::parse(location) {
            Some((location, _)) => Some(location),
            None => None,
        };
        let hash_value = self.hasher.finish();

        self.locations.insert(hash_value, location);
        self.functions.insert( hash_value, name_string);

        self.graph.add_node(hash_value);
    }

    pub fn add_call(&mut self, from: &str, to: &str, location: &str) -> Result<(), String> {
        let from_idx = self
            .functions
            .get_by_right(from)
            .ok_or(format!("No such source found: '{}'", from))?;

        let to_idx = self
            .functions
            .get_by_right(to)
            .ok_or(format!("No such target found: '{}'", to))?;

        let location = match Location::parse(location) {
            Some((loc, _)) => Some(loc),
            None => None,
        };

        self.graph.add_edge(*from_idx, *to_idx, location);

        Ok(())
    }

    pub fn get_location(&self, function: &str) -> &Option<Location> {
        let idx = self.functions.get_by_right(function).unwrap();
        self.locations.get_by_left(idx).unwrap()
    }

    pub fn get_name(&self, loc: Location) -> &String {
        let some_loc = Some(loc);
        let idx = self.locations.get_by_right(&some_loc).unwrap();
        self.functions.get_by_left(idx).unwrap()
    }

    pub fn get_calls(&self, node_idx: GraphIdx) -> EdgesDirected<'_, u64, Option<Location>, Directed> {
        self.graph.edges_directed(node_idx, Direction::Outgoing)
    }

    pub fn get_callers(&self, node_idx: GraphIdx) -> EdgesDirected<'_, u64, Option<Location>, Directed> {
        self.graph.edges_directed(node_idx, Direction::Incoming)
    }

    pub fn parse_file(&mut self, path: &PathBuf) {
        let data = std::fs::read_to_string(path).expect("Unable to read file");
        let mut pairs =
            ci::Parser::parse(ci::Rule::object, &data).unwrap_or_else(|e| panic!("{}", e));
        let syntax_tree = ast::Object::from_pest(&mut pairs).unwrap_or_else(|e| panic!("{}", e));

        let ast::Object {
            kind: graph_kind,
            items: graph_items,
        } = syntax_tree;

        if graph_kind != "graph" {
            panic!(
                "Error: file does not contain a graph at the toplevel {}",
                path.display()
            )
        }

        let nodes: Vec<ast::Object> = graph_items
            .clone()
            .into_iter()
            .filter_map(|item| match item {
                ast::Item::Object(o) if o.kind == "node" => Some(o),
                _ => None,
            })
            .collect();

        let edges: Vec<ast::Object> = graph_items
            .into_iter()
            .filter_map(|item| match item {
                ast::Item::Object(o) if o.kind == "edge" => Some(o),
                _ => None,
            })
            .collect();

        for node in nodes {
            let obj_map = get_object_map(node.items);
            let title = obj_map["title"];
            let label = obj_map["label"];
            let location: &str = label.split("\\n").nth(1).unwrap_or(&label);
            let name = maybe_demangle(title);
            self.add_function(&name, location)
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
