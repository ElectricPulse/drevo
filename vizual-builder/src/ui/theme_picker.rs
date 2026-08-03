use async_trait::async_trait;
use color_eyre::eyre::Result;
use lucide_icons::Icon as Lucide_icon;
use vizual::{
    Vizual_command, Vizual_msg,
    component::{Children, context::Component_context},
    constraint,
    event::Pointer_event,
    geometry::Direction,
    handlers::Retrieve_handler,
    layouter::{expression::Expression, hitbox::Hitbox},
    slot::manager::Slots,
    state::State,
    theme::{Theme, Theme_choice, Theme_manager},
    widget::{
        Focus_provider, Widget_trait,
        custom_widget::Custom_widget_trait,
        widgets::{
            button::Button,
            full::Full,
            icon::Icon,
            menu::{Menu, Shared_menu_item, get_selector},
            text::Text,
        },
    },
};
use vizual_macros::display;

fn label(choice: Theme_choice) -> &'static str {
    match choice {
        Theme_choice::Light => "Light",
        Theme_choice::User => "User",
        Theme_choice::System => "System",
    }
}

struct Theme_menu_item {
    choice: Theme_choice,
    theme: State<Theme>,
}

#[async_trait]
impl Retrieve_handler<Theme_choice> for Theme_menu_item {
    async fn on_retrieve(&mut self) -> Result<Theme_choice> {
        Ok(self.choice)
    }
}

#[async_trait]
impl Custom_widget_trait for Theme_menu_item {
    type Payload = bool;

    async fn layout(
        &mut self,
        _focus: &mut Focus_provider,
        _hitbox: &mut Hitbox,
        _parent: Hitbox,
        _problem: Component_context,
        _text_context: &mut vizual::text::Text_context,
        slots: &mut Slots,
        selected: bool,
    ) -> Result<Children> {
        let style = match selected {
            true => self
                .theme
                .project(|theme| &theme.specific.text.selected_subtitle),
            false => self.theme.project(|theme| &theme.specific.text.subtitle),
        };

        let text = Text::new(label(self.choice), style);
        Ok(vec![display!(text)])
    }
}

pub struct Theme_picker {
    open: State<bool>,
    themes: Theme_manager,
}

impl Theme_picker {
    pub fn new(open: State<bool>, themes: Theme_manager) -> Self {
        Self { open, themes }
    }
}

#[async_trait]
impl Widget_trait for Theme_picker {
    async fn layout(
        &mut self,
        _focus: &mut Focus_provider,
        _hitbox: &mut Hitbox,
        _parent: Hitbox,
        problem: Component_context,
        _text_context: &mut vizual::text::Text_context,
        slots: &mut Slots,
    ) -> Result<Children> {
        let selected_choice = *self.themes.choice.load();
        self.themes.apply();

        let icon = Icon::new(Lucide_icon::Settings, (&self.themes.theme).into());
        let button = Button::around(display!(icon), self.themes.theme.clone());
        let button = display!(button);
        let button = display!(Full::new(button));

        if !*self.open.load() {
            return Ok(vec![button]);
        }

        let choices = [
            Theme_choice::Light,
            Theme_choice::User,
            Theme_choice::System,
        ];
        
        let selected_index = choices
            .iter()
            .position(|choice| *choice == selected_choice)
            .expect("selected theme choice must be present in the theme menu");
        let items = choices
            .into_iter()
            .map(|choice| -> Shared_menu_item<Theme_choice> {
                Theme_menu_item {
                    choice,
                    theme: self.themes.theme.clone(),
                }
                .into_shared()
            })
            .collect::<Vec<_>>();
        let default_item = get_selector(&items[selected_index]);
        let menu = Menu::new(
            items,
            default_item,
            Some(self.themes.choice.clone()),
            self.themes.theme.clone(),
        );
        let menu = display!(menu);
        let button_hitbox = button.get_hitbox().await?;
        let menu_hitbox = menu.get_hitbox().await?;
        // let button_center =
        //     (Expression::from(button_hitbox.get_start_position(Direction::Horizontal))
        //         + button_hitbox.get_end_position(Direction::Horizontal))
        //         / 2.0;
        // let menu_center = (Expression::from(menu_hitbox.get_start_position(Direction::Horizontal))
        //     + menu_hitbox.get_end_position(Direction::Horizontal))
        //     / 2.0;

        // problem
        //     .minimize_difference(menu_center - button_center, 0.0, None, 1)
        //     .await?;
        // problem
        //     .constrain(constraint!(
        //         menu_hitbox.get_start_position(Direction::Vertical)
        //             >= button_hitbox.get_end_position(Direction::Vertical)
        //     ))
        //     .await?;
        // // TODO: Menus could hover over their button when there is not enough room below it.
        // problem
        //     .minimize_difference(
        //         menu_hitbox.get_start_position(Direction::Vertical)
        //             - button_hitbox.get_end_position(Direction::Vertical),
        //         0.0,
        //         None,
        //         1,
        //     )
        //     .await?;

        Ok(vec![button, menu])
    }

    async fn on_mouse_click(&mut self, _mouse: &Pointer_event) -> Result<Vizual_msg> {
        self.open.store(!*self.open.load());
        Vizual_msg::new(Vizual_command::Layout)
    }
}
