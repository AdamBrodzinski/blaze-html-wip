#[derive(Debug, PartialEq, Clone)]
pub(crate) enum TemplateNode {
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
pub(crate) struct AssetNode {
    pub(crate) kind: AssetKind,
    pub(crate) path: String,
    pub(crate) attrs: Vec<Attr>,
}

#[derive(Debug, PartialEq, Clone)]
pub(crate) enum AssetKind {
    Icon,
    Image,
    Preload,
    Script,
    Style,
}

#[derive(Debug, PartialEq, Clone)]
pub(crate) struct Attr {
    pub(crate) name: String,
    pub(crate) value: String,
    pub(crate) quote: char,
}

#[derive(Debug, PartialEq, Clone)]
pub(crate) struct EachNode {
    pub(crate) item_binding: String, // name of 'as' attr, items='@data.people' as='person'
    pub(crate) items_path: Vec<String>, // ["data", "people"],
    pub(crate) children: Vec<TemplateNode>,
}

#[derive(Debug, PartialEq, Clone)]
pub(crate) struct ComponentNode {
    pub(crate) name: String,
    pub(crate) props: Vec<ComponentProp>,
    pub(crate) children: Vec<TemplateNode>,
}

#[derive(Debug, PartialEq, Clone)]
pub(crate) struct ComponentProp {
    pub(crate) name: String,
    pub(crate) value: PropValue,
}

#[derive(Debug, PartialEq, Clone)]
pub(crate) enum PropValue {
    Static(serde_json::Value), // pre-built at parse time, e.g. "Hello" in <Card title="Hello">
    VarPath(Vec<String>),
}

#[derive(Debug, PartialEq, Clone)]
pub(crate) enum ConditionMode {
    Strict, // <If true="@var" or false="@var" - must be boolean
    Truthy, // <If truthy="@var" or falsy="@var" - JS-like coercion
    Exists, // <If exists="@var" - only checks presence of the path
}

#[derive(Debug, PartialEq, Clone)]
pub(crate) struct IfNode {
    pub(crate) condition_path: Vec<String>, // ["foo", "bar"] for @foo.bar
    pub(crate) negate: bool,                // true when using false="@var" or falsy="@var"
    pub(crate) mode: ConditionMode,         // strict boolean vs JS-like truthiness
    pub(crate) children: Vec<TemplateNode>,
}
