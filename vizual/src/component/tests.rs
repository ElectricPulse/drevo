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
        _logical: &mut bool,
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
async fn child_layers_are_inherited_by_their_subtrees() -> Result<()> {
    let variables = Arc::new(Variables::new());
    let problem = Arc::new(Mutex::new(Problem::new(Arc::clone(&variables))));
    let context = Component_context::new(problem);

    let root = component("root", &variables, context.clone());
    let mut layer_two = component("layer-two", &variables, context.clone());
    layer_two.layer = 2;
    let layer_two_child = component("layer-two-child", &variables, context.clone());
    layer_two.lock().await?.children = vec![layer_two_child];

    let mut layer_one = component("layer-one", &variables, context);
    layer_one.layer = 1;
    root.lock().await?.children = vec![layer_two, layer_one];

    let components = root.layered_components().await?;
    assert_eq!(
        components
            .iter()
            .map(|component| component.layer)
            .collect::<Vec<_>>(),
        vec![0, 2, 2, 1]
    );

    let mut paint_order = components;
    paint_order.sort_by_key(|component| (component.layer, component.tree_order));
    let mut names = Vec::new();
    for component in paint_order {
        names.push(component.component.lock().await?.name.clone());
    }
    assert_eq!(
        names,
        vec!["root", "layer-one", "layer-two", "layer-two-child"]
    );

    Ok(())
}

#[tokio::test]
async fn logical_components_stop_component_traversal_at_their_children() -> Result<()> {
    let variables = Arc::new(Variables::new());
    let problem = Arc::new(Mutex::new(Problem::new(Arc::clone(&variables))));
    let context = Component_context::new(problem);

    let root = component("root", &variables, context.clone());
    let logical_child = component("logical", &variables, context.clone());
    logical_child.lock().await?.logical = true;
    let grandchild = component("grandchild", &variables, context);
    logical_child.lock().await?.children = vec![grandchild.clone()];
    root.lock().await?.children = vec![logical_child.clone()];

    let components = root.layered_components().await?;
    assert_eq!(components.len(), 2);
    assert!(components[1].component.compare(&logical_child));

    logical_child.lock().await?.logical = false;
    let components = root.layered_components().await?;
    assert_eq!(components.len(), 3);
    assert!(components[2].component.compare(&grandchild));

    Ok(())
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
