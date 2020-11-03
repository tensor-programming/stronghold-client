mod cache;
mod client;
mod data;
mod provider;

pub use crate::cache::Cache;

#[macro_export]
macro_rules! line_error {
    () => {
        concat!("Error at ", file!(), ":", line!())
    };
    ($str:expr) => {
        concat!($str, " @", file!(), ":", line!())
    };
}

fn main() {
    println!("Hello, world!");
}
