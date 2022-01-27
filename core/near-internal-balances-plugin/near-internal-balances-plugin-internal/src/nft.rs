use near_account::Accounts;
use near_sdk::{
    assert_one_yocto, env,
    json_types::U128,
    log,
    serde_json::{self, json},
    AccountId, Balance, Gas,
};

use crate::{
    core_impl::{
        get_internal_resolve_data, increase_balance, subtract_balance, AccountInfoTrait,
        GAS_BUFFER, GAS_FOR_INTERNAL_RESOLVE, RESOLVE_WITHDRAW_NAME,
    },
    token_id::TokenId,
    OnTransferOpts,
};

const NFT_TRANSFER_CALL_METHOD_NAME: &str = "nft_transfer_call";
const NFT_TRANSFER_METHOD_NAME: &str = "nft_transfer";

// TODO: check over with contract
const GAS_FOR_NFT_RESOLVE_TRANSFER_NEP171: Gas = Gas(5_000_000_000_000);
const GAS_FOR_ON_TRANSFER_NEP171: Gas = Gas(5_000_000_000_000);
const GAS_FOR_NFT_TRANSFER_CALL_NEP171: Gas = Gas(25_000_000_000_000 + 3 * 5_000_000_000_000);

pub fn nft_on_transfer<Info: AccountInfoTrait>(
    accounts: &mut Accounts<Info>,
    _sender_id: AccountId,
    previous_owner_id: AccountId,
    token_id: String,
    msg: String,
) -> bool {
    log!("GOT HERE?");
    let opts: OnTransferOpts = if (&msg).len() == 0 {
        OnTransferOpts { sender_id: previous_owner_id.clone().into() }
    } else {
        serde_json::from_str(&msg)
            .unwrap_or_else(|e| panic!("Failed to deserialize transfer opts: {}", e))
    };
    let contract_id = env::predecessor_account_id();
    let token_id = TokenId::NFT { contract_id, token_id };
    let amount = 1;
    increase_balance(accounts, &opts.sender_id, &token_id, amount);

    false
}

pub fn nft_internal_balance_withdraw_to<Info: AccountInfoTrait>(
    accounts: &mut Accounts<Info>,
    contract_id: AccountId,
    token_id: String,
    recipient: Option<AccountId>,
    msg: Option<String>,
) {
    assert_one_yocto();
    let caller = env::predecessor_account_id();

    // TODO: in sep function
    assert_eq!(env::attached_deposit(), 1, "Expected an attached deposit of 1");

    let recipient = recipient.unwrap_or(caller.clone());

    let prom =
        internal_nft_withdraw(accounts, &caller, contract_id, token_id, recipient, msg, None);
    env::promise_return(prom);
}

fn internal_nft_withdraw<Info: AccountInfoTrait>(
    accounts: &mut Accounts<Info>,
    sender: &AccountId,
    contract_id: AccountId,
    token_id: String,
    recipient: AccountId,
    msg: Option<String>,
    prior_promise: Option<u64>,
) -> u64 {
    let data = get_transfer_data(recipient, &token_id, msg);

    let internal_tok_id =
        TokenId::NFT { contract_id: contract_id.clone(), token_id: token_id.clone() };
    subtract_balance(accounts, sender, &internal_tok_id, 1);

    let ft_transfer_prom = match prior_promise {
        None => {
            let prom = env::promise_batch_create(&contract_id);
            env::promise_batch_action_function_call(
                prom,
                NFT_TRANSFER_METHOD_NAME,
                &data,
                1,
                GAS_FOR_NFT_TRANSFER_CALL_NEP171,
            );
            prom
        }
        Some(prior_prom) => env::promise_then(
            prior_prom,
            contract_id.clone(),
            NFT_TRANSFER_METHOD_NAME,
            &data,
            1,
            GAS_FOR_NFT_TRANSFER_CALL_NEP171,
        ),
    };
    let internal_resolve_args =
        get_internal_resolve_data(&sender, &internal_tok_id, U128::from(1), false).unwrap();
    env::promise_then(
        ft_transfer_prom,
        env::current_account_id(),
        RESOLVE_WITHDRAW_NAME,
        internal_resolve_args.to_string().as_bytes(),
        0,
        GAS_FOR_INTERNAL_RESOLVE,
    )
}

/********** Helper functions **************/

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    const INIT_ACCOUNT_BAL: u128 = 10_000;

    use std::convert::TryFrom;

    use crate::core_impl::internal_balance_get_balance;
    use crate::token_id::TokenId;
    use crate::BalanceInfo;

    use super::*;
    use near_account::{Account, NewInfo};
    use near_contract_standards::storage_management::StorageManagement;
    use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
    use near_sdk::collections::UnorderedMap;
    use near_sdk::test_utils::{accounts, VMContextBuilder};
    use near_sdk::testing_env;
    use near_sdk::MockedBlockchain;

    #[derive(BorshSerialize, BorshDeserialize)]
    struct Info {
        pub internal_balance: UnorderedMap<TokenId, Balance>,
    }

    impl NewInfo for Info {
        fn default_from_account_id(account_id: AccountId) -> Self {
            Self {
                internal_balance: UnorderedMap::new((format!("{}-bals-i", &account_id)).as_bytes()),
            }
        }
    }

    impl BalanceInfo for Info {
        fn get_balance(&self, token_id: &TokenId) -> Balance {
            // TODO: allow for custom balance field
            self.internal_balance.get(token_id).unwrap_or(0)
        }

        fn set_balance(&mut self, token_id: &TokenId, balance: Balance) {
            self.internal_balance.insert(token_id, &balance);
        }
    }

    impl near_account::AccountInfoTrait for Info {}
    impl AccountInfoTrait for Info {}

    // TODO: common utils testing
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

    // TODO: register token's with deposits...
    // TODO: should panic type

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

        subtract_balance(&mut near_accounts, &account, &tok_id, 1);
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
        // TODO: should we limit the tok bal to 1 for NFT?
    }
}

fn get_transfer_data(
    recipient: AccountId,
    token_id: &String,
    custom_message: Option<String>,
) -> Vec<u8> {
    if let Some(msg) = custom_message {
        json!({"receiver_id": recipient, "token_id": token_id, "msg": msg}).to_string().into_bytes()
    } else {
        json!({"receiver_id": recipient, "token_id": token_id}).to_string().into_bytes()
    }
}
