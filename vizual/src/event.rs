use crate::geometry::Point;

#[derive(Clone, Debug)]
pub enum Event {
    Key(KeyEvent),
    Pointer(PointerEvent),
    Wheel(WheelEvent),
    Text(String),
    CloseRequested,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Modifiers {
    pub control: bool,
    pub alt: bool,
    pub shift: bool,
    pub super_key: bool,
}

#[derive(Clone, Debug)]
pub struct KeyEvent {
    pub code: KeyCode,
    pub modifiers: Modifiers,
    pub text: Option<String>,
    pub repeat: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum KeyCode {
    Character(char),
    Enter,
    Escape,
    Tab,
    BackTab,
    Backspace,
    Delete,
    ArrowLeft,
    ArrowRight,
    ArrowUp,
    ArrowDown,
    PageUp,
    PageDown,
    Home,
    End,
    Space,
    Unidentified,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PointerButton {
    Primary,
    Secondary,
    Middle,
    Other(u16),
}

#[derive(Clone, Copy, Debug)]
pub struct PointerEvent {
    pub position: Point,
    pub button: PointerButton,
}

#[derive(Clone, Copy, Debug)]
pub struct WheelEvent {
    pub position: Point,
    /// Scroll distance in the logical-pixel coordinate space used for layout.
    pub delta: Point,
    pub modifiers: Modifiers,
}
