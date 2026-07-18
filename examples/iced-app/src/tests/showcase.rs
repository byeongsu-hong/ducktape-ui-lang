ui_lang::include_app!("src/ui/showcase.ice");

#[test]
fn qr_data_initializes() {
    let _ = Showcase::__boot();
}

#[test]
fn appends_markdown_and_tracks_image_uris() {
    let (mut app, _) = Showcase::__boot();
    assert!(app.help_images.is_empty());

    assert_eq!(app.__update(__ShowcaseMessage::ExtendMarkdown).units(), 0);
    assert_eq!(app.help_images, ["asset://ice"]);
}
