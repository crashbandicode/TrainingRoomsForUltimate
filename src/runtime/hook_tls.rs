//! Scalar storage for hook counters. Boot publishes all four keys together.
//! The allocator may read before publication; it must not initialize TLS.
use crate::model::scalar_tls::{Bank, Scalar};
use super::native_tls::NativeTls;

static BANK: Bank<4> = Bank::new();
pub(crate) fn init() -> Result<(), u32> { BANK.init(&NativeTls) }

pub(crate) trait Word: Copy {
    fn word(self) -> usize;
    fn from_word(value: usize) -> Self;
}
impl Word for u32 {
    fn word(self) -> usize { self as usize }
    fn from_word(value: usize) -> Self { value as u32 }
}
impl Word for u8 {
    fn word(self) -> usize { self as usize }
    fn from_word(value: usize) -> Self { value as u8 }
}

pub(crate) struct Local<T> { index: usize, default: T }
impl<T: Word> Local<T> {
    const fn new(index: usize, default: T) -> Self { Self { index, default } }
    pub(crate) fn try_with<R>(&self, work: impl FnOnce(&Value<'_, T>) -> R) -> Result<R, ()> {
        let scalar = BANK.cell(&NativeTls, self.index, self.default.word()).ok_or(())?;
        Ok(work(&Value { scalar, marker: core::marker::PhantomData }))
    }
}
pub(crate) struct Value<'a, T> {
    scalar: Scalar<'a, NativeTls, 4>,
    marker: core::marker::PhantomData<T>,
}
impl<T: Word> Value<'_, T> {
    pub(crate) fn get(&self) -> T { T::from_word(self.scalar.get()) }
    pub(crate) fn set(&self, value: T) { self.scalar.set(value.word()); }
    pub(crate) fn replace(&self, value: T) -> T { T::from_word(self.scalar.replace(value.word())) }
}

pub(crate) static HOOK_DEPTH: Local<u32> = Local::new(0, 0);
pub(crate) static LOAD_DEPTH: Local<u32> = Local::new(1, 0);
pub(crate) static UNCOUNTED_DEPTH: Local<u32> = Local::new(2, 0);
pub(crate) static CURRENT: Local<u8> = Local::new(3, crate::model::hook_stats::HookId::Other as u8);
