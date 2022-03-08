//! near-accounts allows for keeping track of data associated with an account
//! as well as storage management.
//!
//! Usage is quite simple. Start with the required imports
//! ```ignore
//! use near_account::{
//!     impl_near_accounts_plugin, Account, AccountDeposits, Accounts, NearAccountPlugin,
//!     NearAccountsPluginNonExternal, NewInfo,
//! };
//! ```
//! After, define a struct for what info the contract should store for each account
//! So, for example, if the contract intends to keep track of a message associated with each user
//! ```ignore
//! #[derive(BorshDeserialize, BorshSerialize)]
//! pub struct AccountInfo {
//!     pub message: String,
//! }
//! ```
//! Then, the contract must implement the `NewInfo` trait for `AccountInfo`, so, for example
//! ```ignore
//!   impl NewInfo for AccountInfo {
//!   fn default_from_account_id(account_id: AccountId) -> Self {
//!       Self {
//!           message: "".to_string(),
//!           internal_balance: UnorderedMap::new(format!("{}-bal", account_id).as_bytes()),
//!       }
//!   }
//!  }
//! ```
//!
//! Finally, all that is left to do is define the contract and call the implementing macro
//! ```ignore
//! #[near_bindgen]
//! #[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
//! pub struct Contract {
//!     pub accounts: Accounts<AccountInfo>,
//! }
//!
//! impl_near_accounts_plugin!(Contract, accounts, AccountInfo);
//! ```
//!
//! For documentation on externally defined functions, please see the
//! [NearAccountPlugin trait](trait.NearAccountPlugin.html)
//!
//! For documentation on functions for internal contract use, please see the
//! [NearAccountsPluginNonExternal trait](trait.NearAccountsPluginNonExternal.html)
//! and the [AccountDeposits trait](trait.AccountDeposits.html)

use std::str::FromStr;

use near_contract_standards::storage_management::{
    StorageBalance, StorageBalanceBounds, StorageManagement,
};
use near_sdk::{
    assert_one_yocto,
    borsh::{self, BorshDeserialize, BorshSerialize},
    collections::UnorderedMap,
    env::{self},
    json_types::U128,
    log, AccountId, Promise,
};

mod account;
pub mod macros;

pub use account::Account;
pub use account::{AccountDeposits, AccountInfoTrait};

/// A trait of the `AccountInfo` struct describing how to construct a default new account
pub trait NewInfo {
    fn default_from_account_id(account_id: AccountId) -> Self;
}

/// Defines the functions which will be exposed as methods for the smart contract
pub trait NearAccountPlugin {
    /// Allows for deposit into an account
    /// The amount deposited will be dependent on the amount of attached Near
    /// If an account is not initialized, the minimum amount attached must cover
    /// the storage deposit for initializing an account
    fn accounts_storage_deposit(
        &mut self,
        account_id: Option<near_sdk::AccountId>,
        registration_only: Option<bool>,
    ) -> near_contract_standards::storage_management::StorageBalance;

    /// Allows to withdraw any Near that is attached to an account
    /// but is not currently being used to cover storage
    fn accounts_storage_withdraw(
        &mut self,
        amount: Option<near_sdk::json_types::U128>,
    ) -> near_contract_standards::storage_management::StorageBalance;

    /// Unregister an account. Unregistration only succeeds if force is true
    fn accounts_storage_unregister(&mut self, force: Option<bool>) -> bool;

    /// Gives the storage bounds for a default account.
    /// This function is useful for finding the minimum required storage
    fn accounts_storage_balance_bounds(&self) -> StorageBalanceBounds;

    /// Gives the storage balance of an account
    fn accounts_storage_balance_of(
        &self,
        account_id: near_sdk::AccountId,
    ) -> Option<near_contract_standards::storage_management::StorageBalance>;

    /// Gives the Near balance of an account
    /// This is a duplicate function of `accounts_storage_balance_of`
    /// and is used for code readability
    fn accounts_near_balance_of(
        &self,
        account_id: near_sdk::AccountId,
    ) -> Option<near_contract_standards::storage_management::StorageBalance>;
}

pub trait NearAccountsPluginNonExternal<Info: AccountInfoTrait> {
    /// Get an account and panic if the account is not registered
    fn get_account_checked(&self, account_id: &AccountId) -> Account<Info>;

    /// Check that storage requirements are met for an account after the `closure` is called
    /// ## Arguments
    /// * `closure` - a function which can potentially update and store an account
    fn check_storage<F, T: Sized>(
        &mut self,
        account: &mut Account<Info>,
        account_id: &AccountId,
        closure: F,
    ) -> T
    where
        F: FnOnce(&mut Accounts<Info>, &mut Account<Info>) -> T;

    /// Forcibly removes an account from the accounts map.
    /// Note: use with caution
    fn remove_account_unchecked(&mut self, account_id: &AccountId) -> Option<Account<Info>>;

    /// Inserts/ updates an account without checking that storage bounds are met
    fn insert_account_unchecked(
        &mut self,
        account_id: &AccountId,
        account: &Account<Info>,
    ) -> Option<Account<Info>>;

    /// Inserts/ updates an account and checks storage
    ///
    /// ## Example
    /// ```ignore
    /// self.accounts.insert_account_check_storage(&caller, account);
    /// ```
    fn insert_account_check_storage(
        &mut self,
        account_id: &AccountId,
        account: &mut Account<Info>,
    ) -> Option<Account<Info>>;

    /// Get an account from the accounts map. If it is not found, return `None`
    fn get_account(&self, account_id: &AccountId) -> Option<Account<Info>>;
}

/// Account information and storage cost.
#[derive(BorshSerialize, BorshDeserialize)]
pub struct Accounts<AccountInfoUsed: AccountInfoTrait> {
    pub accounts: UnorderedMap<AccountId, Account<AccountInfoUsed>>,
    pub default_min_storage_bal: u128,
}

impl<Info: AccountInfoTrait> NearAccountsPluginNonExternal<Info> for Accounts<Info> {
    fn get_account_checked(&self, account_id: &AccountId) -> Account<Info> {
        let account = self.accounts.get(account_id);
        if account.is_none() {
            panic!("Account {} is unregistered", account_id);
        }
        account.unwrap()
    }

    fn check_storage<F, T: Sized>(
        &mut self,
        account: &mut Account<Info>,
        account_id: &AccountId,
        closure: F,
    ) -> T
    where
        F: FnOnce(&mut Accounts<Info>, &mut Account<Info>) -> T,
    {
        let ret = account.check_storage(self, closure);
        self.accounts.insert(&account_id, &account);
        ret
    }

    fn remove_account_unchecked(&mut self, account_id: &AccountId) -> Option<Account<Info>> {
        self.accounts.remove(account_id)
    }

    fn insert_account_unchecked(
        &mut self,
        account_id: &AccountId,
        account: &Account<Info>,
    ) -> Option<Account<Info>> {
        self.accounts.insert(account_id, account)
    }

    fn insert_account_check_storage(
        &mut self,
        account_id: &AccountId,
        account: &mut Account<Info>,
    ) -> Option<Account<Info>> {
        self.check_storage(account, account_id, |accounts, account| {
            accounts.accounts.insert(account_id, account)
        })
    }

    fn get_account(&self, account_id: &AccountId) -> Option<Account<Info>> {
        self.accounts.get(account_id)
    }
}

impl<Info: AccountInfoTrait> Accounts<Info> {
    pub fn new() -> Self {
        let mut ret = Accounts::<Info> {
            accounts: UnorderedMap::new(b"accounts-map".to_vec()),
            default_min_storage_bal: 0,
        };
        ret.default_min_storage_bal = ret.get_storage_cost(None, true);
        ret
    }
}

// TODO: PUBLIC FUNCTION WITH GET MIN COST FOR AN ACCOUNT ID (use borsh to j get # bytes)
impl<Info: AccountInfoTrait> Accounts<Info> {
    /// Get the cost of storage
    /// * `unregister` - if set to false then the get_storage_cost will also register the default account with the account id
    pub(crate) fn get_storage_cost(
        &mut self,
        account_id: Option<AccountId>,
        unregister: bool,
    ) -> u128 {
        let storage_prior = env::storage_usage();
        let account_id = account_id.unwrap_or(AccountId::from_str(&"a".repeat(64)).unwrap());
        let default_account = Account::default_from_account_id(account_id.clone());
        self.accounts.insert(&account_id, &default_account);

        // Get the storage cost
        let storage_cost =
            (env::storage_usage() - storage_prior) as u128 * env::storage_byte_cost();
        if unregister {
            self.accounts.remove(&account_id);
        }
        storage_cost
    }
}

/// storage handlers
impl<Info: AccountInfoTrait> StorageManagement for Accounts<Info> {
    fn storage_unregister(&mut self, force: Option<bool>) -> bool {
        if force.unwrap_or(false) == false {
            log!("Can only unregister if force is true");
            false
        } else {
            assert_one_yocto();
            let account_id = env::predecessor_account_id();
            let lookup = self.accounts.remove(&account_id);
            if lookup.is_none() {
                panic!("Cannot unregister a non-existent account");
            } else {
                log!("Deleting account {}", account_id);
                let account = lookup.unwrap();
                Promise::new(env::predecessor_account_id()).transfer(account.near_amount);
            }
            true
        }
    }

    fn storage_withdraw(&mut self, amount: Option<U128>) -> StorageBalance {
        assert_one_yocto();
        let predecessor_account_id = env::predecessor_account_id();
        if let Some(mut account) = self.accounts.get(&predecessor_account_id) {
            let amount = match amount {
                Some(amount) => {
                    if amount.0 > account.get_available_near() {
                        panic!("The amount is greater than the available storage balance");
                    } else {
                        amount.0
                    }
                }
                _ => account.get_available_near(),
            };

            account.near_amount -= amount;
            self.accounts.insert(&predecessor_account_id, &account);
            Promise::new(env::predecessor_account_id()).transfer(amount);
            account.storage_balance()
        } else {
            panic!("The account {} is not registered", &predecessor_account_id);
        }
    }

    fn storage_balance_bounds(&self) -> StorageBalanceBounds {
        StorageBalanceBounds { min: self.default_min_storage_bal.into(), max: None }
    }

    fn storage_balance_of(&self, account_id: AccountId) -> Option<StorageBalance> {
        if let Some(account) = self.accounts.get(&account_id) {
            Some(account.storage_balance())
        } else {
            None
        }
    }

    fn storage_deposit(
        &mut self,
        account_id: Option<AccountId>,
        registration_only: Option<bool>,
    ) -> near_contract_standards::storage_management::StorageBalance {
        let registration_only = registration_only.unwrap_or(false);
        let account_id: AccountId =
            account_id.map(|a| a.into()).unwrap_or(env::predecessor_account_id());
        let amount_attached = env::attached_deposit();
        let registered = self.accounts.get(&account_id);

        if registered.is_some() && registration_only {
            log!("Account already registered");
            Promise::new(env::predecessor_account_id()).transfer(amount_attached);
            let account = registered.unwrap();
            account.storage_balance()
        } else if registered.is_some() {
            let mut account = registered.unwrap();
            account.near_amount += amount_attached;
            self.accounts.insert(&account_id, &account);
            account.storage_balance()
        } else {
            // The account does not yet exist and must be created
            // NOTE: get_storage also registers the account id here
            let storage_cost = self.get_storage_cost(Some(account_id.clone()), false);
            if amount_attached < storage_cost {
                self.accounts.remove(&account_id);
                Promise::new(env::predecessor_account_id()).transfer(amount_attached);
                StorageBalance { available: 0.into(), total: 0.into() }
            } else if registration_only {
                let amount_refund = amount_attached - storage_cost;
                let mut account = self.accounts.get(&account_id).unwrap();
                account.near_amount = storage_cost;
                account.near_used_for_storage = storage_cost;
                self.accounts.insert(&account_id, &account);

                if amount_refund != 0 {
                    Promise::new(env::predecessor_account_id()).transfer(amount_refund);
                }
                account.storage_balance()
            } else {
                let mut account = self.accounts.get(&account_id).unwrap();
                account.near_amount = amount_attached;
                account.near_used_for_storage = storage_cost;
                self.accounts.insert(&account_id, &account);

                account.storage_balance()
            }
        }
    }
}

// TODO: basic unit tests

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {

    use std::convert::TryFrom;

    use super::*;
    use crate::{Account, NewInfo};
    use near_contract_standards::storage_management::StorageManagement;
    use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
    use near_sdk::collections::UnorderedMap;
    use near_sdk::test_utils::{accounts, VMContextBuilder};
    use near_sdk::MockedBlockchain;
    use near_sdk::{testing_env, Balance};

    const default_message: &str = "listening to anime lofi";

    #[derive(BorshSerialize, BorshDeserialize)]
    pub struct Info {
        pub msg: String,
    }

    impl NewInfo for Info {
        fn default_from_account_id(account_id: AccountId) -> Self {
            Self { msg: default_message.to_string() }
        }
    }

    impl AccountInfoTrait for Info {}

    fn get_near_accounts(
        mut context: VMContextBuilder,
    ) -> (AccountId, Accounts<Info>, VMContextBuilder) {
        let mut near_accounts = Accounts::<Info>::new();
        let account: AccountId = accounts(0).into();
        (account, near_accounts, context)
    }

    // mock the context for testing, notice "signer_account_id" that was accessed above from env::
    fn get_context(predecessor_account_id: AccountId) -> VMContextBuilder {
        let init_account_bal = 10_000 * env::storage_byte_cost();
        let mut builder = VMContextBuilder::new();
        builder
            .current_account_id(accounts(0))
            .signer_account_id(predecessor_account_id.clone())
            .predecessor_account_id(predecessor_account_id)
            .account_balance(init_account_bal);
        builder
    }

    fn register_account(
        account: AccountId,
        near_accounts: &mut Accounts<Info>,
        context: &mut VMContextBuilder,
    ) -> Account<Info> {
        let min = near_accounts.storage_balance_bounds().min.0;
        testing_env!(context.attached_deposit(min * 10).build());
        near_accounts
            .storage_deposit(Some(AccountId::try_from(account.clone()).unwrap()), Some(true));
        testing_env!(context.attached_deposit(1).build());
        let near_account = near_accounts.get_account_checked(&account);
        near_account
    }

    #[test]
    /// Test registering a user (with registration only), depositing extra into their account and withdrawing
    fn test_account_storage() {
        let mut context = get_context(accounts(0));
        testing_env!(context.build());
        testing_env!(context.attached_deposit(1).build());
        let (account, mut near_accounts, mut context) = get_near_accounts(context);
        register_account(account.clone(), &mut near_accounts, &mut context);

        // Check the storage post registration
        let storage_bounds = near_accounts.storage_balance_bounds();
        let storage_bal = near_accounts.storage_balance_of(account.clone()).unwrap();
        assert!(storage_bal.total.0 <= storage_bounds.min.0);
        assert_eq!(storage_bal.available.0, 0);

        let adding_near = 1_000_000_000_000_000;
        testing_env!(context.attached_deposit(adding_near).build());
        let storage_bal_new = near_accounts.storage_deposit(Some(account.clone()), None);
        assert_eq!(storage_bal.total.0 + adding_near, storage_bal_new.total.0);
        assert_eq!(storage_bal_new.available.0, adding_near);

        let withdrawing_near = 1_000;
        testing_env!(context.attached_deposit(1).build());
        let storage_bal_new = near_accounts.storage_withdraw(Some(withdrawing_near.into()));

        assert_eq!(storage_bal.total.0 + adding_near - withdrawing_near, storage_bal_new.total.0);
        assert_eq!(storage_bal_new.available.0, adding_near - withdrawing_near);
    }

    #[test]
    #[should_panic(expected = "The amount is greater than the available storage balance")]
    fn test_account_storage_withdraw_too_much() {
        let mut context = get_context(accounts(0));
        testing_env!(context.build());
        testing_env!(context.attached_deposit(1).build());
        let (account, mut near_accounts, mut context) = get_near_accounts(context);
        let account = register_account(account.clone(), &mut near_accounts, &mut context);

        let withdrawing_near = account.near_amount + 1_000;
        near_accounts.storage_withdraw(Some(withdrawing_near.into()));
    }

    #[test]
    fn test_account_storage_unregister() {
        let mut context = get_context(accounts(0));
        testing_env!(context.build());
        testing_env!(context.attached_deposit(1).build());
        let (account, mut near_accounts, mut context) = get_near_accounts(context);
        register_account(account.clone(), &mut near_accounts, &mut context);

        near_accounts.storage_unregister(Some(true));
        assert!(near_accounts.accounts.get(&account).is_none());
    }

    #[test]
    #[should_panic(expected = "Account alice is unregistered")]
    fn test_get_account_checked_panic() {
        let mut context = get_context(accounts(0));
        testing_env!(context.build());
        testing_env!(context.attached_deposit(1).build());

        let (account, mut near_accounts, mut context) = get_near_accounts(context);
        near_accounts.get_account_checked(&account);
    }

    #[test]
    fn test_insert_account_checked_update_storage() {
        let mut context = get_context(accounts(0));
        testing_env!(context.build());
        testing_env!(context.attached_deposit(1).build());
        let (account, mut near_accounts, mut context) = get_near_accounts(context);
        register_account(account.clone(), &mut near_accounts, &mut context);

        let new_message: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
        let storage_incr = new_message.len() - default_message.len();
        let cost = storage_incr as u128 * env::storage_byte_cost();
        testing_env!(context.attached_deposit(cost).build());
        let orig_storage_bal = near_accounts.storage_balance_of(account.clone()).unwrap();
        near_accounts.storage_deposit(None, None);

        let mut account_s = near_accounts.get_account_checked(&account);
        account_s.info.msg = new_message.to_string();
        near_accounts.insert_account_check_storage(&account, &mut account_s);

        let storage_bal = near_accounts.storage_balance_of(account.clone()).unwrap();
        assert_eq!(storage_bal.available.0, 0);
        assert_eq!(storage_bal.total.0, orig_storage_bal.total.0 + cost);
    }

    #[test]
    #[should_panic(expected = "Not enough Near to cover the transaction")]
    fn test_insert_account_checked_not_enough_near() {
        let mut context = get_context(accounts(0));
        testing_env!(context.build());
        testing_env!(context.attached_deposit(1).build());
        let (account, mut near_accounts, mut context) = get_near_accounts(context);
        let mut account_s = register_account(account.clone(), &mut near_accounts, &mut context);

        account_s.info.msg = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".to_string();
        near_accounts.insert_account_check_storage(&account, &mut account_s);
    }

    #[test]
    #[should_panic(expected = "Not enough Near to cover the transaction")]
    fn test_insert_account_checked_non_registered() {
        let mut context = get_context(accounts(0));
        testing_env!(context.build());
        testing_env!(context.attached_deposit(1).build());
        let (account, mut near_accounts, mut context) = get_near_accounts(context);
        let mut account_s = Account::<Info> {
            near_amount: 1_000_000_000,
            near_used_for_storage: 0,
            info: Info::default_from_account_id(account.clone()),
        };
        near_accounts.insert_account_check_storage(&account, &mut account_s);
    }
}
