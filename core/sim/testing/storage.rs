use std::convert::TryFrom;

use near_contract_standards::storage_management::{StorageBalance, StorageBalanceBounds};
use near_sdk::json_types::U128;
use near_sdk::serde::{self, Deserialize, Serialize};
use near_sdk::serde_json::json;
use near_sdk_sim::{call, to_yocto, transaction::ExecutionStatus, view, DEFAULT_GAS};

use near_internal_balances_plugin::TokenId;

use crate::testing::utils::{init_with_macros as init, register_user};
use crate::testing::DEFAULT_TOTAL_SUPPLY;

#[test]
/// deposit into the near accounts, withdraw some near, and unregister an account
fn simulate_deposit_storage() {
    let (root, dummy, _, _, _, alice) = init(DEFAULT_TOTAL_SUPPLY);

    let deposit_amount = 1_000_000;
    let prior_bal: StorageBalance =
        view!(dummy.storage_balance_of(alice.account_id())).unwrap_json();

    call!(alice, dummy.accounts_storage_deposit(None, None), deposit = deposit_amount)
        .assert_success();

    let bal_post_deposit: StorageBalance =
        view!(dummy.storage_balance_of(alice.account_id())).unwrap_json();
    assert_eq!(prior_bal.total.0, bal_post_deposit.total.0 - deposit_amount);
}
