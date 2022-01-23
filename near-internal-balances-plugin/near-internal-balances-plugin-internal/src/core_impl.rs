use near_account::{Account, AccountInfoTrait as DefaultAccountInfo, Accounts, NewInfo};
use near_sdk::{
    assert_one_yocto,
    borsh::{BorshDeserialize, BorshSerialize},
    env,
    json_types::U128,
    log,
    serde_json::{self, json},
    AccountId, Balance, Gas,
};

use crate::{token_id::TokenId, BalanceInfo, OnTransferOpts};

pub trait AccountInfoTrait: DefaultAccountInfo + BalanceInfo {}

pub const GAS_BUFFER: Gas = 5_000_000_000_000;
pub const GAS_FOR_INTERNAL_RESOLVE: Gas = 5_000_000_000_000;

pub fn get_internal_balance<Info: AccountInfoTrait>(
    account: &Account<Info>,
    token_id: &TokenId,
) -> u128 {
    account.info.get_balance(token_id)
}

/// Get the cost of adding 1 balance to a user's account
pub fn get_storage_cost_for_one_balance<Info: AccountInfoTrait>(
    accounts: &mut Accounts<Info>,
    token_id: TokenId,
) -> Balance {
    let account_id = "a".repeat(64);

    accounts.insert_account_unchecked(
        &account_id,
        &Account::default_from_account_id(account_id.clone()),
    );

    let storage_usage_init_with_account = env::storage_usage();

    let mut account = accounts.get_account(&account_id).unwrap();
    account.info.set_balance(&token_id, 0);
    accounts.insert_account_unchecked(&account_id, &account);

    let storage_usage = env::storage_usage();

    // Remove the inserted account
    accounts.remove_account_unchecked(&account_id);

    return (storage_usage - storage_usage_init_with_account) as u128 * env::storage_byte_cost();
}

pub fn balance_transfer<Info: AccountInfoTrait>(
    accounts: &mut Accounts<Info>,
    recipient: &AccountId,
    token_id: &TokenId,
    amount: u128,
    msg: Option<String>,
) {
    assert_one_yocto();
    let caller = env::predecessor_account_id();
    if let Some(msg) = msg {
        log!("Balance transfer message: {}", msg);
    }
    subtract_balance(accounts, &caller, token_id, amount);
    increase_balance(accounts, &recipient, token_id, amount);
}

pub fn increase_balance<Info: AccountInfoTrait>(
    accounts: &mut Accounts<Info>,
    account_id: &AccountId,
    token_id: &TokenId,
    amount: u128,
) {
    let mut account = accounts.get_account_checked(account_id);
    let current_balance = get_internal_balance(&account, token_id);

    log!(
        "Adding {} from {} for token {} with current balance {}",
        amount,
        account_id,
        token_id,
        current_balance
    );

    let updated = current_balance + amount;
    account.info.set_balance(token_id, updated);
    accounts.insert_account_check_storage(account_id, &mut account);
}

pub fn subtract_balance<Info: AccountInfoTrait>(
    accounts: &mut Accounts<Info>,
    account_id: &AccountId,
    token: &TokenId,
    amount: u128,
) {
    let mut account = accounts.get_account_checked(account_id);
    let current_balance = get_internal_balance(&account, token);

    if current_balance < amount {
        panic!("The callee did not deposit sufficient funds. Current balance: {}, requested amount {}, token {}", current_balance, amount, token);
    }

    log!(
        "Subtracting {} from {} for token {} with current balance {}",
        amount,
        account_id,
        token,
        current_balance
    );

    let updated = current_balance - amount;
    account.info.set_balance(token, updated);
    accounts.insert_account_check_storage(account_id, &mut account);
}

/********** Helper functions **************/

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {}
