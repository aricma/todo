#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapacityError {
    pub current: usize,
    pub incoming: usize,
    pub max: usize,
}

pub fn check_capacity(current: usize, incoming: usize, max: usize) -> Result<(), CapacityError> {
    if current + incoming <= max {
        Ok(())
    } else {
        Err(CapacityError {
            current,
            incoming,
            max,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capacity_boundaries() {
        // ADR-0001 semantics with a test-local max (the production value
        // lives in the app: app/constants.rs): max-1+1 ok, max+0 ok,
        // max+1 rejected, empty+max+1 rejected.
        let max = 1000;
        assert!(check_capacity(999, 1, max).is_ok());
        assert!(check_capacity(max, 0, max).is_ok());
        assert!(check_capacity(max, 1, max).is_err());
        assert!(check_capacity(0, max + 1, max).is_err());
    }
}
