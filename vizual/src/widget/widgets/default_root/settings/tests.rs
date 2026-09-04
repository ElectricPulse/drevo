use std::sync::Arc;

use super::*;
use crate::{
    AppProblem, KeyEvent, VizualCommand,
    event::{KeyCode, Modifiers, PointerButton, PointerEvent},
    focus::Focus,
    geometry::{Point, Size},
    graphics::text::TextContext,
    layouter::variables::Variables,
    render_manager::RenderManager,
    slot::ComponentSlot,
    theme::dark_theme,
    widget::widgets::root::Root,
};

#[test]
fn system_label_includes_the_resolved_system_theme() {
    assert_eq!(
        label(ThemeChoice::System, SystemTheme::Dark),
        "System (Dark)"
    );
    assert_eq!(
        label(ThemeChoice::System, SystemTheme::Light),
        "System (Light)"
    );
}

#[tokio::test]
async fn settings_button_focus_can_navigate_the_menu_with_arrow_keys() -> Result<()> {
    let manager = RenderManager::new();
    let choice = Store::new(ThemeChoice::System);
    let mut settings = Settings::new(choice.clone());

    let message = settings
        .on_key_press(crate::widget::KeyPress {
            key: &KeyEvent {
                code: KeyCode::ArrowDown,
                modifiers: Modifiers::default(),
                text: None,
                repeat: false,
            },
            relayout: manager.rerender.clone(),
            window: None,
        })
        .await?;

    assert!(!message.has_command());
    assert_eq!(*choice.read().await?, ThemeChoice::Dark);
    Ok(())
}

#[tokio::test]
async fn menu_is_laid_out_only_while_settings_parent_is_focused() -> Result<()> {
    let mut render_manager = RenderManager::new();
    let rerender = render_manager.rerender;
    let theme = Store::new(dark_theme());
    let settings = Settings::new(Store::new(ThemeChoice::Dark));
    let root = WidgetTrait::into_shared(Root::new(settings));
    let mut root_slot = ComponentSlot::new();
    let mut text_context = TextContext::new();
    let mut focus = Focus::new();
    let variables = Arc::new(Variables::new());

    let mut problem = AppProblem::new(
        root.clone(),
        &mut root_slot,
        Arc::clone(&variables),
        rerender.clone(),
    )
    .await?;
    problem
        .layout(rerender.clone(), theme.clone(), &focus, &mut text_context)
        .await?;
    let settings = problem.root.lock().await?.children[0].clone();
    assert!(settings.lock().await?.focusable);
    assert_eq!(settings.lock().await?.children.len(), 1);

    let solution = problem.solve(Size::new(800.0, 600.0)).await?;
    let _ = problem
        .render(
            rerender.clone(),
            theme.clone(),
            &focus,
            &solution,
            &mut text_context,
        )
        .await?;
    let button_anchor = settings.lock().await?.children[0].clone();
    let button = button_anchor.lock().await?.children[0].clone();
    let button_block = button.lock().await?.children[0].clone();
    let button_rect = button_block.get_hitbox().await?.get_resolved(&solution);
    let command = problem
        .handle_event(
            &crate::event::Event::Pointer(PointerEvent {
                position: Point::new(
                    button_rect.origin.x + button_rect.size.width / 2.0,
                    button_rect.origin.y + button_rect.size.height / 2.0,
                ),
                button: PointerButton::Primary,
            }),
            &solution,
            &mut focus,
        )
        .await?;
    assert!(matches!(command, VizualCommand::None));
    assert!(focus.compare(&button_block));
    assert!(focus.focused_path().await?.contains(&settings));

    assert_eq!(
        render_manager.receiver.0.recv().await,
        Some(crate::RenderRequest::Rerender)
    );

    let previous_problem = problem;
    let mut problem = AppProblem::new(
        root.clone(),
        &mut root_slot,
        Arc::clone(&variables),
        rerender.clone(),
    )
    .await?;
    drop(previous_problem);
    problem
        .layout(rerender.clone(), theme.clone(), &focus, &mut text_context)
        .await?;
    assert_eq!(settings.lock().await?.children.len(), 1);
    assert_eq!(problem.root.lock().await?.children.len(), 2);
    let solution = problem.solve(Size::new(800.0, 600.0)).await?;
    let _ = problem
        .render(
            rerender.clone(),
            theme.clone(),
            &focus,
            &solution,
            &mut text_context,
        )
        .await?;
    assert!(settings.lock().await?.focusable);

    focus.reset();
    let previous_problem = problem;
    let mut problem = AppProblem::new(root, &mut root_slot, variables, rerender.clone()).await?;
    drop(previous_problem);
    problem
        .layout(rerender, theme, &focus, &mut text_context)
        .await?;
    assert_eq!(settings.lock().await?.children.len(), 1);

    Ok(())
}
