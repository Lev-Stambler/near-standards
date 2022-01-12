use near_account::{Account, Accounts, NewInfo};
use near_sdk::{
    assert_one_yocto,
    borsh::{BorshDeserialize, BorshSerialize},
    env,
    json_types::U128,
    log,
    serde_json::{self, json},
    AccountId, Balance, Gas,
};

use crate::{BalanceInfo, OnTransferOpts};

const RESOLVE_FT_NAME: &str = "resolve_internal_ft_transfer_call";
const FT_TRANSFER_CALL_METHOD_NAME: &str = "ft_transfer_call";
const FT_TRANSFER_METHOD_NAME: &str = "ft_transfer";

const GAS_BUFFER: Gas = 5_000_000_000_000;
const GAS_FOR_INTERNAL_RESOLVE: Gas = 5_000_000_000_000;
const GAS_FOR_ON_TRANSFER_NEP141: Gas = 5_000_000_000_000;
const GAS_FOR_FT_RESOLVE_TRANSFER_NEP141: Gas = 5_000_000_000_000;
const GAS_FOR_FT_TRANSFER_CALL_NEP141: Gas = GAS_FOR_FT_RESOLVE_TRANSFER_NEP141
    + GAS_FOR_ON_TRANSFER_NEP141
    + 25_000_000_000_000
    + GAS_BUFFER;

pub fn ft_on_transfer<Info: BorshDeserialize + BorshSerialize + BalanceInfo + NewInfo>(
    accounts: &mut Accounts<Info>,
    sender_id: AccountId,
    amount: String,
    msg: String,
) -> String {
    let opts: OnTransferOpts = if (&msg).len() == 0 {
        OnTransferOpts { sender_id: sender_id.clone().into() }
    } else {
        serde_json::from_str(&msg)
            .unwrap_or_else(|e| panic!("Failed to deserialize transfer opts: {}", e))
    };
    let token_id = env::predecessor_account_id();
    let amount = amount.parse::<u128>().unwrap();
    increase_balance(accounts, &opts.sender_id, &token_id, amount);

    "0".to_string()
}

pub fn get_ft_balance<Info: BorshDeserialize + BorshSerialize + BalanceInfo + NewInfo>(
    account: &Account<Info>,
    token_id: &AccountId,
) -> u128 {
    account.info.get_balance(token_id)
}


pub fn balance_transfer<Info: BorshDeserialize + BorshSerialize + BalanceInfo + NewInfo>(
    accounts: &mut Accounts<Info>,
    recipient: &AccountId,
    token_id: &AccountId,
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
 

pub fn withdraw_to<Info: BorshDeserialize + BorshSerialize + BalanceInfo + NewInfo>(
    accounts: &mut Accounts<Info>,
    amount: u128,
    token_id: AccountId,
    recipient: Option<AccountId>,
    msg: Option<String>,
) {
    assert_one_yocto();
    let caller = env::predecessor_account_id();

    // TODO: in sep function
    assert_eq!(env::attached_deposit(), 1, "Expected an attached deposit of 1");

    let recipient = recipient.unwrap_or(caller.clone());

    let prom = internal_ft_transfer(accounts, &caller, &token_id, recipient, amount, msg, None);
    env::promise_return(prom);
}

fn internal_ft_transfer<Info: BorshDeserialize + BorshSerialize + BalanceInfo + NewInfo>(
    accounts: &mut Accounts<Info>,
    sender: &AccountId,
    token_id: &AccountId,
    recipient: AccountId,
    amount: u128,
    msg: Option<String>,
    prior_promise: Option<u64>,
) -> u64 {
    let data = get_transfer_data(recipient, U128::from(amount), sender.clone(), msg);

    // TODO: update
    subtract_balance(accounts, sender, token_id, amount);

    let ft_transfer_prom = match prior_promise {
        None => {
            let prom = env::promise_batch_create(token_id);
            env::promise_batch_action_function_call(
                prom,
                FT_TRANSFER_METHOD_NAME.as_bytes(),
                &data,
                1,
                GAS_FOR_FT_TRANSFER_CALL_NEP141,
            );
            prom
        }
        Some(prior_prom) => env::promise_then(
            prior_prom,
            token_id.to_string(),
            FT_TRANSFER_METHOD_NAME.as_bytes(),
            &data,
            1,
            GAS_FOR_FT_TRANSFER_CALL_NEP141,
        ),
    };
    let internal_resolve_args =
        get_internal_resolve_data(&sender, token_id, U128::from(amount), false).unwrap();
    env::promise_then(
        ft_transfer_prom,
        env::current_account_id(),
        RESOLVE_FT_NAME.as_bytes(),
        internal_resolve_args.to_string().as_bytes(),
        0,
        GAS_FOR_INTERNAL_RESOLVE,
    )
}

// TODO: integrate
fn internal_ft_transfer_call<Info: BorshDeserialize + BorshSerialize + BalanceInfo + NewInfo>(
    accounts: &mut Accounts<Info>,
    token_id: &AccountId,
    recipient: AccountId,
    amount: U128,
    sender: AccountId,
    prior_promise: Option<u64>,
) -> u64 {
    _internal_ft_transfer_call(
        accounts,
        token_id,
        recipient,
        amount,
        sender,
        prior_promise,
        None,
        1,
    )
}

/// Do an internal transfer and subtract the internal balance for {@param sender}
///
/// If there is a custom message, use that for the ft transfer. If not, use the default On Transfer Message
fn _internal_ft_transfer_call<Info: BorshDeserialize + BorshSerialize + BalanceInfo + NewInfo>(
    accounts: &mut Accounts<Info>,
    token_id: &AccountId,
    recipient: AccountId,
    amount: U128,
    sender: AccountId,
    prior_promise: Option<u64>,
    custom_message: Option<String>,
    amount_near: Balance,
) -> u64 {
    let data = get_transfer_call_data(recipient, amount.clone(), sender.clone(), custom_message);

    let amount_parsed = amount.0;

    subtract_balance(accounts, &sender, token_id, amount_parsed);

    let ft_transfer_prom = match prior_promise {
        None => {
            let prom_batch = env::promise_batch_create(token_id);
            env::promise_batch_action_function_call(
                prom_batch,
                FT_TRANSFER_CALL_METHOD_NAME.as_bytes(),
                &data,
                amount_near,
                GAS_FOR_FT_TRANSFER_CALL_NEP141,
            );
            prom_batch
        }
        Some(prior_prom) => env::promise_then(
            prior_prom,
            token_id.to_string(),
            FT_TRANSFER_CALL_METHOD_NAME.as_bytes(),
            &data,
            amount_near,
            GAS_FOR_FT_TRANSFER_CALL_NEP141,
        ),
    };
    let internal_resolve_args = get_internal_resolve_data(&sender, &token_id, amount, true).unwrap();
    env::promise_then(
        ft_transfer_prom,
        env::current_account_id(),
        RESOLVE_FT_NAME.as_bytes(),
        internal_resolve_args.to_string().as_bytes(),
        0,
        GAS_FOR_INTERNAL_RESOLVE,
    )
}

/// Resolve the ft transfer by updating the amount used in the balances
/// `is_ft_call` - If false, assume that an ft_transfer occurred
/// @returns the amount used
pub fn resolve_internal_ft_transfer_call<
    Info: BorshDeserialize + BorshSerialize + BalanceInfo + NewInfo,
>(
    accounts: &mut Accounts<Info>,
    account_id: &AccountId,
    token_id: AccountId,
    amount: U128,
    is_ft_call: bool,
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
            let amount_used = if is_ft_call {
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

pub fn increase_balance<Info: BorshDeserialize + BorshSerialize + BalanceInfo + NewInfo>(
    accounts: &mut Accounts<Info>,
    account_id: &AccountId,
    token_id: &AccountId,
    amount: u128,
) {
    let mut account = accounts.get_account_checked(account_id);
    let current_balance = get_ft_balance(&account, token_id);

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

pub fn subtract_balance<Info: BorshDeserialize + BorshSerialize + BalanceInfo + NewInfo>(
    accounts: &mut Accounts<Info>,
    account_id: &AccountId,
    token_id: &AccountId,
    amount: u128,
) {
    let mut account = accounts.get_account_checked(account_id);
    let current_balance = get_ft_balance(&account, token_id);

    if current_balance < amount {
        panic!("The callee did not deposit sufficient funds. Current balance: {}, requested amount {}, token {}", current_balance, amount, token_id);
    }

    log!(
        "Subtracting {} from {} for token {} with current balance {}",
        amount,
        account_id,
        token_id,
        current_balance
    );

    let updated = current_balance - amount;
    account.info.set_balance(token_id, updated);
    accounts.insert_account_check_storage(account_id, &mut account);
}

/********** Helper functions **************/
fn get_internal_resolve_data(
    sender: &AccountId,
    token_id: &AccountId,
    amount: U128,
    is_ft_call: bool,
) -> Result<String, serde_json::error::Error> {
    let internal_resolve_args = json!({"account_id": sender, "token_id": token_id, "amount": amount, "is_ft_call": is_ft_call});
    Ok(internal_resolve_args.to_string())
}

fn get_transfer_data(
    recipient: AccountId,
    amount: U128,
    sender: AccountId,
    custom_message: Option<String>,
) -> Vec<u8> {
    if let Some(msg) = custom_message {
        json!({"receiver_id": recipient, "amount": amount, "msg": msg}).to_string().into_bytes()
    } else {
        json!({"receiver_id": recipient, "amount": amount}).to_string().into_bytes()
    }
}

fn get_transfer_call_data(
    recipient: String,
    amount: U128,
    sender: String,
    custom_message: Option<String>,
) -> Vec<u8> {
    if let Some(msg) = custom_message {
        json!({ "receiver_id": recipient, "amount": amount, "msg": msg}).to_string().into_bytes()
    } else {
        let on_transfer_opts = OnTransferOpts { sender_id: sender };
        // TODO: unwrapping ok?
        json!({ "receiver_id": recipient, "amount": amount, "msg": serde_json::to_string(&on_transfer_opts).unwrap() })
					.to_string()
					.into_bytes()
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    const INIT_ACCOUNT_BAL: u128 = 10_000;

    use std::convert::TryFrom;

    use super::*;
    use near_account::NewInfo;
    use near_contract_standards::storage_management::StorageManagement;
    use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
    use near_sdk::collections::UnorderedMap;
    use near_sdk::json_types::ValidAccountId;
    use near_sdk::test_utils::{accounts, VMContextBuilder};
    use near_sdk::testing_env;
    use near_sdk::MockedBlockchain;

    #[derive(BorshSerialize, BorshDeserialize)]
    struct Info {
        pub internal_balance: UnorderedMap<AccountId, Balance>,
    }
    impl NewInfo for Info {
        fn default_from_account_id(account_id: AccountId) -> Self {
            Self {
                internal_balance: UnorderedMap::new((format!("{}-bals-i", &account_id)).as_bytes()),
            }
        }
    }

    impl BalanceInfo for Info {
        fn get_balance(&self, token_id: &AccountId) -> Balance {
            // TODO: allow for custom balance field
            self.internal_balance.get(token_id).unwrap_or(0)
        }

        fn set_balance(&mut self, token_id: &AccountId, balance: Balance) {
            self.internal_balance.insert(token_id, &balance);
        }
    }

    fn get_near_accounts(
        mut context: VMContextBuilder,
    ) -> (AccountId, AccountId, Accounts<Info>, Account<Info>, VMContextBuilder) {
        let mut near_accounts = Accounts::<Info>::new();
        let account: AccountId = accounts(0).into();
        let tok: AccountId = accounts(2).into();
        let min = near_accounts.storage_balance_bounds().min.0;
        testing_env!(context.attached_deposit(min * 10).build());
        near_accounts
            .storage_deposit(Some(ValidAccountId::try_from(account.clone()).unwrap()), None);
        testing_env!(context.attached_deposit(1).build());
        let near_account = near_accounts.get_account_checked(&account);

        (account, tok, near_accounts, near_account, context)
    }

    // mock the context for testing, notice "signer_account_id" that was accessed above from env::
    fn get_context(predecessor_account_id: ValidAccountId) -> VMContextBuilder {
        let mut builder = VMContextBuilder::new();
        builder
            .current_account_id(accounts(0))
            .signer_account_id(predecessor_account_id.clone())
            .predecessor_account_id(predecessor_account_id)
            .account_balance(INIT_ACCOUNT_BAL);
        builder
    }

    // TODO: register token's with deposits...
    // TODO: should panic type

    #[test]
    #[should_panic]
    fn test_ft_not_enough_balance() {
        let mut context = get_context(accounts(2));
        testing_env!(context.build());
        testing_env!(context.attached_deposit(1).build());
        let (account, tok, mut near_accounts, near_account, context) = get_near_accounts(context);
        withdraw_to(&mut near_accounts, 1_000, tok, None, None);
    }

    #[test]
    fn test_ft_adding_balances_and_then_subtracting() {}

    #[test]
    fn test_ft_subtracting_balances() {
        let mut context = get_context(accounts(2));
        testing_env!(context.build());
        let (account, tok, mut near_accounts, near_account, context) = get_near_accounts(context);

        ft_on_transfer(&mut near_accounts, account.clone(), 1000.to_string(), "".to_string());
        let near_account = near_accounts.get_account_checked(&account);
        let bal = get_ft_balance(&near_account, &tok);
        assert_eq!(bal, 1_000);

        subtract_balance(&mut near_accounts, &account, &tok, 100);
        let near_account = near_accounts.get_account_checked(&account);
        let bal = get_ft_balance(&near_account, &tok);
        assert_eq!(bal, 900);

        subtract_balance(&mut near_accounts, &account, &tok, 100);
        let near_account = near_accounts.get_account_checked(&account);
        let bal = get_ft_balance(&near_account, &tok);
        assert_eq!(bal, 800);
    }

    #[test]
    fn test_on_transfer() {
        let mut context = get_context(accounts(2));
        testing_env!(context.build());
        let (account, tok, mut near_accounts, near_account, context) = get_near_accounts(context);

        let bal = get_ft_balance(&near_account, &tok);
        assert_eq!(bal, 0);

        let amount_unused =
            ft_on_transfer(&mut near_accounts, account.clone(), 1000.to_string(), "".to_string());
        assert_eq!(amount_unused, "0");

        let near_account = near_accounts.get_account_checked(&account);
        let bal = get_ft_balance(&near_account, &tok);
        assert_eq!(bal, 1_000);

        let amount_unused = ft_on_transfer(
            &mut near_accounts,
            accounts(1).into(),
            100.to_string(),
            serde_json::to_string(&OnTransferOpts { sender_id: account.clone() }).unwrap(),
        );

        assert_eq!(amount_unused, "0");
        let near_account = near_accounts.get_account_checked(&account);
        let bal = get_ft_balance(&near_account, &tok);
        assert_eq!(bal, 1_100);
    }
}
