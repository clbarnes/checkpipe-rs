#![doc = include_str!("../README.md")]
use std::collections::hash_map::DefaultHasher;
use std::hash::Hasher;
use std::io::{Read, Write};

/// Trait representing a computation performed on some bytes.
///
/// The computation must be independent of how the bytes are chunked when passed in.
pub trait Check {
    /// The result type of the computation.
    type Output;

    /// Update based on a new chunk of bytes.
    /// All bytes must be processed.
    fn update(&mut self, buf: &[u8]);

    /// Return the output of the computation (so far).
    fn output(&self) -> Self::Output;
}

impl<H: Hasher> Check for H {
    type Output = u64;

    fn update(&mut self, buf: &[u8]) {
        self.write(buf)
    }

    fn output(&self) -> Self::Output {
        self.finish()
    }
}

/// Struct wrapping over a [Check] type and some other type which handles bytes (usually a reader/writer).
///
/// Implements [Read] and/or [Write] if `T` does.
/// It is possible for the checker to get out of sync with the actual bytes
/// written if bytes are buffered and execution is interrupted before `.flush()` is called.
/// The same is true if a failed read advances the underlying reader without returning the bytes read.
pub struct Checker<C: Check, T> {
    checker: C,
    inner: T,
}

impl<C: Check, T> Checker<C, T> {
    pub fn new(checker: C, inner: T) -> Self {
        Self { checker, inner }
    }

    /// Insert a new [Check] struct, returning the old one.
    pub fn replace_checker(&mut self, new: C) -> C {
        std::mem::replace(&mut self.checker, new)
    }

    /// Insert a new inner value (reader/writer), returning the old one.
    pub fn replace_inner(&mut self, inner: T) -> T {
        std::mem::replace(&mut self.inner, inner)
    }

    /// Destroy the struct and create a new one, re-using the existing inner value (reader/writer).
    ///
    /// This allows the [Check] struct to be replaced with one of a different type.
    pub fn rebuild_with_checker<C2: Check>(self, hasher: C2) -> (Checker<C2, T>, C) {
        let (h1, r) = self.into_parts();
        (Checker::new(hasher, r), h1)
    }

    /// Destroy the struct and create a new one, re-using the existing [Check] struct.
    ///
    /// This allows the inner value to be replaced with one of a different type.
    pub fn rebuild_with_inner<T2>(self, inner: T2) -> (Checker<C, T2>, T) {
        let (h, inner1) = self.into_parts();
        (Checker::new(h, inner), inner1)
    }

    /// Destroy the struct, returning its component [Check] and inner structs as a tuple.
    pub fn into_parts(self) -> (C, T) {
        (self.checker, self.inner)
    }

    /// Return the current output value for all bytes read.
    pub fn output(&self) -> C::Output {
        self.checker.output()
    }
}

impl<T> Checker<DefaultHasher, T> {
    /// Use the given inner value and an empty [DefaultHasher] as the checker.
    pub fn new_default_hasher(inner: T) -> Self {
        Self {
            checker: DefaultHasher::default(),
            inner,
        }
    }

    /// Replace the internal hasher with an empty [DefaultHasher], returning the old one.
    pub fn reset_hasher(&mut self) -> DefaultHasher {
        std::mem::take(&mut self.checker)
    }
}

impl<C: Default + Check, T> Checker<C, T> {
    pub fn new_default(inner: T) -> Self {
        Self { checker: Default::default(), inner }
    }
}

impl<C: Check, R: Read> Read for Checker<C, R> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let out = self.inner.read(buf)?;
        self.checker.update(&buf[..out]);
        Ok(out)
    }
}

impl<C: Check, W: Write> Write for Checker<C, W> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let out = self.inner.write(buf)?;
        self.checker.update(&buf[..out]);
        Ok(out)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.inner.flush()
    }
}

/// Type implementing [Check] used by [Counter] for counting bytes as they pass through.
#[derive(Debug, Default)]
pub struct InnerCounter(usize);

impl Check for InnerCounter {
    type Output = usize;

    fn update(&mut self, buf: &[u8]) {
        self.0 += buf.len();
    }

    fn output(&self) -> Self::Output {
        self.0
    }
}

/// Type which counts the number of bytes passed through.
/// Useful for wrapping readers/writers before (or after) they are wrapped in compressors.
pub type Counter<T> = Checker<InnerCounter, T>;
