use super::*;
use crate::event::KeyEvent;

#[derive(Clone, Copy)]
struct OrdinaryMenuItem(usize);

#[async_trait]
impl WidgetTrait for OrdinaryMenuItem {}

#[async_trait]
impl RetrieveHandler<usize> for OrdinaryMenuItem {
    async fn on_retrieve(&mut self) -> Result<State<usize>> {
        Ok(self.0.into())
    }
}

#[test]
fn ordinary_widgets_automatically_satisfy_menu_item_trait() {
    fn assert_menu_item<T: MenuItemTrait<usize>>() {}

    assert_menu_item::<OrdinaryMenuItem>();
}

#[tokio::test]
async fn menu_initializes_and_submits() -> Result<()> {
    let first: MenuItem<usize> = Box::new(OrdinaryMenuItem(0));
    let second: MenuItem<usize> = Box::new(OrdinaryMenuItem(1));
    let mut menu = Menu::new(vec![first, second], 0).await?;
    assert_eq!(*menu.on_retrieve().await?.read().await?, 0);
    Ok(())
}

#[tokio::test]
async fn menu_item_container_submits_on_enter() -> Result<()> {
    let selected_store = Store::new(0);
    let submitted = Store::new(0);
    let variables = crate::layouter::variables::Variables::new();
    let button_delta = variables.make("delta", "delta", "delta");
    let mut container = MenuItemContainer {
        index: 1,
        selected: false,
        widget: Box::new(OrdinaryMenuItem(42)),
        selected_store: selected_store.clone(),
        submitted: submitted.clone(),
        button_delta,
        item_block: true,
    };

    let manager = crate::render_manager::RenderManager::new();
    let message = container
        .on_key_press(crate::widget::KeyPress {
            key: &KeyEvent {
                code: KeyCode::Enter,
                modifiers: crate::event::Modifiers::default(),
                text: None,
                repeat: false,
            },
            relayout: manager.rerender.for_component(0),
            window: None,
        })
        .await?;

    assert!(!message.has_command());
    assert_eq!(*selected_store.read().await?, 1);
    assert_eq!(*submitted.read().await?, 42);
    Ok(())
}
