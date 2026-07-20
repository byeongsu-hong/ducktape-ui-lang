use crate::schema;
use serde_json::{Value, json};
use std::collections::{BTreeSet, HashMap};
use std::fs;
use std::io::{self, BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

struct DiagnosticReport {
    target: String,
    diagnostic: Option<Value>,
}

pub fn run_stdio() -> Result<(), String> {
    let stdin = io::stdin();
    let stdout = io::stdout();
    serve(&mut BufReader::new(stdin.lock()), &mut stdout.lock()).map_err(|error| error.to_string())
}

fn serve(reader: &mut impl BufRead, writer: &mut impl Write) -> io::Result<()> {
    let mut documents = HashMap::<String, String>::new();
    let mut diagnostic_reports = HashMap::<String, DiagnosticReport>::new();
    let mut shutdown = false;

    while let Some(message) = read_message(reader)? {
        let method = message.get("method").and_then(Value::as_str).unwrap_or("");
        let id = message.get("id").cloned();
        if shutdown && method != "exit" {
            if let Some(id) = id {
                request_error(writer, id, -32600, "server is shutting down")?;
            }
            continue;
        }
        match method {
            "initialize" => {
                if let Some(id) = id {
                    respond(
                        writer,
                        id,
                        json!({
                            "capabilities": {
                                "positionEncoding": "utf-16",
                                "textDocumentSync": { "openClose": true, "change": 1 },
                                "documentFormattingProvider": true,
                                "completionProvider": { "resolveProvider": false },
                                "definitionProvider": false,
                                "renameProvider": false,
                            },
                            "serverInfo": {
                                "name": "ice-lsp",
                                "version": env!("CARGO_PKG_VERSION"),
                            },
                        }),
                    )?;
                }
            }
            "shutdown" => {
                shutdown = true;
                if let Some(id) = id {
                    respond(writer, id, Value::Null)?;
                }
            }
            "exit" => {
                if shutdown {
                    break;
                }
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "LSP exit received before shutdown",
                ));
            }
            "textDocument/didOpen" => {
                let params = &message["params"]["textDocument"];
                if let (Some(uri), Some(text)) = (params["uri"].as_str(), params["text"].as_str()) {
                    documents.insert(uri.to_owned(), text.to_owned());
                    reanalyze_open_roots(writer, &documents, &mut diagnostic_reports)?;
                }
            }
            "textDocument/didChange" => {
                let uri = message["params"]["textDocument"]["uri"].as_str();
                let text = message["params"]["contentChanges"]
                    .as_array()
                    .and_then(|changes| changes.last())
                    .and_then(|change| change["text"].as_str());
                if let (Some(uri), Some(text)) = (uri, text) {
                    documents.insert(uri.to_owned(), text.to_owned());
                    reanalyze_open_roots(writer, &documents, &mut diagnostic_reports)?;
                }
            }
            "textDocument/didClose" => {
                if let Some(uri) = message["params"]["textDocument"]["uri"].as_str() {
                    documents.remove(uri);
                    reanalyze_open_roots(writer, &documents, &mut diagnostic_reports)?;
                }
            }
            "textDocument/formatting" => {
                if let Some(id) = id {
                    let uri = message["params"]["textDocument"]["uri"].as_str();
                    match uri.and_then(|uri| documents.get(uri)) {
                        Some(source) => {
                            let formatted = ui_lang_core::format_fragment(source);
                            let edits = if formatted == *source {
                                Vec::new()
                            } else {
                                vec![json!({
                                    "range": whole_document_range(source),
                                    "newText": formatted,
                                })]
                            };
                            respond(writer, id, Value::Array(edits))?;
                        }
                        None => invalid_params(writer, id, "document is not open")?,
                    }
                }
            }
            "textDocument/completion" => {
                if let Some(id) = id {
                    respond(writer, id, Value::Array(schema::completion_items()))?;
                }
            }
            "initialized" | "$/cancelRequest" => {}
            _ if id.is_some() => {
                request_error(writer, id.unwrap(), -32601, "method not found")?;
            }
            _ => {}
        }
    }
    Ok(())
}

fn reanalyze_open_roots(
    writer: &mut impl Write,
    documents: &HashMap<String, String>,
    reports: &mut HashMap<String, DiagnosticReport>,
) -> io::Result<()> {
    let overlays = source_overlays(documents);
    // ponytail: Add a dependency graph only if profiling shows open-root scale matters.
    let next = documents
        .iter()
        .filter(|(_, source)| ui_lang_core::source_is_app(source))
        .map(|(uri, source)| (uri.clone(), analyze_diagnostics(uri, source, &overlays)))
        .collect::<HashMap<_, _>>();
    let targets = reports
        .values()
        .chain(next.values())
        .map(|report| report.target.clone())
        .collect::<BTreeSet<_>>();
    *reports = next;
    for target in targets {
        publish_aggregated(writer, reports, &target)?;
    }
    Ok(())
}

fn source_overlays(documents: &HashMap<String, String>) -> HashMap<PathBuf, String> {
    documents
        .iter()
        .filter_map(|(uri, source)| {
            file_uri_path(uri).map(|path| {
                let path = path.canonicalize().unwrap_or(path);
                (path, source.clone())
            })
        })
        .collect()
}

fn analyze_diagnostics(
    uri: &str,
    source: &str,
    overlays: &HashMap<PathBuf, String>,
) -> DiagnosticReport {
    let analysis = file_uri_path(uri)
        .filter(|path| path.is_file())
        .map_or_else(
            || ui_lang_core::analyze(source),
            |path| ui_lang_core::analyze_file_with_overlays(path, overlays),
        );
    match analysis {
        Ok(_) => DiagnosticReport {
            target: uri.to_owned(),
            diagnostic: None,
        },
        Err(error) => {
            let (target, target_source) = diagnostic_target(uri, source, overlays, &error);
            let mut message = error.message;
            if let Some(hint) = error.hint {
                message.push_str("\nhint: ");
                message.push_str(&hint);
            }
            DiagnosticReport {
                target,
                diagnostic: Some(json!({
                    "range": diagnostic_range(&target_source, error.line, error.column),
                    "severity": 1,
                    "code": error.code,
                    "source": "ice",
                    "message": message,
                })),
            }
        }
    }
}

fn publish_aggregated(
    writer: &mut impl Write,
    reports: &HashMap<String, DiagnosticReport>,
    uri: &str,
) -> io::Result<()> {
    let diagnostics = reports
        .values()
        .filter(|report| report.target == uri)
        .filter_map(|report| report.diagnostic.clone())
        .collect::<Vec<_>>();
    write_message(
        writer,
        &json!({
            "jsonrpc": "2.0",
            "method": "textDocument/publishDiagnostics",
            "params": { "uri": uri, "diagnostics": diagnostics },
        }),
    )
}

fn diagnostic_target(
    root_uri: &str,
    root_source: &str,
    overlays: &HashMap<PathBuf, String>,
    error: &ui_lang_core::Error,
) -> (String, String) {
    let Some(error_path) = error.path.as_deref().map(Path::new) else {
        return (root_uri.to_owned(), root_source.to_owned());
    };
    if file_uri_path(root_uri).is_some_and(|root_path| same_file(&root_path, error_path)) {
        return (root_uri.to_owned(), root_source.to_owned());
    }
    if let Some(source) = overlays.get(error_path) {
        return (file_path_uri(error_path), source.clone());
    }
    match fs::read_to_string(error_path) {
        Ok(source) => (file_path_uri(error_path), source),
        Err(_) => (root_uri.to_owned(), root_source.to_owned()),
    }
}

fn same_file(left: &Path, right: &Path) -> bool {
    match (left.canonicalize(), right.canonicalize()) {
        (Ok(left), Ok(right)) => left == right,
        _ => left == right,
    }
}

fn file_uri_path(uri: &str) -> Option<PathBuf> {
    let path = uri.strip_prefix("file://")?;
    let path = path.strip_prefix("localhost").unwrap_or(path);
    if !path.starts_with('/') {
        return None;
    }
    let bytes = path.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%' {
            let high = hex(*bytes.get(index + 1)?)?;
            let low = hex(*bytes.get(index + 2)?)?;
            decoded.push((high << 4) | low);
            index += 3;
        } else {
            decoded.push(bytes[index]);
            index += 1;
        }
    }
    String::from_utf8(decoded).ok().map(PathBuf::from)
}

fn hex(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

fn file_path_uri(path: &Path) -> String {
    let mut uri = String::from("file://");
    for byte in path.to_string_lossy().bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'/' | b':' | b'.' | b'-' | b'_' | b'~') {
            uri.push(char::from(byte));
        } else {
            uri.push_str(&format!("%{byte:02X}"));
        }
    }
    uri
}

fn diagnostic_range(source: &str, one_based_line: usize, one_based_column: usize) -> Value {
    let line = one_based_line
        .saturating_sub(1)
        .min(source.split('\n').count().saturating_sub(1));
    let text = source.split('\n').nth(line).unwrap_or("");
    let character = one_based_column.saturating_sub(1).min(text.chars().count());
    let start = text
        .chars()
        .take(character)
        .map(char::len_utf16)
        .sum::<usize>();
    let end = start
        + text
            .chars()
            .nth(character)
            .map(char::len_utf16)
            .unwrap_or(0);
    json!({
        "start": { "line": line, "character": start },
        "end": { "line": line, "character": end },
    })
}

fn whole_document_range(source: &str) -> Value {
    let line = source.bytes().filter(|byte| *byte == b'\n').count();
    let character = source
        .rsplit_once('\n')
        .map_or(source, |(_, tail)| tail)
        .encode_utf16()
        .count();
    json!({
        "start": { "line": 0, "character": 0 },
        "end": { "line": line, "character": character },
    })
}

fn respond(writer: &mut impl Write, id: Value, result: Value) -> io::Result<()> {
    write_message(
        writer,
        &json!({ "jsonrpc": "2.0", "id": id, "result": result }),
    )
}

fn invalid_params(writer: &mut impl Write, id: Value, message: &str) -> io::Result<()> {
    request_error(writer, id, -32602, message)
}

fn request_error(writer: &mut impl Write, id: Value, code: i64, message: &str) -> io::Result<()> {
    write_message(
        writer,
        &json!({
            "jsonrpc": "2.0",
            "id": id,
            "error": { "code": code, "message": message },
        }),
    )
}

fn read_message(reader: &mut impl BufRead) -> io::Result<Option<Value>> {
    let mut length = None;
    let mut started = false;
    loop {
        let mut line = String::new();
        if reader.read_line(&mut line)? == 0 {
            return if started {
                Err(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    "incomplete LSP headers",
                ))
            } else {
                Ok(None)
            };
        }
        started = true;
        let line = line.trim_end_matches(['\r', '\n']);
        if line.is_empty() {
            break;
        }
        if let Some((name, value)) = line.split_once(':')
            && name.eq_ignore_ascii_case("Content-Length")
        {
            length = Some(value.trim().parse::<usize>().map_err(|_| {
                io::Error::new(io::ErrorKind::InvalidData, "invalid Content-Length")
            })?);
        }
    }

    let length = length
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "missing Content-Length"))?;
    let mut body = vec![0; length];
    reader.read_exact(&mut body)?;
    serde_json::from_slice(&body)
        .map(Some)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))
}

fn write_message(writer: &mut impl Write, message: &Value) -> io::Result<()> {
    let body = serde_json::to_vec(message)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
    write!(writer, "Content-Length: {}\r\n\r\n", body.len())?;
    writer.write_all(&body)?;
    writer.flush()
}

#[cfg(test)]
mod tests {
    use super::{
        diagnostic_range, file_path_uri, file_uri_path, read_message, serve, whole_document_range,
    };
    use serde_json::{Value, json};
    use std::fs;
    use std::io::{BufReader, Cursor};
    use std::path::{Path, PathBuf};
    use std::time::{SystemTime, UNIX_EPOCH};

    const APP_WITH_PART: &str = "app Demo\nuse \"part.ice\"\ntheme\n  background #000000\n  foreground #ffffff\n  primary #333333\n  danger #ff0000\nview\n  Broken()\n";

    struct Fixture(PathBuf);

    impl Fixture {
        fn new() -> Self {
            let nonce = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let path =
                std::env::temp_dir().join(format!("cargo-ice-lsp-{}-{nonce}", std::process::id()));
            fs::create_dir_all(&path).unwrap();
            Self(path)
        }

        fn write(&self, relative: &str, source: &str) {
            fs::write(self.0.join(relative), source).unwrap();
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

    fn frame(message: &Value, output: &mut Vec<u8>) {
        let body = serde_json::to_vec(message).unwrap();
        output.extend_from_slice(format!("Content-Length: {}\r\n\r\n", body.len()).as_bytes());
        output.extend_from_slice(&body);
    }

    fn run(messages: &[Value]) -> std::io::Result<Vec<Value>> {
        let mut input = Vec::new();
        for message in messages {
            frame(message, &mut input);
        }

        let mut output = Vec::new();
        serve(&mut BufReader::new(Cursor::new(input)), &mut output)?;

        let mut reader = BufReader::new(Cursor::new(output));
        let mut messages = Vec::new();
        while let Some(message) = read_message(&mut reader)? {
            messages.push(message);
        }
        Ok(messages)
    }

    fn response(messages: &[Value], id: impl Into<Value>) -> &Value {
        let id = id.into();
        messages.iter().find(|message| message["id"] == id).unwrap()
    }

    #[test]
    fn initializes_and_shuts_down_with_honest_capabilities() {
        let messages = run(&[
            json!({ "jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {} }),
            json!({ "jsonrpc": "2.0", "id": 2, "method": "shutdown" }),
            json!({ "jsonrpc": "2.0", "method": "exit" }),
        ])
        .unwrap();

        let capabilities = &response(&messages, 1)["result"]["capabilities"];
        assert_eq!(capabilities["positionEncoding"], "utf-16");
        assert_eq!(capabilities["textDocumentSync"]["change"], 1);
        assert_eq!(capabilities["documentFormattingProvider"], true);
        assert_eq!(capabilities["completionProvider"]["resolveProvider"], false);
        assert_eq!(capabilities["definitionProvider"], false);
        assert_eq!(capabilities["renameProvider"], false);
        assert_eq!(response(&messages, 2)["result"], Value::Null);
    }

    #[test]
    fn publishes_diagnostics_for_open_and_change() {
        let uri = "file:///tmp/demo.ice";
        let messages = run(&[
            json!({ "jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {} }),
            json!({
                "jsonrpc": "2.0",
                "method": "textDocument/didOpen",
                "params": { "textDocument": { "uri": uri, "text": "app Demo\ntheme\n  background #000000\n  foreground #ffffff\n  primary #333333\n  danger #ff0000\nview\n  wat\n" } },
            }),
            json!({
                "jsonrpc": "2.0",
                "method": "textDocument/didChange",
                "params": {
                    "textDocument": { "uri": uri },
                    "contentChanges": [{ "text": "app Demo\ntheme\n    background #000000\n    foreground #ffffff\n    primary #333333\n    danger #ff0000\nview\n    text \"Hi\"\n" }],
                },
            }),
            json!({ "jsonrpc": "2.0", "id": 2, "method": "shutdown" }),
            json!({ "jsonrpc": "2.0", "method": "exit" }),
        ])
        .unwrap();

        let diagnostics = messages
            .iter()
            .filter(|message| message["method"] == "textDocument/publishDiagnostics")
            .collect::<Vec<_>>();
        assert_eq!(diagnostics.len(), 2);
        assert_eq!(
            diagnostics[0]["params"]["diagnostics"]
                .as_array()
                .unwrap()
                .len(),
            1
        );
        assert_eq!(
            diagnostics[0]["params"]["diagnostics"][0]["range"],
            json!({
                "start": { "line": 7, "character": 0 },
                "end": { "line": 7, "character": 1 },
            })
        );
        assert!(
            diagnostics[1]["params"]["diagnostics"]
                .as_array()
                .unwrap()
                .is_empty(),
            "{diagnostics:?}"
        );
    }

    #[test]
    fn publishes_imported_errors_at_the_imported_file() {
        let fixture = Fixture::new();
        fixture.write("app.ice", "app Saved\nview\n  text \"Saved\"\n");
        fixture.write("part.ice", "component Broken()\n  wat\n");
        let root_uri = file_path_uri(&fixture.path("app.ice"));
        let part_uri = file_path_uri(&fixture.path("part.ice"));
        let overlay = "app Overlay\nuse \"part.ice\"\nview\n  Broken()\n";

        let messages = run(&[
            json!({ "jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {} }),
            json!({
                "jsonrpc": "2.0",
                "method": "textDocument/didOpen",
                "params": { "textDocument": { "uri": root_uri, "text": overlay } },
            }),
            json!({ "jsonrpc": "2.0", "id": 2, "method": "shutdown" }),
            json!({ "jsonrpc": "2.0", "method": "exit" }),
        ])
        .unwrap();

        let published = messages
            .iter()
            .find(|message| message["method"] == "textDocument/publishDiagnostics")
            .unwrap();
        assert_eq!(published["params"]["uri"], part_uri);
        assert_eq!(published["params"]["diagnostics"][0]["code"], "E064");
        assert_eq!(
            published["params"]["diagnostics"][0]["range"],
            json!({
                "start": { "line": 1, "character": 0 },
                "end": { "line": 1, "character": 1 },
            })
        );
    }

    #[test]
    fn unsaved_import_errors_recover_on_edit() {
        let fixture = Fixture::new();
        fixture.write("app.ice", APP_WITH_PART);
        fixture.write("part.ice", "component Broken()\n  text \"Saved\"\n");
        let root_uri = file_path_uri(&fixture.path("app.ice"));
        let part_uri = file_path_uri(&fixture.path("part.ice"));

        let messages = run(&[
            json!({ "jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {} }),
            json!({
                "jsonrpc": "2.0",
                "method": "textDocument/didOpen",
                "params": { "textDocument": { "uri": root_uri, "text": APP_WITH_PART } },
            }),
            json!({
                "jsonrpc": "2.0",
                "method": "textDocument/didOpen",
                "params": { "textDocument": { "uri": part_uri, "text": "component Broken()\n  wat\n" } },
            }),
            json!({
                "jsonrpc": "2.0",
                "method": "textDocument/didChange",
                "params": {
                    "textDocument": { "uri": part_uri },
                    "contentChanges": [{ "text": "component Broken()\n  text \"Unsaved\"\n" }],
                },
            }),
            json!({ "jsonrpc": "2.0", "id": 2, "method": "shutdown" }),
            json!({ "jsonrpc": "2.0", "method": "exit" }),
        ])
        .unwrap();

        let published = messages
            .iter()
            .filter(|message| {
                message["method"] == "textDocument/publishDiagnostics"
                    && message["params"]["uri"] == part_uri
            })
            .collect::<Vec<_>>();
        assert_eq!(published.len(), 2);
        assert_eq!(published[0]["params"]["diagnostics"][0]["code"], "E064");
        assert!(
            published[1]["params"]["diagnostics"]
                .as_array()
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn closing_an_import_overlay_falls_back_to_disk() {
        let fixture = Fixture::new();
        fixture.write("app.ice", APP_WITH_PART);
        fixture.write("part.ice", "component Broken()\n  wat\n");
        let root_uri = file_path_uri(&fixture.path("app.ice"));
        let part_uri = file_path_uri(&fixture.path("part.ice"));

        let messages = run(&[
            json!({ "jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {} }),
            json!({
                "jsonrpc": "2.0",
                "method": "textDocument/didOpen",
                "params": { "textDocument": { "uri": root_uri, "text": APP_WITH_PART } },
            }),
            json!({
                "jsonrpc": "2.0",
                "method": "textDocument/didOpen",
                "params": { "textDocument": { "uri": part_uri, "text": "component Broken()\n  text \"Unsaved\"\n" } },
            }),
            json!({
                "jsonrpc": "2.0",
                "method": "textDocument/didClose",
                "params": { "textDocument": { "uri": part_uri } },
            }),
            json!({ "jsonrpc": "2.0", "id": 2, "method": "shutdown" }),
            json!({ "jsonrpc": "2.0", "method": "exit" }),
        ])
        .unwrap();

        let published = messages
            .iter()
            .filter(|message| {
                message["method"] == "textDocument/publishDiagnostics"
                    && message["params"]["uri"] == part_uri
            })
            .collect::<Vec<_>>();
        let counts = published
            .iter()
            .map(|message| message["params"]["diagnostics"].as_array().unwrap().len())
            .collect::<Vec<_>>();
        assert_eq!(counts, [1, 0, 1]);
        assert_eq!(published[0]["params"]["diagnostics"][0]["code"], "E064");
        assert_eq!(published[2]["params"]["diagnostics"][0]["code"], "E064");
    }

    #[test]
    fn fragments_keep_root_owned_diagnostics_aggregated() {
        let fixture = Fixture::new();
        fixture.write("one.ice", "app One\nview\n  text \"Saved\"\n");
        fixture.write("two.ice", "app Two\nview\n  text \"Saved\"\n");
        fixture.write("part.ice", "component Broken()\n  wat\n");
        let one_uri = file_path_uri(&fixture.path("one.ice"));
        let two_uri = file_path_uri(&fixture.path("two.ice"));
        let part_uri = file_path_uri(&fixture.path("part.ice"));
        let one = "app One\nuse \"part.ice\"\nview\n  Broken()\n";
        let two = "app Two\nuse \"part.ice\"\nview\n  Broken()\n";
        let fragment = "component Broken()\n  text \"Open buffer\"\n";

        let messages = run(&[
            json!({ "jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {} }),
            json!({
                "jsonrpc": "2.0",
                "method": "textDocument/didOpen",
                "params": { "textDocument": { "uri": one_uri, "text": one } },
            }),
            json!({
                "jsonrpc": "2.0",
                "method": "textDocument/didOpen",
                "params": { "textDocument": { "uri": two_uri, "text": two } },
            }),
            json!({
                "jsonrpc": "2.0",
                "method": "textDocument/didOpen",
                "params": { "textDocument": { "uri": part_uri, "text": fragment } },
            }),
            json!({
                "jsonrpc": "2.0",
                "method": "textDocument/didClose",
                "params": { "textDocument": { "uri": part_uri } },
            }),
            json!({
                "jsonrpc": "2.0",
                "method": "textDocument/didClose",
                "params": { "textDocument": { "uri": one_uri } },
            }),
            json!({
                "jsonrpc": "2.0",
                "method": "textDocument/didClose",
                "params": { "textDocument": { "uri": two_uri } },
            }),
            json!({ "jsonrpc": "2.0", "id": 2, "method": "shutdown" }),
            json!({ "jsonrpc": "2.0", "method": "exit" }),
        ])
        .unwrap();

        let published = messages
            .iter()
            .filter(|message| {
                message["method"] == "textDocument/publishDiagnostics"
                    && message["params"]["uri"] == part_uri
            })
            .collect::<Vec<_>>();
        let counts = published
            .iter()
            .map(|message| message["params"]["diagnostics"].as_array().unwrap().len())
            .collect::<Vec<_>>();
        assert_eq!(counts, [1, 2, 0, 2, 1, 0]);
        assert!(published.iter().all(|message| {
            message["params"]["diagnostics"]
                .as_array()
                .unwrap()
                .iter()
                .all(|diagnostic| diagnostic["code"] == "E064")
        }));
    }

    #[test]
    fn formats_open_documents_and_completes_from_the_schema() {
        let uri = "file:///tmp/demo.ice";
        let source = "app Demo\ntheme\n    background #000000\n    foreground #ffffff\n    primary #333333\n    danger #ff0000\nview\n    text \"😀\"";
        let messages = run(&[
            json!({ "jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {} }),
            json!({
                "jsonrpc": "2.0",
                "method": "textDocument/didOpen",
                "params": { "textDocument": { "uri": uri, "text": source } },
            }),
            json!({
                "jsonrpc": "2.0",
                "id": 2,
                "method": "textDocument/formatting",
                "params": { "textDocument": { "uri": uri }, "options": {} },
            }),
            json!({
                "jsonrpc": "2.0",
                "id": 3,
                "method": "textDocument/completion",
                "params": { "textDocument": { "uri": uri }, "position": { "line": 7, "character": 0 } },
            }),
            json!({ "jsonrpc": "2.0", "id": 4, "method": "shutdown" }),
            json!({ "jsonrpc": "2.0", "method": "exit" }),
        ])
        .unwrap();

        assert!(
            response(&messages, 2)["result"][0]["newText"]
                .as_str()
                .unwrap()
                .contains("\n  text \"😀\"\n")
        );
        assert_eq!(
            response(&messages, 2)["result"][0]["range"],
            json!({
                "start": { "line": 0, "character": 0 },
                "end": { "line": 7, "character": 13 },
            })
        );
        let component = response(&messages, 3)["result"]
            .as_array()
            .unwrap()
            .iter()
            .find(|item| item["label"] == "component")
            .unwrap();
        assert_eq!(component["insertText"], "component ${1:Name}(${2})\n  $0");
        assert_eq!(component["insertTextFormat"], 2);
    }

    #[test]
    fn returns_json_rpc_errors_and_rejects_requests_after_shutdown() {
        let messages = run(&[
            json!({ "jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {} }),
            json!({ "jsonrpc": "2.0", "id": "unknown", "method": "ice/unknown" }),
            json!({
                "jsonrpc": "2.0",
                "id": 2,
                "method": "textDocument/formatting",
                "params": { "textDocument": { "uri": "file:///not-open.ice" }, "options": {} },
            }),
            json!({ "jsonrpc": "2.0", "id": 3, "method": "shutdown" }),
            json!({ "jsonrpc": "2.0", "id": 4, "method": "textDocument/completion" }),
            json!({ "jsonrpc": "2.0", "method": "exit" }),
        ])
        .unwrap();

        assert_eq!(response(&messages, "unknown")["error"]["code"], -32601);
        assert_eq!(response(&messages, 2)["error"]["code"], -32602);
        assert_eq!(response(&messages, 3)["result"], Value::Null);
        assert_eq!(response(&messages, 4)["error"]["code"], -32600);
    }

    #[test]
    fn exit_before_shutdown_is_an_error() {
        let mut input = Vec::new();
        frame(&json!({ "jsonrpc": "2.0", "method": "exit" }), &mut input);
        let error = serve(&mut BufReader::new(Cursor::new(input)), &mut Vec::new()).unwrap_err();

        assert_eq!(error.kind(), std::io::ErrorKind::InvalidData);
        assert_eq!(error.to_string(), "LSP exit received before shutdown");
    }

    #[test]
    fn ranges_use_utf16_and_clamp_to_the_document() {
        assert_eq!(
            diagnostic_range("a😀b\n", 1, 2),
            json!({
                "start": { "line": 0, "character": 1 },
                "end": { "line": 0, "character": 3 },
            })
        );
        assert_eq!(
            diagnostic_range("a😀b\n", 99, 99),
            json!({
                "start": { "line": 1, "character": 0 },
                "end": { "line": 1, "character": 0 },
            })
        );
        assert_eq!(
            whole_document_range("first\n😀x"),
            json!({
                "start": { "line": 0, "character": 0 },
                "end": { "line": 1, "character": 3 },
            })
        );
        let path = Path::new("/tmp/Ice Demo/😀.ice");
        assert_eq!(file_uri_path(&file_path_uri(path)).as_deref(), Some(path));
    }
}
