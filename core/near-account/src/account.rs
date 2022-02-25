use near_contract_standards::storage_management::StorageBalance;
use near_sdk::{
    borsh::{self, BorshDeserialize, BorshSerialize},
    env,
    json_types::U128,
    AccountId, Balance, Promise, StorageUsage,
};

use crate::{Accounts, NewInfo};

/// The `Info` struct must implement this trait which consists of the composition of
/// BorshSerialize, BorshDeserialize, and NewInfo
pub trait AccountInfoTrait: BorshSerialize + BorshDeserialize + NewInfo {}

/// Account information and storage cost.
#[derive(BorshSerialize, BorshDeserialize, Default, Debug)]
pub struct Account<Info: AccountInfoTrait> {
    /// Native NEAR amount sent to the contract
    pub near_amount: Balance,
    pub near_used_for_storage: Balance,
    pub info: Info,
}

impl<Info: AccountInfoTrait> NewInfo for Account<Info> {
    fn default_from_account_id(account_id: AccountId) -> Self {
        Self {
            near_amount: 0,
            near_used_for_storage: 0,
            info: Info::default_from_account_id(account_id),
        }
    }
}

/// Functions for dealing with Near Deposits for an individual account
pub trait AccountDeposits<Info: AccountInfoTrait> {
    /// Check that storage is paid for and call the closure function
    fn check_storage<F, T: Sized>(&mut self, accounts: &mut Accounts<Info>, closure: F) -> T
    where
        F: FnOnce(&mut Accounts<Info>, &mut Self) -> T;

    fn get_available_near(&self) -> Balance;

    fn storage_balance(&self) -> StorageBalance;

    fn near_deposit(&mut self) -> StorageBalance;

    fn near_balance(&self) -> Option<StorageBalance>;

    fn near_withdraw(&mut self, account_id: AccountId, amount: Option<u128>) -> StorageBalance;
}

impl<Info: AccountInfoTrait> AccountDeposits<Info> for Account<Info> {
    fn check_storage<F, T: Sized>(&mut self, accounts: &mut Accounts<Info>, closure: F) -> T
    where
        F: FnOnce(&mut Accounts<Info>, &mut Self) -> T,
    {
        let storage_start = env::storage_usage();

        let ret = closure(accounts, self);

        let storage_end = env::storage_usage();

        if storage_end == storage_start {
            ret
        } else if storage_end > storage_start {
            let storage_cost = (storage_end - storage_start) as u128 * env::storage_byte_cost();
            let free_near = self.get_available_near();
            if free_near < storage_cost {
                panic!("Not enough Near to cover the transaction");
            }
            self.near_used_for_storage += storage_cost;
            ret
        } else {
            let storage_refund = (storage_start - storage_end) as u128 * env::storage_byte_cost();
            self.near_used_for_storage =
                self.near_used_for_storage.checked_sub(storage_refund).unwrap_or(0);
            ret
        }
    }

    fn storage_balance(&self) -> StorageBalance {
        StorageBalance {
            total: U128::from(self.near_amount),
            available: U128::from(self.get_available_near()),
        }
    }

    fn get_available_near(&self) -> Balance {
        let free_near = self.near_amount - self.near_used_for_storage;
        free_near
    }

    fn near_deposit(&mut self) -> StorageBalance {
        let amount = env::attached_deposit();
        self.near_amount += amount;
        self.near_balance().unwrap()
    }

    fn near_withdraw(&mut self, receiver_id: AccountId, amount: Option<u128>) -> StorageBalance {
        let free = self.get_available_near();
        let withdraw_amount = if amount.is_some() {
            let amount = amount.unwrap();
            if free < amount {
                panic!("Cannot withdraw more than {} near", free);
            }
            amount
        } else {
            free
        };
        self.near_amount -= withdraw_amount;
        Promise::new(receiver_id).transfer(withdraw_amount);
        self.near_balance().unwrap()
    }

    fn near_balance(&self) -> Option<StorageBalance> {
        Some(StorageBalance {
            total: U128::from(self.near_used_for_storage),
            available: U128::from(self.get_available_near()),
        })
    }
}
