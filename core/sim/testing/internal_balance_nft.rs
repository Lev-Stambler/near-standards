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
fn simulate_nft_simple_internal_balances_test() {
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

    println!("{:?}", ret.logs());
    ret.assert_success();

    let nft_id = TokenId::NFT { contract_id: nft.account_id(), token_id: NFT_TOKEN_ID.to_string() };
    let nft_bal_root_internal: U128 =
        view!(dummy.internal_balance_get_balance(root.account_id(), nft_id.clone())).unwrap_json();

    let root_has_nft = has_nft(&nft, root.account_id());
    assert_eq!(nft_bal_root_internal.0, amount_transfer);
    assert!(root_has_nft == false);

    // Withdraw back into the callee's account
    call!(root, dummy.internal_balance_withdraw_to(amount_transfer.into(), nft_id.clone(), None, None), deposit = 1)
        .assert_success();

    let root_has_nft = has_nft(&nft, root.account_id());
    let ft_bal_root_post_withdraw: U128 = view!(ft.ft_balance_of(root.account_id())).unwrap_json();
    assert_eq!(root_has_nft, true);
    assert_eq!(ft_bal_root_post_withdraw.0, 0);
}
#[test]
fn simulate_simple_internal_balances_test_with_sender_id() {
    let (root, dummy, ft, nft, mt, alice) = init(DEFAULT_TOTAL_SUPPLY);
    let amount_transfer = 1_000;

    let ft_bal_root: U128 = view!(ft.ft_balance_of(root.account_id())).unwrap_json();
    let ft_id = TokenId::FT { contract_id: ft.account_id() };

    call!(
        root,
        ft.ft_transfer_call(
            dummy.account_id(),
            amount_transfer.into(),
            None,
            json!({"sender_id": alice.account_id()}).to_string()
        ),
        deposit = 1
    )
    .assert_success();

    let ft_bal_alice_internal: U128 =
        view!(dummy.internal_balance_get_balance(alice.account_id(), ft_id.clone())).unwrap_json();
    let ft_bal_root_post_transfer: U128 = view!(ft.ft_balance_of(root.account_id())).unwrap_json();

    assert_eq!(ft_bal_root.0 - ft_bal_root_post_transfer.0, amount_transfer);
    assert_eq!(ft_bal_alice_internal.0, amount_transfer);

    // Withdraw back into the callee's account
    call!(alice, dummy.internal_balance_withdraw_to(amount_transfer.into(), ft_id.clone(), None, None), deposit = 1)
        .assert_success();

    let ft_bal_alice_post_withdraw: U128 =
        view!(ft.ft_balance_of(alice.account_id())).unwrap_json();
    assert_eq!(amount_transfer, ft_bal_alice_post_withdraw.0);

    let ft_bal_alice_internal: U128 =
        view!(dummy.internal_balance_get_balance(alice.account_id(), ft_id.clone())).unwrap_json();
    assert_eq!(ft_bal_alice_internal.0, 0);
}
