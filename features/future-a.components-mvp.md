# Template Components

The template component (v1) provides a simple way to re-use chunks of html
inside of your template. This first version does not let you pass props down
into the template.

Example:


component file: src/components/person.html
```html
<div>Person: Jane</div>
```

template file: src/pages/people.html
```html
<main>
  <h1>@@title</h1>
  <component name="person"></component>
  <component name="person"></component>
<main>
```

template render output with data {"title": "People"}
```html
<main>
  <h1>People</h1>
  <div>Person: Jane</div>
  <div>Person: Jane</div>
<main>
```

## Acceptance Criteria
- should replace the component with the html
- should not implement a slot at this time
- should not implement props at this time


## Examples
```rust
let data = json!({"name": "Jane", "age": "45"});
let tmpl = "name: @name, age: @age";
let result = replace_variables(tmpl, &data);
assert_eq!(result, "name: Jane, age: 45");
```
