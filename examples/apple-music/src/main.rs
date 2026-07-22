ui_lang::include_app!("src/ui/music.ice");

mod liquid_glass;
mod mock_api;

fn main() -> iced::Result {
    Music::run()
}
