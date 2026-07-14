//! Asset node rendering
//!
//! Renders AssetNode (Script/Style) to HTML output with cache busting

use crate::ast::{AssetKind, AssetNode};

impl AssetNode {
    /// Writes the asset node HTML directly into the provided buffer.
    /// This avoids allocations compared to returning a new String.
    pub fn write_html(&self, buf: &mut String) -> crate::error::Result<()> {
        match self.kind {
            AssetKind::Script => {
                buf.push_str(r#"<script src=""#);
                write_asset_url(&self.path, buf);
                write_cache_param(&self.path, buf)?;
                buf.push('"');
                for attr in &self.attrs {
                    buf.push(' ');
                    buf.push_str(&attr.name);
                    buf.push('=');
                    buf.push(attr.quote);
                    buf.push_str(&attr.value);
                    buf.push(attr.quote);
                }
                buf.push_str("></script>");
            }
            AssetKind::Style => {
                buf.push_str(r#"<link rel="stylesheet" href=""#);
                write_asset_url(&self.path, buf);
                write_cache_param(&self.path, buf)?;
                buf.push('"');
                for attr in &self.attrs {
                    buf.push(' ');
                    buf.push_str(&attr.name);
                    buf.push('=');
                    buf.push(attr.quote);
                    buf.push_str(&attr.value);
                    buf.push(attr.quote);
                }
                buf.push('>');
            }
        }
        Ok(())
    }
}

fn write_asset_url(path: &str, buf: &mut String) {
    if !path.starts_with('/') {
        buf.push('/');
    }
    buf.push_str(path);
}

#[cfg(feature = "cache-bust")]
fn write_cache_param(path: &str, buf: &mut String) -> crate::error::Result<()> {
    use crate::error::BlazeError;
    let hash = hash_file(path).map_err(|e| BlazeError::asset_io(path, e))?;
    buf.push_str("?v=");
    buf.push_str(&hash);
    Ok(())
}

#[cfg(not(feature = "cache-bust"))]
fn write_cache_param(_path: &str, _buf: &mut String) -> crate::error::Result<()> {
    Ok(())
}

#[cfg(feature = "cache-bust")]
fn hash_file(path: &str) -> std::io::Result<String> {
    let mut file = std::fs::File::open(path)?;
    let mut hasher = blake3::Hasher::new();
    hasher.update_reader(&mut file)?;
    Ok(hasher.finalize().to_hex()[..32].to_string())
}

#[cfg(test)]
mod url_tests {
    use super::*;

    #[test]
    fn adds_leading_slash_to_relative_path() {
        let mut buf = String::new();
        write_asset_url("foo.js", &mut buf);
        assert_eq!(buf, "/foo.js");
    }

    #[test]
    fn preserves_root_relative_path() {
        let mut buf = String::new();
        write_asset_url("/foo.js", &mut buf);
        assert_eq!(buf, "/foo.js");
    }
}

#[cfg(all(test, not(feature = "cache-bust")))]
mod no_cache_tests {
    use super::*;

    #[test]
    fn renders_root_relative_urls_without_query_parameters() {
        let script = AssetNode {
            kind: AssetKind::Script,
            path: "foo.js".to_string(),
            attrs: vec![],
        };
        let style = AssetNode {
            kind: AssetKind::Style,
            path: "foo.css".to_string(),
            attrs: vec![],
        };
        let mut buf = String::new();

        script.write_html(&mut buf).unwrap();
        style.write_html(&mut buf).unwrap();

        assert_eq!(
            buf,
            r#"<script src="/foo.js"></script><link rel="stylesheet" href="/foo.css">"#
        );
    }
}

#[cfg(all(test, feature = "cache-bust"))]
mod tests {
    use super::*;
    use crate::ast::Attr;

    const JS_HASH: &str = "a6f2ed7be4c8834436f238d65249b651";
    const CSS_HASH: &str = "a0ff2dc6b477abd5ca51c463f720d3ab";

    #[test]
    fn script_renders_with_hash() {
        let asset = AssetNode {
            kind: AssetKind::Script,
            path: "test_files/asset.js".to_string(),
            attrs: vec![],
        };
        let mut buf = String::new();
        asset.write_html(&mut buf).unwrap();
        assert_eq!(
            buf,
            format!(r#"<script src="/test_files/asset.js?v={JS_HASH}"></script>"#)
        );
    }

    #[test]
    fn script_renders_with_attrs() {
        let asset = AssetNode {
            kind: AssetKind::Script,
            path: "test_files/asset.js".to_string(),
            attrs: vec![
                Attr {
                    name: "foo".to_string(),
                    value: "bar".to_string(),
                    quote: '"',
                },
                Attr {
                    name: "baz".to_string(),
                    value: "qux".to_string(),
                    quote: '"',
                },
            ],
        };
        let mut buf = String::new();
        asset.write_html(&mut buf).unwrap();
        assert_eq!(
            buf,
            format!(
                r#"<script src="/test_files/asset.js?v={JS_HASH}" foo="bar" baz="qux"></script>"#
            )
        );
    }

    #[test]
    fn script_preserves_single_quotes() {
        let asset = AssetNode {
            kind: AssetKind::Script,
            path: "test_files/asset.js".to_string(),
            attrs: vec![
                Attr {
                    name: "foo".to_string(),
                    value: "bar".to_string(),
                    quote: '\'',
                },
                Attr {
                    name: "baz".to_string(),
                    value: "qux".to_string(),
                    quote: '"',
                },
            ],
        };
        let mut buf = String::new();
        asset.write_html(&mut buf).unwrap();
        assert_eq!(
            buf,
            format!(
                r#"<script src="/test_files/asset.js?v={JS_HASH}" foo='bar' baz="qux"></script>"#
            )
        );
    }

    #[test]
    fn style_renders_with_hash() {
        let asset = AssetNode {
            kind: AssetKind::Style,
            path: "test_files/asset.css".to_string(),
            attrs: vec![],
        };
        let mut buf = String::new();
        asset.write_html(&mut buf).unwrap();
        assert_eq!(
            buf,
            format!(r#"<link rel="stylesheet" href="/test_files/asset.css?v={CSS_HASH}">"#)
        );
    }

    #[test]
    fn style_renders_with_attrs() {
        let asset = AssetNode {
            kind: AssetKind::Style,
            path: "test_files/asset.css".to_string(),
            attrs: vec![
                Attr {
                    name: "foo".to_string(),
                    value: "bar".to_string(),
                    quote: '"',
                },
                Attr {
                    name: "baz".to_string(),
                    value: "qux".to_string(),
                    quote: '"',
                },
            ],
        };
        let mut buf = String::new();
        asset.write_html(&mut buf).unwrap();
        assert_eq!(
            buf,
            format!(
                r#"<link rel="stylesheet" href="/test_files/asset.css?v={CSS_HASH}" foo="bar" baz="qux">"#
            )
        );
    }

    #[test]
    fn returns_error_on_missing_file() {
        use crate::error::BlazeError;

        let asset = AssetNode {
            kind: AssetKind::Script,
            path: "nonexistent.js".to_string(),
            attrs: vec![],
        };
        let mut buf = String::new();
        let err = asset.write_html(&mut buf).unwrap_err();
        assert!(matches!(err, BlazeError::Io(_)));
        assert!(err.to_string().contains("nonexistent.js"));
    }
}
