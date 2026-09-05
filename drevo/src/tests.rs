use super::*;
use crate::widget::widgets::{
    default_root::DefaultRoot, layout::grid::Grid, paragraph::Paragraph,
    positioning::anchor::Anchor, scroll::Scroll, text::Text,
};
use crate::{
    geometry::{Direction, Rect},
    graphics::text::StyledText,
};

use crate::widget::{LayoutInput, RenderInput};

#[derive(Clone)]
struct OffsetClick;

#[derive(Clone)]
struct FocusableBox;

#[async_trait::async_trait]
impl WidgetTrait for FocusableBox {
    async fn layout(
        &mut self,
        LayoutInput {
            focus,
            hitbox,
            formula,
            ..
        }: LayoutInput<'_>,
    ) -> Result<component::Children> {
        focus.set_interactive(true);
        for direction in [Direction::Horizontal, Direction::Vertical] {
            hitbox
                .set_static_dimension(formula, direction, 20.0)
                .await?;
        }
        Ok(Vec::new())
    }

    async fn render(&mut self, RenderInput { .. }: RenderInput<'_, '_>) -> Result<()> {
        Ok(())
    }
}

#[async_trait::async_trait]
impl WidgetTrait for OffsetClick {
    async fn layout(
        &mut self,
        LayoutInput {
            focus,
            hitbox,
            formula,
            ..
        }: LayoutInput<'_>,
    ) -> Result<component::Children> {
        focus.set_interactive(true);
        hitbox
            .set_static_dimension(formula, crate::geometry::Direction::Horizontal, 100.0)
            .await?;
        hitbox
            .set_static_dimension(formula, crate::geometry::Direction::Vertical, 20.0)
            .await?;

        Ok(Vec::new())
    }

    async fn render(&mut self, RenderInput { .. }: RenderInput<'_, '_>) -> Result<()> {
        Ok(())
    }

    async fn on_mouse_click(&mut self, _input: crate::widget::MouseEvent<'_>) -> Result<VizualMsg> {
        VizualMsg::new(VizualCommand::Quit)
    }
}

#[tokio::test]
async fn default_root_solves_without_implicit_component_shrink_wrapping() -> Result<()> {
    let render_manager = RenderManager::new();
    let rerender = render_manager.rerender.clone();
    let theme = Store::new(theme::dark_theme());
    let body = Anchor::top_left(Text::new("Body"));
    let application = DefaultRoot::new("Test", Grid::new((body,), 0.0)).into_shared();
    let root = Root::new(application).into_shared();
    let mut root_slot = ComponentSlot::new();
    let variables = Arc::new(Variables::new());
    let mut text_context = TextContext::new();
    let focus = Focus::new();
    let mut problem = AppProblem::new(root, &mut root_slot, variables, rerender.clone()).await?;

    problem
        .layout(rerender, theme, &focus, &mut text_context)
        .await?;
    let _ = problem.solve(Size::new(800.0, 600.0)).await?;

    Ok(())
}

#[tokio::test]
async fn clicking_outside_the_focused_component_clears_focus() -> Result<()> {
    let mut render_manager = RenderManager::new();
    let rerender = render_manager.rerender.clone();
    let layout = render_manager.layout.clone();
    let theme = Store::new(theme::dark_theme());
    let root = Root::new(Anchor::top_left(FocusableBox)).into_shared();
    let mut root_slot = ComponentSlot::new();
    let variables = Arc::new(Variables::new());
    let mut text_context = TextContext::new();
    let focus = Focus::new();
    let mut problem = AppProblem::new(root, &mut root_slot, variables, rerender.clone()).await?;

    problem
        .layout(layout, theme.clone(), &focus, &mut text_context)
        .await?;
    let solution = problem.solve(Size::new(100.0, 100.0)).await?;
    let anchor = problem.root.lock().await?.children[0].clone();
    let focusable = anchor.lock().await?.children[0].clone();
    let _ = problem
        .render(rerender, theme, &focus, &solution, &mut text_context)
        .await?;

    let command = problem
        .handle_event(
            &Event::Pointer(PointerEvent {
                position: Point::new(10.0, 10.0),
                button: PointerButton::Primary,
            }),
            &solution,
            &focus,
        )
        .await?;
    assert!(matches!(command, VizualCommand::None));
    assert!(focus.compare(&focusable).await?);
    assert_eq!(
        render_manager.receiver.0.recv().await,
        Some(crate::RenderRequest::Layout)
    );

    let command = problem
        .handle_event(
            &Event::Pointer(PointerEvent {
                position: Point::new(80.0, 80.0),
                button: PointerButton::Primary,
            }),
            &solution,
            &focus,
        )
        .await?;
    assert!(matches!(command, VizualCommand::None));
    assert!(focus.upgrade().await?.is_none());
    assert_eq!(
        render_manager.receiver.0.recv().await,
        Some(crate::RenderRequest::Layout)
    );

    Ok(())
}

#[tokio::test]
async fn width_constrained_paragraph_derives_its_wrapped_height() -> Result<()> {
    let render_manager = RenderManager::new();
    let rerender = render_manager.rerender.clone();
    let theme = Store::new(theme::dark_theme());
    let content = "a paragraph which wraps over several lines";
    let width = 80.0;
    let mut paragraph = Paragraph::new(Direction::Horizontal, width);
    paragraph.set_styled_content(content, theme::dark_theme().specific.text.paragraph);
    let root = Root::new(Anchor::top_left(paragraph)).into_shared();
    let mut root_slot = ComponentSlot::new();
    let variables = Arc::new(Variables::new());
    let mut text_context = TextContext::new();
    let expected_height = f64::from(
        text_context
            .build_wrapped_layout(
                &StyledText::styled(content, theme::dark_theme().specific.text.paragraph),
                width as f32,
            )
            .await?
            .height(),
    );
    let focus = Focus::new();
    let mut problem = AppProblem::new(root, &mut root_slot, variables, rerender.clone()).await?;

    problem
        .layout(rerender, theme, &focus, &mut text_context)
        .await?;
    let solution = problem.solve(Size::new(300.0, 200.0)).await?;
    let anchor = problem.root.lock().await?.children[0].clone();
    let paragraph = anchor.lock().await?.children[0].clone();
    let paragraph = paragraph.get_hitbox().await?.get_resolved(&solution);

    assert!((paragraph.size.width - width).abs() < 1e-6);
    assert!((paragraph.size.height - expected_height).abs() < 1e-6);
    Ok(())
}

#[tokio::test]
async fn height_constrained_paragraph_derives_a_fitting_width() -> Result<()> {
    let render_manager = RenderManager::new();
    let rerender = render_manager.rerender.clone();
    let theme = Store::new(theme::dark_theme());
    let content = "one two three four five six seven eight nine ten eleven twelve";
    let height = 60.0;
    let mut paragraph = Paragraph::new(Direction::Vertical, height);
    paragraph.set_styled_content(content, theme::dark_theme().specific.text.paragraph);
    let root = Root::new(Anchor::top_left(paragraph)).into_shared();
    let mut root_slot = ComponentSlot::new();
    let variables = Arc::new(Variables::new());
    let mut text_context = TextContext::new();
    let natural_width = f64::from(
        text_context
            .build_layout(&StyledText::styled(
                content,
                theme::dark_theme().specific.text.paragraph,
            ))
            .await?
            .full_width(),
    );
    let focus = Focus::new();
    let mut problem = AppProblem::new(root, &mut root_slot, variables, rerender.clone()).await?;

    problem
        .layout(rerender, theme, &focus, &mut text_context)
        .await?;
    let solution = problem.solve(Size::new(600.0, 200.0)).await?;
    let anchor = problem.root.lock().await?.children[0].clone();
    let paragraph = anchor.lock().await?.children[0].clone();
    let paragraph = paragraph.get_hitbox().await?.get_resolved(&solution);
    let wrapped_height = f64::from(
        text_context
            .build_wrapped_layout(
                &StyledText::styled(content, theme::dark_theme().specific.text.paragraph),
                paragraph.size.width as f32,
            )
            .await?
            .height(),
    );

    assert!((paragraph.size.height - height).abs() < 1e-6);
    assert!(paragraph.size.width < natural_width);
    assert!(wrapped_height <= height);
    Ok(())
}

#[tokio::test]
async fn scroll_lays_out_content_with_offset() -> Result<()> {
    let mut render_manager = RenderManager::new();
    let rerender = render_manager.rerender.clone();
    let theme = Store::new(theme::dark_theme());
    let root = Root::new(Scroll::new(Text::new("Scrollable content ".repeat(20)))).into_shared();
    let mut root_slot = ComponentSlot::new();
    let variables = Arc::new(Variables::new());
    let mut text_context = TextContext::new();
    let focus = Focus::new();
    let mut problem = AppProblem::new(root, &mut root_slot, variables, rerender.clone()).await?;

    problem
        .layout(rerender.clone(), theme.clone(), &focus, &mut text_context)
        .await?;
    let solution = problem.solve(Size::new(100.0, 100.0)).await?;
    let scroll = problem.root.lock().await?.children[0].clone();
    let (scroll_content, content) = widget::widgets::scroll::find_scroll_content_and_child(&scroll)
        .await?
        .unwrap();
    let scroll_rect = scroll.get_hitbox().await?.get_resolved(&solution);
    let scroll_content_rect = scroll_content.get_hitbox().await?.get_resolved(&solution);
    let content_rect = content.get_hitbox().await?.get_resolved(&solution);

    assert_eq!(scroll_rect, Rect::new(0.0, 0.0, 100.0, 100.0));
    assert_eq!(content_rect.origin, scroll_content_rect.origin);
    assert!(content_rect.size.width > 100.0);

    let _scene = problem
        .render(
            rerender.clone(),
            theme.clone(),
            &focus,
            &solution,
            &mut text_context,
        )
        .await?;
    assert!(scroll.lock().await?.focusable);
    focus.set(&scroll).await?;
    let command = problem
        .handle_event(
            &Event::Key(KeyEvent {
                code: KeyCode::ArrowRight,
                modifiers: Modifiers::default(),
                text: None,
                repeat: false,
            }),
            &solution,
            &focus,
        )
        .await?;
    assert!(matches!(command, VizualCommand::None));
    assert_eq!(
        render_manager.receiver.0.recv().await,
        Some(crate::RenderRequest::Rerender)
    );

    Ok(())
}

#[tokio::test]
async fn scroll_routes_pointer_events_in_transformed_frame_coordinates() -> Result<()> {
    let mut render_manager = RenderManager::new();
    let rerender = render_manager.rerender.clone();
    let theme = Store::new(theme::dark_theme());
    let root = Root::new(Scroll::new(OffsetClick)).into_shared();
    let mut root_slot = ComponentSlot::new();
    let variables = Arc::new(Variables::new());
    let mut text_context = TextContext::new();
    let focus = Focus::new();
    let mut problem = AppProblem::new(root, &mut root_slot, variables, rerender.clone()).await?;

    problem
        .layout(rerender.clone(), theme.clone(), &focus, &mut text_context)
        .await?;
    let solution = problem.solve(Size::new(80.0, 80.0)).await?;
    let scroll = problem.root.lock().await?.children[0].clone();
    focus.set(&scroll).await?;

    let _scene = problem
        .render(
            rerender.clone(),
            theme.clone(),
            &focus,
            &solution,
            &mut text_context,
        )
        .await?;

    let command = problem
        .handle_event(
            &Event::Key(KeyEvent {
                code: KeyCode::ArrowRight,
                modifiers: Modifiers::default(),
                text: None,
                repeat: false,
            }),
            &solution,
            &focus,
        )
        .await?;
    assert!(matches!(command, VizualCommand::None));
    assert_eq!(
        render_manager.receiver.0.recv().await,
        Some(crate::RenderRequest::Rerender)
    );

    problem
        .layout(rerender.clone(), theme.clone(), &focus, &mut text_context)
        .await?;
    let solution = problem.solve(Size::new(80.0, 80.0)).await?;

    let _scene = problem
        .render(rerender, theme, &focus, &solution, &mut text_context)
        .await?;
    let command = problem
        .handle_event(
            &Event::Pointer(PointerEvent {
                position: Point::new(20.0, 20.0),
                button: PointerButton::Primary,
            }),
            &solution,
            &focus,
        )
        .await?;

    assert!(matches!(command, VizualCommand::Quit));
    Ok(())
}
