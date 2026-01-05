use nom::IResult;

#[derive(Debug, PartialEq)]

pub enum TemplateNode {
    Text(String),  // anything that is not a template construct
    Asset(String), // css/js/img script tag, appends cache busting query param
}
// todo: nodes to use later
// Escaped,
// Variable(String),
// Each(...)
// Component(...)
// Script(...)
// Style(...)
