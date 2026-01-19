#![allow(unused)]
use nom::IResult;

#[derive(Debug, PartialEq, Clone)]
pub enum TemplateNode {
    Asset(AssetNode),         // css/js/img script tag, appends cache busting query param
    Component(ComponentNode), // <MyComponent /> or <MyComponent>...</MyComponent>
    Each(EachNode),           // <Each items="@list" as="item">...</Each>
    Escaped,                  // @@ character that renders to @
    If(IfNode),               // <If true="@var">...</If> or <If false="@var">...</If>
    Slot,                     // <slot/> placeholder inside component templates
    Text(String),             // anything that is not a template construct
    Variable(Vec<String>),    // @foo - HTML-escaped output (safe by default)
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
    Static(String),
    VarPath(Vec<String>),
}

#[derive(Debug, PartialEq, Clone)]
pub enum ConditionMode {
    Strict, // true="@var" or false="@var" - must be boolean
    Truthy, // truthy="@var" or falsy="@var" - JS-like coercion
}

#[derive(Debug, PartialEq, Clone)]
pub struct IfNode {
    pub condition_path: Vec<String>, // ["foo", "bar"] for @foo.bar
    pub negate: bool,                // true when using false="@var" or falsy="@var"
    pub mode: ConditionMode,         // strict boolean vs JS-like truthiness
    pub children: Vec<TemplateNode>,
}
