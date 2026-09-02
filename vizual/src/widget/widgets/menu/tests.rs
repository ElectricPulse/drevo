use super::*;
use crate::event::Key_event;

#[derive(Clone, Copy)]
struct Ordinary_menu_item(usize);

#[async_trait]
impl Widget_trait for Ordinary_menu_item {}

#[async_trait]
impl Retrieve_handler<usize> for Ordinary_menu_item {
    async fn on_retrieve(&mut self) -> Result<State<usize>> {
        Ok(self.0.into())
    }
}

#[test]
fn ordinary_widgets_automatically_satisfy_menu_item_trait() {
    fn assert_menu_item<T: Menu_item_trait<usize>>() {}

    assert_menu_item::<Ordinary_menu_item>();
}

#[tokio::test]
async fn menu_initializes_and_submits() -> Result<()> {
    let first: Menu_item<usize> = Box::new(Ordinary_menu_item(0));
    let second: Menu_item<usize> = Box::new(Ordinary_menu_item(1));
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
    let mut container = Menu_item_container {
        index: 1,
        selected: false,
        widget: Box::new(Ordinary_menu_item(42)),
        selected_store: selected_store.clone(),
        submitted: submitted.clone(),
        button_delta,
        item_block: true,
    };

    let manager = crate::render_manager::Render_manager::new();
    let message = container
        .on_key_press(crate::widget::Key_press {
            key: &Key_event {
                code: Key_code::Enter,
                modifiers: crate::event::Modifiers::default(),
                text: None,
                repeat: false,
            },
            relayout: manager.rerender.for_component(0),
        })
        .await?;

    assert!(!message.has_command());
    assert_eq!(*selected_store.read().await?, 1);
    assert_eq!(*submitted.read().await?, 42);
    Ok(())
}
