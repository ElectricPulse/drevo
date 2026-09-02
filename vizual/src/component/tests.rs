use super::*;
use crate::{
    focus::Focus,
    layouter::{Formula, variables::Variables},
    widget::Widget_trait,
};

#[derive(Clone)]
struct Empty_widget;

#[derive(Clone, crate::macros::Widget_trait)]
struct Derived_widget {
    widget: Empty_widget,
}

#[async_trait::async_trait]
impl Widget_trait for Empty_widget {}

#[test]
fn widget_derive_forwards_the_current_trait_interface() {
    fn assert_widget<Widget: Widget_trait>() {}

    assert_widget::<Derived_widget>();
    let _ = Derived_widget {
        widget: Empty_widget,
    };
}

fn component(name: &str, variables: &Variables, problem: Component_context) -> Shared_component {
    Shared_component::new(Arc::new(Mutex::new(Component {
        id: 0,
        name: name.to_string(),
        debug: Component_debug::new("test".to_string()),
        hitbox: Hitbox::new(
            variables,
            name.to_string(),
            name.to_string(),
            "test".to_string(),
        ),
        formula: None,
        variables: Arc::new(Variables::new()),
        layout_signal: None,
        widget: Box::new(Empty_widget),
        focusable: false,
        parent: None,
        children: Vec::new(),
        layout_children: Vec::new(),
        slot_manager: Slot_records::new(problem),
        logical: false,
        mask: false,
    })))
}

#[tokio::test]
async fn focused_path_contains_the_target_and_its_parents() -> Result<()> {
    let variables = Arc::new(Variables::new());
    let formula = Arc::new(Mutex::new(Formula::new(Arc::clone(&variables))));
    let context = Component_context::new(formula);
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
