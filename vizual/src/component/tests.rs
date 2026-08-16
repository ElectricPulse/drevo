use super::*;
use crate::{
    focus::Focus,
    layouter::{Problem, variables::Variables},
    widget::Widget_trait,
};

#[derive(Clone)]
struct Empty_widget;

#[derive(Clone, vizual_macros::Widget_trait)]
struct Derived_widget {
    widget: Empty_widget,
}

#[async_trait::async_trait]
impl Widget_trait for Empty_widget {
    async fn layout(
        &mut self,
        _render: crate::Render,
        _theme: crate::state::Store<crate::theme::Theme>,
        _focus: &mut crate::widget::Focus_provider,
        _hitbox: &mut Hitbox,
        _parent: Hitbox,
        _problem: Component_context,
        _text_context: &mut crate::graphics::text::Text_context,
        _slots: &mut crate::slot::manager::Slots,
        _root: &crate::component::Shared_component,
    ) -> Result<Children> {
        Ok(vec![])
    }
}

#[test]
fn widget_derive_forwards_the_current_trait_interface() {
    fn assert_widget<Widget: Widget_trait>() {}

    assert_widget::<Derived_widget>();
    let _ = Derived_widget {
        widget: Empty_widget,
    };
}

fn component(
    name: &str,
    variables: &Variables,
    problem: Component_context,
) -> Shared_component {
    Shared_component::new(Arc::new(Mutex::new(Component {
        name: name.to_string(),
        debug: Component_debug::new("test".to_string()),
        hitbox: Hitbox::new(
            variables,
            name.to_string(),
            name.to_string(),
            "test".to_string(),
        ),
        widget: Box::new(Empty_widget),
        focusable: false,
        parent: None,
        children: Vec::new(),
        slot_manager: Slot_records::new(problem),
        logical: false,
    })))
}

#[tokio::test]
async fn focused_path_contains_the_target_and_its_parents() -> Result<()> {
    let variables = Arc::new(Variables::new());
    let problem = Arc::new(Mutex::new(Problem::new(Arc::clone(&variables))));
    let context = Component_context::new(problem);
    let parent = component("parent", &variables, context.clone());
    let child = component("child", &variables, context.clone());
    let unrelated = component("unrelated", &variables, context);
    child.lock().await?.parent = Some(parent.as_reference());
    parent.lock().await?.children = vec![child.clone()];

    let mut focus = Focus::new();
    focus.set(&child);
    let focused_path = focus.focused_path().await?;

    assert!(focused_path.contains(&child));
    assert!(focused_path.contains(&parent));
    assert!(!focused_path.contains(&unrelated));
    Ok(())
}
