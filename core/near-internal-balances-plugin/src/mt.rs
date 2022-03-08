use near_account::Accounts;
use near_sdk::{
    assert_one_yocto, env, ext_contract,
    json_types::U128,
    serde_json::{self, json},
    AccountId, Balance, Gas, Promise, PromiseOrValue, ONE_YOCTO,
};

use crate::{
    core_impl::{
        get_internal_resolve_promise, internal_balance_increase, internal_balance_subtract,
        BalanceAccountInfoTrait,
    },
    token_id::TokenId,
    OnTransferOpts,
};

#[ext_contract(ext_mt_receiver)]
pub trait MultiTokenContract {
    /// Returns true if token should be returned to `sender_id`
    fn mt_transfer(
        &mut self,
        receiver_id: AccountId,
        token_id: String,
        amount: U128,
        memo: Option<String>,
    ) -> PromiseOrValue<()>;
}

const GAS_FOR_MT_TRANSFER: Gas = Gas(5_000_000_000_000);

pub fn mt_on_transfer<Info: BalanceAccountInfoTrait>(
    accounts: &mut Accounts<Info>,
    sender_id: AccountId,
    token_ids: Vec<String>,
    amounts: Vec<U128>,
    msg: String,
) -> Vec<U128> {
    let opts: OnTransferOpts = if (&msg).len() == 0 {
        OnTransferOpts { sender_id: sender_id.clone().into() }
    } else {
        serde_json::from_str(&msg)
            .unwrap_or_else(|e| panic!("Failed to deserialize transfer opts: {}", e))
    };
    if token_ids.len() != amounts.len() {
        panic!("Expected the number of tokens to equal the number of amounts");
    }

    for i in 0..token_ids.len() {
        let token_id = TokenId::MT {
            contract_id: env::predecessor_account_id(),
            token_id: token_ids[i].clone(),
        };
        internal_balance_increase(accounts, &opts.sender_id, &token_id, amounts[i].0)
    }

    vec![0.into(); token_ids.len()]
}

pub fn mt_internal_balance_withdraw_to<Info: BalanceAccountInfoTrait>(
    accounts: &mut Accounts<Info>,
    amount: u128,
    contract_id: AccountId,
    token_id: String,
    recipient: Option<AccountId>,
    msg: Option<String>,
) -> Promise {
    assert_one_yocto();
    let caller = env::predecessor_account_id();

    let recipient = recipient.unwrap_or(caller.clone());

    let prom = internal_mt_withdraw(
        accounts,
        &caller,
        contract_id,
        token_id,
        recipient,
        amount,
        msg,
        None,
    );
    prom
}

fn internal_mt_withdraw<Info: BalanceAccountInfoTrait>(
    accounts: &mut Accounts<Info>,
    sender: &AccountId,
    contract_id: AccountId,
    token_id: String,
    recipient: AccountId,
    amount: u128,
    memo: Option<String>,
    prior_promise: Option<Promise>,
) -> Promise {
    let transfer_prom = ext_mt_receiver::mt_transfer(
        recipient,
        token_id.clone(),
        amount.into(),
        memo,
        contract_id.clone(),
        ONE_YOCTO,
        GAS_FOR_MT_TRANSFER,
    );

    let prom = match prior_promise {
        None => transfer_prom,
        Some(prior) => prior.then(transfer_prom),
    };

    let internal_token_id =
        TokenId::MT { contract_id: contract_id.clone(), token_id: token_id.clone() };

    internal_balance_subtract(accounts, sender, &internal_token_id, amount);

    prom.then(
        get_internal_resolve_promise(&sender, &internal_token_id, U128::from(amount), false)
            .unwrap(),
    )
}

/// Do an internal transfer and subtract the internal balance for {@param sender}
///
/// If there is a custom message, use that for the ft transfer. If not, use the default On Transfer Message
/********** Helper functions **************/

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    const INIT_ACCOUNT_BAL: u128 = 10_000;

    use std::convert::TryFrom;

    use crate::core_impl::internal_balance_get_balance;
    use crate::token_id::TokenId;
    use crate::utils::test_utils::{Info, get_near_accounts};
    use crate::BalanceInfo;

    use super::*;
    use near_account::{Account, NewInfo};
    use near_contract_standards::storage_management::StorageManagement;
    use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
    use near_sdk::collections::UnorderedMap;
    use near_sdk::test_utils::{accounts, VMContextBuilder};
    use near_sdk::testing_env;
    use near_sdk::MockedBlockchain;

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
    fn test_mt_not_enough_balance() {
        let mut context = get_context(accounts(2));
        testing_env!(context.build());
        testing_env!(context.attached_deposit(1).build());
        let (account, tok, mut near_accounts, near_account, context) = get_near_accounts(context);
        let contract_id = accounts(3);
        let mt_token_id = "my tok".to_string();
        mt_internal_balance_withdraw_to(
            &mut near_accounts,
            1_000,
            contract_id,
            mt_token_id,
            None,
            None,
        );
    }

    #[test]
    fn test_ft_adding_balances_and_then_subtracting() {}

    #[test]
    fn test_ft_subtracting_balances() {
        let mut context = get_context(accounts(2));
        testing_env!(context.build());
        let (account, tok, mut near_accounts, near_account, context) = get_near_accounts(context);

        let mt_token_id = "my tok".to_string();
        let tok_id = TokenId::MT { contract_id: tok.clone(), token_id: mt_token_id.clone() };

        let contract_id = accounts(3);

        mt_on_transfer(
            &mut near_accounts,
            account.clone(),
            vec![mt_token_id],
            vec![1_000.into()],
            "".to_string(),
        );
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

        let mt_token_id = "my tok".to_string();
        let tok_id = TokenId::MT { contract_id: tok.clone(), token_id: mt_token_id.clone() };
        let bal = internal_balance_get_balance(&near_account, &tok_id);
        assert_eq!(bal, 0);


        let amount_unused = mt_on_transfer(
            &mut near_accounts,
            account.clone(),
            vec![mt_token_id.clone()],
            vec![1000.into()],
            "".to_string(),
        );

        assert_eq!(amount_unused.len(), 1);
        assert_eq!(amount_unused[0].0, 0);

        let near_account = near_accounts.get_account_checked(&account);

        let bal = internal_balance_get_balance(&near_account, &tok_id);
        assert_eq!(bal, 1_000);

        let amount_unused = mt_on_transfer(
            &mut near_accounts,
            accounts(1).into(),
            vec![mt_token_id],
            vec![100.into()],
            serde_json::to_string(&OnTransferOpts { sender_id: account.clone() }).unwrap(),
        );

        assert_eq!(amount_unused.len(), 1);
        assert_eq!(amount_unused[0].0, 0);
        let near_account = near_accounts.get_account_checked(&account);
        let bal = internal_balance_get_balance(&near_account, &tok_id);
        assert_eq!(bal, 1_100);
    }
}
