use crate::ValidAccountId;
use core::fmt;
use near_sdk::AccountId;
use std::fmt::Display;

use near_sdk::{
    borsh::{self, BorshDeserialize, BorshSerialize},
    serde::{self, Deserialize, Serialize},
};

#[derive(Serialize, Deserialize, BorshDeserialize, BorshSerialize, Debug)]
#[serde(crate = "near_sdk::serde")]
pub enum ValidTokenId {
    MT { contract_id: ValidAccountId, token_id: ValidAccountId },
    FT { contract_id: ValidAccountId },
    NFT { contract_id: ValidAccountId, token_id: ValidAccountId },
}

#[derive(Serialize, Deserialize, BorshDeserialize, BorshSerialize, Debug)]
#[serde(crate = "near_sdk::serde")]
pub enum TokenId {
    MT { contract_id: AccountId, token_id: AccountId },
    FT { contract_id: AccountId },
    NFT { contract_id: AccountId, token_id: AccountId },
}

impl Display for TokenId {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // Customize so only `x` and `y` are denoted.
        write!(f, "{:?}", self)
    }
}

impl TokenId {
    pub(crate) fn new_ft(contract_id: String) -> Self {
        Self::FT { contract_id }
    }
}

impl Into<TokenId> for ValidTokenId {
    fn into(self) -> TokenId {
        match self {
            Self::MT { contract_id, token_id } => {
                TokenId::MT { contract_id: contract_id.into(), token_id: token_id.into() }
            }
            Self::NFT { contract_id, token_id } => {
                TokenId::MT { contract_id: contract_id.into(), token_id: token_id.into() }
            }
            Self::FT { contract_id } => TokenId::FT { contract_id: contract_id.into() },
        }
    }
}
