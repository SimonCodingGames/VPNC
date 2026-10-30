//! Secret wrapper that masks values in `Debug` and `Display`.

use std::fmt;
use std::mem::ManuallyDrop;
use zeroize::{Zeroize, ZeroizeOnDrop};

/// Opaque wrapper around sensitive data.
///
/// `Debug` and `Display` never reveal the inner value.
#[derive(Clone, Zeroize, ZeroizeOnDrop)]
pub struct Secret<T: Zeroize>(T);

impl<T: Zeroize> Secret<T> {
    /// Wraps a sensitive value.
    pub fn new(value: T) -> Self {
        Self(value)
    }

    /// Borrows the inner value.
    pub fn expose(&self) -> &T {
        &self.0
    }

    /// Mutably borrows the inner value.
    pub fn expose_mut(&mut self) -> &mut T {
        &mut self.0
    }

    /// Consumes the wrapper and returns the inner value without zeroizing it.
    pub fn into_inner(self) -> T {
        let me = ManuallyDrop::new(self);
        // Safety: `me` is not dropped, so we take ownership of the inner value exactly once.
        unsafe { std::ptr::read(&me.0) }
    }
}

impl<T: Zeroize> fmt::Debug for Secret<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Secret([REDACTED])")
    }
}

impl<T: Zeroize> fmt::Display for Secret<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("[REDACTED]")
    }
}

impl<T: Zeroize> From<T> for Secret<T> {
    fn from(value: T) -> Self {
        Self::new(value)
    }
}

impl Secret<String> {
    /// Returns the secret as a string slice.
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl Secret<Vec<u8>> {
    /// Returns the secret as a byte slice.
    pub fn as_slice(&self) -> &[u8] {
        self.0.as_slice()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn debug_masks_secret() {
        let secret = Secret::new(String::from("super-secret-password"));
        let rendered = format!("{secret:?}");
        assert_eq!(rendered, "Secret([REDACTED])");
        assert!(!rendered.contains("super-secret"));
    }

    #[test]
    fn display_masks_secret() {
        let secret = Secret::new(String::from("token-abc"));
        let rendered = format!("{secret}");
        assert_eq!(rendered, "[REDACTED]");
        assert!(!rendered.contains("token"));
    }

    #[test]
    fn expose_returns_inner() {
        let secret = Secret::new(String::from("value"));
        assert_eq!(secret.expose(), "value");
    }
}
