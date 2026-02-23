use glob::glob;
use indicatif::{ProgressBar, ProgressStyle};

use rayon::iter::ParallelIterator;

use std::path::PathBuf;

mod callgraph_info;
mod location;
mod stack_usage;

use crate::callgraph_info::CallGraph;
use crate::stack_usage::StackUsage;

#[derive(clap::Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long, action = clap::ArgAction::SetTrue)]
    verbose: bool,

    path: PathBuf,
}

fn main() -> Result<(), std::io::Error> {
    let args = <Args as clap::Parser>::parse();

    let path_str = args
        .path
        .to_str()
        .expect("Could not convert path to string")
        .to_owned();

    let ci_glob = path_str.clone() + "/**/*.ci";
    let callgraph_info_files: Vec<PathBuf> = glob(ci_glob.as_str())
        .expect("Failed to find callgraph-info files!")
        .flatten()
        .collect();

    let su_glob = path_str.clone() + "/**/*.su";
    let stack_usage_files: Vec<PathBuf> = glob(su_glob.as_str())
        .expect("Failed to find any stack-usage files!")
        .flatten()
        .collect();

    let mut call_graph = CallGraph::new();

    let sty = ProgressStyle::with_template("[{pos}/{len}] {msg} {spinner:.green}").unwrap();
    let ci_pb = ProgressBar::new(stack_usage_files.len().try_into().unwrap());
    ci_pb.set_style(sty.clone());
    ci_pb.set_message("Loading callgraph info...");

    for path in callgraph_info_files.iter() {
        ci_pb.inc(1);
        call_graph.parse_file(path);
    }
    ci_pb.finish_with_message("Loading callgraph info files... Done!");

    //println!("Nodes: {:?}", call_graph.graph.nodes().len());
    //println!("Edges: {:?}", call_graph.graph.all_edges().count());

    let su_pb = ProgressBar::new(stack_usage_files.len().try_into().unwrap());
    su_pb.set_style(sty);
    su_pb.set_message("Loading stack usage files... Done!");

    let mut stack_usages: Vec<StackUsage> = stack_usage_files
        .iter()
        .flat_map(|path| {
            su_pb.inc(1);
            let data = std::fs::read_to_string(path).expect("Unable to read file");
            data.lines().map(StackUsage::parse).collect::<Vec<_>>()
        })
        .collect();

    su_pb.finish_with_message("Loading stack usage files... Done!");

    stack_usages.sort_unstable();
    stack_usages.reverse();

    println!("Top ten largest stack-using functions:");

    for usage in stack_usages.iter().take(10) {
        let stack = usage.stack_usage;
        let function = &usage.function;
        let name = &function.name;
        println!("  ({stack}) {name}");
    }

    Ok(())
}
