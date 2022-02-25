use near_account::Accounts;
use near_sdk::{
    assert_one_yocto, env, ext_contract,
    json_types::U128,
    log,
    serde_json::{self, json},
    AccountId, Balance, Gas, Promise, ONE_YOCTO,
};

use crate::{
    core_impl::{
        get_internal_resolve_promise, internal_balance_increase, internal_balance_subtract,
        AccountInfoTrait,
    },
    token_id::TokenId,
    OnTransferOpts,
};

#[ext_contract(ext_ft)]
trait FTContract {
    fn ft_transfer(
        &mut self,
        receiver_id: AccountId,
        amount: U128,
        memo: Option<String>,
    ) -> Promise;
}

const GAS_FOR_FT_TRANSFER_CALL_NEP141: Gas = Gas(25_000_000_000_000 + 3 * 5_000_000_000_000);

pub fn ft_on_transfer<Info: AccountInfoTrait>(
    accounts: &mut Accounts<Info>,
    sender_id: AccountId,
    amount: String,
    msg: String,
) -> String {
    log!("GOT HERE?");
    let opts: OnTransferOpts = if (&msg).len() == 0 {
        OnTransferOpts { sender_id: sender_id.clone().into() }
    } else {
        serde_json::from_str(&msg)
            .unwrap_or_else(|e| panic!("Failed to deserialize transfer opts: {}", e))
    };
    let token_id = env::predecessor_account_id();
    let amount = amount.parse::<u128>().unwrap();
    internal_balance_increase(
        accounts,
        &opts.sender_id,
        &TokenId::FT { contract_id: token_id },
        amount,
    );

    "0".to_string()
}

pub fn ft_internal_balance_withdraw_to<Info: AccountInfoTrait>(
    accounts: &mut Accounts<Info>,
    amount: u128,
    token_id: AccountId,
    recipient: Option<AccountId>,
    msg: Option<String>,
) -> Promise {
    assert_one_yocto();
    let caller = env::predecessor_account_id();

    let recipient = recipient.unwrap_or(caller.clone());

    let prom = internal_ft_withdraw(accounts, &caller, &token_id, recipient, amount, msg, None);
    prom
}

fn internal_ft_withdraw<Info: AccountInfoTrait>(
    accounts: &mut Accounts<Info>,
    sender: &AccountId,
    contract_id: &AccountId,
    recipient: AccountId,
    amount: u128,
    memo: Option<String>,
    prior_promise: Option<Promise>,
) -> Promise {
    internal_balance_subtract(
        accounts,
        sender,
        &TokenId::FT { contract_id: contract_id.clone() },
        amount,
    );

    let transfer_prom = ext_ft::ft_transfer(
        recipient.clone(),
        amount.into(),
        memo,
        contract_id.clone(),
        ONE_YOCTO,
        GAS_FOR_FT_TRANSFER_CALL_NEP141,
    );

    let ft_transfer_prom = match prior_promise {
        None => transfer_prom,
        Some(prior_prom) => prior_prom.then(transfer_prom),
    };

    ft_transfer_prom.then(
        get_internal_resolve_promise(
            &sender,
            &TokenId::FT { contract_id: contract_id.clone() },
            U128::from(amount),
            false,
        )
        .unwrap(),
    )
}

/********** Helper functions **************/

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    const INIT_ACCOUNT_BAL: u128 = 10_000;

    use std::convert::TryFrom;

    use crate::core_impl::internal_balance_get_balance;
    use crate::token_id::TokenId;
    use crate::utils::test_utils::Info;
    use crate::BalanceInfo;

    use super::*;
    use near_account::{Account, NewInfo};
    use near_contract_standards::storage_management::StorageManagement;
    use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
    use near_sdk::collections::UnorderedMap;
    use near_sdk::test_utils::{accounts, VMContextBuilder};
    use near_sdk::testing_env;
    use near_sdk::MockedBlockchain;

    impl near_account::AccountInfoTrait for Info {}
    impl AccountInfoTrait for Info {}

    fn get_near_accounts(
        mut context: VMContextBuilder,
    ) -> (AccountId, AccountId, Accounts<Info>, Account<Info>, VMContextBuilder) {
        let mut near_accounts = Accounts::<Info>::new();
        let account: AccountId = accounts(0).into();
        let tok: AccountId = accounts(2).into();
        let min = near_accounts.storage_balance_bounds().min.0;
        testing_env!(context.attached_deposit(min * 10).build());
        near_accounts.storage_deposit(Some(AccountId::try_from(account.clone()).unwrap()), None);
        testing_env!(context.attached_deposit(1).build());
        let near_account = near_accounts.get_account_checked(&account);

        (account, tok, near_accounts, near_account, context)
    }

    // mock the context for testing, notice "signer_account_id" that was accessed above from env::
    fn get_context(predecessor_account_id: AccountId) -> VMContextBuilder {
        let mut builder = VMContextBuilder::new();
        builder
            .current_account_id(accounts(0))
            .signer_account_id(predecessor_account_id.clone())
            .predecessor_account_id(predecessor_account_id)
            .account_balance(INIT_ACCOUNT_BAL);
        builder
    }

    #[test]
    #[should_panic]
    fn test_ft_not_enough_balance() {
        let mut context = get_context(accounts(2));
        testing_env!(context.build());
        testing_env!(context.attached_deposit(1).build());
        let (account, tok, mut near_accounts, near_account, context) = get_near_accounts(context);
        ft_internal_balance_withdraw_to(&mut near_accounts, 1_000, tok, None, None);
    }

    #[test]
    fn test_ft_adding_balances_and_then_subtracting() {}

    #[test]
    fn test_ft_subtracting_balances() {
        let mut context = get_context(accounts(2));
        testing_env!(context.build());
        let (account, tok, mut near_accounts, near_account, context) = get_near_accounts(context);

        let tok_id = TokenId::FT { contract_id: tok.clone() };

        ft_on_transfer(&mut near_accounts, account.clone(), 1000.to_string(), "".to_string());
        let near_account = near_accounts.get_account_checked(&account);
        let bal = internal_balance_get_balance(&near_account, &tok_id);
        assert_eq!(bal, 1_000);

        internal_balance_subtract(&mut near_accounts, &account, &tok_id, 100);
        let near_account = near_accounts.get_account_checked(&account);
        let bal = internal_balance_get_balance(&near_account, &tok_id);
        assert_eq!(bal, 900);

        internal_balance_subtract(&mut near_accounts, &account, &tok_id, 100);
        let near_account = near_accounts.get_account_checked(&account);
        let bal = internal_balance_get_balance(&near_account, &tok_id);
        assert_eq!(bal, 800);
    }

    #[test]
    fn test_on_transfer() {
        let mut context = get_context(accounts(2));
        testing_env!(context.build());
        let (account, tok, mut near_accounts, near_account, context) = get_near_accounts(context);

        let tok_id = TokenId::FT { contract_id: tok.clone() };
        let bal = internal_balance_get_balance(&near_account, &tok_id);
        assert_eq!(bal, 0);

        let amount_unused =
            ft_on_transfer(&mut near_accounts, account.clone(), 1000.to_string(), "".to_string());
        assert_eq!(amount_unused, "0");

        let near_account = near_accounts.get_account_checked(&account);
        let tok_id = TokenId::FT { contract_id: tok.clone() };
        let bal = internal_balance_get_balance(&near_account, &tok_id);
        assert_eq!(bal, 1_000);

        let amount_unused = ft_on_transfer(
            &mut near_accounts,
            accounts(1).into(),
            100.to_string(),
            serde_json::to_string(&OnTransferOpts { sender_id: account.clone() }).unwrap(),
        );

        assert_eq!(amount_unused, "0");
        let near_account = near_accounts.get_account_checked(&account);
        let tok_id = TokenId::FT { contract_id: tok.clone() };
        let bal = internal_balance_get_balance(&near_account, &tok_id);
        assert_eq!(bal, 1_100);
    }
}

// fn get_transfer_data(
//     recipient: AccountId,
//     amount: U128,
//     sender: AccountId,
//     custom_message: Option<String>,
// ) -> Vec<u8> {
//     if let Some(msg) = custom_message {
//         json!({"receiver_id": recipient, "amount": amount, "msg": msg}).to_string().into_bytes()
//     } else {
//         json!({"receiver_id": recipient, "amount": amount}).to_string().into_bytes()
//     }
// }
