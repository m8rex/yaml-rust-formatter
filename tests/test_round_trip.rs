extern crate yaml_rust_formatter;

use yaml_rust_formatter::{YamlEmitter, YamlInput, YamlLoader, YamlOutput};

fn roundtrip(original: &YamlInput) {
    let output: YamlOutput = original.clone().into();
    let mut emitted = String::new();
    YamlEmitter::new(&mut emitted).dump(&output).unwrap();

    let documents = YamlLoader::load_from_str(&emitted).unwrap();
    println!("emitted {}", emitted);

    assert_eq!(documents.len(), 1);
    assert_eq!(documents[0], *original);
}

fn double_roundtrip(original: &str) {
    let parsed = YamlLoader::load_from_str(&original).unwrap();

    let output: YamlOutput = parsed[0].clone().into();
    let mut serialized = String::new();
    YamlEmitter::new(&mut serialized).dump(&output).unwrap();

    let reparsed = YamlLoader::load_from_str(&serialized).unwrap();

    assert_eq!(parsed, reparsed);
}

#[test]
fn test_escape_character() {
    let y = YamlInput::String("\x1b".to_owned());
    roundtrip(&y);
}

#[test]
fn test_colon_in_string() {
    let y = YamlInput::String("x: %".to_owned());
    roundtrip(&y);
}

#[test]
fn test_numberlike_strings() {
    let docs = [
        r#"x: "1234""#,
        r#"x: "01234""#,
        r#""1234""#,
        r#""01234""#,
        r#"" 01234""#,
        r#""0x1234""#,
        r#"" 0x1234""#,
    ];

    for doc in &docs {
        roundtrip(&YamlInput::String(doc.to_string()));
        double_roundtrip(&doc);
    }
}

/// Example from https://github.com/chyh1990/yaml-rust/issues/133
#[test]
fn test_issue133() {
    let doc = YamlLoader::load_from_str("\"0x123\"")
        .unwrap()
        .pop()
        .unwrap();
    assert_eq!(doc, YamlInput::String("0x123".to_string()));

    let output: YamlOutput = doc.clone().into();
    let mut out_str = String::new();
    YamlEmitter::new(&mut out_str).dump(&output).unwrap();
    let doc2 = YamlLoader::load_from_str(&out_str).unwrap().pop().unwrap();
    assert_eq!(doc, doc2); // This failed because the type has changed to a number now
}
