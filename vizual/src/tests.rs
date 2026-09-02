use super::*;
use crate::widget::widgets::{
    default_root::Default_root, layout::grid::Grid, paragraph::Paragraph,
    positioning::anchor::Anchor, scroll::Scroll, text::Text,
};
use crate::{
    geometry::{Direction, Rect},
    graphics::text::Styled_text,
};

use crate::widget::{Layout_input, Render_input};

#[derive(Clone)]
struct Offset_click;

#[derive(Clone)]
struct Focusable_box;

#[async_trait::async_trait]
impl Widget_trait for Focusable_box {
    async fn layout(
        &mut self,
        Layout_input {
            focus,
            hitbox,
            problem,
            ..
        }: Layout_input<'_>,
    ) -> Result<component::Children> {
        focus.set_interactive(true);
        for direction in [Direction::Horizontal, Direction::Vertical] {
            hitbox
                .set_static_dimension(&problem, direction, 20.0)
                .await?;
        }
        Ok(Vec::new())
    }

    async fn render(&mut self, Render_input { focus, .. }: Render_input<'_, '_>) -> Result<()> {
        focus.set_interactive(true);
        Ok(())
    }
}

#[async_trait::async_trait]
impl Widget_trait for Offset_click {
    async fn layout(
        &mut self,
        Layout_input {
            focus,
            hitbox,
            problem,
            ..
        }: Layout_input<'_>,
    ) -> Result<component::Children> {
        focus.set_interactive(true);
        hitbox
            .set_static_dimension(&problem, crate::geometry::Direction::Horizontal, 100.0)
            .await?;
        hitbox
            .set_static_dimension(&problem, crate::geometry::Direction::Vertical, 20.0)
            .await?;

        Ok(Vec::new())
    }

    async fn render(&mut self, Render_input { focus, .. }: Render_input<'_, '_>) -> Result<()> {
        focus.set_interactive(true);
        Ok(())
    }

    async fn on_mouse_click(&mut self, _pointer: &Pointer_event) -> Result<Vizual_msg> {
        Vizual_msg::new(Vizual_command::Quit)
    }
}

#[tokio::test]
async fn default_root_solves_without_implicit_component_shrink_wrapping() -> Result<()> {
    let render_manager = Render_manager::new();
    let render = render_manager.render.clone();
    let theme = Store::new(theme::dark_theme());
    let body = Anchor::top_left(Text::new("Body"));
    let application = Default_root::new("Test", Grid::new((body,), 0.0)).into_shared();
    let root = Root::new(application).into_shared();
    let mut root_slot = Component_slot::new();
    let variables = Arc::new(Variables::new());
    let mut text_context = Text_context::new();
    let focus = Focus::new();
    let mut problem = App_problem::new(root, &mut root_slot, variables).await?;

    problem
        .layout(render, theme, &focus, &mut text_context)
        .await?;
    let _ = problem.solve(Size::new(800.0, 600.0)).await?;

    Ok(())
}

#[tokio::test]
async fn clicking_outside_the_focused_component_clears_focus() -> Result<()> {
    let render_manager = Render_manager::new();
    let render = render_manager.render;
    let theme = Store::new(theme::dark_theme());
    let root = Root::new(Anchor::top_left(Focusable_box)).into_shared();
    let mut root_slot = Component_slot::new();
    let variables = Arc::new(Variables::new());
    let mut text_context = Text_context::new();
    let mut focus = Focus::new();
    let mut problem = App_problem::new(root, &mut root_slot, variables).await?;

    problem
        .layout(render.clone(), theme.clone(), &focus, &mut text_context)
        .await?;
    let solution = problem.solve(Size::new(100.0, 100.0)).await?;
    let anchor = problem.root.lock().await?.children[0].clone();
    let focusable = anchor.lock().await?.children[0].clone();
    let _ = problem
        .render(render, theme, &focus, &solution, &mut text_context)
        .await?;

    let command = problem
        .handle_event(
            &Event::Pointer(Pointer_event {
                position: Point::new(10.0, 10.0),
                button: Pointer_button::Primary,
            }),
            &solution,
            &mut focus,
        )
        .await?;
    assert!(matches!(command, Vizual_command::None));
    assert!(focus.compare(&focusable));

    let command = problem
        .handle_event(
            &Event::Pointer(Pointer_event {
                position: Point::new(80.0, 80.0),
                button: Pointer_button::Primary,
            }),
            &solution,
            &mut focus,
        )
        .await?;
    assert!(matches!(command, Vizual_command::None));
    assert!(focus.upgrade().is_none());

    Ok(())
}

#[tokio::test]
async fn width_constrained_paragraph_derives_its_wrapped_height() -> Result<()> {
    let render_manager = Render_manager::new();
    let render = render_manager.render.clone();
    let theme = Store::new(theme::dark_theme());
    let content = "a paragraph which wraps over several lines";
    let width = 80.0;
    let mut paragraph = Paragraph::new(Direction::Horizontal, width);
    paragraph.set_styled_content(content, theme::dark_theme().specific.text.paragraph);
    let root = Root::new(Anchor::top_left(paragraph)).into_shared();
    let mut root_slot = Component_slot::new();
    let variables = Arc::new(Variables::new());
    let mut text_context = Text_context::new();
    let expected_height = f64::from(
        text_context
            .build_wrapped_layout(
                &Styled_text::styled(content, theme::dark_theme().specific.text.paragraph),
                width as f32,
            )
            .height(),
    );
    let focus = Focus::new();
    let mut problem = App_problem::new(root, &mut root_slot, variables).await?;

    problem
        .layout(render, theme, &focus, &mut text_context)
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
    let render_manager = Render_manager::new();
    let render = render_manager.render.clone();
    let theme = Store::new(theme::dark_theme());
    let content = "one two three four five six seven eight nine ten eleven twelve";
    let height = 60.0;
    let mut paragraph = Paragraph::new(Direction::Vertical, height);
    paragraph.set_styled_content(content, theme::dark_theme().specific.text.paragraph);
    let root = Root::new(Anchor::top_left(paragraph)).into_shared();
    let mut root_slot = Component_slot::new();
    let variables = Arc::new(Variables::new());
    let mut text_context = Text_context::new();
    let natural_width = f64::from(
        text_context
            .build_layout(&Styled_text::styled(
                content,
                theme::dark_theme().specific.text.paragraph,
            ))
            .full_width(),
    );
    let focus = Focus::new();
    let mut problem = App_problem::new(root, &mut root_slot, variables).await?;

    problem
        .layout(render, theme, &focus, &mut text_context)
        .await?;
    let solution = problem.solve(Size::new(600.0, 200.0)).await?;
    let anchor = problem.root.lock().await?.children[0].clone();
    let paragraph = anchor.lock().await?.children[0].clone();
    let paragraph = paragraph.get_hitbox().await?.get_resolved(&solution);
    let wrapped_height = f64::from(
        text_context
            .build_wrapped_layout(
                &Styled_text::styled(content, theme::dark_theme().specific.text.paragraph),
                paragraph.size.width as f32,
            )
            .height(),
    );

    assert!((paragraph.size.height - height).abs() < 1e-6);
    assert!(paragraph.size.width < natural_width);
    assert!(wrapped_height <= height);
    Ok(())
}

#[tokio::test]
async fn scroll_lays_out_content_with_offset() -> Result<()> {
    let mut render_manager = Render_manager::new();
    let render = render_manager.render.clone();
    let theme = Store::new(theme::dark_theme());
    let root = Root::new(Scroll::new(Text::new("Scrollable content ".repeat(20)))).into_shared();
    let mut root_slot = Component_slot::new();
    let variables = Arc::new(Variables::new());
    let mut text_context = Text_context::new();
    let mut focus = Focus::new();
    let mut problem = App_problem::new(root, &mut root_slot, variables).await?;

    problem
        .layout(render.clone(), theme.clone(), &focus, &mut text_context)
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
            render.clone(),
            theme.clone(),
            &focus,
            &solution,
            &mut text_context,
        )
        .await?;
    assert!(scroll.lock().await?.focusable);
    focus.set(&scroll);
    let command = problem
        .handle_event(
            &Event::Key(Key_event {
                code: Key_code::Arrow_right,
                modifiers: Modifiers::default(),
                text: None,
                repeat: false,
            }),
            &solution,
            &mut focus,
        )
        .await?;
    assert!(matches!(command, Vizual_command::None));
    let crate::Render_request::Layout(id) = render_manager.receiver.0.recv().await.unwrap() else {
        panic!("scroll event must signal its component");
    };
    assert!(problem.root.invalidate_formula(id).await?);

    Ok(())
}

#[tokio::test]
async fn scroll_routes_pointer_events_in_transformed_frame_coordinates() -> Result<()> {
    let mut render_manager = Render_manager::new();
    let render = render_manager.render.clone();
    let theme = Store::new(theme::dark_theme());
    let root = Root::new(Scroll::new(Offset_click)).into_shared();
    let mut root_slot = Component_slot::new();
    let variables = Arc::new(Variables::new());
    let mut text_context = Text_context::new();
    let mut focus = Focus::new();
    let mut problem = App_problem::new(root, &mut root_slot, variables).await?;

    problem
        .layout(render.clone(), theme.clone(), &focus, &mut text_context)
        .await?;
    let solution = problem.solve(Size::new(80.0, 80.0)).await?;
    let scroll = problem.root.lock().await?.children[0].clone();
    focus.set(&scroll);

    let _scene = problem
        .render(
            render.clone(),
            theme.clone(),
            &focus,
            &solution,
            &mut text_context,
        )
        .await?;

    let command = problem
        .handle_event(
            &Event::Key(Key_event {
                code: Key_code::Arrow_right,
                modifiers: Modifiers::default(),
                text: None,
                repeat: false,
            }),
            &solution,
            &mut focus,
        )
        .await?;
    assert!(matches!(command, Vizual_command::None));
    let crate::Render_request::Layout(id) = render_manager.receiver.0.recv().await.unwrap() else {
        panic!("scroll event must signal its component");
    };
    assert!(problem.root.invalidate_formula(id).await?);

    problem
        .layout(render.clone(), theme.clone(), &focus, &mut text_context)
        .await?;
    let solution = problem.solve(Size::new(80.0, 80.0)).await?;

    let _scene = problem
        .render(render, theme, &focus, &solution, &mut text_context)
        .await?;
    let command = problem
        .handle_event(
            &Event::Pointer(Pointer_event {
                position: Point::new(20.0, 20.0),
                button: Pointer_button::Primary,
            }),
            &solution,
            &mut focus,
        )
        .await?;

    assert!(matches!(command, Vizual_command::Quit));
    Ok(())
}
