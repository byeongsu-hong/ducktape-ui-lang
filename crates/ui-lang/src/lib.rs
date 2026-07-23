use proc_macro::TokenStream;
use std::path::PathBuf;
use std::str::FromStr;

#[proc_macro]
pub fn include_app(input: TokenStream) -> TokenStream {
    expand(input).unwrap_or_else(|message| {
        TokenStream::from_str(&format!("compile_error!({message:?});"))
            .expect("compile_error token stream")
    })
}

fn expand(input: TokenStream) -> Result<TokenStream, String> {
    let relative = parse_literal(&input.to_string())?;
    let manifest = std::env::var("CARGO_MANIFEST_DIR")
        .map_err(|_| "ui-lang: CARGO_MANIFEST_DIR is unavailable".to_owned())?;
    let path = PathBuf::from(manifest).join(relative);
    let display = path.display().to_string();
    let compiled = ui_lang_core::compile_file(&path).map_err(|error| error.render(&display))?;
    TokenStream::from_str(&compiled.rust).map_err(|error| {
        format!(
            "ui-lang generated invalid Rust for {}: {error}\n{}",
            path.display(),
            compiled.rust,
        )
    })
}

fn parse_literal(input: &str) -> Result<String, String> {
    let input = input.trim();
    if input.len() < 2 || !input.starts_with('"') || !input.ends_with('"') {
        return Err("ui_lang::include_app! expects one manifest-relative string literal".into());
    }
    let value = &input[1..input.len() - 1];
    if value.contains('\\') {
        return Err("ui_lang::include_app! paths must use `/` and cannot contain escapes".into());
    }
    let bytes = value.as_bytes();
    if bytes.get(1) == Some(&b':') && bytes[0].is_ascii_alphabetic()
        || PathBuf::from(value).components().any(|component| {
            matches!(
                component,
                std::path::Component::Prefix(_) | std::path::Component::RootDir
            )
        })
    {
        return Err(
            "ui_lang::include_app! paths must be relative to the manifest directory".into(),
        );
    }
    Ok(value.into())
}

#[cfg(test)]
mod tests {
    use super::parse_literal;

    #[test]
    fn include_paths_are_manifest_relative() {
        assert_eq!(parse_literal(r#""ui/app.ice""#).unwrap(), "ui/app.ice");
        assert_eq!(parse_literal(r#""../app.ice""#).unwrap(), "../app.ice");
        for path in [
            r#""/tmp/app.ice""#,
            r#""C:/tmp/app.ice""#,
            r#""ui\\app.ice""#,
        ] {
            assert!(parse_literal(path).is_err(), "accepted {path}");
        }
    }
}
