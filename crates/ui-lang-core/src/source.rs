use crate::{Document, Error, Span, check, codegen, parser};
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub struct FileCompilation {
    pub rust: String,
    pub dependencies: Vec<PathBuf>,
}

#[derive(Clone, Debug)]
struct Origin {
    path: PathBuf,
    line: usize,
}

#[derive(Debug)]
struct LoadedSource {
    source: String,
    origins: Vec<Origin>,
    dependencies: Vec<PathBuf>,
}

pub fn source_is_app(source: &str) -> bool {
    source
        .lines()
        .any(|line| line.len() == line.trim_start().len() && line.starts_with("app "))
}

pub fn analyze_file(path: impl AsRef<Path>) -> Result<Document, Error> {
    let loaded = load(path.as_ref())?;
    analyze_loaded(&loaded)
}

pub fn compile_file(path: impl AsRef<Path>) -> Result<FileCompilation, Error> {
    let loaded = load(path.as_ref())?;
    let document = analyze_loaded(&loaded)?;
    let root = loaded
        .dependencies
        .first()
        .expect("a loaded source always has a root");
    let mut rust = codegen::generate(&document, &root.display().to_string())
        .map_err(|error| remap_error(error, &loaded))?;
    for dependency in loaded.dependencies.iter().skip(1) {
        rust.push_str(&format!(
            "const _: &str = include_str!({:?});\n",
            dependency.display().to_string()
        ));
    }
    Ok(FileCompilation {
        rust,
        dependencies: loaded.dependencies,
    })
}

fn analyze_loaded(loaded: &LoadedSource) -> Result<Document, Error> {
    let mut document = parser::parse(&loaded.source).map_err(|error| remap_error(error, loaded))?;
    check::check(&mut document).map_err(|error| remap_error(error, loaded))?;
    check_assets(&document, loaded).map_err(|error| remap_error(error, loaded))?;
    Ok(document)
}

fn check_assets(document: &Document, loaded: &LoadedSource) -> Result<(), Error> {
    let root = loaded
        .dependencies
        .first()
        .expect("a loaded source always has a root");
    let parent = root.parent().unwrap_or_else(|| Path::new("."));
    for font in &document.settings.fonts {
        let path = parent.join(&font.path);
        if !path.is_file() {
            return Err(Error::new(
                "E192",
                &font.span,
                format!("cannot read font file `{}`", path.display()),
            ));
        }
    }
    if let Some(icon) = document
        .settings
        .window
        .as_ref()
        .and_then(|window| window.icon.as_ref())
    {
        let path = parent.join(&icon.path);
        if !path.is_file() {
            return Err(Error::new(
                "E192",
                &icon.span,
                format!("cannot read window icon file `{}`", path.display()),
            ));
        }
        let actual = fs::metadata(&path)
            .map_err(|error| {
                Error::new(
                    "E192",
                    &icon.span,
                    format!(
                        "cannot inspect window icon file `{}`: {error}",
                        path.display()
                    ),
                )
            })?
            .len();
        if actual != icon.byte_len as u64 {
            return Err(Error::new(
                "E193",
                &icon.span,
                format!(
                    "window icon `{}` has {actual} RGBA bytes; expected {} for {} × {}",
                    path.display(),
                    icon.byte_len,
                    icon.width,
                    icon.height
                ),
            ));
        }
    }
    Ok(())
}

fn load(path: &Path) -> Result<LoadedSource, Error> {
    let root = canonical(path, path, 1)?;
    let root_source = fs::read_to_string(&root)
        .map_err(|error| file_error("E181", &root, 1, format!("cannot read .ice file: {error}")))?;
    if !source_is_app(&root_source) {
        return Err(file_error(
            "E183",
            &root,
            1,
            "an app root must declare `app Name`; import this fragment from an app instead",
        ));
    }
    let mut loaded = LoadedSource {
        source: String::new(),
        origins: Vec::new(),
        dependencies: Vec::new(),
    };
    let mut included = HashSet::new();
    let mut stack = Vec::new();
    load_into(&root, &root, 1, &mut loaded, &mut included, &mut stack)?;
    Ok(loaded)
}

fn load_into(
    path: &Path,
    imported_from: &Path,
    import_line: usize,
    loaded: &mut LoadedSource,
    included: &mut HashSet<PathBuf>,
    stack: &mut Vec<PathBuf>,
) -> Result<(), Error> {
    if let Some(start) = stack.iter().position(|entry| entry == path) {
        let mut cycle = stack[start..]
            .iter()
            .map(|entry| entry.display().to_string())
            .collect::<Vec<_>>();
        cycle.push(path.display().to_string());
        return Err(file_error(
            "E182",
            imported_from,
            import_line,
            format!("cyclic `use`: {}", cycle.join(" -> ")),
        ));
    }
    if !included.insert(path.to_owned()) {
        return Ok(());
    }

    stack.push(path.to_owned());
    loaded.dependencies.push(path.to_owned());
    let source = fs::read_to_string(path)
        .map_err(|error| file_error("E181", path, 1, format!("cannot read .ice file: {error}")))?;
    for (index, raw) in source.lines().enumerate() {
        let line = index + 1;
        if raw.len() == raw.trim_start().len() && raw.starts_with("use ") {
            let relative = parse_use(raw, path, line)?;
            let target = canonical(
                &path
                    .parent()
                    .unwrap_or_else(|| Path::new("."))
                    .join(relative),
                path,
                line,
            )?;
            load_into(&target, path, line, loaded, included, stack)?;
        } else {
            loaded.source.push_str(raw);
            loaded.source.push('\n');
            loaded.origins.push(Origin {
                path: path.to_owned(),
                line,
            });
        }
    }
    stack.pop();
    Ok(())
}

fn parse_use<'a>(source: &'a str, path: &Path, line: usize) -> Result<&'a str, Error> {
    let value = source[4..].trim();
    if value.len() < 2 || !value.starts_with('"') || !value.ends_with('"') {
        return Err(file_error(
            "E180",
            path,
            line,
            "imports use `use \"relative/file.ice\"`",
        ));
    }
    let value = &value[1..value.len() - 1];
    let import = Path::new(value);
    if value.contains('\\')
        || import.is_absolute()
        || import.extension().and_then(|ext| ext.to_str()) != Some("ice")
    {
        return Err(file_error(
            "E180",
            path,
            line,
            "import paths must be relative `/` paths ending in `.ice`",
        ));
    }
    Ok(value)
}

fn canonical(path: &Path, source: &Path, line: usize) -> Result<PathBuf, Error> {
    path.canonicalize().map_err(|error| {
        file_error(
            "E181",
            source,
            line,
            format!("cannot read `{}`: {error}", path.display()),
        )
    })
}

fn remap_error(mut error: Error, loaded: &LoadedSource) -> Error {
    if let Some(origin) = error
        .line
        .checked_sub(1)
        .and_then(|index| loaded.origins.get(index))
    {
        error.line = origin.line;
        error.path = Some(origin.path.display().to_string());
    }
    error
}

fn file_error(code: &'static str, path: &Path, line: usize, message: impl Into<String>) -> Error {
    Error::new(code, &Span::line(line), message).at_path(path.display().to_string())
}

#[cfg(test)]
mod tests {
    use super::{compile_file, source_is_app};
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    struct Fixture(PathBuf);

    impl Fixture {
        fn new() -> Self {
            let nonce = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let path =
                std::env::temp_dir().join(format!("ui-lang-source-{}-{nonce}", std::process::id()));
            fs::create_dir_all(&path).unwrap();
            Self(path)
        }

        fn write(&self, relative: &str, source: &str) {
            let path = self.0.join(relative);
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).unwrap();
            }
            fs::write(path, source).unwrap();
        }

        fn path(&self, relative: &str) -> PathBuf {
            self.0.join(relative)
        }
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            fs::remove_dir_all(&self.0).unwrap();
        }
    }

    #[test]
    fn compiles_relative_and_nested_imports_once() {
        let fixture = Fixture::new();
        fixture.write(
            "app.ice",
            "app Demo\nuse \"shared/theme.ice\"\nuse \"parts/body.ice\"\nview\n  Card()\n",
        );
        fixture.write(
            "shared/theme.ice",
            "theme\n  foreground #ffffff\n  background #000000\n  primary #333333\n  danger #ff0000\n",
        );
        fixture.write(
            "parts/body.ice",
            "use \"../shared/theme.ice\"\ncomponent Card()\n  text \"Hello\" @text-foreground\n",
        );

        let compiled = compile_file(fixture.path("app.ice")).unwrap();

        assert_eq!(compiled.dependencies.len(), 3);
        assert!(compiled.rust.contains("struct Demo"));
        assert_eq!(compiled.rust.matches("include_str!").count(), 3);
    }

    #[test]
    fn reports_an_import_cycle_at_the_imported_file() {
        let fixture = Fixture::new();
        fixture.write(
            "app.ice",
            "app Demo\nuse \"part.ice\"\nview\n  text \"Hi\"\n",
        );
        fixture.write("part.ice", "use \"app.ice\"\n");

        let error = compile_file(fixture.path("app.ice")).unwrap_err();

        assert_eq!(error.code, "E182");
        assert!(error.path.as_deref().unwrap().ends_with("part.ice"));
        assert_eq!(error.line, 1);
        assert!(error.message.contains("cyclic `use`"));
    }

    #[test]
    fn reports_a_missing_import_at_the_use_site() {
        let fixture = Fixture::new();
        fixture.write(
            "app.ice",
            "app Demo\nuse \"missing.ice\"\nview\n  text \"Hi\"\n",
        );

        let error = compile_file(fixture.path("app.ice")).unwrap_err();

        assert_eq!(error.code, "E181");
        assert_eq!(error.line, 2);
        assert!(error.path.as_deref().unwrap().ends_with("app.ice"));
        assert!(error.message.contains("missing.ice"));
    }

    #[test]
    fn checks_and_embeds_app_font_files_relative_to_the_root() {
        let fixture = Fixture::new();
        fixture.write(
            "app.ice",
            "app Demo\n  font \"assets/Brand.ttf\"\n  window\n    icon-rgba \"assets/app.rgba\" 2 1\ntheme\n  background #000000\n  foreground #ffffff\n  primary #333333\n  danger #ff0000\nview\n  text \"Hi\"\n",
        );
        fixture.write("assets/Brand.ttf", "font bytes");
        fixture.write("assets/app.rgba", "RGBAABC\n");

        let compiled = compile_file(fixture.path("app.ice")).unwrap();
        let font = fixture.path("assets/Brand.ttf");
        assert!(compiled.rust.contains(&format!(
            ".font(include_bytes!({:?}).as_slice())",
            font.display().to_string()
        )));
        let icon = fixture.path("assets/app.rgba");
        assert!(
            compiled
                .rust
                .contains(&format!("include_bytes!({:?})", icon.display().to_string()))
        );
    }

    #[test]
    fn reports_a_missing_app_font_at_its_setting() {
        let fixture = Fixture::new();
        fixture.write(
            "app.ice",
            "app Demo\n  font \"assets/Missing.ttf\"\ntheme\n  background #000000\n  foreground #ffffff\n  primary #333333\n  danger #ff0000\nview\n  text \"Hi\"\n",
        );

        let error = compile_file(fixture.path("app.ice")).unwrap_err();
        assert_eq!(error.code, "E192");
        assert_eq!(error.line, 2);
        assert!(error.path.as_deref().unwrap().ends_with("app.ice"));
        assert!(error.message.contains("assets/Missing.ttf"));

        fixture.write(
            "app.ice",
            "app Demo\n  window\n    icon-rgba \"assets/missing.rgba\" 2 1\ntheme\n  background #000000\n  foreground #ffffff\n  primary #333333\n  danger #ff0000\nview\n  text \"Hi\"\n",
        );
        let error = compile_file(fixture.path("app.ice")).unwrap_err();
        assert_eq!(error.code, "E192");
        assert_eq!(error.line, 3);
        assert!(error.message.contains("assets/missing.rgba"));

        fixture.write("assets/wrong.rgba", "RGBA");
        fixture.write(
            "app.ice",
            "app Demo\n  window\n    icon-rgba \"assets/wrong.rgba\" 2 1\ntheme\n  background #000000\n  foreground #ffffff\n  primary #333333\n  danger #ff0000\nview\n  text \"Hi\"\n",
        );
        let error = compile_file(fixture.path("app.ice")).unwrap_err();
        assert_eq!(error.code, "E193");
        assert_eq!(error.line, 3);
        assert!(error.message.contains("has 4 RGBA bytes; expected 8"));
    }

    #[test]
    fn remaps_language_errors_to_the_fragment() {
        let fixture = Fixture::new();
        fixture.write("app.ice", "app Demo\nuse \"part.ice\"\nview\n  Broken()\n");
        fixture.write("part.ice", "component Broken()\n  wat\n");

        let error = compile_file(fixture.path("app.ice")).unwrap_err();

        assert_eq!(error.line, 2);
        assert!(error.path.as_deref().unwrap().ends_with("part.ice"));
    }

    #[test]
    fn only_top_level_app_declarations_make_roots() {
        assert!(source_is_app("app Demo\nview\n  text \"Hi\"\n"));
        assert!(!source_is_app("component Card()\n  text \"app Demo\"\n"));
    }

    #[test]
    fn rejects_a_fragment_as_an_app_root() {
        let fixture = Fixture::new();
        fixture.write("part.ice", "component Card()\n  text \"Hi\"\n");

        let error = compile_file(fixture.path("part.ice")).unwrap_err();

        assert_eq!(error.code, "E183");
        assert!(error.message.contains("app root"));
    }
}
