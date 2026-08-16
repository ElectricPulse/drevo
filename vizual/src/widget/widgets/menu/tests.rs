use super::*;

#[derive(Clone, Copy)]
struct Ordinary_menu_item(usize);

#[async_trait]
impl Widget_trait for Ordinary_menu_item {
    async fn layout(
        &mut self,
        _render: crate::Render,
        _theme: Store<Theme>,
        _focus: &mut Focus_provider,
        _hitbox: &mut Hitbox,
        _parent: Hitbox,
        _problem: Component_context,
        _text_context: &mut crate::graphics::text::Text_context,
        _slots: &mut Slots,
        _logical: &mut bool,
    ) -> Result<Children> {
        Ok(vec![])
    }
}

#[async_trait]
impl Retrieve_handler<usize> for Ordinary_menu_item {
    async fn on_retrieve(&mut self) -> Result<usize> {
        Ok(self.0)
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
    let menu = Menu::new(vec![first, second], 0).await?;
    assert_eq!(*menu.submitted.read().await?, 0);
    Ok(())
}

#[tokio::test]
async fn menu_item_container_submits_on_enter() -> Result<()> {
    let selected_store = Store::new(0);
    let submitted = Store::new(0);
    let variables = crate::layouter::variables::Variables::new();
    let button_delta = variables.make(good_lp::variable(), "delta", "delta", "delta");
    let mut container = Menu_item_container {
        index: 1,
        selected: false,
        widget: Box::new(Ordinary_menu_item(42)),
        selected_store: selected_store.clone(),
        submitted: submitted.clone(),
        button_delta,
    };

    let message = container
        .on_key_press(&Key_event {
            code: Key_code::Enter,
            modifiers: crate::event::Modifiers::default(),
            text: None,
            repeat: false,
        })
        .await?;

    assert!(message.has_command());
    assert_eq!(*selected_store.read().await?, 1);
    assert_eq!(*submitted.read().await?, 42);
    Ok(())
}
