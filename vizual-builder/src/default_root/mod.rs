pub mod header;

use color_eyre::eyre::Result;
use vizual::{
    component::{Children, context::Component_context},
    geometry::Direction,
    layouter::{hitbox::Hitbox, objective::Objective, screen::SCREEN},
    slot::manager::Slots,
    state::State,
    theme::Theme_manager,
    widget::{
        Focus_provider, Shared_widget, Widget_trait,
        widgets::{full::Full, layout::Layout, paper::Paper, space::Space},
    },
};
use vizual_macros::display;

use self::header::Header;

pub struct Default_root<T: Widget_trait> {
    title: String,
    widget: Shared_widget<T>,
    themes: Theme_manager,
    header_open: Option<State<bool>>,
}

impl<T: Widget_trait> Default_root<T> {
    pub fn new(title: impl Into<String>, widget: Shared_widget<T>, themes: Theme_manager) -> Self {
        Self {
            title: title.into(),
            widget,
            themes,
            header_open: None,
        }
    }
}

#[async_trait::async_trait]
impl<T: Widget_trait> Widget_trait for Default_root<T> {
    async fn layout(
        &mut self,
        render: vizual::Render,
        _focus: &mut Focus_provider,
        _hitbox: &mut Hitbox,
        _parent: Hitbox,
        _problem: Component_context,
        _text_context: &mut vizual::text::Text_context,
        slots: &mut Slots,
    ) -> Result<Children> {
        let widget = Space::right(
            display!(self.widget.clone()),
            SCREEN.width,
            Objective::default(),
            1,
        );
        let widget = Paper::new(display!(widget), (&self.themes.theme).into());
        let header_open = self
            .header_open
            .get_or_insert_with(|| render.new_state(false))
            .clone();
        let header = Header::new(self.title.clone(), header_open, self.themes.clone());
        let header = display!(header);
        let header = Full::width(header);
        let layout = Layout::new(
            Direction::Vertical,
            vec![display!(header), display!(widget)],
            (&self.themes.theme).into(),
            Objective::default(),
            2,
        );
        let layout = Full::new(display!(layout));
        let root_style = self.themes.theme.project(|theme| &theme.specific.root);
        let root = Paper::new(display!(layout), root_style);

        Ok(vec![display!(root)])
    }
}
