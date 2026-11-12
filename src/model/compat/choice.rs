//! `byteflags!` and `count!` for the carried menu-choice groups, written for this repository (not carried).
//!
//! The fork's cycle-preload config declares its menu choices (`CyclePreloadSlots`, `CyclePreloadSources`, ...) with the macro of the
//! third-party crate `byteflags` (MIT by its LICENSE file, no `license` field). That macro names a struct field after every choice,
//! in capitals, so a crate using it gets a `non_snake_case` warning for each choice: the only ways to a clean `clippy -D warnings`
//! are a lint `allow` (a workaround, not coded without approval) or not using that macro. The crate also pulls three git
//! dependencies (`rand`, `getrandom`, `byteflags` itself) for one `get_random` the carried code does not call. This module is the
//! macro the carried file needs, over a plain `[u8; N]`: the same constants (`ALL_CONSTS`, `ALL_NAMES`, one constant per choice),
//! the same saturating arithmetic (`+`, `-`, `*`), `to_byte_vec`, `contains`, `empty`, `all`, `Display`, and the same JSON shape
//! (an array of N small integers, one per choice). `tools/carry_pkg6.py` rewrites the carried file's `use byteflags::{byteflags,
//! count}` to this module (the manifest's `modules.byteflags`), so the carried text is unchanged.

/// Number of token trees: `count!(A B C)` is 3.
macro_rules! count {
    () => (0usize);
    ($x:tt $($xs:tt)*) => (1usize + $crate::engine::compat::choice::count!($($xs)*));
}
pub(crate) use count;

/// Index of `name` in `names` (const: a choice constant is its unit vector).
pub const fn index_of(names: &[&str], name: &str) -> usize {
    let wanted = name.as_bytes();
    let mut index = 0;
    while index < names.len() {
        let candidate = names[index].as_bytes();
        if candidate.len() == wanted.len() {
            let mut at = 0;
            while at < wanted.len() && candidate[at] == wanted[at] {
                at += 1;
            }
            if at == wanted.len() {
                return index;
            }
        }
        index += 1;
    }
    panic!("choice name not declared");
}

macro_rules! byteflags {
    (
        $(#[$outer:meta])*
        $vis:vis struct $Group:ident {
            $(
                $choice_vis:vis $Choice:ident = $label:literal,
            )*
        }
    ) => {
        $(#[$outer])*
        #[derive(Clone, Copy, Debug, PartialEq, Eq)]
        $vis struct $Group([u8; $crate::engine::compat::choice::count!($($Choice)*)]);

        impl $Group {
            const WIDTH: usize = $crate::engine::compat::choice::count!($($Choice)*);

            /// The constant names, in declaration order.
            pub const ALL_FIELDS: [&'static str; $crate::engine::compat::choice::count!($($Choice)*)] = [$(stringify!($Choice),)*];
            /// The labels, in declaration order.
            pub const ALL_NAMES: [&'static str; $crate::engine::compat::choice::count!($($Choice)*)] = [$($label,)*];

            const fn unit(index: usize) -> Self {
                let mut elements = [0u8; $crate::engine::compat::choice::count!($($Choice)*)];
                elements[index] = 1;
                Self(elements)
            }

            $(
                $choice_vis const $Choice: Self =
                    Self::unit($crate::engine::compat::choice::index_of(&Self::ALL_FIELDS, stringify!($Choice)));
            )*

            /// One constant per choice, to compare by index.
            pub const ALL_CONSTS: [Self; $crate::engine::compat::choice::count!($($Choice)*)] = [$(Self::$Choice,)*];

            pub const fn empty() -> Self {
                Self([0u8; $crate::engine::compat::choice::count!($($Choice)*)])
            }

            pub const fn all() -> Self {
                Self([1u8; $crate::engine::compat::choice::count!($($Choice)*)])
            }

            pub fn to_byte_vec(self) -> Vec<u8> {
                self.0.to_vec()
            }

            pub fn to_vec(self) -> Vec<Self> {
                Self::ALL_CONSTS.iter().filter(|item| self.contains(item)).copied().collect()
            }

            /// Every choice `other` has set is set here too (only zero / nonzero counts).
            pub fn contains(&self, other: &Self) -> bool {
                self.0.iter().zip(other.0.iter()).all(|(mine, theirs)| *theirs == 0 || *mine > 0)
            }

            pub fn union(&self, other: Self) -> Self {
                *self + other
            }

            /// The values of `self` that are nonzero in `other`.
            pub fn left_intersection(&self, other: Self) -> Self {
                let mut out = *self;
                for (value, mask) in out.0.iter_mut().zip(other.0.iter()) {
                    if *mask == 0 {
                        *value = 0;
                    }
                }
                out
            }

            fn zip_with(self, other: Self, op: impl Fn(u8, u8) -> u8) -> Self {
                let mut out = self;
                for (value, rhs) in out.0.iter_mut().zip(other.0.iter()) {
                    *value = op(*value, *rhs);
                }
                out
            }
        }

        impl ::std::fmt::Display for $Group {
            fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                let mut first = true;
                for (value, label) in self.0.iter().zip(Self::ALL_NAMES.iter()) {
                    if *value > 0 {
                        if !first {
                            f.write_str(" + ")?;
                        }
                        f.write_str(label)?;
                        first = false;
                    }
                }
                Ok(())
            }
        }

        impl ::std::ops::Add for $Group {
            type Output = Self;
            fn add(self, rhs: Self) -> Self {
                self.zip_with(rhs, u8::saturating_add)
            }
        }

        impl ::std::ops::AddAssign for $Group {
            fn add_assign(&mut self, rhs: Self) {
                *self = *self + rhs;
            }
        }

        impl ::std::ops::Sub for $Group {
            type Output = Self;
            fn sub(self, rhs: Self) -> Self {
                self.zip_with(rhs, u8::saturating_sub)
            }
        }

        impl ::std::ops::SubAssign for $Group {
            fn sub_assign(&mut self, rhs: Self) {
                *self = *self - rhs;
            }
        }

        impl ::std::ops::Mul<u8> for $Group {
            type Output = Self;
            fn mul(self, rhs: u8) -> Self {
                let mut out = self;
                for value in out.0.iter_mut() {
                    *value = value.saturating_mul(rhs);
                }
                out
            }
        }

        impl ::std::ops::MulAssign<u8> for $Group {
            fn mul_assign(&mut self, rhs: u8) {
                *self = *self * rhs;
            }
        }

        impl ::std::ops::Mul for $Group {
            type Output = Self;
            fn mul(self, rhs: Self) -> Self {
                self.zip_with(rhs, u8::saturating_mul)
            }
        }

        impl ::std::ops::MulAssign for $Group {
            fn mul_assign(&mut self, rhs: Self) {
                *self = *self * rhs;
            }
        }

        impl ::serde::Serialize for $Group {
            fn serialize<S: ::serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
                serializer.collect_seq(self.0.iter())
            }
        }

        impl<'de> ::serde::Deserialize<'de> for $Group {
            fn deserialize<D: ::serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
                struct Elements;
                impl<'de> ::serde::de::Visitor<'de> for Elements {
                    type Value = $Group;
                    fn expecting(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                        write!(f, "an array of {} small integers", $Group::WIDTH)
                    }
                    fn visit_seq<A: ::serde::de::SeqAccess<'de>>(self, mut seq: A) -> Result<$Group, A::Error> {
                        let mut elements = [0u8; $crate::engine::compat::choice::count!($($Choice)*)];
                        for (index, slot) in elements.iter_mut().enumerate() {
                            *slot = seq
                                .next_element()?
                                .ok_or_else(|| ::serde::de::Error::invalid_length(index, &self))?;
                        }
                        Ok($Group(elements))
                    }
                }
                deserializer.deserialize_seq(Elements)
            }
        }
    };
}
pub(crate) use byteflags;

#[cfg(test)]
mod tests {
    lib_unit_tests! {
    byteflags! {
        pub struct Pick {
            pub FIRST = "First",
            pub SECOND = "Second",
            pub THIRD = "Third",
        }
    }

    fn arithmetic_saturates_like_the_crate_it_replaces() {
        assert_eq!((Pick::FIRST + Pick::THIRD).to_byte_vec(), [1, 0, 1]);
        assert_eq!((Pick::all() - Pick::SECOND).to_byte_vec(), [1, 0, 1]);
        assert_eq!((Pick::FIRST - Pick::SECOND), Pick::FIRST);
        assert_eq!((Pick::FIRST * 250u8 + Pick::FIRST * 250u8).to_byte_vec(), [255, 0, 0]);
        assert_eq!((Pick::all() * Pick::SECOND), Pick::SECOND);
        let mut value = Pick::empty();
        value += Pick::SECOND;
        value *= 3u8;
        assert_eq!(value.to_byte_vec(), [0, 3, 0]);
        value -= Pick::SECOND;
        assert_eq!(value.to_byte_vec(), [0, 2, 0]);
    }

    fn contains_only_looks_at_zero_and_nonzero() {
        let two = Pick::FIRST + Pick::THIRD;
        assert!(two.contains(&Pick::FIRST));
        assert!(!two.contains(&Pick::SECOND));
        assert!(two.contains(&Pick::empty()));
        assert!((Pick::FIRST * 4u8).contains(&Pick::FIRST));
        assert_eq!(two.to_vec(), vec![Pick::FIRST, Pick::THIRD]);
        assert_eq!((Pick::FIRST * 4u8 + Pick::SECOND * 2u8).left_intersection(Pick::SECOND).to_byte_vec(), [0, 2, 0]);
        assert_eq!(Pick::FIRST.union(Pick::SECOND).to_byte_vec(), [1, 1, 0]);
    }

    fn display_joins_the_labels_of_the_set_choices() {
        assert_eq!(Pick::empty().to_string(), "");
        assert_eq!(Pick::SECOND.to_string(), "Second");
        assert_eq!((Pick::FIRST + Pick::THIRD).to_string(), "First + Third");
    }

    /// rc.9: this module's cases in one table. Each case runs on its own thread, as the test harness ran it.
    #[test]
    fn case_table() {
        crate::test_table::run(&[
            ("arithmetic_saturates_like_the_crate_it_replaces", arithmetic_saturates_like_the_crate_it_replaces as fn()),
            ("contains_only_looks_at_zero_and_nonzero", contains_only_looks_at_zero_and_nonzero as fn()),
            ("display_joins_the_labels_of_the_set_choices", display_joins_the_labels_of_the_set_choices as fn()),
        ]);
    }
    }
}
