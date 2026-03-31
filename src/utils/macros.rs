//! Macro definitions used across runa.
//!
//! This module contains macros that are used in multiple places in the codebase.
//! They are defined here to avoid code duplication and to keep the codebase clean.

#[macro_export]
macro_rules! getters {
    ($( #[$meta:meta] )? $method:ident => $field:ident : &$type:ty, $($rest:tt)+) => {
        $crate::getters!($( #[$meta] )? $method => $field : &$type);
        $crate::getters!($($rest)+);
    };

    ($( #[$meta:meta] )? $method:ident => $field:ident : $type:ty, $($rest:tt)+) => {
        $crate::getters!($( #[$meta] )? $method => $field : $type);
        $crate::getters!($($rest)+);
    };

    ($( #[$meta:meta] )? $field:ident : &$type:ty, $($rest:tt)+) => {
        $crate::getters!($( #[$meta] )? $field : &$type);
        $crate::getters!($($rest)+);
    };

    ($( #[$meta:meta] )? $field:ident : $type:ty, $($rest:tt)+) => {
        $crate::getters!($( #[$meta] )? $field : $type);
        $crate::getters!($($rest)+);
    };

    ($( #[$meta:meta] )? $method:ident => $field:ident : &$type:ty $(,)?) => {
        $( #[$meta] )?
        #[inline]
        pub(crate) fn $method(&self) -> &$type {
            &self.$field
        }
    };

    ($( #[$meta:meta] )? $method:ident => $field:ident : $type:ty $(,)?) => {
        $( #[$meta] )?
        #[inline]
        pub(crate) fn $method(&self) -> $type {
            self.$field
        }
    };

    ($( #[$meta:meta] )? $field:ident : &$type:ty $(,)?) => {
        $( #[$meta] )?
        #[inline]
        pub(crate) fn $field(&self) -> &$type {
            &self.$field
        }
    };

    ($( #[$meta:meta] )? $field:ident : $type:ty $(,)?) => {
        $( #[$meta] )?
        #[inline]
        pub(crate) fn $field(&self) -> $type {
            self.$field
        }
    };

    () => {};
}

#[macro_export]
macro_rules! key_accessor {
    ($($name:ident),+ $(,)?) => {
        impl Keys {
            $(
                #[inline]
                pub(crate) fn $name(&self) -> &[String] {
                    &self.$name
                }
            )+
        }
    };
}
