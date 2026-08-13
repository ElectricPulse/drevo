use crate::geometry::Point;

#[derive(Clone, Debug)]
pub enum Event {
    Key(Key_event),
    Pointer(Pointer_event),
    Wheel(Wheel_event),
    Text(String),
    Close_requested,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Modifiers {
    pub control: bool,
    pub alt: bool,
    pub shift: bool,
    pub super_key: bool,
}

#[derive(Clone, Debug)]
pub struct Key_event {
    pub code: Key_code,
    pub modifiers: Modifiers,
    pub text: Option<String>,
    pub repeat: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Key_code {
    Character(char),
    Enter,
    Escape,
    Tab,
    Back_tab,
    Backspace,
    Delete,
    Arrow_left,
    Arrow_right,
    Arrow_up,
    Arrow_down,
    Page_up,
    Page_down,
    Home,
    End,
    Space,
    Unidentified,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Pointer_button {
    Primary,
    Secondary,
    Middle,
    Other(u16),
}

#[derive(Clone, Copy, Debug)]
pub struct Pointer_event {
    pub position: Point,
    pub button: Pointer_button,
}

#[derive(Clone, Copy, Debug)]
pub enum Wheel_delta {
    Lines(Point),
    Logical_pixels(Point),
}

#[derive(Clone, Copy, Debug)]
pub struct Wheel_event {
    pub position: Point,
    pub delta: Wheel_delta,
    pub modifiers: Modifiers,
}
