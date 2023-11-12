macro_rules! kilo_bytes {
    ($value:expr) => {
        $value as u64 * 1024
    };
}
pub(crate) use kilo_bytes;

macro_rules! mega_bytes {
    ($value:expr) => {
        kilo_bytes!($value as u64 * 1024)
    };
}
pub(crate) use mega_bytes;

macro_rules! giga_bytes {
    ($value:expr) => {
        mega_bytes!($value as u64 * 1024)
    };
}
pub(crate) use giga_bytes;

macro_rules! tera_bytes {
    ($value:expr) => {
        giga_bytes!($value as u64 * 1024)
    };
}
pub(crate) use tera_bytes;
