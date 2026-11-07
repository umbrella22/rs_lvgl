//! Explicit controlled values for typed form constructors.
//!
//! A binding is consumed during view construction. Only the ordinary value
//! descriptor and one event handler remain; it never borrows mutable app state.
use crate::{typed, Node};
use alloc::string::String;

/// A current model value and a mapper from native input to a component action.
pub struct Binding<T, F> {
    value: T,
    map: F,
}

/// Pair a current value with its action mapper for a typed form control.
///
/// `binding(self.speed, FanSettingsAction::SpeedChanged)` does not assign to
/// `self.speed`. Native input queues the mapped action; `App::update` decides
/// whether to accept it. Text mappers receive `&str` for the callback duration.
pub fn binding<T, F>(value: T, map: F) -> Binding<T, F> {
    Binding { value, map }
}

mod private {
    pub trait Sealed {}
    #[cfg(feature = "widget-slider")]
    impl Sealed for crate::SliderRange {}
    impl Sealed for i32 {}
    impl Sealed for bool {}
    impl Sealed for alloc::string::String {}
    impl Sealed for &str {}
    impl Sealed for &alloc::string::String {}
    impl<T, F> Sealed for super::Binding<T, F> {}
}

/// Accepted numeric constructor inputs. Use an `i32` or [`binding`].
pub trait NumericInput<M>: private::Sealed {
    #[doc(hidden)]
    fn into_numeric_node<K>(self, build: impl FnOnce(i32) -> Node<M, K>) -> Node<M, K>
    where
        K: typed::Supports<typed::capability::Numeric>;
}
impl<M> NumericInput<M> for i32 {
    fn into_numeric_node<K>(self, build: impl FnOnce(i32) -> Node<M, K>) -> Node<M, K>
    where
        K: typed::Supports<typed::capability::Numeric>,
    {
        build(self)
    }
}
impl<M, F: FnMut(i32) -> M + 'static> NumericInput<M> for Binding<i32, F> {
    fn into_numeric_node<K>(self, build: impl FnOnce(i32) -> Node<M, K>) -> Node<M, K>
    where
        K: typed::Supports<typed::capability::Numeric>,
    {
        build(self.value).on_value_changed(self.map)
    }
}

/// Accepted toggle constructor inputs. Use a `bool` or [`binding`].
pub trait ToggleInput<M>: private::Sealed {
    #[doc(hidden)]
    fn into_toggle_node<K>(self, build: impl FnOnce(bool) -> Node<M, K>) -> Node<M, K>
    where
        K: typed::Supports<typed::capability::Toggle>;
}
impl<M> ToggleInput<M> for bool {
    fn into_toggle_node<K>(self, build: impl FnOnce(bool) -> Node<M, K>) -> Node<M, K>
    where
        K: typed::Supports<typed::capability::Toggle>,
    {
        build(self)
    }
}
impl<M, F: FnMut(bool) -> M + 'static> ToggleInput<M> for Binding<bool, F> {
    fn into_toggle_node<K>(self, build: impl FnOnce(bool) -> Node<M, K>) -> Node<M, K>
    where
        K: typed::Supports<typed::capability::Toggle>,
    {
        build(self.value).on_toggle(self.map)
    }
}

/// Accepted textarea inputs: `String`, `&String`, `&str`, or their binding.
pub trait TextInput<M>: private::Sealed {
    #[doc(hidden)]
    fn into_text_node<K>(self, build: impl FnOnce(String) -> Node<M, K>) -> Node<M, K>
    where
        K: typed::Supports<typed::capability::Text>;
}
macro_rules! text_input {
    ($($text:ty),*) => {$(
        impl<M> TextInput<M> for $text {
            fn into_text_node<K>(self, build: impl FnOnce(String) -> Node<M, K>) -> Node<M, K>
            where K: typed::Supports<typed::capability::Text> {
                build(self.into())
            }
        }
        impl<M, F: FnMut(&str) -> M + 'static> TextInput<M> for Binding<$text, F> {
            fn into_text_node<K>(self, build: impl FnOnce(String) -> Node<M, K>) -> Node<M, K>
            where K: typed::Supports<typed::capability::Text> {
                build(self.value.into()).on_text_changed(self.map)
            }
        }
    )*};
}
text_input!(String, &str, &String);

/// Accepted range constructor inputs: [`crate::SliderRange`] or its [`binding`].
#[cfg(feature = "widget-slider")]
pub trait RangeInput<M>: private::Sealed {
    #[doc(hidden)]
    fn into_range_node<K>(self, build: impl FnOnce(crate::SliderRange) -> Node<M, K>) -> Node<M, K>
    where
        K: typed::Supports<typed::capability::Range>;
}
#[cfg(feature = "widget-slider")]
impl<M> RangeInput<M> for crate::SliderRange {
    fn into_range_node<K>(self, build: impl FnOnce(crate::SliderRange) -> Node<M, K>) -> Node<M, K>
    where
        K: typed::Supports<typed::capability::Range>,
    {
        build(self)
    }
}
#[cfg(feature = "widget-slider")]
impl<M, F: FnMut(crate::SliderRange) -> M + 'static> RangeInput<M>
    for Binding<crate::SliderRange, F>
{
    fn into_range_node<K>(self, build: impl FnOnce(crate::SliderRange) -> Node<M, K>) -> Node<M, K>
    where
        K: typed::Supports<typed::capability::Range>,
    {
        build(self.value).on_range_changed(self.map)
    }
}
