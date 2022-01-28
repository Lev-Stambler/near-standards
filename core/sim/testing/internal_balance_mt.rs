use std::convert::TryFrom;

use near_contract_standards::storage_management::{StorageBalance, StorageBalanceBounds};
use near_sdk::json_types::U128;
use near_sdk::serde::{self, Deserialize, Serialize};
use near_sdk::serde_json::json;
use near_sdk_sim::{call, to_yocto, transaction::ExecutionStatus, view, DEFAULT_GAS};

use near_internal_balances_plugin::TokenId;

use crate::testing::utils::{init_with_macros as init, register_user};
use crate::testing::DEFAULT_TOTAL_SUPPLY;

use super::{MT_FT_ID, MT_NFT_ID};

fn run_internal_balance_test(
    token_id: &str,
    amount_transfer: Option<u128>,
    sender_account_root: bool,
) {
    let (root, dummy, ft, nft, mt, alice) = init(DEFAULT_TOTAL_SUPPLY);
    let amount_transfer = amount_transfer.unwrap_or(1_000);
    let receiver = if sender_account_root { &root } else { &alice };

    let mt_ft_bal_root: U128 =
        view!(mt.balance_of(root.account_id(), token_id.to_string())).unwrap_json();

    let message = if sender_account_root {
        "".to_string()
    } else {
        json!({"sender_id": alice.account_id()}).to_string()
    };
    call!(
        root,
        mt.mt_transfer_call(
            dummy.account_id(),
            token_id.to_string(),
            amount_transfer.into(),
            None,
            message
        ),
        deposit = 1
    )
    .assert_success();

    let mt_id = TokenId::MT { contract_id: mt.account_id(), token_id: token_id.to_string() };
    let mt_bal_internal: U128 =
        view!(dummy.internal_balance_get_balance(receiver.account_id(), mt_id.clone()))
            .unwrap_json();

    let mt_bal_root_post_transfer: U128 =
        view!(mt.balance_of(root.account_id(), token_id.to_string())).unwrap_json();

    assert_eq!(mt_ft_bal_root.0 - mt_bal_root_post_transfer.0, amount_transfer);
    assert_eq!(mt_bal_internal.0, amount_transfer);

    let mt_bal_pre_withdraw: U128 =
        view!(mt.balance_of(receiver.account_id(), token_id.to_string())).unwrap_json();

    // Withdraw back into the callee's account
    call!(
        receiver,
        dummy.internal_balance_withdraw_to(amount_transfer.into(), mt_id.clone(), None, None),
        deposit = 1
    )
    .assert_success();

    let mt_bal_post_withdraw: U128 =
        view!(mt.balance_of(receiver.account_id(), token_id.to_string())).unwrap_json();
    assert_eq!(amount_transfer, mt_bal_post_withdraw.0 - mt_bal_pre_withdraw.0);
}

#[test]
fn simulate_mt_ft_simple_internal_balances_test() {
    run_internal_balance_test(MT_FT_ID, None, true);
}

#[test]
fn simulate_mt_nft_simple_internal_balances_test() {
    run_internal_balance_test(MT_NFT_ID, Some(1), true);
}

#[test]
fn simulate_mt_nft_simple_internal_balances_test_with_sender_id() {
    run_internal_balance_test(MT_NFT_ID, Some(1), true);
}

#[test]
fn simulate_mt_ft_simple_internal_balances_test_with_sender_id() {
    run_internal_balance_test(MT_FT_ID, None, false);
}
