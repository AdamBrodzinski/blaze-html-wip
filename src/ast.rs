#![allow(unused)]
use nom::IResult;

#[derive(Debug, PartialEq, Clone)]
pub enum AssetKind {
    Script,
    Style,
}

#[derive(Debug, PartialEq, Clone)]
pub struct Attr {
    pub name: String,
    pub value: String,
    pub quote: char,
}

#[derive(Debug, PartialEq, Clone)]
pub struct AssetNode {
    pub kind: AssetKind,
    pub path: String,
    pub attrs: Vec<Attr>,
}

#[derive(Debug, PartialEq, Clone)]
pub struct EachNode {
    pub item_binding: String, // name of 'as' attr, items='@data.people' as='person'
    pub items_path: Vec<String>, // ["data", "people"],
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
