use near_account::{Account, Accounts};
use near_sdk::{
    assert_one_yocto,
    borsh::{self, BorshDeserialize, BorshSerialize},
    collections::UnorderedMap,
    env::{self},
    json_types::U128,
    log,
    serde::{Deserialize, Serialize},
    AccountId, Balance, Promise, PromiseOrValue,
};

pub mod core_impl;
pub mod ft;
mod macros;
mod mt;
pub mod nft;
pub mod token_id;
pub use macros::*;
pub use token_id::TokenId;

pub trait NearFTInternalBalance: SudoInternalBalanceHandlers + InternalBalanceHandlers {}

#[derive(Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub struct OnTransferOpts {
    // The account to log the transfer to
    pub sender_id: AccountId,
}

pub trait BalanceInfo {
    fn get_balance(&self, token_id: &TokenId) -> Balance;
    fn set_balance(&mut self, token_id: &TokenId, balance: Balance);
}

pub trait SudoInternalBalanceHandlers {
    /// Do a checked subtraction of an account balance
    fn subtract_balance(&mut self, account_id: &AccountId, token_id: &TokenId, amount: Balance);
    /// Do a checked addition to an account balance
    fn increase_balance(&mut self, account_id: &AccountId, token_id: &TokenId, amount: Balance);
    /// Same as get_ft_balance but without the serializable types
    fn get_balance_internal(&self, account_id: &AccountId, token_id: &TokenId) -> Balance;
    /// Get the storage cost for one balance account
    fn get_storage_cost_for_one_balance(&mut self, token_id: TokenId) -> Balance;
    /// Same as balance transfer but internal types
    fn internal_balance_transfer_internal(
        &mut self,
        recipient: AccountId,
        token_id: TokenId,
        amount: u128,
        message: Option<String>,
    );
}

pub trait InternalBalanceHandlers {
    fn ft_on_transfer(&mut self, sender_id: AccountId, amount: String, msg: String) -> String;

    fn nft_on_transfer(
        &mut self,
        sender_id: AccountId,
        previous_owner_id: AccountId,
        token_id: String,
        msg: String,
    ) -> bool;

    // TODO:
    // fn nft_on_transfer(&mut self, sender_id: String, amount: String, msg: String) -> String;
    fn internal_balance_get_balance(&self, account_id: AccountId, token_id: TokenId) -> U128;
    fn resolve_internal_withdraw_call(
        &mut self,
        account_id: AccountId,
        token_id: TokenId,
        amount: U128,
        is_call: bool,
    ) -> U128;

    fn internal_balance_withdraw_to(
        &mut self,
        amount: U128,
        token_id: TokenId,
        recipient: Option<AccountId>,
        msg: Option<String>,
    ) -> ();

    fn internal_balance_transfer(
        &mut self,
        recipient: AccountId,
        token_id: TokenId,
        amount: U128,
        message: Option<String>,
    );
}
