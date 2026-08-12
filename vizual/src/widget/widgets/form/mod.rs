pub mod text_input;

use async_trait::async_trait;
use color_eyre::eyre::Result;
use derive_where::derive_where;
use std::future::Future;
use std::pin::Pin;
use vizual_macros::display;

use super::super::{Focus_provider, Shared_widget, Widget, Widget_trait};
use super::menu::Menu;
use super::title_block::Title_block;
use crate::{
    component::{Children, context::Component_context},
    display::Display,
    event::{Event, Key_code, Key_event, Pointer_event},
    geometry::Rect,
    handlers::Retrieve_handler,
    layouter::hitbox::Hitbox,
    slot::manager::Slots,
    state::State,
    sync::Thread_safe,
    theme::Theme,
    utils::get_next_index,
};

pub trait Field<Config>: Widget_trait {
    fn get_name(&self) -> &str;
    fn submit(&self, config: &mut Config) -> Result<()>;
}

dyn_clone::clone_trait_object!(<Config> Field<Config>);

#[async_trait]
impl<Config: 'static> Widget_trait for Box<dyn Field<Config>> {
    async fn layout(
        &mut self,
        render: crate::Render,
        theme: State<Theme>,
        focus: &mut Focus_provider,
        hitbox: &mut Hitbox,
        parent: Hitbox,
        problem: Component_context,
        text_context: &mut crate::text::Text_context,
        slots: &mut Slots,
    ) -> Result<Children> {
        (**self)
            .layout(
                render,
                theme,
                focus,
                hitbox,
                parent,
                problem,
                text_context,
                slots,
            )
            .await
    }

    async fn render(
        &mut self,
        theme: State<Theme>,
        focus: &mut Focus_provider,
        hitbox: Rect,
        display: &mut Display<'_>,
    ) -> Result<Option<Hitbox>> {
        (**self).render(theme, focus, hitbox, display).await
    }

    async fn on_all_events(&mut self, event: &Event) -> Result<crate::Vizual_msg> {
        (**self).on_all_events(event).await
    }

    async fn on_mouse_click(&mut self, mouse: &Pointer_event) -> Result<crate::Vizual_msg> {
        (**self).on_mouse_click(mouse).await
    }

    async fn on_key_press(&mut self, key: &Key_event) -> Result<crate::Vizual_msg> {
        (**self).on_key_press(key).await
    }

    async fn on_other_event(&mut self, event: &Event) -> Result<crate::Vizual_msg> {
        (**self).on_other_event(event).await
    }

    async fn forward_event(&mut self, event: &Event) -> Result<crate::Vizual_msg> {
        (**self).forward_event(event).await
    }
}

pub type Shared_field<Config> = Shared_widget<Box<dyn Field<Config>>>;

#[derive_where(Clone, Default)]
pub struct Fields<Config: 'static> {
    fields: Vec<Shared_field<Config>>,
}

impl<Config: 'static> Fields<Config> {
    pub fn add<Generic_field>(&mut self, field: Shared_field<Config>) {
        self.fields.push(field);
    }
}

pub type Submit_future = Pin<Box<dyn Future<Output = Result<()>> + Send>>;

pub trait Submit_callback_trait<Config>:
    Fn(Config) -> Submit_future + Thread_safe + dyn_clone::DynClone
{
}

impl<Config, Callback> Submit_callback_trait<Config> for Callback where
    Callback: Fn(Config) -> Submit_future + Thread_safe + Clone
{
}

dyn_clone::clone_trait_object!(<Config> Submit_callback_trait<Config>);

pub type Submit_callback<Config> = Box<dyn Submit_callback_trait<Config>>;

#[derive(Clone)]
pub struct Form<Config: Clone + Thread_safe> {
    field_index: usize,
    field_active: bool,
    fields: Fields<Config>,
    config: Config,
    exitting: bool,
    exit_menu: Shared_widget<Menu<bool>>,
    on_submit: Submit_callback<Config>,
}

impl<Config: Clone + Thread_safe> Form<Config> {
    pub fn new<Submit_fn, Submit_future_impl>(
        fields: Fields<Config>,
        config: Config,
        on_submit: Submit_fn,
        render: crate::Render,
    ) -> Self
    where
        Submit_fn: Fn(Config) -> Submit_future_impl + Thread_safe + Clone,
        Submit_future_impl: Future<Output = Result<()>> + Send + 'static,
    {
        assert!(
            !fields.fields.is_empty(),
            "Form requires at least one field"
        );

        Self {
            config,
            field_index: 0,
            field_active: false,
            exitting: false,
            exit_menu: Menu::boolean(false, render).into_shared(),
            fields,
            on_submit: Box::new(move |config| -> Submit_future { Box::pin(on_submit(config)) }),
        }
    }
}

impl<Config: Clone + Thread_safe> Form<Config> {
    async fn title(&mut self) -> Result<String> {
        if self.exitting {
            Ok("Do you want to exit and submit?".to_owned())
        } else {
            let name = self.get_current_field().lock().await?.get_name().to_owned();
            Ok(format!(
                "{}/{} - {}",
                self.field_index + 1,
                self.get_number_of_fields(),
                name
            ))
        }
    }

    async fn ensure_exit_menu_closed(&mut self) -> Result<()> {
        self.exit_menu.lock().await?.set_selected(false)?;
        self.field_active = false;
        self.exitting = false;
        Ok(())
    }

    fn get_current_field(&mut self) -> &mut Shared_field<Config> {
        self.fields
            .fields
            .get_mut(self.field_index)
            .expect("Expected self.field_index to be in range")
    }

    fn get_number_of_fields(&self) -> usize {
        self.fields.fields.len()
    }

    async fn submit(&mut self) -> Result<()> {
        for field in self.fields.fields.iter_mut() {
            field.lock().await?.submit(&mut self.config)?;
        }

        (self.on_submit)(self.config.clone()).await
    }
}

#[async_trait]
impl<Config: Clone + Thread_safe> Widget_trait for Form<Config> {
    async fn layout(
        &mut self,
        _render: crate::Render,
        _theme: State<Theme>,
        focus: &mut Focus_provider,
        _hitbox: &mut Hitbox,
        _parent: Hitbox,
        _problem: Component_context,
        _text_context: &mut crate::text::Text_context,
        slots: &mut Slots,
    ) -> Result<Children> {
        focus.set_active(true);
        let child: Widget = if self.exitting {
            let exit_menu = self.exit_menu.clone();
            Box::new(exit_menu)
        } else {
            let field = self.get_current_field().clone();
            Box::new(field)
        };

        let block = Title_block::new(child, self.title().await?);
        Ok(vec![display!(block)])
    }

    async fn on_key_press(&mut self, key: &Key_event) -> Result<crate::Vizual_msg> {
        if self.exitting {
            let (message, submitted, selected) = {
                let mut exit_menu = self.exit_menu.lock().await?;
                let message = exit_menu.on_key_press(key).await?;
                //let submitted = exit_menu.submitted;
                let submitted = false;
                let selected = if submitted {
                    exit_menu.on_retrieve().await?
                } else {
                    false
                };
                (message, submitted, selected)
            };

            if submitted {
                if selected {
                    self.submit().await?;
                    return crate::Vizual_msg::new(crate::Vizual_command::Quit);
                }

                self.ensure_exit_menu_closed().await?;
            }

            return Ok(message);
        }

        match key.code {
            Key_code::Enter => {
                self.field_index = get_next_index(self.get_number_of_fields(), self.field_index);
                self.field_active = false;
                return crate::Vizual_msg::new(crate::Vizual_command::Layout);
            }
            Key_code::Arrow_right => {
                if self.get_number_of_fields() - 1 == self.field_index {
                    self.field_active = false;
                    self.exitting = true;
                    return crate::Vizual_msg::new(crate::Vizual_command::Layout);
                }
            }
            _ => {}
        }

        crate::Vizual_msg::none()
    }
}
