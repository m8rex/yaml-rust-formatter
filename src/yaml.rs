use crate::parser::*;
use crate::scanner::{Marker, ScanError, TScalarStyle, TokenType};
use linked_hash_map::LinkedHashMap;
use std::collections::BTreeMap;
use std::f64;
use std::i64;
use std::mem;
use std::ops::Index;
use std::string;
use std::vec;

/// Based on yaml_rust
/// A read YAML node is stored as this `YamlInput` enumeration, which provides an easy way to
/// access your YAML document.
///
/// # Examples
///
/// ```
/// use yaml_rust_formatter::YamlInput;
/// let foo = YamlInput::from_str("-123"); // convert the string to the appropriate YAML type
/// assert_eq!(foo.as_i64().unwrap(), -123);
/// // iterate over an Array
/// let vec = YamlInput::Array(vec![YamlInput::Integer(1), YamlInput::Integer(2)]);
/// for v in vec.as_vec().unwrap() {
///     assert!(v.as_i64().is_some());
/// }
/// ```
#[derive(Clone, PartialEq, PartialOrd, Debug, Eq, Ord, Hash)]
pub enum YamlInput {
    /// Float types are stored as String and parsed on demand.
    /// Note that f64 does NOT implement Eq trait and can NOT be stored in BTreeMap.
    Real(string::String),
    /// YAML int is stored as i64.
    Integer(i64),
    /// YAML scalar.
    String(string::String),
    /// YAML bool, e.g. `true` or `false`.
    Boolean(bool),
    /// YAML array, can be accessed as a `Vec`.
    Array(self::ArrayInput),
    /// YAML hash, can be accessed as a `LinkedHashMap`.
    ///
    /// Insertion order will match the order of insertion into the map.
    Hash(self::HashInput),
    /// Anchored: The name and the value
    Anchored(string::String, Box<YamlInput>),
    /// Aliased: The name and the value, the value is only none if the anchor that is aliased doesn't exist
    Aliased(string::String, Option<Box<YamlInput>>),
    /// YAML null, e.g. `null` or `~`.
    Null,
    /// Accessing a nonexistent node via the Index trait returns `BadValue`. This
    /// simplifies error handling in the calling code. Invalid type conversion also
    /// returns `BadValue`.
    BadValue,
}

pub type ArrayInput = Vec<YamlInput>;
pub type HashInput = LinkedHashMap<YamlInput, YamlInput>;

/// A write YAML node is stored as this `YamlOutput` enumeration, which provides an easy way to
/// write your YAML document.
///
/// # Examples
///
/// ```
/// use yaml_rust_formatter::YamlOutput;
/// let vec = YamlOutput::Array(vec![YamlOutput::Integer(1), YamlOutput::Integer(2)]);
/// ```
#[derive(Clone, PartialEq, PartialOrd, Debug, Eq, Ord, Hash)]
pub enum YamlOutput {
    /// Float types are stored as String and parsed on demand.
    /// Note that f64 does NOT implement Eq trait and can NOT be stored in BTreeMap.
    Real(string::String),
    /// YAML int is stored as i64.
    Integer(i64),
    /// YAML scalar.
    String(string::String),
    /// YAML bool, e.g. `true` or `false`.
    Boolean(bool),
    /// YAML array, can be accessed as a `Vec`.
    Array(self::ArrayOutput),
    /// YAML hash, can be accessed as a `LinkedHashMap`.
    ///
    /// Insertion order will match the order of insertion into the map.
    Hash(self::HashOutput),
    /// Anchored data: The name and the value
    Anchored(string::String, Box<YamlOutput>),
    /// Alias
    Alias(string::String),
    /// YAML null, e.g. `null` or `~`.
    Null,
    /// Accessing a nonexistent node via the Index trait returns `BadValue`. This
    /// simplifies error handling in the calling code. Invalid type conversion also
    /// returns `BadValue`.
    BadValue,
}

pub type ArrayOutput = Vec<YamlOutput>;
pub type HashOutput = LinkedHashMap<YamlOutput, YamlOutput>;

impl std::convert::Into<YamlOutput> for YamlInput {
    fn into(self) -> YamlOutput {
        match self {
            Self::Real(s) => YamlOutput::Real(s),
            Self::Integer(i) => YamlOutput::Integer(i),
            Self::String(s) => YamlOutput::String(s),
            Self::Boolean(b) => YamlOutput::Boolean(b),
            Self::Array(v) => YamlOutput::Array(v.into_iter().map(|a| a.into()).collect()),
            Self::Hash(h) => {
                YamlOutput::Hash(h.into_iter().map(|(k, v)| (k.into(), v.into())).collect())
            }
            Self::Anchored(s, i) => YamlOutput::Anchored(s, Box::new((*i).into())),
            Self::Aliased(s, _) => YamlOutput::Alias(s),
            Self::Null => YamlOutput::Null,
            Self::BadValue => YamlOutput::BadValue,
        }
    }
}

// parse f64 as Core schema
// See: https://github.com/chyh1990/yaml-rust/issues/51
fn parse_f64(v: &str) -> Option<f64> {
    match v {
        ".inf" | ".Inf" | ".INF" | "+.inf" | "+.Inf" | "+.INF" => Some(f64::INFINITY),
        "-.inf" | "-.Inf" | "-.INF" => Some(f64::NEG_INFINITY),
        ".nan" | "NaN" | ".NAN" => Some(f64::NAN),
        _ => v.parse::<f64>().ok(),
    }
}

pub struct YamlLoader {
    docs: Vec<YamlInput>,
    // states
    // (current node, anchor) tuple
    doc_stack: Vec<(YamlInput, Option<String>)>,
    key_stack: Vec<YamlInput>,
    anchor_map: BTreeMap<String, YamlInput>,
}

impl MarkedEventReceiver for YamlLoader {
    fn on_event(&mut self, ev: Event, _: Marker) {
        // println!("EV {:?}", ev);
        match ev {
            Event::DocumentStart => {
                // do nothing
            }
            Event::DocumentEnd => {
                match self.doc_stack.len() {
                    // empty document
                    0 => self.docs.push(YamlInput::BadValue),
                    1 => self.docs.push(self.doc_stack.pop().unwrap().0),
                    _ => unreachable!(),
                }
            }
            Event::SequenceStart(aid) => {
                self.doc_stack.push((YamlInput::Array(Vec::new()), aid));
            }
            Event::SequenceEnd => {
                let node = self.doc_stack.pop().unwrap();
                if let Some(anchor) = node.1 {
                    self.insert_new_node((
                        YamlInput::Anchored(anchor.clone(), Box::new(node.0)),
                        Some(anchor.clone()),
                    ));
                } else {
                    self.insert_new_node(node);
                }
            }
            Event::MappingStart(aid) => {
                self.doc_stack
                    .push((YamlInput::Hash(HashInput::new()), aid));
                self.key_stack.push(YamlInput::BadValue);
            }
            Event::MappingEnd => {
                self.key_stack.pop().unwrap();
                let node = self.doc_stack.pop().unwrap();
                if let Some(anchor) = node.1 {
                    self.insert_new_node((
                        YamlInput::Anchored(anchor.clone(), Box::new(node.0)),
                        Some(anchor.clone()),
                    ));
                } else {
                    self.insert_new_node(node);
                }
            }
            Event::Scalar(v, style, aid, tag) => {
                let node = if style != TScalarStyle::Plain {
                    YamlInput::String(v)
                } else if let Some(TokenType::Tag(ref handle, ref suffix)) = tag {
                    // XXX tag:yaml.org,2002:
                    if handle == "!!" {
                        match suffix.as_ref() {
                            "bool" => {
                                // "true" or "false"
                                match v.parse::<bool>() {
                                    Err(_) => YamlInput::BadValue,
                                    Ok(v) => YamlInput::Boolean(v),
                                }
                            }
                            "int" => match v.parse::<i64>() {
                                Err(_) => YamlInput::BadValue,
                                Ok(v) => YamlInput::Integer(v),
                            },
                            "float" => match parse_f64(&v) {
                                Some(_) => YamlInput::Real(v),
                                None => YamlInput::BadValue,
                            },
                            "null" => match v.as_ref() {
                                "~" | "null" => YamlInput::Null,
                                _ => YamlInput::BadValue,
                            },
                            _ => YamlInput::String(v),
                        }
                    } else {
                        YamlInput::String(v)
                    }
                } else {
                    // Datatype is not specified, or unrecognized
                    YamlInput::from_str(&v)
                };

                if let Some(anchor) = aid {
                    self.insert_new_node((
                        YamlInput::Anchored(anchor.clone(), Box::new(node)),
                        Some(anchor.clone()),
                    ));
                } else {
                    self.insert_new_node((node, None));
                }
            }
            Event::Alias(id) => {
                let node = YamlInput::Aliased(
                    id.clone(),
                    self.anchor_map
                        .get(&id)
                        .clone()
                        .map(|a| Box::new(a.clone())),
                );
                self.insert_new_node((node, None));
            }
            _ => { /* ignore */ }
        }
        // println!("DOC {:?}", self.doc_stack);
    }
}

impl YamlLoader {
    fn insert_new_node(&mut self, node: (YamlInput, Option<String>)) {
        // valid anchor id starts from 1
        if let Some(anchor) = node.1.as_ref() {
            self.anchor_map.insert(anchor.clone(), node.0.clone());
        }
        if self.doc_stack.is_empty() {
            self.doc_stack.push(node);
        } else {
            let parent = self.doc_stack.last_mut().unwrap();
            match *parent {
                (YamlInput::Array(ref mut v), _) => v.push(node.0),
                (YamlInput::Hash(ref mut h), _) => {
                    let cur_key = self.key_stack.last_mut().unwrap();
                    // current node is a key
                    if cur_key.is_badvalue() {
                        *cur_key = node.0;
                    // current node is a value
                    } else {
                        let mut newkey = YamlInput::BadValue;
                        mem::swap(&mut newkey, cur_key);
                        h.insert(newkey, node.0);
                    }
                }
                _ => unreachable!(),
            }
        }
    }

    pub fn load_from_str(source: &str) -> Result<Vec<YamlInput>, ScanError> {
        let mut loader = YamlLoader {
            docs: Vec::new(),
            doc_stack: Vec::new(),
            key_stack: Vec::new(),
            anchor_map: BTreeMap::new(),
        };
        let mut parser = Parser::new(source.chars());
        parser.load(&mut loader, true)?;
        Ok(loader.docs)
    }
}

macro_rules! define_as (
    ($name:ident, $t:ident, $yt:ident) => (
pub fn $name(&self) -> Option<$t> {
    match *self {
        Self::$yt(v) => Some(v),
        Self::Aliased(ref _s, ref v_opt) => v_opt.as_ref().map(|v| v.$name()).flatten(),
        Self::Anchored(ref _s, ref v) => v.$name(),
        _ => None
    }
}
    );
);

macro_rules! define_as_ref (
    ($name:ident, $t:ty, $yt:ident) => (
pub fn $name(&self) -> Option<$t> {
    match *self {
        Self::$yt(ref v) => Some(v),
        Self::Aliased(ref _s, ref v_opt) => v_opt.as_ref().map(|v| v.$name()).flatten(),
        Self::Anchored(ref _s, ref v) => v.$name(),
        _ => None
    }
}
    );
);

macro_rules! define_into (
    ($name:ident, $t:ty, $yt:ident) => (
pub fn $name(self) -> Option<$t> {
    match self {
        Self::$yt(v) => Some(v),
        Self::Aliased(_s, v_opt) => v_opt.map(|v| v.$name()).flatten(),
        Self::Anchored(_s, v) => v.$name(),
        _ => None
    }
}
    );
);

impl YamlInput {
    define_as!(as_bool, bool, Boolean);
    define_as!(as_i64, i64, Integer);

    define_as_ref!(as_str, &str, String);
    define_as_ref!(as_hash, &HashInput, Hash);
    define_as_ref!(as_vec, &ArrayInput, Array);

    define_into!(into_bool, bool, Boolean);
    define_into!(into_i64, i64, Integer);
    define_into!(into_string, String, String);
    define_into!(into_hash, HashInput, Hash);
    define_into!(into_vec, ArrayInput, Array);

    pub fn is_null(&self) -> bool {
        matches!(*self, Self::Null)
    }

    pub fn is_badvalue(&self) -> bool {
        matches!(*self, Self::BadValue)
    }

    pub fn is_array(&self) -> bool {
        matches!(*self, Self::Array(_))
    }

    pub fn as_f64(&self) -> Option<f64> {
        match *self {
            Self::Real(ref v) => parse_f64(v),
            _ => None,
        }
    }

    pub fn into_f64(self) -> Option<f64> {
        match self {
            Self::Real(ref v) => parse_f64(v),
            _ => None,
        }
    }
}

#[cfg_attr(feature = "cargo-clippy", allow(should_implement_trait))]
impl YamlInput {
    // Not implementing FromStr because there is no possibility of Error.
    // This function falls back to Yaml::String if nothing else matches.
    pub fn from_str(v: &str) -> Self {
        if let Some(value) = v.strip_prefix("0x") {
            if let Ok(i) = i64::from_str_radix(&value, 16) {
                return Self::Integer(i);
            }
        }
        if let Some(value) = v.strip_prefix("0o") {
            if let Ok(i) = i64::from_str_radix(&value, 8) {
                return Self::Integer(i);
            }
        }
        if let Some(value) = v.strip_prefix('+') {
            if let Ok(i) = value.parse::<i64>() {
                return Self::Integer(i);
            }
        }
        match v {
            "~" | "null" => Self::Null,
            "true" => Self::Boolean(true),
            "false" => Self::Boolean(false),
            _ if v.parse::<i64>().is_ok() => Self::Integer(v.parse::<i64>().unwrap()),
            // try parsing as f64
            _ if parse_f64(v).is_some() => Self::Real(v.to_owned()),
            _ => Self::String(v.to_owned()),
        }
    }
}

static BAD_VALUE: YamlInput = YamlInput::BadValue;
impl<'a> Index<&'a str> for YamlInput {
    type Output = Self;

    fn index(&self, idx: &'a str) -> &Self {
        let key = Self::String(idx.to_owned());
        match self.as_hash() {
            Some(h) => h.get(&key).unwrap_or(&BAD_VALUE),
            None => &BAD_VALUE,
        }
    }
}

impl Index<usize> for YamlInput {
    type Output = Self;

    fn index(&self, idx: usize) -> &Self {
        if let Some(v) = self.as_vec() {
            v.get(idx).unwrap_or(&BAD_VALUE)
        } else if let Some(v) = self.as_hash() {
            let key = Self::Integer(idx as i64);
            v.get(&key).unwrap_or(&BAD_VALUE)
        } else {
            &BAD_VALUE
        }
    }
}

impl IntoIterator for YamlInput {
    type Item = Self;
    type IntoIter = YamlInputIter;

    fn into_iter(self) -> Self::IntoIter {
        YamlInputIter {
            yaml: self.into_vec().unwrap_or_else(Vec::new).into_iter(),
        }
    }
}

pub struct YamlInputIter {
    yaml: vec::IntoIter<YamlInput>,
}

impl Iterator for YamlInputIter {
    type Item = YamlInput;

    fn next(&mut self) -> Option<YamlInput> {
        self.yaml.next()
    }
}

#[cfg(test)]
mod test {
    use crate::yaml::*;
    use std::f64;
    #[test]
    fn test_coerce() {
        let s = "---
a: 1
b: 2.2
c: [1, 2]
";
        let out = YamlLoader::load_from_str(&s).unwrap();
        let doc = &out[0];
        assert_eq!(doc["a"].as_i64().unwrap(), 1i64);
        assert_eq!(doc["b"].as_f64().unwrap(), 2.2f64);
        assert_eq!(doc["c"][1].as_i64().unwrap(), 2i64);
        assert!(doc["d"][0].is_badvalue());
    }

    #[test]
    fn test_empty_doc() {
        let s: String = "".to_owned();
        YamlLoader::load_from_str(&s).unwrap();
        let s: String = "---".to_owned();
        assert_eq!(YamlLoader::load_from_str(&s).unwrap()[0], YamlInput::Null);
    }

    #[test]
    fn test_parser() {
        let s: String = "
# comment
a0 bb: val
a1:
    b1: 4
    b2: d
a2: 4 # i'm comment
a3: [1, 2, 3]
a4:
    - - a1
      - a2
    - 2
a5: 'single_quoted'
a6: \"double_quoted\"
a7: 你好
"
        .to_owned();
        let out = YamlLoader::load_from_str(&s).unwrap();
        let doc = &out[0];
        assert_eq!(doc["a7"].as_str().unwrap(), "你好");
    }

    #[test]
    fn test_multi_doc() {
        let s = "
'a scalar'
---
'a scalar'
---
'a scalar'
";
        let out = YamlLoader::load_from_str(&s).unwrap();
        assert_eq!(out.len(), 3);
    }

    #[test]
    fn test_anchor() {
        let s = "
a1: &DEFAULT
    b1: 4
    b2: d
a2: *DEFAULT
";
        let out = YamlLoader::load_from_str(&s).unwrap();
        let doc = &out[0];
        assert_eq!(doc["a2"]["b1"].as_i64().unwrap(), 4);
    }

    #[test]
    fn test_bad_anchor() {
        // allow invalid tags
        let s = "
a1: &DEFAULT
    b1: 4
    b2: *DEFAULT
";
        let out = YamlLoader::load_from_str(&s).unwrap();
        let doc = &out[0];
        assert_eq!(
            doc["a1"]["b2"],
            YamlInput::Aliased("DEFAULT".to_string(), None)
        );
    }

    #[test]
    fn test_github_27() {
        // https://github.com/chyh1990/yaml-rust/issues/27
        let s = "&a";
        let out = YamlLoader::load_from_str(&s).unwrap();
        let doc = &out[0];
        assert_eq!(doc.as_str().unwrap(), "");
    }

    #[test]
    fn test_plain_datatype() {
        let s = "
- 'string'
- \"string\"
- string
- 123
- -321
- 1.23
- -1e4
- ~
- null
- true
- false
- !!str 0
- !!int 100
- !!float 2
- !!null ~
- !!bool true
- !!bool false
- 0xFF
# bad values
- !!int string
- !!float string
- !!bool null
- !!null val
- 0o77
- [ 0xF, 0xF ]
- +12345
- [ true, false ]
";
        let out = YamlLoader::load_from_str(&s).unwrap();
        let doc = &out[0];

        assert_eq!(doc[0].as_str().unwrap(), "string");
        assert_eq!(doc[1].as_str().unwrap(), "string");
        assert_eq!(doc[2].as_str().unwrap(), "string");
        assert_eq!(doc[3].as_i64().unwrap(), 123);
        assert_eq!(doc[4].as_i64().unwrap(), -321);
        assert_eq!(doc[5].as_f64().unwrap(), 1.23);
        assert_eq!(doc[6].as_f64().unwrap(), -1e4);
        assert!(doc[7].is_null());
        assert!(doc[8].is_null());
        assert_eq!(doc[9].as_bool().unwrap(), true);
        assert_eq!(doc[10].as_bool().unwrap(), false);
        assert_eq!(doc[11].as_str().unwrap(), "0");
        assert_eq!(doc[12].as_i64().unwrap(), 100);
        assert_eq!(doc[13].as_f64().unwrap(), 2.0);
        assert!(doc[14].is_null());
        assert_eq!(doc[15].as_bool().unwrap(), true);
        assert_eq!(doc[16].as_bool().unwrap(), false);
        assert_eq!(doc[17].as_i64().unwrap(), 255);
        assert!(doc[18].is_badvalue());
        assert!(doc[19].is_badvalue());
        assert!(doc[20].is_badvalue());
        assert!(doc[21].is_badvalue());
        assert_eq!(doc[22].as_i64().unwrap(), 63);
        assert_eq!(doc[23][0].as_i64().unwrap(), 15);
        assert_eq!(doc[23][1].as_i64().unwrap(), 15);
        assert_eq!(doc[24].as_i64().unwrap(), 12345);
        assert!(doc[25][0].as_bool().unwrap());
        assert!(!doc[25][1].as_bool().unwrap());
    }

    #[test]
    fn test_bad_hyphen() {
        // See: https://github.com/chyh1990/yaml-rust/issues/23
        let s = "{-";
        assert!(YamlLoader::load_from_str(&s).is_err());
    }

    #[test]
    fn test_issue_65() {
        // See: https://github.com/chyh1990/yaml-rust/issues/65
        let b = "\n\"ll\\\"ll\\\r\n\"ll\\\"ll\\\r\r\r\rU\r\r\rU";
        assert!(YamlLoader::load_from_str(&b).is_err());
    }

    #[test]
    fn test_bad_docstart() {
        assert!(YamlLoader::load_from_str("---This used to cause an infinite loop").is_ok());
        assert_eq!(
            YamlLoader::load_from_str("----"),
            Ok(vec![YamlInput::String(String::from("----"))])
        );
        assert_eq!(
            YamlLoader::load_from_str("--- #here goes a comment"),
            Ok(vec![YamlInput::Null])
        );
        assert_eq!(
            YamlLoader::load_from_str("---- #here goes a comment"),
            Ok(vec![YamlInput::String(String::from("----"))])
        );
    }

    #[test]
    fn test_plain_datatype_with_into_methods() {
        let s = "
- 'string'
- \"string\"
- string
- 123
- -321
- 1.23
- -1e4
- true
- false
- !!str 0
- !!int 100
- !!float 2
- !!bool true
- !!bool false
- 0xFF
- 0o77
- +12345
- -.INF
- .NAN
- !!float .INF
";
        let mut out = YamlLoader::load_from_str(&s).unwrap().into_iter();
        let mut doc = out.next().unwrap().into_iter();

        assert_eq!(doc.next().unwrap().into_string().unwrap(), "string");
        assert_eq!(doc.next().unwrap().into_string().unwrap(), "string");
        assert_eq!(doc.next().unwrap().into_string().unwrap(), "string");
        assert_eq!(doc.next().unwrap().into_i64().unwrap(), 123);
        assert_eq!(doc.next().unwrap().into_i64().unwrap(), -321);
        assert_eq!(doc.next().unwrap().into_f64().unwrap(), 1.23);
        assert_eq!(doc.next().unwrap().into_f64().unwrap(), -1e4);
        assert_eq!(doc.next().unwrap().into_bool().unwrap(), true);
        assert_eq!(doc.next().unwrap().into_bool().unwrap(), false);
        assert_eq!(doc.next().unwrap().into_string().unwrap(), "0");
        assert_eq!(doc.next().unwrap().into_i64().unwrap(), 100);
        assert_eq!(doc.next().unwrap().into_f64().unwrap(), 2.0);
        assert_eq!(doc.next().unwrap().into_bool().unwrap(), true);
        assert_eq!(doc.next().unwrap().into_bool().unwrap(), false);
        assert_eq!(doc.next().unwrap().into_i64().unwrap(), 255);
        assert_eq!(doc.next().unwrap().into_i64().unwrap(), 63);
        assert_eq!(doc.next().unwrap().into_i64().unwrap(), 12345);
        assert_eq!(doc.next().unwrap().into_f64().unwrap(), f64::NEG_INFINITY);
        assert!(doc.next().unwrap().into_f64().is_some());
        assert_eq!(doc.next().unwrap().into_f64().unwrap(), f64::INFINITY);
    }

    #[test]
    fn test_hash_order() {
        let s = "---
b: ~
a: ~
c: ~
";
        let out = YamlLoader::load_from_str(&s).unwrap();
        let first = out.into_iter().next().unwrap();
        let mut iter = first.into_hash().unwrap().into_iter();
        assert_eq!(
            Some((YamlInput::String("b".to_owned()), YamlInput::Null)),
            iter.next()
        );
        assert_eq!(
            Some((YamlInput::String("a".to_owned()), YamlInput::Null)),
            iter.next()
        );
        assert_eq!(
            Some((YamlInput::String("c".to_owned()), YamlInput::Null)),
            iter.next()
        );
        assert_eq!(None, iter.next());
    }

    #[test]
    fn test_integer_key() {
        let s = "
0:
    important: true
1:
    important: false
";
        let out = YamlLoader::load_from_str(&s).unwrap();
        let first = out.into_iter().next().unwrap();
        assert_eq!(first[0]["important"].as_bool().unwrap(), true);
    }

    #[test]
    fn test_indentation_equality() {
        let four_spaces = YamlLoader::load_from_str(
            r#"
hash:
    with:
        indentations
"#,
        )
        .unwrap()
        .into_iter()
        .next()
        .unwrap();

        let two_spaces = YamlLoader::load_from_str(
            r#"
hash:
  with:
    indentations
"#,
        )
        .unwrap()
        .into_iter()
        .next()
        .unwrap();

        let one_space = YamlLoader::load_from_str(
            r#"
hash:
 with:
  indentations
"#,
        )
        .unwrap()
        .into_iter()
        .next()
        .unwrap();

        let mixed_spaces = YamlLoader::load_from_str(
            r#"
hash:
     with:
               indentations
"#,
        )
        .unwrap()
        .into_iter()
        .next()
        .unwrap();

        assert_eq!(four_spaces, two_spaces);
        assert_eq!(two_spaces, one_space);
        assert_eq!(four_spaces, mixed_spaces);
    }

    #[test]
    fn test_two_space_indentations() {
        // https://github.com/kbknapp/clap-rs/issues/965

        let s = r#"
subcommands:
  - server:
    about: server related commands
subcommands2:
  - server:
      about: server related commands
subcommands3:
 - server:
    about: server related commands
            "#;

        let out = YamlLoader::load_from_str(&s).unwrap();
        let doc = &out.into_iter().next().unwrap();

        println!("{:#?}", doc);
        assert_eq!(doc["subcommands"][0]["server"], YamlInput::Null);
        assert!(doc["subcommands2"][0]["server"].as_hash().is_some());
        assert!(doc["subcommands3"][0]["server"].as_hash().is_some());
    }

    #[test]
    fn test_recursion_depth_check_objects() {
        let s = "{a:".repeat(10_000) + &"}".repeat(10_000);
        assert!(YamlLoader::load_from_str(&s).is_err());
    }

    #[test]
    fn test_recursion_depth_check_arrays() {
        let s = "[".repeat(10_000) + &"]".repeat(10_000);
        assert!(YamlLoader::load_from_str(&s).is_err());
    }
}
