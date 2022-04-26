extern crate yaml_rust_formatter;

use std::env;
use std::fs::File;
use std::io::prelude::*;
use yaml_rust_formatter::yaml;

fn print_indent(indent: usize) {
    for _ in 0..indent {
        print!("    ");
    }
}

fn dump_node(doc: &yaml::YamlOutput, indent: usize) {
    match *doc {
        yaml::YamlOutput::Array(ref v) => {
            for x in v {
                dump_node(x, indent + 1);
            }
        }
        yaml::YamlOutput::Hash(ref h) => {
            for (k, v) in h {
                print_indent(indent);
                println!("{:?}:", k);
                dump_node(v, indent + 1);
            }
        }
        _ => {
            print_indent(indent);
            println!("{:?}", doc);
        }
    }
}

fn main() {
    let args: Vec<_> = env::args().collect();
    let mut f = File::open(&args[1]).unwrap();
    let mut s = String::new();
    f.read_to_string(&mut s).unwrap();

    let docs = yaml::YamlLoader::load_from_str(&s).unwrap();
    for doc in docs.into_iter() {
        let output_doc = doc.into();
        println!("---");
        dump_node(&output_doc, 0);
    }
}
