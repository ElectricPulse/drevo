use color_eyre::eyre::Result;
use vizual::{
    render_manager::Render_manager,
    widget::{Widget_trait as _, widgets::paragraph::Paragraph},
};

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;

    let render_manager = Render_manager::new();
    let mut paragraph = Paragraph::new();
    paragraph.set_content("Hello from Vizual".into());

    vizual::run("Vizual example", paragraph.into_shared(), render_manager)
}
