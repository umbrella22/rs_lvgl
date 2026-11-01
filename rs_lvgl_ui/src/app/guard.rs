//! Reentrancy guard for mutually exclusive UI operations.

use crate::UiError;
use core::cell::Cell;

pub(super) struct BusyGuard(*const Cell<bool>);

impl Drop for BusyGuard {
    fn drop(&mut self) {
        // SAFETY: the guard is created and dropped inside one `&mut self`
        // method. `busy` is a field of the same `Ui`, which cannot move while
        // the method holds `&mut self`.
        unsafe { (*self.0).set(false) };
    }
}

pub(super) fn enter(busy: &Cell<bool>) -> Result<BusyGuard, UiError> {
    if busy.replace(true) {
        return Err(UiError::Reentrant);
    }
    Ok(BusyGuard(busy as *const Cell<bool>))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nested_mutation_is_rejected() {
        let busy = Cell::new(false);
        let guard = enter(&busy).expect("first entry succeeds");
        assert_eq!(enter(&busy).err(), Some(UiError::Reentrant));
        drop(guard);
        assert!(enter(&busy).is_ok());
    }
}
