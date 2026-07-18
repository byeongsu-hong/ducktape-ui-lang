#[derive(Clone, Debug, PartialEq)]
pub struct EditorCommand {
    pub save: bool,
}

#[derive(Debug)]
pub struct DemoHighlighter {
    token: String,
    line: usize,
}

impl iced::advanced::text::Highlighter for DemoHighlighter {
    type Settings = String;
    type Highlight = ();
    type Iterator<'a> = std::option::IntoIter<(std::ops::Range<usize>, ())>;

    fn new(settings: &Self::Settings) -> Self {
        Self {
            token: settings.clone(),
            line: 0,
        }
    }

    fn update(&mut self, settings: &Self::Settings) {
        self.token.clone_from(settings);
        self.line = 0;
    }

    fn change_line(&mut self, line: usize) {
        self.line = line;
    }

    fn highlight_line(&mut self, line: &str) -> Self::Iterator<'_> {
        self.line += 1;
        line.find(&self.token)
            .map(|start| (start..start + self.token.len(), ()))
            .into_iter()
    }

    fn current_line(&self) -> usize {
        self.line
    }
}

pub fn editor_keys(
    event: iced::widget::text_editor::KeyPress,
    readonly: bool,
) -> Option<iced::widget::text_editor::Binding<EditorCommand>> {
    if event.key.to_latin(event.physical_key) == Some('s') && event.modifiers.command() {
        Some(iced::widget::text_editor::Binding::Custom(EditorCommand {
            save: true,
        }))
    } else if readonly {
        None
    } else {
        iced::widget::text_editor::Binding::from_key_press(event)
    }
}

pub fn editor_highlight<'a, Message: 'a>(
    editor: iced::widget::text_editor::TextEditor<
        'a,
        iced::advanced::text::highlighter::PlainText,
        Message,
    >,
    token: String,
) -> iced::Element<'a, Message> {
    editor
        .highlight_with::<DemoHighlighter>(token, |_, theme| {
            iced::advanced::text::highlighter::Format {
                color: Some(theme.palette().primary),
                font: Some(iced::Font::MONOSPACE),
            }
        })
        .into()
}

pub fn editor_surface(
    theme: &iced::Theme,
    status: iced::widget::text_editor::Status,
    readonly: bool,
) -> iced::widget::text_editor::Style {
    let mut style = iced::widget::text_editor::default(theme, status);
    if readonly {
        style.value = theme.palette().text;
    }
    style
}
