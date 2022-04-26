extern crate yaml_rust_formatter;
#[macro_use]
extern crate quickcheck;

use quickcheck::TestResult;
use yaml_rust_formatter::{YamlEmitter, YamlLoader, YamlOutput};

quickcheck! {
    fn test_check_weird_keys(xs: Vec<String>) -> TestResult {
        let mut out_str = String::new();
        let input = YamlOutput::Array(xs.into_iter().map(YamlOutput::String).collect());
        {
            let mut emitter = YamlEmitter::new(&mut out_str);
            emitter.dump(&input).unwrap();
        }
        match YamlLoader::load_from_str(&out_str) {
            Ok(output) => TestResult::from_bool(output.len() == 1 && input == output[0].clone().into()),
            Err(err) => TestResult::error(err.to_string()),
        }
    }
}
