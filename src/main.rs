use glob::glob;
use indicatif::{ProgressBar, ProgressStyle, ProgressIterator};

use indicatif::ParallelProgressIterator;
use rayon::iter::{ParallelIterator, IntoParallelRefIterator};

use std::string::ToString;
use std::{collections::HashMap, fs, path::PathBuf};

use crate::callgraph_info::CallGraph;

mod callgraph_info;

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
        .into_iter()
        .flatten()
        .collect();

    let su_glob = path_str.clone() + "/**/*.su";
    let stack_usage_files = glob(su_glob.as_str()).expect("Failed to find any stack-usage files!");

    let mut call_graph = CallGraph::new();

    let sty = ProgressStyle::with_template(
        "[{elapsed_precise}] {bar:.white} {pos:>7}/{len:7} {msg:>}",
    )
    .unwrap()
    .progress_chars("##-");

    //let bar = ProgressBar::new(callgraph_info_files.len().try_into().unwrap());
    //bar.set_style(sty);
    //bar.set_message("Loading callgraph info files");

    callgraph_info_files.iter().progress_with_style(sty).for_each(|path| {
        call_graph.parse_file(path);
    });
    //bar.finish();

    println!("Nodes: {:?}", call_graph.graph.nodes().len());
    println!("Edges: {:?}", call_graph.graph.all_edges().count());

    Ok(())
}
