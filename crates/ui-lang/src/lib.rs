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
    Ok(value.into())
}
