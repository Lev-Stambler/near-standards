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

pub const DEFAULT_TOTAL_SUPPLY: u128 = 1_000_000_000_000;

#[test]
fn simulate_simple_storage_test() {
    let (_, dummy, ft, nft, mt, alice) = init(DEFAULT_TOTAL_SUPPLY);
    let storage_bal: StorageBalanceTmp =
        view!(dummy.accounts_storage_balance_of(alice.account_id())).unwrap_json();
    let init_free = storage_bal.available.0;

    call!(alice, dummy.write_message("AAAAA".to_string()), deposit = 1).assert_success();

    let storage_bal: StorageBalanceTmp =
        view!(dummy.accounts_storage_balance_of(alice.account_id())).unwrap_json();
    let post_free = storage_bal.available.0;

    let message: String = view!(dummy.get_message(alice.account_id())).unwrap_json();

    assert_eq!(init_free, post_free + 5 * near_sdk::env::storage_byte_cost());

    call!(alice, dummy.write_message("".to_string()), deposit = 1).assert_success();

    let storage_bal: StorageBalanceTmp =
        view!(dummy.accounts_storage_balance_of(alice.account_id())).unwrap_json();
    let final_free = storage_bal.available.0;
    assert_eq!(init_free, final_free);
}
