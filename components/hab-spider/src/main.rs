
// Copyright (c) 2017 Chef Software Inc. and/or applicable contributors
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

#[macro_use]
extern crate log;
extern crate env_logger;
extern crate time;
extern crate petgraph;
extern crate walkdir;
extern crate habitat_core as hab_core;
extern crate builder_core as bldr_core;
extern crate habitat_builder_protocol as protocol;
extern crate clap;

use std::io::{self, Write};
use clap::{Arg, App};
use time::PreciseTime;
use bldr_core::package_graph::PackageGraph;
use bldr_core::file_walker::FileWalker;

fn main() {
    env_logger::init().unwrap();

    let matches = App::new("hab-spider")
        .version("0.1.0")
        .about("Habitat package graph builder")
        .arg(Arg::with_name("PATH")
            .help("The path to the packages root")
            .required(true)
            .index(1))
        .get_matches();

    let path = matches.value_of("PATH").unwrap();

    println!("Crawling packages... please wait.");

    let mut graph = PackageGraph::new();
    let start_time = PreciseTime::now();
    let (ncount, ecount) = graph.build(FileWalker::new(&path));
    let end_time = PreciseTime::now();

    println!("OK: {} nodes, {} edges ({} sec)",
             ncount,
             ecount,
             start_time.to(end_time));

    println!("\nAvailable commands: help, stats, top, find, rdeps, exit\n");

    let mut done = false;
    while !done {
        print!("spider> ");
        io::stdout().flush().ok().expect("Could not flush stdout");

        let mut cmd = String::new();
        io::stdin().read_line(&mut cmd).unwrap();

        let v: Vec<&str> = cmd.trim_right().split_whitespace().collect();

        if v.len() > 0 {
            match v[0].to_lowercase().as_str() {
                "help" => do_help(),
                "stats" => do_stats(&graph),
                "top" => {
                    let count = if v.len() < 2 {
                        10
                    } else {
                        v[1].parse::<usize>().unwrap()
                    };
                    do_top(&graph, count);
                }
                "find" => {
                    if v.len() < 2 {
                        println!("Missing search term\n")
                    } else {
                        let max = if v.len() > 2 {
                            v[2].parse::<usize>().unwrap()
                        } else {
                            10
                        };
                        do_find(&graph, v[1].to_lowercase().as_str(), max)
                    }
                }
                "rdeps" => {
                    if v.len() < 2 {
                        println!("Missing package name\n")
                    } else {
                        let max = if v.len() > 2 {
                            v[2].parse::<usize>().unwrap()
                        } else {
                            10
                        };
                        do_rdeps(&graph, v[1].to_lowercase().as_str(), max)
                    }
                }
                "exit" => done = true,
                _ => println!("Unknown command\n"),
            }
        }
    }
}

fn do_help() {
    println!("COMMANDS:");
    println!("  help                    Print this message");
    println!("  stats                   Print graph statistics");
    println!("  top [<count>]           Print nodes with the most reverse dependencies");
    println!("  find  <term> [<max>]    Find packages that match the search term, up to max items");
    println!("  rdeps <name> [<max>]    Print the reverse dependencies for the package, up to max");
    println!("  exit                    Exit the application\n");
}

fn do_stats(graph: &PackageGraph) {
    let stats = graph.stats();

    println!("Node count: {}", stats.node_count);
    println!("Edge count: {}", stats.edge_count);
    println!("Connected components: {}", stats.connected_comp);
    println!("Is cyclic: {}\n", stats.is_cyclic);
}

fn do_top(graph: &PackageGraph, count: usize) {
    let start_time = PreciseTime::now();
    let top = graph.top(count);
    let end_time = PreciseTime::now();

    println!("OK: {} items ({} sec)\n",
             top.len(),
             start_time.to(end_time));

    for (name, count) in top {
        println!("{}: {}", name, count);
    }
    println!("");
}

fn do_find(graph: &PackageGraph, phrase: &str, max: usize) {
    let start_time = PreciseTime::now();
    let mut v = graph.search(phrase);
    let end_time = PreciseTime::now();

    println!("OK: {} items ({} sec)\n", v.len(), start_time.to(end_time));

    if v.is_empty() {
        println!("No matching packages found")
    } else {
        if v.len() > max {
            v.drain(max..);
        }
        for s in v {
            println!("{}", s);
        }
    }

    println!("");
}

fn do_rdeps(graph: &PackageGraph, name: &str, max: usize) {
    let start_time = PreciseTime::now();

    match graph.rdeps(name) {
        Some(mut rdeps) => {
            let end_time = PreciseTime::now();
            println!("OK: {} items ({} sec)\n",
                     rdeps.len(),
                     start_time.to(end_time));

            if rdeps.len() > max {
                rdeps.drain(max..);
            }

            for s in rdeps {
                println!("{}", s);
            }
        }
        None => println!("No entries found"),
    }

    println!("");
}
