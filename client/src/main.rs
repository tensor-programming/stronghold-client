#[cfg(test)]
mod actor_test_client;
mod cache;
mod client;
mod data;
mod provider;
mod secret;
mod snap;

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
