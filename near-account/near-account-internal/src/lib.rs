use near_contract_standards::storage_management::{
    StorageBalance, StorageBalanceBounds, StorageManagement,
};
use near_sdk::{
    assert_one_yocto,
    borsh::{self, BorshDeserialize, BorshSerialize},
    collections::UnorderedMap,
    env::{self},
    json_types::{ValidAccountId, U128},
    log, AccountId, Balance, Promise,
};

pub use account::Account;
pub use account::{AccountDeposits, AccountInfoTrait};

mod account;

pub trait NewInfo {
    fn default_from_account_id(account_id: AccountId) -> Self;
}

/// Account information and storage cost.
#[derive(BorshSerialize, BorshDeserialize)]
pub struct Accounts<AccountInfoUsed: AccountInfoTrait> {
    pub accounts: UnorderedMap<AccountId, Account<AccountInfoUsed>>,
    pub default_min_storage_bal: u128,
}

impl<Info: AccountInfoTrait> Accounts<Info> {
    /// Get an account and panic if the account is not registered
    pub fn get_account_checked(&self, account_id: &AccountId) -> Account<Info> {
        let account = self.accounts.get(account_id);
        if account.is_none() {
            panic!("Account {} is unregistered", account_id);
        }
        account.unwrap()
    }

    pub fn check_storage<F, T: Sized>(
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

    pub fn remove_account_unchecked(
        &mut self,
        account_id: &AccountId,
    ) -> Option<Account<Info>> {
        self.accounts.remove(account_id)
    }

    pub fn insert_account_unchecked(
        &mut self,
        account_id: &AccountId,
        account: &Account<Info>,
    ) -> Option<Account<Info>> {
        self.accounts.insert(account_id, account)
    }

    pub fn insert_account_check_storage(
        &mut self,
        account_id: &AccountId,
        account: &mut Account<Info>,
    ) -> Option<Account<Info>> {
        self.check_storage(account, account_id, |accounts, account| {
            accounts.accounts.insert(account_id, account)
        })
    }

    pub fn get_account(&self, account_id: &AccountId) -> Option<Account<Info>> {
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

impl<Info: AccountInfoTrait> Accounts<Info> {
    /// Get the cost of storage
    /// * `unregister` - if set to false then the get_storage_cost will also register the default account with the account id
    pub(crate) fn get_storage_cost(
        &mut self,
        account_id: Option<AccountId>,
        unregister: bool,
    ) -> u128 {
        let storage_prior = env::storage_usage();
        let account_id = account_id.unwrap_or("a".repeat(64));
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
            // TODO: make macro for this (sep lib)
            assert_eq!(env::attached_deposit(), 1, "Expected 1 Near");
            let account_id = env::predecessor_account_id();
            let lookup = self.accounts.remove(&account_id);
            if lookup.is_none() {
                panic!("Cannot unregister a non-existant account");
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
            env::panic(
                format!("The account {} is not registered", &predecessor_account_id).as_bytes(),
            );
        }
    }

    fn storage_balance_bounds(&self) -> StorageBalanceBounds {
        StorageBalanceBounds {
            min: self.default_min_storage_bal.into(),
            max: None,
        }
    }

    fn storage_balance_of(&self, account_id: ValidAccountId) -> Option<StorageBalance> {
        if let Some(account) = self.accounts.get(&account_id.into()) {
            Some(account.storage_balance())
        } else {
            None
        }
    }

    fn storage_deposit(
        &mut self,
        account_id: Option<ValidAccountId>,
        registration_only: Option<bool>,
    ) -> near_contract_standards::storage_management::StorageBalance {
        let registration_only = registration_only.unwrap_or(false);
        let account_id: AccountId = account_id
            .map(|a| a.into())
            .unwrap_or(env::predecessor_account_id());
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
            // NOTE: get_storage also registers the account id here
            let storage_cost = self.get_storage_cost(Some(account_id.clone()), false);
            let min_storage_cost = self.storage_balance_bounds().min.0;
            if amount_attached < storage_cost || amount_attached < min_storage_cost {
                self.accounts.remove(&account_id);
                Promise::new(env::predecessor_account_id()).transfer(amount_attached);
                StorageBalance {
                    available: 0.into(),
                    total: 0.into(),
                }
            } else if registration_only {
                let amount_refund = storage_cost - amount_attached;
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
