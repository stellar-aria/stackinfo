use std::collections::HashMap;
use std::path::PathBuf;

use bimap::BiMap;

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

pub struct CallGraph<'a> {
  pub graph: DiGraphMap<&'a str, String>,
  pub functions: BiMap<String, &'a str>,
}

impl CallGraph<'_> {
  pub fn new() -> CallGraph<'static> {
      CallGraph {
          graph: DiGraphMap::<&str, String>::new(),
          functions: BiMap::new(),
      }
  }

  pub fn add_function(&mut self, name: String, location: &str) {
      let node_idx = self.graph.add_node(location.clone());
      self.functions.insert(name, node_idx);
  }

  pub fn add_call(
      &mut self,
      from: String,
      to: String,
      location: String,
  ) -> Result<String, String> {
      let from_idx = *self
          .functions
          .get_by_left(&from)
          .ok_or(format!("No such source found: '{}'", from))?;
      let to_idx = *self
          .functions
          .get_by_left(&to)
          .ok_or(format!("No such target found: '{}'", to))?;
      let result = self.graph.add_edge(from_idx, to_idx, location).unwrap();
      Ok(result)
  }

  pub fn get_location(&self, function: &str) -> &str {
      self.functions.get_by_left(function).unwrap()
  }

  pub fn get_name(&self, node: &str) -> String {
      (*self.functions.get_by_right(node).unwrap()).to_string()
  }

  pub fn get_calls(&self, function: &str) -> EdgesDirected<'_, &str, String, Directed> {
      let node_idx = *self.functions.get_by_left(function).unwrap();
      self.graph.edges_directed(node_idx, Direction::Outgoing)
  }

  pub fn get_callers(&self, function: &str) -> EdgesDirected<'_, &str, String, Directed> {
      let node_idx = *self.functions.get_by_left(function).unwrap();
      self.graph.edges_directed(node_idx, Direction::Incoming)
  }

  pub fn parse_file(&mut self, path: &PathBuf) {
      let data = std::fs::read_to_string(path.clone()).expect("Unable to read file");
      let mut pairs = ci::Parser::parse(ci::Rule::object, &data)
          .unwrap_or_else(|e| panic!("{}", e));
      let syntax_tree =
          ast::Object::from_pest(&mut pairs).unwrap_or_else(|e| panic!("{}", e));

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
          self.add_function(name, label)
      }

      for edge in edges {
          let obj_map = get_object_map(edge.items);
          let label = match obj_map.get("label") {
              Some(l) => l,
              None => "intrinsic",
          };
          let source = maybe_demangle(&obj_map["sourcename"]);
          let target = maybe_demangle(&obj_map["targetname"]);
          let add = self.add_call(source, target, label.to_string());
          match add {
              Ok(_) => (),
              Err(_e) => (), //println!("Error at file {}: {}", path.display(), e),
          }
      }
  }
}
