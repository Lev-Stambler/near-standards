use std::convert::TryFrom;

use near_contract_standards::storage_management::{StorageBalance, StorageBalanceBounds};
use near_sdk::json_types::U128;
use near_sdk::serde::{self, Deserialize, Serialize};
use near_sdk::serde_json::json;
use near_sdk_sim::{call, to_yocto, transaction::ExecutionStatus, view, DEFAULT_GAS};

use near_internal_balances_plugin::TokenId;

use crate::testing::utils::{init_with_macros as init, register_user};

#[derive(Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub struct StorageBalanceTmp {
    pub total: U128,
    pub available: U128,
}

const DEFAULT_TOTAL_SUPPLY: u128 = 1_000_000_000_000;

#[test]
fn simulate_simple_storage_test() {
    let (_, dummy, ft, alice) = init(DEFAULT_TOTAL_SUPPLY);
    let storage_bal: StorageBalanceTmp =
        view!(dummy.accounts_storage_balance_of(alice.account_id())).unwrap_json();
    let init_free = storage_bal.available.0;

    call!(alice, dummy.write_message("AAAAA".to_string()), deposit = 1).assert_success();

    let storage_bal: StorageBalanceTmp =
        view!(dummy.accounts_storage_balance_of(alice.account_id())).unwrap_json();
    let post_free = storage_bal.available.0;

    let message: String = view!(dummy.get_message(alice.account_id())).unwrap_json();
    assert_eq!("AAAAA", message);

    assert_eq!(init_free, post_free + 5 * near_sdk::env::storage_byte_cost());

    call!(alice, dummy.write_message("".to_string()), deposit = 1).assert_success();

    let storage_bal: StorageBalanceTmp =
        view!(dummy.accounts_storage_balance_of(alice.account_id())).unwrap_json();
    let final_free = storage_bal.available.0;
    assert_eq!(init_free, final_free);
}

#[test]
fn simulate_simple_internal_balances_test() {
    let (root, dummy, ft, alice) = init(DEFAULT_TOTAL_SUPPLY);
    let amount_transfer = 1_000;

    let ft_bal_root: U128 = view!(ft.ft_balance_of(root.account_id())).unwrap_json();

    call!(
        root,
        ft.ft_transfer_call(dummy.account_id(), amount_transfer.into(), None, "".to_string()),
        deposit = 1
    )
    .assert_success();

    let ft_id = TokenId::FT { contract_id: ft.account_id() };
    let ft_bal_root_internal: U128 =
        view!(dummy.get_internal_balance(root.account_id(), ft_id.clone())).unwrap_json();
    let ft_bal_root_post_transfer: U128 = view!(ft.ft_balance_of(root.account_id())).unwrap_json();

    assert_eq!(ft_bal_root.0 - ft_bal_root_post_transfer.0, amount_transfer);
    assert_eq!(ft_bal_root_internal.0, amount_transfer);

    // Withdraw back into the callee's account
    call!(root, dummy.withdraw_to(amount_transfer.into(), ft_id.clone(), None, None), deposit = 1)
        .assert_success();

    let ft_bal_root_post_withdraw: U128 = view!(ft.ft_balance_of(root.account_id())).unwrap_json();
    assert_eq!(ft_bal_root.0, ft_bal_root_post_withdraw.0);
}
// TODO: sim specific deposit to
#[test]
fn simulate_simple_internal_balances_test_with_sender_id() {
    let (root, dummy, ft, alice) = init(DEFAULT_TOTAL_SUPPLY);
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
        view!(dummy.get_internal_balance(alice.account_id(), ft_id.clone())).unwrap_json();
    let ft_bal_root_post_transfer: U128 = view!(ft.ft_balance_of(root.account_id())).unwrap_json();

    assert_eq!(ft_bal_root.0 - ft_bal_root_post_transfer.0, amount_transfer);
    assert_eq!(ft_bal_alice_internal.0, amount_transfer);

    // Withdraw back into the callee's account
    call!(alice, dummy.withdraw_to(amount_transfer.into(), ft_id.clone(), None, None), deposit = 1)
        .assert_success();

    let ft_bal_alice_post_withdraw: U128 =
        view!(ft.ft_balance_of(alice.account_id())).unwrap_json();
    assert_eq!(amount_transfer, ft_bal_alice_post_withdraw.0);

    let ft_bal_alice_internal: U128 =
        view!(dummy.get_internal_balance(alice.account_id(), ft_id.clone())).unwrap_json();
    assert_eq!(ft_bal_alice_internal.0, 0);
}
// TODO: sim specificdeposit to
