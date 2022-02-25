use core::fmt;
use near_sdk::AccountId;
use std::fmt::Display;

use near_sdk::{
    borsh::{self, BorshDeserialize, BorshSerialize},
    serde::{self, Deserialize, Serialize},
};

#[derive(Serialize, Deserialize, BorshDeserialize, BorshSerialize, Debug, Clone)]
#[serde(crate = "near_sdk::serde")]
pub enum TokenId {
    MT { contract_id: AccountId, token_id: String },
    FT { contract_id: AccountId },
    NFT { contract_id: AccountId, token_id: String },
}

impl Display for TokenId {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // Customize so only `x` and `y` are denoted.
        write!(f, "{:?}", self)
    }
}