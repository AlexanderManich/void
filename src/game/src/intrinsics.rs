// Intrinsics are built-in functions that correspond directly to some compiler instruction.

#[inline]
pub fn find_least_significant_set_bit(test: u32) -> Option<u32> {
    if test == 0 {
        return None;
    }
    // We use bsf compiler instruction through this call, instead of looping through every bit in test value.
    Some(test.trailing_zeros())
}
