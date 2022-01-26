use std::str::FromStr;

use crate::ft::ft_internal_balance_withdraw_to;
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

pub const RESOLVE_WITHDRAW_NAME: &str = "resolve_internal_ft_withdraw_call";
pub const GAS_BUFFER: Gas = Gas(5_000_000_000_000u64);
pub const GAS_FOR_INTERNAL_RESOLVE: Gas = Gas(5_000_000_000_000u64);

pub fn internal_balance_get_balance<Info: AccountInfoTrait>(
    account: &Account<Info>,
    token_id: &TokenId,
) -> u128 {
    account.info.get_balance(token_id)
}


/// Resolve the ft transfer by updating the amount used in the balances
/// `is_ft_call` - If false, assume that an ft_transfer occurred
/// @returns the amount used
pub fn resolve_internal_withdraw_call<Info: AccountInfoTrait>(
    accounts: &mut Accounts<Info>,
    account_id: &AccountId,
    token_id: TokenId,
    amount: U128,
    is_call: bool,
) -> U128 {
    let amount: u128 = amount.into();
    if amount == 0 {
        return U128(0);
    }
    // let account = accounts.get_account_checked(account_id);
    match near_sdk::utils::promise_result_as_success() {
        None => {
            log!("The FT transfer call failed, redepositing funds");
            increase_balance(accounts, account_id, &token_id, amount);
            U128(0)
        }
        Some(data) => {
            let amount_used = if is_call {
                let amount_used_str: String = serde_json::from_slice(data.as_slice())
                    .unwrap_or_else(|e| {
                        panic!("Failed to deserialize ft_transfer_call result {}", e)
                    });
                amount_used_str
                    .parse::<u128>()
                    .unwrap_or_else(|e| panic!("Failed to parse result with {}", e))
            } else {
                amount
            };
            // TODO: err handling?
            let amount_unused = amount - amount_used;
            log!("Amount unused {}", amount_unused);
            if amount_unused > 0 {
                increase_balance(accounts, account_id, &token_id, amount_unused);
            }
            U128(amount_used)
        }
    }
}

pub fn get_internal_resolve_data(
    sender: &AccountId,
    token_id: &TokenId,
    amount: U128,
    is_call: bool,
) -> Result<String, serde_json::error::Error> {
    let internal_resolve_args = json!({"account_id": sender, "token_id": token_id, "amount": amount, "is_call": is_call});
    Ok(internal_resolve_args.to_string())
}

/// Get the cost of adding 1 balance to a user's account
pub fn get_storage_cost_for_one_balance<Info: AccountInfoTrait>(
    accounts: &mut Accounts<Info>,
    token_id: TokenId,
) -> Balance {
    let account_id = AccountId::from_str(&"a".repeat(64)).unwrap();

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

pub fn internal_balance_withdraw_to<Info: AccountInfoTrait>(
    accounts: &mut Accounts<Info>,
    amount: u128,
    token_id: &TokenId,
    recipient: Option<AccountId>,
    msg: Option<String>,
) {
    assert_one_yocto();
    let caller = env::predecessor_account_id();
    match token_id {
        TokenId::FT { contract_id } => {
            ft_internal_balance_withdraw_to(accounts, amount, contract_id.clone(), recipient, msg)
        }
        TokenId::MT { contract_id, token_id } => {
            todo!()
        }
        TokenId::NFT { contract_id, token_id } => {
            todo!()
        }
    }
}

pub fn internal_balance_transfer<Info: AccountInfoTrait>(
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
    let current_balance = internal_balance_get_balance(&account, token_id);

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
    let current_balance = internal_balance_get_balance(&account, token);

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
