use near_account::Accounts;
use near_sdk::{
    assert_one_yocto, env, ext_contract,
    json_types::U128,
    serde_json::{self, json},
    AccountId, Gas, Promise, ONE_YOCTO,
};

use crate::{
    core_impl::{
        get_internal_resolve_promise, internal_balance_increase, internal_balance_subtract,
        BalanceAccountInfoTrait,
    },
    token_id::TokenId,
    OnTransferOpts,
};

const GAS_FOR_NFT_TRANSFER_CALL_NEP171: Gas = Gas(25_000_000_000_000 + 3 * 5_000_000_000_000);

#[ext_contract(ext_nft)]
trait NFTContract {
    fn nft_transfer(
        &mut self,
        receiver_id: AccountId,
        token_id: String,
        memo: Option<String>,
    ) -> Promise;
}

pub fn nft_on_transfer<Info: BalanceAccountInfoTrait>(
    accounts: &mut Accounts<Info>,
    _sender_id: AccountId,
    previous_owner_id: AccountId,
    token_id: String,
    msg: String,
) -> bool {
    let opts: OnTransferOpts = if (&msg).len() == 0 {
        OnTransferOpts { sender_id: previous_owner_id.clone().into() }
    } else {
        serde_json::from_str(&msg)
            .unwrap_or_else(|e| panic!("Failed to deserialize transfer opts: {}", e))
    };
    let contract_id = env::predecessor_account_id();
    let token_id = TokenId::NFT { contract_id, token_id };
    let amount = 1;
    internal_balance_increase(accounts, &opts.sender_id, &token_id, amount);

    false
}

pub fn nft_internal_balance_withdraw_to<Info: BalanceAccountInfoTrait>(
    accounts: &mut Accounts<Info>,
    contract_id: AccountId,
    token_id: String,
    recipient: Option<AccountId>,
    msg: Option<String>,
) -> Promise {
    assert_one_yocto();
    let caller = env::predecessor_account_id();

    let recipient = recipient.unwrap_or(caller.clone());

    let prom =
        internal_nft_withdraw(accounts, &caller, contract_id, token_id, recipient, msg, None);
    prom
}

fn internal_nft_withdraw<Info: BalanceAccountInfoTrait>(
    accounts: &mut Accounts<Info>,
    sender: &AccountId,
    contract_id: AccountId,
    token_id: String,
    recipient: AccountId,
    memo: Option<String>,
    prior_promise: Option<Promise>,
) -> Promise {
    let internal_tok_id =
        TokenId::NFT { contract_id: contract_id.clone(), token_id: token_id.clone() };
    internal_balance_subtract(accounts, sender, &internal_tok_id, 1);

    let transfer_prom = ext_nft::nft_transfer(
        recipient.clone(),
        token_id.clone(),
        memo,
        contract_id.clone(),
        ONE_YOCTO,
        GAS_FOR_NFT_TRANSFER_CALL_NEP171,
    );
    let nft_transfer_prom = match prior_promise {
        None => transfer_prom,
        Some(prior_prom) => prior_prom.then(transfer_prom),
    };

    nft_transfer_prom.then(
        get_internal_resolve_promise(&sender, &internal_tok_id, U128::from(1), false).unwrap(),
    )
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    const INIT_ACCOUNT_BAL: u128 = 10_000;

    use std::convert::TryFrom;

    use crate::core_impl::internal_balance_get_balance;
    use crate::token_id::TokenId;
    use crate::utils::test_utils::{get_near_accounts, Info};
    use crate::BalanceInfo;

    use super::*;
    use near_account::{Account, NewInfo};
    use near_contract_standards::storage_management::StorageManagement;
    use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
    use near_sdk::collections::UnorderedMap;
    use near_sdk::test_utils::{accounts, VMContextBuilder};
    use near_sdk::MockedBlockchain;
    use near_sdk::{testing_env, Balance};

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
    fn test_nft_not_enough_balance() {
        let mut context = get_context(accounts(2));
        testing_env!(context.build());
        testing_env!(context.attached_deposit(1).build());
        let (account, tok, mut near_accounts, near_account, context) = get_near_accounts(context);
        nft_internal_balance_withdraw_to(&mut near_accounts, account, tok.to_string(), None, None);
    }

    #[test]
    fn test_nft_adding_balances_and_then_subtracting() {}

    #[test]
    fn test_nft_subtracting_balances() {
        let mut context = get_context(accounts(2));
        testing_env!(context.build());
        let (account, tok, mut near_accounts, near_account, context) = get_near_accounts(context);

        let tok_id_str = "tok".to_string();
        let tok_id = TokenId::NFT { contract_id: tok, token_id: tok_id_str.clone() };

        nft_on_transfer(
            &mut near_accounts,
            account.clone(),
            account.clone(),
            tok_id_str,
            "".to_string(),
        );
        let near_account = near_accounts.get_account_checked(&account);
        let bal = internal_balance_get_balance(&near_account, &tok_id);
        assert_eq!(bal, 1);

        internal_balance_subtract(&mut near_accounts, &account, &tok_id, 1);
        let near_account = near_accounts.get_account_checked(&account);
        let bal = internal_balance_get_balance(&near_account, &tok_id);
        assert_eq!(bal, 0);
    }

    #[test]
    fn test_on_transfer() {
        let mut context = get_context(accounts(2));
        testing_env!(context.build());
        let (account, tok, mut near_accounts, near_account, context) = get_near_accounts(context);

        let tok_id_str = "tok".to_string();
        let tok_id = TokenId::NFT { contract_id: tok.clone(), token_id: tok_id_str.clone() };

        let bal = internal_balance_get_balance(&near_account, &tok_id);
        assert_eq!(bal, 0);

        let success = nft_on_transfer(
            &mut near_accounts,
            account.clone(),
            account.clone(),
            tok_id_str.clone(),
            "".to_string(),
        );
        assert_eq!(success, true);

        let near_account = near_accounts.get_account_checked(&account);
        let bal = internal_balance_get_balance(&near_account, &tok_id);
        assert_eq!(bal, 1);
    }
}
