use crate::variables::get_json_value;

pub fn rewrite_each(template: &str, data: &serde_json::Value) -> String {
    let mut result = String::with_capacity(template.len());
    let mut remaining = template;

    'outer: while !remaining.is_empty() {
        // Find the next <each tag
        if let Some(start_pos) = remaining.find("<each") {
            // Add everything before the tag
            result.push_str(&remaining[..start_pos]);

            // Check if this is a valid tag (followed by > or whitespace)
            let after_tag = &remaining[start_pos + 5..];
            if !after_tag.is_empty() {
                let first_char = after_tag.chars().next().unwrap();
                if first_char == '>' || first_char.is_whitespace() {
                    // Find the end of the opening tag, handling attributes with quotes
                    let mut tag_end_pos = 0;
                    let mut in_quotes = false;
                    let mut quote_char = '\0';

                    for (i, ch) in after_tag.char_indices() {
                        if !in_quotes && ch == '>' {
                            tag_end_pos = i;
                            break;
                        } else if ch == '"' || ch == '\'' {
                            if !in_quotes {
                                in_quotes = true;
                                quote_char = ch;
                            } else if ch == quote_char {
                                in_quotes = false;
                            }
                        }
                    }

                    if tag_end_pos > 0 || after_tag.starts_with('>') {
                        // Extract the full opening tag to parse attributes
                        let opening_tag = &after_tag[..tag_end_pos];

                        // Parse the items attribute
                        let items_key = parse_items_attribute(opening_tag)
                            .expect("each tag must have an 'items' attribute");

                        let pos_after_open = start_pos + 5 + tag_end_pos + 1;

                        // Now find the matching closing tag, handling nesting
                        let mut depth = 1;
                        let mut search_pos = pos_after_open;

                        while depth > 0 && search_pos < remaining.len() {
                            // Look for either <each or </each
                            if search_pos + 5 <= remaining.len()
                                && remaining[search_pos..].starts_with("<each")
                            {
                                // Check if it's a valid opening tag
                                if search_pos + 5 < remaining.len() {
                                    let next_char = remaining.as_bytes()[search_pos + 5];
                                    if next_char == b'>' || next_char.is_ascii_whitespace() {
                                        depth += 1;
                                        search_pos += 5;
                                        continue;
                                    }
                                }
                            } else if search_pos + 7 <= remaining.len()
                                && remaining[search_pos..].starts_with("</each>")
                            {
                                depth -= 1;
                                if depth == 0 {
                                    // Found matching closing tag
                                    let content = &remaining[pos_after_open..search_pos];

                                    // Look up the array data
                                    let array_data = get_json_value(data, &items_key)
                                        .expect("items attribute must reference valid data");
                                    let array = array_data
                                        .as_array()
                                        .expect("items must reference an array");

                                    // Repeat content for each item in array
                                    if !array.is_empty() {
                                        let repeated = content.repeat(array.len());
                                        result.push_str(&repeated);
                                    }

                                    remaining = &remaining[search_pos + 7..];
                                    continue 'outer;
                                }
                                search_pos += 7;
                                continue;
                            }
                            search_pos += 1;
                        }

                        // If we get here, no matching closing tag was found
                        result.push_str(&remaining[start_pos..]);
                        break;
                    } else {
                        // No closing > for opening tag
                        result.push_str(&remaining[start_pos..]);
                        break;
                    }
                } else {
                    // Not a valid <each tag (e.g., <eachother>)
                    result.push_str(&remaining[start_pos..start_pos + 5]);
                    remaining = &remaining[start_pos + 5..];
                }
            } else {
                // End of string after <each
                result.push_str(&remaining[start_pos..]);
                break;
            }
        } else {
            // No more <each tags
            result.push_str(remaining);
            break;
        }
    }

    result
}

fn parse_items_attribute(tag_content: &str) -> Option<String> {
    // Look for items='value' or items="value"
    if let Some(items_pos) = tag_content.find("items=") {
        let after_items = &tag_content[items_pos + 6..];
        if let Some(quote_char) = after_items.chars().next() {
            if quote_char == '"' || quote_char == '\'' {
                // Find closing quote
                if let Some(end_pos) = after_items[1..].find(quote_char) {
                    return Some(after_items[1..end_pos + 1].to_string());
                }
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use crate::render_template_str;
    use serde_json::json;

    #[test]
    #[should_panic(expected = "each tag must have an 'items' attribute")]
    fn basic_each_replacement() {
        let tmpl = "List: <each anything='foo'>Hello</each>";
        let data_1 = json!(()); // no data needed yet

        render_template_str(tmpl, &data_1);
    }

    #[test]
    #[should_panic(expected = "each tag must have an 'items' attribute")]
    fn nested_each_tags() {
        let tmpl = "<each>outer <each>inner</each> more</each>";
        let data = json!(());

        render_template_str(tmpl, &data);
    }

    #[test]
    #[should_panic(expected = "each tag must have an 'items' attribute")]
    fn multiple_each_siblings() {
        let tmpl = "<each>First</each> middle <each>Second</each>";
        let data = json!(());

        render_template_str(tmpl, &data);
    }

    #[test]
    #[should_panic(expected = "each tag must have an 'items' attribute")]
    fn deeply_nested_each() {
        let tmpl = "<each>1 <each>2 <each>3</each> 2end</each> 1end</each>";
        let data = json!(());

        render_template_str(tmpl, &data);
    }

    #[test]
    #[should_panic(expected = "each tag must have an 'items' attribute")]
    fn each_with_attributes() {
        let tmpl = r#"<each class="test" id="1">Content</each>"#;
        let data = json!(());

        render_template_str(tmpl, &data);
    }

    #[test]
    #[should_panic(expected = "each tag must have an 'items' attribute")]
    fn each_with_quoted_gt() {
        let tmpl = r#"<each attr="value > test">Content</each>"#;
        let data = json!(());

        render_template_str(tmpl, &data);
    }

    #[test]
    fn not_each_tag() {
        let tmpl = "<eachother>Content</eachother>";
        let data = json!(());

        let result = render_template_str(tmpl, &data);
        assert_eq!(result, "<eachother>Content</eachother>");
    }

    #[test]
    fn unclosed_each_tag() {
        let tmpl = "<each items='list'>Content without closing";
        let data = json!({"list": ["a"]});

        let result = render_template_str(tmpl, &data);
        assert_eq!(result, "<each items='list'>Content without closing");
    }

    #[test]
    fn each_block_basic_two_items_test() {
        let tmpl = "List: <each items='list'>Hello</each>";
        let data_1 = json!({"list": ["a", "b"]});

        let result1 = render_template_str(tmpl, &data_1);
        assert_eq!(result1, "List: HelloHello");
    }

    #[test]
    fn each_block_basic_three_items_test() {
        let tmpl = "List: <each items='list'>Hello</each>";
        let data_2 = json!({"list": ["a", "b", "c"]});

        let result2 = render_template_str(tmpl, &data_2);
        assert_eq!(result2, "List: HelloHelloHello");
    }

    #[test]
    fn each_empty_array() {
        let tmpl = "Before<each items='list'>Item</each>After";
        let data = json!({"list": []});

        let result = render_template_str(tmpl, &data);
        assert_eq!(result, "BeforeAfter");
    }

    #[test]
    #[should_panic(expected = "items attribute must reference valid data")]
    fn each_missing_data_key() {
        let tmpl = "<each items='missing'>Content</each>";
        let data = json!({});

        render_template_str(tmpl, &data);
    }

    #[test]
    #[should_panic(expected = "items must reference an array")]
    fn each_non_array_data() {
        let tmpl = "<each items='notarray'>Content</each>";
        let data = json!({"notarray": "string"});

        render_template_str(tmpl, &data);
    }

    #[test]
    fn each_with_nested_preserves_inner() {
        let tmpl = "<each items='list'>Outer <each items='inner'>Inner</each> End</each>";
        let data = json!({"list": ["a", "b"], "inner": ["x"]});

        let result = render_template_str(tmpl, &data);
        assert_eq!(
            result,
            "Outer <each items='inner'>Inner</each> EndOuter <each items='inner'>Inner</each> End"
        );
    }

    #[test]
    fn each_with_single_quotes() {
        let tmpl = "<each items='list'>Hi</each>";
        let data = json!({"list": ["x", "y"]});

        let result = render_template_str(tmpl, &data);
        assert_eq!(result, "HiHi");
    }

    #[test]
    fn each_with_double_quotes() {
        let tmpl = r#"<each items="list">Hi</each>"#;
        let data = json!({"list": ["x", "y"]});

        let result = render_template_str(tmpl, &data);
        assert_eq!(result, "HiHi");
    }
}
