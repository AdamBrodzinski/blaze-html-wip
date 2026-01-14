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
    pub attrs: Vec<(String, String)>,
}

#[derive(Debug, PartialEq, Clone)]
pub enum TemplateNode {
    Text(String),     // anything that is not a template construct
    Asset(AssetNode), // css/js/img script tag, appends cache busting query param
    Escaped,          // @@ character that renders to @
    Variable(String), // @foo or @foo.bar
}
// todo: nodes to use later
// Escaped,
// Variable(String),
// Each(...)
// Component(...)
// Script(...)
// Style(...)
