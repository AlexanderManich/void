macro_rules! push_size {
    ($arena:expr, $type_:ty) => {{
        push_size($arena, size_of::<$type_>()) as *mut $type_
    }};
}
pub(crate) use push_size;

macro_rules! push_array {
    ($arena:expr, $count: expr, $type_:ty) => {{
        push_size($arena, $count as usize * size_of::<$type_>()) as *mut $type_
    }};
}
pub(crate) use push_array;
