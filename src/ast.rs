#![allow(unused)]
use nom::IResult;

#[derive(Debug, PartialEq, Clone)]
pub enum TemplateNode {
    Asset(AssetNode), // script, stylesheet, image, preload, or icon with cache busting
    Component(ComponentNode), // <MyComponent /> or <MyComponent>...</MyComponent>
    Each(EachNode),   // <Each items="@list" as="item">...</Each>
    Escaped,          // @@ character that renders to @
    If(IfNode), // <If true="@var">...</If>, <If false="@var">...</If>, <If exists="@var">...</If>
    Include(String), // <Include path="..."/> - splices raw file contents verbatim
    Slot,       // <Slot/> placeholder inside component templates
    Text(String), // anything that is not a template construct
    Variable(Vec<String>), // @foo - HTML-escaped output (safe by default)
    VariableRaw(Vec<String>), // @!foo - raw/unescaped output (opt-in)
}

#[derive(Debug, PartialEq, Clone)]
pub struct AssetNode {
    pub kind: AssetKind,
    pub path: String,
    pub attrs: Vec<Attr>,
}

#[derive(Debug, PartialEq, Clone)]
pub enum AssetKind {
    Icon,
    Image,
    Preload,
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
pub struct EachNode {
    pub item_binding: String, // name of 'as' attr, items='@data.people' as='person'
    pub items_path: Vec<String>, // ["data", "people"],
    pub children: Vec<TemplateNode>,
}

#[derive(Debug, PartialEq, Clone)]
pub struct ComponentNode {
    pub name: String,
    pub props: Vec<ComponentProp>,
    pub children: Vec<TemplateNode>,
}

#[derive(Debug, PartialEq, Clone)]
pub struct ComponentProp {
    pub name: String,
    pub value: PropValue,
}

#[derive(Debug, PartialEq, Clone)]
pub enum PropValue {
    Static(serde_json::Value), // pre-built at parse time, e.g. "Hello" in <Card title="Hello">
    VarPath(Vec<String>),
}

#[derive(Debug, PartialEq, Clone)]
pub enum ConditionMode {
    Strict, // <If true="@var" or false="@var" - must be boolean
    Truthy, // <If truthy="@var" or falsy="@var" - JS-like coercion
    Exists, // <If exists="@var" - only checks presence of the path
}

#[derive(Debug, PartialEq, Clone)]
pub struct IfNode {
    pub condition_path: Vec<String>, // ["foo", "bar"] for @foo.bar
    pub negate: bool,                // true when using false="@var" or falsy="@var"
    pub mode: ConditionMode,         // strict boolean vs JS-like truthiness
    pub children: Vec<TemplateNode>,
}
