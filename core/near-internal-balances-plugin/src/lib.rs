//! near-internal-balances-plugin builds on top of the [Near Accounts library](https://docs.rs/near-account/latest/near_account/)
//! Allows for users to deposit FTs, NFTs, and MTs into the contract and keep a list of balance.
//! I.e. say alice.near calls ft_transfer_call to a contract implementing Near internal balances, the contract
//! will "remember" how much Alice transfers. An example use case would be with Ref Finance where the users are required to
//! deposit tokens into the DEX smart contract in order to swap etc.
//! 
//! Usage is quite simple, here is a basic example
//! ```
//! use near_account::{
//!     impl_near_accounts_plugin, AccountDeposits, AccountInfoTrait, Accounts, NearAccountPlugin,
//!     NearAccountsPluginNonExternal, NewInfo,
//! };
//! use near_internal_balances_plugin::impl_near_balance_plugin;
//! 
//! use near_contract_standards::storage_management::StorageManagement;
//! use near_internal_balances_plugin::token_id::TokenId;
//! use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
//! use near_sdk::collections::{LazyOption, UnorderedMap};
//! use near_sdk::json_types::U128;
//! use near_sdk::{
//!     assert_one_yocto, env, log, near_bindgen, AccountId, Balance, PanicOnDefault, PromiseOrValue,
//! };
//! 
//! #[derive(BorshDeserialize, BorshSerialize)]
//! pub struct AccountInfo {
//!     pub internal_balance: UnorderedMap<TokenId, Balance>,
//! }
//! 
//! impl NewInfo for AccountInfo {
//!     fn default_from_account_id(account_id: AccountId) -> Self {
//!         Self {
//!             internal_balance: UnorderedMap::new(format!("{}-bal", account_id).as_bytes()),
//!         }
//!     }
//! }
//! #[near_bindgen]
//! #[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
//! pub struct Contract {
//!     pub accounts: Accounts<AccountInfo>,
//! }
//! 
//! impl_near_accounts_plugin!(Contract, accounts, AccountInfo);
//! impl_near_balance_plugin!(Contract, accounts, AccountInfo, internal_balance);
//! ```
//! 
//! 
//! As an aside, because this builds off of Near-Accounts, users need to register themselves with the smart contract in order to deal with
//! storage. Please see [The Near Accounts Documentation](https://docs.rs/near-account/latest/near_account/trait.NearAccountPlugin.html) for more information.

use near_account::{Account, Accounts};
use near_sdk::{
    env::{self},
    json_types::U128,
    serde::{Deserialize, Serialize},
    AccountId, Balance, Promise,
};

pub mod core_impl;
pub mod ft;
mod macros;
pub mod mt;
pub mod nft;
pub mod token_id;
pub use macros::*;
pub use token_id::TokenId;
mod utils;

/// NearInternalBalance gets implemented onto the contract struct with the addition
/// of the `impl_near_balance_plugin!` macro
/// 
/// `InternalBalanceHandlers` get exposed as public methods in the smart contract.
/// `SudoInternalBalanceHandlers` do not get exposed as public methods in the smart contract and are meant for internal contract use.
///
/// See the [internal balance handlers](trait.InternalBalanceHandlers.html) and [sudo internal balance handlers](trait.SudoInternalBalanceHandlers.html)
pub trait NearInternalBalance: SudoInternalBalanceHandlers + InternalBalanceHandlers {}

#[derive(Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub struct OnTransferOpts {
    // The account to log the transfer to
    pub sender_id: AccountId,
}

/// Implemented by the macro and is used to get balances
pub trait BalanceInfo {
    fn get_balance(&self, token_id: &TokenId) -> Balance;
    fn set_balance(&mut self, token_id: &TokenId, balance: Balance);
    fn get_all_tokens(&self) -> Vec<TokenId>;
}

/// The following methods get implemented by the contract struct but are not exposed as external, callable methods.
pub trait SudoInternalBalanceHandlers {
    /// Do a checked subtraction of an account balance
    fn internal_balance_subtract(
        &mut self,
        account_id: &AccountId,
        token_id: &TokenId,
        amount: Balance,
    );
    /// Do a checked addition to an account balance
    fn internal_balance_increase(
        &mut self,
        account_id: &AccountId,
        token_id: &TokenId,
        amount: Balance,
    );
    /// Same as get_ft_balance but without the serializable types
    fn internal_balance_get_internal(&self, account_id: &AccountId, token_id: &TokenId) -> Balance;
    /// Get the storage cost for one balance account
    fn internal_balance_get_storage_cost(&mut self, token_id: TokenId) -> Balance;
    /// Same as balance transfer but internal types
    fn internal_balance_transfer_internal(
        &mut self,
        recipient: AccountId,
        token_id: TokenId,
        amount: u128,
        message: Option<String>,
    );
}

/// The following methods get implemented by the contract struct and are exposed as external or internal methods
pub trait InternalBalanceHandlers {
    /// Called by an FT contract when a `ft_transfer_call` is done on an FT.
    /// msg is expected to either be `""` if the caller wants to register the transferred balance to their account
    /// or a JSON serialization of [OnTransferOpts](struct.OnTransferOpts.html)
    /// 
    /// This method is external
    fn ft_on_transfer(&mut self, sender_id: AccountId, amount: String, msg: String) -> String;

    /// Called by an NFT contract when a `nft_transfer_call` is done on an NFT.
    /// msg is expected to either be `""` if the caller wants to register the transferred balance to their account
    /// or a JSON serialization of [OnTransferOpts](struct.OnTransferOpts.html)
    /// 
    /// This method is external
    fn nft_on_transfer(
        &mut self,
        sender_id: AccountId,
        previous_owner_id: AccountId,
        token_id: String,
        msg: String,
    ) -> bool;

    /// Called by an MT contract when a `mt_transfer_call` is done on an MT.
    /// msg is expected to either be `""` if the caller wants to register the transferred balance to their account
    /// or a JSON serialization of [OnTransferOpts](struct.OnTransferOpts.html)
    /// 
    /// This method is external
    fn mt_on_transfer(
        &mut self,
        sender_id: AccountId,
        token_ids: Vec<String>,
        amounts: Vec<U128>,
        msg: String,
    ) -> Vec<U128>;

    /// Get the balance of an account for a given [TokenId](struct.TokenId.html)
    /// 
    /// This method is external
    fn internal_balance_get_balance(&self, account_id: AccountId, token_id: TokenId) -> U128;

    /// Get all balances for an account
    /// 
    /// This method is external
    fn internal_balance_get_all_balances(&self, account_id: AccountId) -> Vec<(TokenId, U128)>;

    /// Used as a callback when withdrawing funds
    /// 
    /// This method is internal
    fn resolve_internal_withdraw_call(
        &mut self,
        account_id: AccountId,
        token_id: TokenId,
        amount: U128,
        is_call: bool,
    ) -> U128;

    /// Withdraw from the caller's internal balance registered in the smart contract
    fn internal_balance_withdraw_to(
        &mut self,
        amount: U128,
        token_id: TokenId,
        recipient: Option<AccountId>,
        msg: Option<String>,
    ) -> Promise;

    /// Internally transfer `amount` of `token_id` from the caller to the recipient.
    /// This method will fail if the recipient is not registered with the smart contract.
    fn internal_balance_transfer(
        &mut self,
        recipient: AccountId,
        token_id: TokenId,
        amount: U128,
        message: Option<String>,
    );
}
