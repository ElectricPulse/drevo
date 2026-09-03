use super::*;
use crate::{
    focus::Focus,
    layouter::{Formula, variables::Variables},
    widget::WidgetTrait,
};

#[derive(Clone)]
struct EmptyWidget;

#[derive(Clone, crate::macros::WidgetTrait)]
struct DerivedWidget {
    widget: EmptyWidget,
}

#[async_trait::async_trait]
impl WidgetTrait for EmptyWidget {}

#[test]
fn widget_derive_forwards_the_current_trait_interface() {
    fn assert_widget<Widget: WidgetTrait>() {}

    assert_widget::<DerivedWidget>();
    let _ = DerivedWidget {
        widget: EmptyWidget,
    };
}

fn component(name: &str, variables: &Variables, problem: ComponentContext) -> SharedComponent {
    SharedComponent::new(Arc::new(Mutex::new(Component {
        id: 0,
        name: name.to_string(),
        debug: ComponentDebug::new("test".to_string()),
        hitbox: Hitbox::new(
            variables,
            name.to_string(),
            name.to_string(),
            "test".to_string(),
        ),
        formula: None,
        variables: Arc::new(Variables::new()),
        widget: Box::new(EmptyWidget),
        focusable: false,
        parent: None,
        children: Vec::new(),
        layout_children: Vec::new(),
        slot_manager: SlotRecords::new(problem),
        logical: false,
        mask: false,
    })))
}

#[tokio::test]
async fn focused_path_contains_the_target_and_its_parents() -> Result<()> {
    let variables = Arc::new(Variables::new());
    let formula = Arc::new(Mutex::new(Formula::new(Arc::clone(&variables))));
    let context = ComponentContext::new(formula);
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
