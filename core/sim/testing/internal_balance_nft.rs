use std::convert::TryFrom;

use near_contract_standards::non_fungible_token::Token;
use near_contract_standards::storage_management::{StorageBalance, StorageBalanceBounds};
use near_sdk::json_types::U128;
use near_sdk::serde::{self, Deserialize, Serialize};
use near_sdk::serde_json::json;
use near_sdk::AccountId;
use near_sdk_sim::{call, to_yocto, transaction::ExecutionStatus, view, DEFAULT_GAS};
use near_sdk_sim::{ContractAccount, UserAccount};

use near_internal_balances_plugin::TokenId;

use crate::testing::utils::{init_with_macros as init, register_user};
use crate::testing::{DEFAULT_TOTAL_SUPPLY, NFT_TOKEN_ID};

fn has_nft(nft: &ContractAccount<nft::ContractContract>, account: AccountId) -> bool {
    let bal: Vec<Token> = view!(nft.nft_tokens_for_owner(account, None, None)).unwrap_json();
    bal.iter().find(|t| t.token_id == NFT_TOKEN_ID).is_some()
}

#[test]
fn simulate_nft_simple_internal_balances_test_no_sender_id() {
    let (root, dummy, ft, nft, mt, alice) = init(DEFAULT_TOTAL_SUPPLY);
    let amount_transfer = 1;

    let ret = call!(
        root,
        nft.nft_transfer_call(
            dummy.account_id(),
            NFT_TOKEN_ID.to_string(),
            None,
            None,
            "".to_string()
        ),
        deposit = 1
    );

    ret.assert_success();

    let nft_id = TokenId::NFT { contract_id: nft.account_id(), token_id: NFT_TOKEN_ID.to_string() };
    let nft_bal_root_internal: U128 =
        view!(dummy.internal_balance_get_balance(root.account_id(), nft_id.clone())).unwrap_json();

    let root_has_nft = has_nft(&nft, root.account_id());
    assert_eq!(nft_bal_root_internal.0, amount_transfer);
    assert!(root_has_nft == false);

    // Withdraw back into the callee's account
    call!(
        root,
        dummy.internal_balance_withdraw_to(amount_transfer.into(), nft_id.clone(), None, None),
        deposit = 1
    )
    .assert_success();

    let root_has_nft = has_nft(&nft, root.account_id());
    assert_eq!(root_has_nft, true);
}

#[test]
fn simulate_nft_simple_internal_balances_test_with_sender_id() {
    let (root, dummy, ft, nft, mt, alice) = init(DEFAULT_TOTAL_SUPPLY);
    let amount_transfer = 1;

    call!(
        root,
        nft.nft_transfer_call(
            dummy.account_id(),
            NFT_TOKEN_ID.to_string(),
            None,
            None,
            json!({"sender_id": alice.account_id()}).to_string()
        ),
        deposit = 1
    )
    .assert_success();

    let nft_id = TokenId::NFT { contract_id: nft.account_id(), token_id: NFT_TOKEN_ID.to_string() };
    let nft_bal_alice_internal: U128 =
        view!(dummy.internal_balance_get_balance(alice.account_id(), nft_id.clone())).unwrap_json();

    let root_has_nft = has_nft(&nft, root.account_id());
    assert_eq!(nft_bal_alice_internal.0, amount_transfer);
    assert!(root_has_nft == false);

    // Withdraw back into the callee's account
    call!(
        alice,
        dummy.internal_balance_withdraw_to(amount_transfer.into(), nft_id.clone(), None, None),
        deposit = 1
    )
    .assert_success();

    let alice_has_nft = has_nft(&nft, alice.account_id());
    assert_eq!(alice_has_nft, true);
}
