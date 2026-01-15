#![allow(unused)]
use nom::IResult;

#[derive(Debug, PartialEq, Clone)]
pub enum AssetKind {
    Script,
    Style,
}

#[derive(Debug, PartialEq, Clone)]
pub struct AssetNode {
    pub kind: AssetKind,
    pub path: String,
    /// (name, value, quote_char)
    pub attrs: Vec<(String, String, char)>,
}

#[derive(Debug, PartialEq, Clone)]
pub enum TemplateNode {
    Text(String),     // anything that is not a template construct
    Asset(AssetNode), // css/js/img script tag, appends cache busting query param
    Escaped,          // @@ character that renders to @
    Variable(Vec<String>),    // @foo → HTML-escaped output (safe by default)
    VariableRaw(Vec<String>), // @!foo → raw/unescaped output (opt-in)
}
// todo: nodes to use later
// Escaped,
// Variable(String),
// Each(...)
// Component(...)
// Script(...)
// Style(...)
