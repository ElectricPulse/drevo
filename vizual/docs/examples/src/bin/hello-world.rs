use color_eyre::eyre::Result;
use vizual::{
    geometry::Direction,
    render_manager::Render_manager,
    theme,
    widget::{Widget_trait as _, widgets::paragraph::Paragraph},
};

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;

    let render_manager = Render_manager::new();
    let mut paragraph = Paragraph::new(Direction::Horizontal, 320.0);
    paragraph.set_styled_content("Hello from Vizual", theme::dark_theme().specific.text.paragraph);

    vizual::run("Vizual example", paragraph.into_shared(), render_manager)
}
