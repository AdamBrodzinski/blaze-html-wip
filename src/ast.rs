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
pub struct EachNode {
    /// Path to the collection (e.g., ["people"] or ["person", "skills"])
    pub items_path: Vec<String>,
    /// The variable name to bind each item (e.g., "person")
    pub item_binding: String,
    /// Child nodes (may contain nested Each, variables, text, etc.)
    pub children: Vec<TemplateNode>,
}

#[derive(Debug, PartialEq, Clone)]
pub enum TemplateNode {
    Text(String),             // anything that is not a template construct
    Asset(AssetNode),         // css/js/img script tag, appends cache busting query param
    Escaped,                  // @@ character that renders to @
    Variable(Vec<String>),    // @foo → HTML-escaped output (safe by default)
    VariableRaw(Vec<String>), // @!foo → raw/unescaped output (opt-in)
    Each(EachNode),           // <Each items="@list" as="item">...</Each>
}
// todo: nodes to use later
// Component(...)
