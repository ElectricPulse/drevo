use crate::widget::{Widget, Widget_trait};

/// A trait for converting collections of widgets into a [`Vec<Widget>`].
///
/// A new trait was created for this because otherwise one would have to generate a
/// `From<(tuple)>` for `Vec<Widget>`, which isn't allowed because of Rust's orphan rules
/// (`Vec` and standard tuples are both defined outside this crate). Without this trait,
/// `Vec<Widget>` would have to become a newtype, which is annoying.
///
/// Furthermore, `Vec` cannot be heterogeneous, which is why a tuple system was chosen.
/// This allows callers to pass tuples of diverse widget types—such as
/// `(Anchor::new(), Alignment::new(), Configurator::new())`—without having to manually
/// convert or box every element into `Widget`.
pub trait Into_widgets {
    fn into_widgets(self) -> Vec<Widget>;

    fn into(self) -> Vec<Widget>
    where
        Self: Sized,
    {
        self.into_widgets()
    }
}

impl Into_widgets for () {
    fn into_widgets(self) -> Vec<Widget> {
        Vec::new()
    }
}

impl Into_widgets for Vec<Widget> {
    fn into_widgets(self) -> Vec<Widget> {
        self
    }
}

macro_rules! impl_into_widgets_for_tuples {
    ($(($($T:ident),+ $(,)?))+) => {
        $(
            #[allow(non_snake_case)]
            impl<$($T: Widget_trait + 'static),+> Into_widgets for ($($T,)+) {
                fn into_widgets(self) -> Vec<Widget> {
                    let ($($T,)+) = self;
                    vec![$(Box::new($T)),+]
                }
            }
        )+
    };
}

impl_into_widgets_for_tuples! {
    (T1,)
    (T1, T2)
    (T1, T2, T3)
    (T1, T2, T3, T4)
    (T1, T2, T3, T4, T5)
    (T1, T2, T3, T4, T5, T6)
    (T1, T2, T3, T4, T5, T6, T7)
    (T1, T2, T3, T4, T5, T6, T7, T8)
    (T1, T2, T3, T4, T5, T6, T7, T8, T9)
    (T1, T2, T3, T4, T5, T6, T7, T8, T9, T10)
    (T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11)
    (T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12)
    (T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13)
    (T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14)
    (T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15)
    (T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16)
}
