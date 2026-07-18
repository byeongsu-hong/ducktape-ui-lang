macro_rules! example {
    ($file:literal) => {
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../examples/iced-app/src/ui/",
            $file
        ))
    };
}

pub(crate) use example;
