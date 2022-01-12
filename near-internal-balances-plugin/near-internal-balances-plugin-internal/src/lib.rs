use near_account::Accounts;
use near_sdk::{
    assert_one_yocto,
    borsh::{self, BorshDeserialize, BorshSerialize},
    collections::UnorderedMap,
    env::{self},
    json_types::{ValidAccountId, U128},
    log,
    serde::{Deserialize, Serialize},
    AccountId, Balance, Promise,
};

pub mod core_impl;
mod macros;
pub use macros::*;

pub trait NearFTInternalBalance: SudoInternalBalanceFungibleToken + InternalBalanceFungibleTokenHandlers{}

#[derive(Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub struct OnTransferOpts {
    // The account to log the transfer to
    pub sender_id: AccountId,
}

pub trait BalanceInfo {
    fn get_balance(&self, token_id: &AccountId) -> Balance;
    fn set_balance(&mut self, token_id: &AccountId, balance: Balance);
}

pub trait SudoInternalBalanceFungibleToken {
    fn subtract_balance(&mut self, account_id: &AccountId, token_id: &AccountId, amount: Balance);
    fn increase_balance(&mut self, account_id: &AccountId, token_id: &AccountId, amount: Balance);
    fn get_ft_balance_internal(&self, account_id: &AccountId, token_id: &AccountId) -> Balance;
}

pub trait InternalBalanceFungibleTokenHandlers {
    fn ft_on_transfer(&mut self, sender_id: String, amount: String, msg: String) -> String;
    fn get_ft_balance(&self, account_id: ValidAccountId, token_id: ValidAccountId) -> U128;
    fn resolve_internal_ft_transfer_call(
        &mut self,
        account_id: ValidAccountId,
        token_id: ValidAccountId,
        amount: U128,
        is_ft_call: bool,
    ) -> U128;
    fn withdraw_to(
        &mut self,
        amount: U128,
        token_id: ValidAccountId,
        recipient: Option<ValidAccountId>,
        msg: Option<String>,
    );

    fn balance_transfer(
        &mut self,
        recipient: ValidAccountId,
        token_id: ValidAccountId,
        amount: U128,
        message: Option<String>,
    );
}
