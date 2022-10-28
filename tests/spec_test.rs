#![allow(dead_code)]
#![allow(non_upper_case_globals)]
extern crate yaml_rust_formatter;

use yaml_rust_formatter::parser::{Event, EventReceiver, Parser};
use yaml_rust_formatter::scanner::TScalarStyle;

// These names match the names used in the C++ test suite.
#[cfg_attr(feature = "cargo-clippy", allow(enum_variant_names))]
#[derive(Clone, PartialEq, PartialOrd, Debug)]
enum TestEvent {
    OnDocumentStart,
    OnDocumentEnd,
    OnSequenceStart,
    OnSequenceEnd,
    OnMapStart,
    OnMapEnd,
    OnScalar,
    OnAlias,
    OnNull,
}

struct YamlChecker {
    pub evs: Vec<TestEvent>,
}

impl EventReceiver for YamlChecker {
    fn on_event(&mut self, ev: Event) {
        let tev = match ev {
            Event::DocumentStart => TestEvent::OnDocumentStart,
            Event::DocumentEnd => TestEvent::OnDocumentEnd,
            Event::SequenceStart(..) => TestEvent::OnSequenceStart,
            Event::SequenceEnd => TestEvent::OnSequenceEnd,
            Event::MappingStart(..) => TestEvent::OnMapStart,
            Event::MappingEnd => TestEvent::OnMapEnd,
            Event::Scalar(ref v, style, _, _) => {
                if v == "~" && style == TScalarStyle::Plain {
                    TestEvent::OnNull
                } else {
                    TestEvent::OnScalar
                }
            }
            Event::Alias(_) => TestEvent::OnAlias,
            _ => return, // ignore other events
        };
        self.evs.push(tev);
    }
}

fn str_to_test_events(docs: &str) -> Vec<TestEvent> {
    println!("{}", docs);
    let mut p = YamlChecker { evs: Vec::new() };
    let mut parser = Parser::new(docs.chars());
    parser.load(&mut p, true).unwrap();
    p.evs
}

macro_rules! assert_next {
    ($v:expr, $p:pat) => {
        match $v.next().unwrap() {
            $p => {}
            e => {
                panic!("unexpected event: {:?}", e);
            }
        }
    };
}

// auto generated from handler_spec_test.cpp
include!("specexamples.rs.inc");
include!("spec_test.rs.inc");

// hand-crafted tests
//#[test]
//fn test_hc_alias() {
//}

#[test]
fn test_mapvec_legal() {
    use yaml_rust_formatter::yaml::{ArrayInput, HashInput, YamlInput};
    use yaml_rust_formatter::{YamlEmitter, YamlLoader};

    // Emitting a `map<map<seq<_>>, _>` should result in legal yaml that
    // we can parse.

    let mut key = ArrayInput::new();
    key.push(YamlInput::Integer(1));
    key.push(YamlInput::Integer(2));
    key.push(YamlInput::Integer(3));

    let mut keyhash = HashInput::new();
    keyhash.insert(YamlInput::String("key".into()), YamlInput::Array(key));

    let mut val = ArrayInput::new();
    val.push(YamlInput::Integer(4));
    val.push(YamlInput::Integer(5));
    val.push(YamlInput::Integer(6));

    let mut hash = HashInput::new();
    hash.insert(YamlInput::Hash(keyhash), YamlInput::Array(val));

    let mut out_str = String::new();
    {
        let mut emitter = YamlEmitter::new(&mut out_str);
        emitter.dump(&YamlInput::Hash(hash).into()).unwrap();
    }

    // At this point, we are tempted to naively render like this:
    //
    //  ```yaml
    //  ---
    //  {key:
    //      - 1
    //      - 2
    //      - 3}:
    //    - 4
    //    - 5
    //    - 6
    //  ```
    //
    // However, this doesn't work, because the key sequence [1, 2, 3] is
    // rendered in block mode, which is not legal (as far as I can tell)
    // inside the flow mode of the key. We need to either fully render
    // everything that's in a key in flow mode (which may make for some
    // long lines), or use the explicit map identifier '?':
    //
    //  ```yaml
    //  ---
    //  ?
    //    key:
    //      - 1
    //      - 2
    //      - 3
    //  :
    //    - 4
    //    - 5
    //    - 6
    //  ```

    YamlLoader::load_from_str(&out_str).unwrap();
}
