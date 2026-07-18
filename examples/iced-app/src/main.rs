ui_lang::include_app!("src/ui/tasks.ice");

mod backend;

fn main() -> iced::Result {
    Tasks::run()
}

#[cfg(test)]
mod tests;
