use async_trait::async_trait;
use color_eyre::eyre::{Result, eyre};
use derive_where::derive_where;
use std::marker::PhantomData;
use vizual::geometry::Direction;
use vizual::{
    component::{Children, context::Component_context},
    handlers::Retrieve_handler,
    layouter::hitbox::Hitbox,
    slot::manager::Slots,
    state::State,
    sync::Thread_safe,
    theme::Theme,
    widget::{
        Focus_provider, Shared_widget, Widget, Widget_trait,
        custom_widget::Custom_widget_trait,
        widgets::{
            layout::axis::Axis,
            menu::{Menu, Shared_menu_item, get_selector},
            positioning::anchor::{Anchor, Anchors},
            text::Text,
        },
    },
};
use vizual_macros::display;

use crate::Field;

#[derive_where(Clone)]
struct Default_leaf_value<Value: Thread_safe> {
    label: String,
    value: PhantomData<Value>,
}

#[async_trait]
impl<Value: Thread_safe> Retrieve_handler<Option<Value>> for Default_leaf_value<Value> {
    async fn on_retrieve(&mut self) -> Result<Option<Value>> {
        Ok(None)
    }
}

#[async_trait]
impl<Value: Thread_safe> Custom_widget_trait for Default_leaf_value<Value> {
    type Payload = bool;

    async fn layout(
        &mut self,
        _render: vizual::Render,
        theme: State<Theme>,
        _focus: &mut Focus_provider,
        _hitbox: &mut Hitbox,
        _parent: Hitbox,
        _problem: Component_context,
        _text_context: &mut vizual::text::Text_context,
        slots: &mut Slots,
        selected: bool,
    ) -> Result<Children> {
        let mut text = Text::new(format!("Default - {}", self.label));
        text.style.set(match selected {
            true => theme.load().specific.text.selected_subtitle,
            false => theme.load().specific.text.subtitle,
        });
        let text = Anchor::new(text, Anchors::top_left());

        Ok(vec![display!(text)])
    }
}

#[derive_where(Clone)]
struct Custom_leaf_value<Value: Thread_safe> {
    field: Shared_widget<Box<dyn Field<Value>>>,
}

#[async_trait]
impl<Value: Thread_safe> Retrieve_handler<Option<Value>> for Custom_leaf_value<Value> {
    async fn on_retrieve(&mut self) -> Result<Option<Value>> {
        let value = self
            .field
            .lock()
            .await?
            .on_retrieve()
            .await?
            .ok_or_else(|| eyre!("Expected to get value from custom field"))?;
        Ok(Some(value))
    }
}

#[async_trait]
impl<Value: Thread_safe> Custom_widget_trait for Custom_leaf_value<Value> {
    type Payload = bool;

    async fn layout(
        &mut self,
        _render: vizual::Render,
        theme: State<Theme>,
        _focus: &mut Focus_provider,
        _hitbox: &mut Hitbox,
        _parent: Hitbox,
        _problem: Component_context,
        _text_context: &mut vizual::text::Text_context,
        slots: &mut Slots,
        selected: bool,
    ) -> Result<Children> {
        let mut title = Text::new("Custom");
        title.style.set(match selected {
            true => theme.load().specific.text.selected_subtitle,
            false => theme.load().specific.text.subtitle,
        });
        let title = Anchor::new(title, Anchors::top_left());
        let field = Anchor::new(self.field.clone(), Anchors::top_left());
        let contents: Vec<Widget> = match selected {
            true => vec![Box::new(title), Box::new(field)],
            false => vec![Box::new(title)],
        };
        let axis = Axis::new(Direction::Vertical, contents);

        Ok(vec![display!(axis)])
    }
}

#[derive_where(Clone)]
pub struct Optional_setting<Value: Clone + Thread_safe> {
    menu: Shared_widget<Menu<Option<Value>>>,
}

impl<Value: Clone + Thread_safe> Optional_setting<Value> {
    pub fn new(
        default_value: impl Into<String>,
        is_default: bool,
        field: impl Field<Value> + 'static,
        render: vizual::Render,
    ) -> Self {
        let field = Widget_trait::into_shared(Box::new(field) as Box<dyn Field<Value>>);
        let default_item: Shared_menu_item<Option<Value>> = Default_leaf_value {
            label: default_value.into(),
            value: PhantomData,
        }
        .into_shared()
        .into();
        let custom_item: Shared_menu_item<Option<Value>> =
            Custom_leaf_value { field }.into_shared().into();
        let items = vec![default_item, custom_item];
        let default_item = get_selector(&items[usize::from(!is_default)]);
        let menu = Widget_trait::into_shared(Menu::new(items, default_item, render));

        Self { menu }
    }
}

#[async_trait]
impl<Value: Clone + Thread_safe> Widget_trait for Optional_setting<Value> {
    async fn layout(
        &mut self,
        _render: vizual::Render,
        _theme: State<Theme>,
        _focus: &mut Focus_provider,
        _hitbox: &mut Hitbox,
        _parent: Hitbox,
        _problem: Component_context,
        _text_context: &mut vizual::text::Text_context,
        slots: &mut Slots,
    ) -> Result<Children> {
        let menu = self.menu.clone();
        Ok(vec![display!(menu)])
    }
}

#[async_trait]
impl<Value: Clone + Thread_safe> Retrieve_handler<Option<Value>> for Optional_setting<Value> {
    async fn on_retrieve(&mut self) -> Result<Option<Value>> {
        self.menu.on_retrieve().await
    }
}
