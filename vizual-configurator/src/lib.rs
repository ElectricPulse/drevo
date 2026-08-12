#![warn(rustdoc::broken_intra_doc_links)]
//! Tree-based configuration editing for Vizual applications.
//!
//! Implement [`Tree`] to describe editable fields and produce a serializable
//! configuration.

pub mod widgets;

use async_recursion::async_recursion;
use async_trait::async_trait;
use color_eyre::eyre::{Result, WrapErr, eyre};
use derive_where::derive_where;
use indexmap::IndexMap;
use serde::Serialize;
use std::{
    fs,
    path::{Path, PathBuf},
    sync::Arc,
};
use vizual::{
    Vizual_command, Vizual_msg, check_quit_event,
    component::{Children, context::Component_context},
    display::Display,
    event::{Event, Key_code, Key_event, Pointer_event},
    geometry::{Direction, Rect},
    handlers::{Retrieve_handler, Submit_handler},
    layouter::{hitbox::Hitbox, objective::Objective},
    slot::{Component_slot, manager::Slots},
    state::State,
    sync::{Mutex, Thread_safe},
    theme::Theme,
    widget::{
        Focus_provider, Shared_widget, Widget, Widget_trait,
        widgets::{
            button::Button,
            layout::{axis::Axis, grid::Grid},
            linebreak::Linebreak,
            popup::Popup,
            positioning::{
                anchor::{Anchor, Anchors, Position as Anchor_position},
                space::Space,
            },
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

dyn_clone::clone_trait_object!(<Value> Field<Value>);

#[async_trait]
impl<Value: 'static> Widget_trait for Box<dyn Field<Value>> {
    async fn layout(
        &mut self,
        render: vizual::Render,
        theme: State<Theme>,
        focus: &mut Focus_provider,
        hitbox: &mut Hitbox,
        parent: Hitbox,
        problem: Component_context,
        text_context: &mut vizual::text::Text_context,
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

    async fn on_all_events(&mut self, event: &Event) -> Result<Vizual_msg> {
        (**self).on_all_events(event).await
    }

    async fn on_mouse_click(&mut self, mouse: &Pointer_event) -> Result<Vizual_msg> {
        (**self).on_mouse_click(mouse).await
    }

    async fn on_key_press(&mut self, key: &Key_event) -> Result<Vizual_msg> {
        (**self).on_key_press(key).await
    }

    async fn on_other_event(&mut self, event: &Event) -> Result<Vizual_msg> {
        (**self).on_other_event(event).await
    }

    async fn forward_event(&mut self, event: &Event) -> Result<Vizual_msg> {
        (**self).forward_event(event).await
    }
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

#[derive_where(Clone)]
struct Tree_view<T: Tree> {
    tree: Arc<Mutex<T>>,
    configurator_state: Arc<Mutex<Configurator_state>>,
}

#[derive(Clone)]
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
        node: &Configuration_tree_branch,
        selected_cursor: &[String],
        cursor: &[String],
        theme: State<Theme>,
        problem: &Component_context,
        button_delta: vizual::layouter::variable::Variable,
    ) -> Result<Vec<Widget>> {
        const INDENT: usize = 50;

        let mut buttons: Vec<Widget> = vec![];

        for (name, child) in &node.0 {
            let mut child_cursor = cursor.to_vec();
            child_cursor.push(name.clone());
            let depth = cursor.len();

            let mut text = Text::new(name);
            text.style.set(match selected_cursor == child_cursor {
                true => theme.load().specific.text.selected_subtitle,
                false => theme.load().specific.text.subtitle,
            });

            let mut button = Button::new(
                text,
                Field_click_handler {
                    configurator_state: self.configurator_state.clone(),
                    cursor: child_cursor.clone(),
                },
            );

            button.delta = Some(button_delta.clone());

            let button = Space::left(button, (INDENT * depth) as f64, Objective::default(), 2);
            let button = Anchor::new(button, Anchors::top_left());

            buttons.push(Box::new(button));

            if let Configuration_tree::Branch(branch) = child {
                let mut child_tree = self
                    .render_tree(
                        branch,
                        selected_cursor,
                        &child_cursor,
                        theme.clone(),
                        problem,
                        button_delta.clone(),
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
        _render: vizual::Render,
        theme: State<Theme>,
        focus: &mut Focus_provider,
        _hitbox: &mut Hitbox,
        _parent: Hitbox,
        problem: Component_context,
        _text_context: &mut vizual::text::Text_context,
        slots: &mut Slots,
    ) -> Result<Children> {
        focus.set_active(true);
        let cursor = self.configurator_state.lock().await?.cursor.clone();
        let button_delta = problem
            .add_delta("configurator-tree-button-delta", 2)
            .await?;

        let tree = self.tree.lock().await?.get_tree();
        let buttons = self
            .render_tree(&tree, &cursor, &[], theme, &problem, button_delta)
            .await?;

        let axis = Axis::new(Direction::Vertical, buttons);

        let block = Title_block::new(axis, "Config");
        Ok(vec![display!(block)])
    }

    async fn render(
        &mut self,
        _theme: State<Theme>,
        focus: &mut Focus_provider,
        _hitbox: Rect,
        _display: &mut Display<'_>,
    ) -> Result<Option<Hitbox>> {
        focus.set_active(true);
        Ok(None)
    }

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

#[derive_where(Clone)]
pub struct Config_manager<T: Tree> {
    tree: Arc<Mutex<T>>,
    configuration_path: PathBuf,
    submit_handler: Box<dyn Submit_handler<bool>>,
}

#[derive_where(Clone)]
struct Config_manager_handle<T: Tree> {
    manager: Arc<Mutex<Config_manager<T>>>,
}

struct Configurator_state {
    cursor: Vec<String>,
}

/// A widget editor for a [`Tree`].
#[derive_where(Clone)]
pub struct Configurator<T: Tree> {
    tree: Arc<Mutex<T>>,
    configurator_state: Arc<Mutex<Configurator_state>>,
    config_manager: Config_manager_handle<T>,
    submit: Shared_widget<Popup>,
    submitting: bool,
    popup_slot: Component_slot,
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
    render: vizual::Render,
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
        submit: Popup::new(config_manager, render).into_shared(),
        submitting: false,
        popup_slot: Component_slot::new(),
    })
}

#[async_trait]
impl<T: Tree> Widget_trait for Configurator<T> {
    async fn layout(
        &mut self,
        _render: vizual::Render,
        theme: State<Theme>,
        _focus: &mut Focus_provider,
        hitbox: &mut Hitbox,
        _parent: Hitbox,
        problem: Component_context,
        _text_context: &mut vizual::text::Text_context,
        slots: &mut Slots,
    ) -> Result<Children> {
        let tree_view = Tree_view {
            tree: self.tree.clone(),
            configurator_state: self.configurator_state.clone(),
        };
        let cursor = self.configurator_state.lock().await?.cursor.clone();

        //TODO: this menu could later be moved into the menu item of the tree to make it clearer
        let field: Option<Widget> = {
            let tree = self.tree.lock().await?;

            if let Ok(leaf) = tree.get_tree().get_leaf(&cursor) {
                let description = Text::new(leaf.description);
                let description = Anchor::new(description, Anchors::top_left());

                let axis = Axis::new(
                    Direction::Vertical,
                    vec![
                        Box::new(description),
                        Box::new(Linebreak::new()),
                        leaf.widget,
                    ],
                );

                let leaf = Title_block::new(axis, leaf.name);

                Some(Box::new(leaf))
            } else {
                None
            }
        };

        let gap = theme.load().semantic.axis.gap;
        let tree_view = Anchor::new(
            tree_view,
            Anchors {
                horizontal: Some(Anchor_position::Start),
                vertical: Some(Anchor_position::Start),
            },
        );
        let mut children: Vec<Widget> = vec![Box::new(tree_view)];

        if let Some(field) = field {
            let field = Anchor::new(
                field,
                Anchors {
                    horizontal: Some(Anchor_position::End),
                    vertical: Some(Anchor_position::Start),
                },
            );

            children.push(Box::new(field));
        }

        let mut text = Text::new("Apply");
        text.style.set(theme.load().specific.text.selected_subtitle);
        let button = Button::new(text, self.config_manager.clone());
        let button = Anchor::new(
            button,
            Anchors {
                horizontal: Some(Anchor_position::End),
                vertical: Some(Anchor_position::End),
            },
        );

        children.push(Box::new(button));

        let grid = Grid::new(children, gap);

        if self.submitting {
            let popup = self
                .popup_slot
                .set_child(self.submit.clone(), problem.clone(), hitbox)
                .await?;

            let mut popup = popup;
            popup.layer = 1;
            return Ok(vec![display!(grid), popup]);
        }

        Ok(vec![display!(grid)])
    }

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
