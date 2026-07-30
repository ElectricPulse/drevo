#![warn(rustdoc::broken_intra_doc_links)]
//! Tree-based configuration editing for Vizual applications.
//!
//! Implement [`Tree`] to describe editable fields and produce a serializable
//! configuration.

use async_recursion::async_recursion;
use async_trait::async_trait;
use color_eyre::eyre::{Result, WrapErr, eyre};
use indexmap::IndexMap;
use serde::Serialize;
use std::{
    fs,
    marker::PhantomData,
    path::{Path, PathBuf},
    sync::Arc,
};
use vizual::{
    Vizual_command, Vizual_msg, check_quit_event,
    component::{Child, context::Component_context},
    display::Display,
    event::{Key_code, Key_event},
    geometry::{Direction, Rect},
    handlers::{Retrieve_handler, Submit_handler},
    layouter::{hitbox::Hitbox, objective::Objective},
    slot::{Component_slot, manager::Slots},
    state::State,
    sync::{Mutex, Thread_safe},
    theme::Theme,
    utils::get_strings_id,
    widget::{
        Control, Focus_provider, Shared_widget, Widget, Widget_trait, Widget_type,
        widgets::{
            align::{Align, Alignments},
            anchor::{Anchor, Anchors, Position as Anchor_position},
            button::Button,
            grid::Grid,
            layout::{Layout, Style as Layout_style},
            linebreak::Linebreak,
            menu::{Menu, Menu_item_trait, Shared_menu_item, get_selector},
            popup::Popup,
            space::Space,
            text::Text,
            title_block::Title_block,
        },
    },
};
use vizual_macros::display;

#[async_trait]
/// Supplies the fields displayed by a [`Configurator`].
pub trait Tree: Thread_safe {
    type Configuration: Serialize;

    fn get_tree(&self) -> Configuration_tree_branch;
    async fn create_config(&mut self) -> Result<Self::Configuration>;
}

#[async_trait]
/// A widget field that can return an optional configured value.
pub trait Field<Value>: Widget_trait + Retrieve_handler<Option<Value>> {}

#[async_trait]
impl<Value: 'static> Widget_trait for Box<dyn Field<Value>> {
    async fn layout(
        &mut self,
        focus: &mut Focus_provider,
        hitbox: Hitbox,
        parent: Hitbox,
        problem: Component_context,
        text_context: &mut vizual::text::Text_context,
        slots: &mut Slots,
    ) -> Result<Widget_type> {
        (**self)
            .layout(focus, hitbox, parent, problem, text_context, slots)
            .await
    }

    async fn render(
        &mut self,
        focus: &mut Focus_provider,
        hitbox: Rect,
        display: &mut Display<'_>,
    ) -> Result<Option<Hitbox>> {
        (**self).render(focus, hitbox, display).await
    }
}

struct Default_leaf_value<Value: Thread_safe> {
    label: String,
    theme: State<Theme>,
    value: PhantomData<Value>,
}

#[async_trait]
impl<Value: Thread_safe> Retrieve_handler<Option<Value>> for Default_leaf_value<Value> {
    async fn on_retrieve(&mut self) -> Result<Option<Value>> {
        Ok(None)
    }
}

#[async_trait]
impl<Value: Thread_safe> Menu_item_trait<Option<Value>> for Default_leaf_value<Value> {
    async fn layout(
        &mut self,
        selected: bool,
        _focus: &mut Focus_provider,
        _hitbox: Hitbox,
        _parent: Hitbox,
        _problem: Component_context,
        _text_context: &mut vizual::text::Text_context,
        slots: &mut Slots,
    ) -> Result<Child> {
        let text = Text::new(format!("Default - {}", self.label))
            .set_style(self.theme.load().semantic.text.subtitle(selected));

        Ok(display!(text))
    }
}

struct Custom_leaf_value<Value: Thread_safe> {
    field: Shared_widget<Box<dyn Field<Value>>>,
    theme: State<Theme>,
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
impl<Value: Thread_safe> Menu_item_trait<Option<Value>> for Custom_leaf_value<Value> {
    async fn layout(
        &mut self,
        selected: bool,
        _focus: &mut Focus_provider,
        _hitbox: Hitbox,
        _parent: Hitbox,
        _problem: Component_context,
        _text_context: &mut vizual::text::Text_context,
        slots: &mut Slots,
    ) -> Result<Child> {
        let title =
            Text::new("Custom").set_style(self.theme.load().semantic.text.subtitle(selected));
        let field = self.field.clone();
        let contents = match selected {
            true => vec![Some(display!(title)), Some(display!(field))],
            false => vec![Some(display!(title))],
        };
        let layout = Layout::new(
            Direction::Vertical,
            contents,
            Layout_style::default(self.theme.clone()),
            Objective::default(),
            2,
        );

        Ok(display!(layout))
    }
}

/// Builds a menu for choosing a default value or editing a custom value.
pub fn configuration_menu<Value: Thread_safe>(
    default_value: impl Into<String>,
    is_default: bool,
    field: impl Field<Value> + 'static,
    theme: State<Theme>,
) -> Menu<Option<Value>> {
    let field = (Box::new(field) as Box<dyn Field<Value>>).into_shared();
    let default_item = Arc::new(Mutex::new(Default_leaf_value {
        label: default_value.into(),
        theme: theme.clone(),
        value: PhantomData,
    })) as Shared_menu_item<Option<Value>>;
    let custom_item = Arc::new(Mutex::new(Custom_leaf_value {
        field,
        theme: theme.clone(),
    })) as Shared_menu_item<Option<Value>>;
    let items = vec![default_item, custom_item];
    let default_item = get_selector(&items[usize::from(!is_default)]);

    Menu::new(items, default_item, theme)
}

/// An ordered group of configuration nodes.
pub struct Configuration_tree_branch(pub IndexMap<String, Configuration_tree>);

impl Configuration_tree_branch {
    fn get_node(self, cursor: &[String]) -> Result<Configuration_tree> {
        let mut node = Configuration_tree::Branch(self);

        for key in cursor {
            node = match node {
                Configuration_tree::Branch(mut branch) => branch
                    .0
                    .shift_remove(key)
                    .ok_or_else(|| eyre!("Expected key to exist"))?,
                Configuration_tree::Leaf(_) => return Err(eyre!("Expected branch")),
            };
        }

        Ok(node)
    }

    pub fn get_branch(self, cursor: &[String]) -> Result<Self> {
        self.get_node(cursor)?
            .into_branch()
            .map_err(|_| eyre!("Expected branch"))
    }

    pub fn get_leaf(self, cursor: &[String]) -> Result<Configuration_tree_leaf> {
        self.get_node(cursor)?
            .into_leaf()
            .map_err(|_| eyre!("Expected leaf"))
    }
}

/// A single editable configuration field.
pub struct Configuration_tree_leaf {
    pub widget: Widget,
    pub description: String,
    pub name: String,
}

/// A branch or editable leaf in a configuration tree.
pub enum Configuration_tree {
    Branch(Configuration_tree_branch),
    Leaf(Configuration_tree_leaf),
}

impl Configuration_tree {
    pub fn new_leaf<T: Widget_trait>(
        field: &Shared_widget<T>,
        name: impl Into<String>,
        description: impl Into<String>,
    ) -> Self {
        Self::Leaf(Configuration_tree_leaf {
            widget: field.clone().into(),
            description: description.into(),
            name: name.into(),
        })
    }

    fn into_branch(self) -> std::result::Result<Configuration_tree_branch, Self> {
        match self {
            Self::Branch(branch) => Ok(branch),
            value => Err(value),
        }
    }

    fn into_leaf(self) -> std::result::Result<Configuration_tree_leaf, Self> {
        match self {
            Self::Leaf(leaf) => Ok(leaf),
            value => Err(value),
        }
    }
}

struct Tree_view<T: Tree> {
    tree: Arc<Mutex<T>>,
    configurator_state: Arc<Mutex<Configurator_state>>,
    theme: State<Theme>,
}

struct Field_click_handler {
    cursor: Vec<String>,
    configurator_state: Arc<Mutex<Configurator_state>>,
}

#[async_trait]
impl Submit_handler<String> for Field_click_handler {
    // I don't use label here as I think this argument should later be removed
    async fn on_submit(&mut self, _label: Option<String>) -> Result<Vizual_msg> {
        self.configurator_state.lock().await?.cursor = self.cursor.clone();

        // This functionality of letting the mouse event bubble up to find the parent
        // could be better documented
        Vizual_msg::new_propagated(Vizual_command::Layout)
    }
}

impl<T: Tree> Tree_view<T> {
    #[async_recursion]
    async fn render_tree(
        &mut self,
        slots: &mut Slots,
        node: &Configuration_tree_branch,
        selected_cursor: &[String],
        cursor: &[String],
        problem: &Component_context,
        button_delta: vizual::layouter::variable::Variable,
    ) -> Result<Vec<Option<Child>>> {
        const INDENT: usize = 20;

        let mut buttons: Vec<Option<Child>> = vec![];

        for (name, child) in &node.0 {
            let mut child_cursor = cursor.to_vec();
            child_cursor.push(name.clone());
            let depth = cursor.len();

            let mut button = Button::new(
                name,
                Box::new(Field_click_handler {
                    configurator_state: self.configurator_state.clone(),
                    cursor: child_cursor.clone(),
                }),
                self.theme.clone(),
            );

            button.active = selected_cursor == child_cursor;
            button.delta = Some(button_delta);

            let button = slots.set(get_strings_id(&child_cursor) + 1, button).await?;
            let button = Space::left(button, (INDENT * depth) as f64, Objective::default(), 2);

            // Since cursor should be unique for every button we can use it to generate id
            let button = slots.set(get_strings_id(&child_cursor), button).await?;

            buttons.push(Some(button));

            if let Configuration_tree::Branch(branch) = child {
                let mut child_tree = self
                    .render_tree(
                        slots,
                        branch,
                        selected_cursor,
                        &child_cursor,
                        problem,
                        button_delta,
                    )
                    .await?;
                buttons.append(&mut child_tree);
            }
        }

        Ok(buttons)
    }

    async fn move_to_sibling(&mut self, offset: isize) -> Result<()> {
        let cursor = self.configurator_state.lock().await?.cursor.clone();
        let (leaf_key, branch_cursor) = cursor
            .split_last()
            .ok_or_else(|| eyre!("Cursor can't be empty"))?;

        let tree = self.tree.lock().await?;
        let branch = tree.get_tree().get_branch(branch_cursor)?;
        let index = branch
            .0
            .get_index_of(leaf_key)
            .ok_or_else(|| eyre!("Expected leaf"))?;
        let new_key = branch
            .0
            .get_index(index.saturating_add_signed(offset))
            .map(|(key, _)| key.to_string());
        drop(tree);

        if let Some(new_key) = new_key {
            let mut configurator_state = self.configurator_state.lock().await?;

            if configurator_state.cursor == cursor {
                let leaf_key = configurator_state
                    .cursor
                    .last_mut()
                    .ok_or_else(|| eyre!("Cursor can't be empty"))?;
                *leaf_key = new_key;
            }
        }

        Ok(())
    }
}

#[async_trait]
impl<T: Tree> Widget_trait for Tree_view<T> {
    async fn layout(
        &mut self,
        focus: &mut Focus_provider,
        _hitbox: Hitbox,
        _parent: Hitbox,
        problem: Component_context,
        _text_context: &mut vizual::text::Text_context,
        slots: &mut Slots,
    ) -> Result<Widget_type> {
        focus.set_active(true);
        let cursor = self.configurator_state.lock().await?.cursor.clone();
        let button_delta = problem
            .add_delta("configurator-tree-button-delta", 2)
            .await?;

        let tree = self.tree.lock().await?.get_tree();
        let buttons = self
            .render_tree(slots, &tree, &cursor, &[], &problem, button_delta)
            .await?;

        let layout = Layout::new(
            Direction::Vertical,
            buttons,
            Layout_style::default(self.theme.clone()),
            Objective::default(),
            2,
        );

        let block = Title_block::new(display!(layout), "Config", self.theme.clone());

        Ok(Widget_type::Virtual(Box::new(block)))
    }

    async fn render(
        &mut self,
        focus: &mut Focus_provider,
        _hitbox: Rect,
        _display: &mut Display<'_>,
    ) -> Result<Option<Hitbox>> {
        focus.set_active(true);
        Ok(None)
    }
}

#[async_trait]
impl<T: Tree> Control for Tree_view<T> {
    async fn on_key_press(&mut self, key: &Key_event) -> Result<Vizual_msg> {
        match key.code {
            Key_code::Arrow_left => {
                let mut configurator_state = self.configurator_state.lock().await?;

                if configurator_state.cursor.len() > 1 {
                    let _ = configurator_state.cursor.pop();
                    return Vizual_msg::new(Vizual_command::Layout);
                }

                Vizual_msg::none()
            }
            Key_code::Arrow_right => {
                let cursor = self.configurator_state.lock().await?.cursor.clone();
                let tree = self.tree.lock().await?;
                let branch = match tree.get_tree().get_branch(&cursor) {
                    Ok(branch) => branch,
                    Err(_) => return Vizual_msg::none(),
                };
                let child_name = branch
                    .0
                    .get_index(0)
                    .map(|(child_name, _)| child_name.clone());
                drop(tree);

                if let Some(child_name) = child_name {
                    let mut configurator_state = self.configurator_state.lock().await?;

                    if configurator_state.cursor == cursor {
                        configurator_state.cursor.push(child_name);
                    }

                    return Vizual_msg::new(Vizual_command::Layout);
                }

                Vizual_msg::none()
            }
            Key_code::Arrow_down => {
                self.move_to_sibling(1).await?;
                Vizual_msg::new(Vizual_command::Layout)
            }
            Key_code::Arrow_up => {
                self.move_to_sibling(-1).await?;
                Vizual_msg::new(Vizual_command::Layout)
            }
            _ => Vizual_msg::none(),
        }
    }
}

pub struct Config_manager<T: Tree> {
    tree: Arc<Mutex<T>>,
    configuration_path: PathBuf,
    submit_handler: Box<dyn Submit_handler<bool>>,
}

struct Config_manager_handle<T: Tree> {
    manager: Arc<Mutex<Config_manager<T>>>,
}

impl<T: Tree> Clone for Config_manager_handle<T> {
    fn clone(&self) -> Self {
        Self {
            manager: Arc::clone(&self.manager),
        }
    }
}

struct Configurator_state {
    cursor: Vec<String>,
}

/// A widget editor for a [`Tree`].
pub struct Configurator<T: Tree> {
    tree: Arc<Mutex<T>>,
    configurator_state: Arc<Mutex<Configurator_state>>,
    config_manager: Config_manager_handle<T>,
    theme: State<Theme>,
    submit: Shared_widget<Popup>,
    submitting: bool,
    popup_slot: Component_slot,
}

#[async_trait]
impl<T: Tree> Control for Configurator<T> {
    async fn on_key_press(&mut self, key: &Key_event) -> Result<Vizual_msg> {
        if check_quit_event(key) {
            if !self.submitting {
                self.submitting = true;

                return Vizual_msg::new(Vizual_command::Focus(self.popup_slot.get_reference()));
            }

            return Vizual_msg::none();
        }

        Vizual_msg::none()
    }
}

impl<T: Tree> Config_manager<T> {
    async fn save(&mut self) -> Result<()> {
        let config = self.tree.lock().await?.create_config().await?;
        let string =
            serde_saphyr::to_string(&config).wrap_err("Failed to serialize configuration")?;
        fs::write(&self.configuration_path, string).wrap_err("Failed to save configuration")?;
        Ok(())
    }

    async fn complete(&mut self, should_save: bool) -> Result<Vizual_msg> {
        self.submit_handler.on_submit(Some(should_save)).await
    }
}

#[async_trait]
impl<T: Tree> Submit_handler<bool> for Config_manager_handle<T> {
    async fn on_submit(&mut self, should_save: Option<bool>) -> Result<Vizual_msg> {
        let should_save = should_save.ok_or_else(|| eyre!("No popup action selected"))?;
        let mut manager = self.manager.lock().await?;

        if should_save {
            manager.save().await?;
        }

        manager.complete(should_save).await
    }
}

#[async_trait]
impl<T: Tree> Submit_handler<String> for Config_manager_handle<T> {
    async fn on_submit(&mut self, _label: Option<String>) -> Result<Vizual_msg> {
        self.manager.lock().await?.save().await?;
        Vizual_msg::new(Vizual_command::Layout)
    }
}

/// Creates a configurator that optionally saves YAML to `configuration_path`.
pub fn configurator<T: Tree>(
    configuration_path: impl AsRef<Path>,
    tree: T,
    submit_handler: impl Submit_handler<bool>,
    theme: State<Theme>,
) -> Result<Configurator<T>> {
    let child_name = tree
        .get_tree()
        .0
        .get_index(0)
        .map(|(child_name, _)| child_name.to_string())
        .ok_or_else(|| eyre!("Expected atleast one leaf"))?;

    let tree = Arc::new(Mutex::new(tree));

    let config_manager = Config_manager_handle {
        manager: Arc::new(Mutex::new(Config_manager {
            tree: tree.clone(),
            configuration_path: configuration_path.as_ref().to_owned(),
            submit_handler: Box::new(submit_handler) as Box<dyn Submit_handler<bool>>,
        })),
    };
    let configurator_state = Arc::new(Mutex::new(Configurator_state {
        cursor: vec![child_name],
    }));

    Ok(Configurator {
        tree,
        configurator_state,
        config_manager: config_manager.clone(),
        theme: theme.clone(),
        submit: Popup::new(config_manager, theme).into_shared(),
        submitting: false,
        popup_slot: Component_slot::new(),
    })
}

#[async_trait]
impl<T: Tree> Widget_trait for Configurator<T> {
    async fn layout(
        &mut self,
        _focus: &mut Focus_provider,
        hitbox: Hitbox,
        _parent: Hitbox,
        problem: Component_context,
        _text_context: &mut vizual::text::Text_context,
        slots: &mut Slots,
    ) -> Result<Widget_type> {
        let tree_view = Tree_view {
            tree: self.tree.clone(),
            configurator_state: self.configurator_state.clone(),
            theme: self.theme.clone(),
        };
        let cursor = self.configurator_state.lock().await?.cursor.clone();

        //TODO: this menu could later be moved into the menu item of the tree to make it clearer
        let field: Option<Child> = {
            let tree = self.tree.lock().await?;

            if let Ok(leaf) = tree.get_tree().get_leaf(&cursor) {
                let description = Text::new(leaf.description)
                    .set_style(self.theme.load().semantic.text.paragraph());
                let linebreak = Linebreak::new(self.theme.clone());
                let widget = leaf.widget;
                let layout = Layout::new(
                    Direction::Vertical,
                    vec![
                        Some(display!(description)),
                        Some(display!(linebreak)),
                        Some(display!(widget)),
                    ],
                    Layout_style::default(self.theme.clone()),
                    Objective::default(),
                    2,
                );

                let leaf = Title_block::new(
                    display!(layout),
                    format!("Value - {}", leaf.name),
                    self.theme.clone(),
                );

                Some(display!(leaf))
            } else {
                None
            }
        };

        let Layout_style::Gap(gap) = Layout_style::default(self.theme.clone());
        let tree_view = display!(tree_view);

        let button = Button::new(
            "Apply",
            Box::new(self.config_manager.clone()),
            self.theme.clone(),
        );

        let tree_view = Anchor::new(
            tree_view,
            Anchors {
                horizontal: Some(Anchor_position::Start),
                vertical: Some(Anchor_position::End),
            },
        );
        let mut children = vec![display!(tree_view)];

        if let Some(field) = field {
            let field = Anchor::new(
                field,
                Anchors {
                    horizontal: None,
                    vertical: Some(Anchor_position::Start),
                },
            );
            let field = Align::new(
                display!(field),
                Alignments {
                    horizontal: Some(Objective::Minimize),
                    vertical: None,
                },
            );
            children.push(display!(field));
        }

        let button = Anchor::new(
            display!(button),
            Anchors {
                horizontal: Some(Anchor_position::End),
                vertical: Some(Anchor_position::End),
            },
        );

        children.push(display!(button));

        let grid = Grid::new(children, gap);

        if self.submitting {
            let popup = self
                .popup_slot
                .set(self.submit.clone(), problem.clone())
                .await?;

            let popup = Space::full(popup, Objective::default(), 2);
            return Widget_type::visual(vec![display!(grid), display!(popup)], hitbox, &problem)
                .await;
        }

        Ok(Widget_type::Virtual(Box::new(grid)))
    }
}
