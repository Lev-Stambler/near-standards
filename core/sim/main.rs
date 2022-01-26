#[cfg(test)]
mod testing {
    pub mod internal_balance_ft;
    pub mod internal_balance_mt;
    pub mod internal_balance_nft;
    pub mod utils;
    pub mod with_macros;

    pub use crate::testing::utils::*;
    pub use crate::testing::with_macros::*;
}

fn main() {}
