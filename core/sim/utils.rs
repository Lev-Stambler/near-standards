use std::str::FromStr;

use dummy::ContractContract as DummyContract;
use ft::ContractContract as FTContract;
use near_sdk::serde_json::json;
use near_sdk::{json_types::U128, AccountId};
use near_sdk_sim::{
    deploy, init_simulator, to_yocto, ContractAccount, UserAccount, DEFAULT_GAS, STORAGE_AMOUNT,
};

// Load in contract bytes at runtime
near_sdk_sim::lazy_static_include::lazy_static_include_bytes! {
    DUMMY_BYTES => "res/dummy.wasm",
    FT_BYTES => "res/ft.wasm",
}

const DUMMY_ID: &str = "dummy";
const FT_ID: &str = "ft";

// Register the given `user` with FT contract
pub fn register_user(user: &near_sdk_sim::UserAccount) {
    user.call(
        AccountId::from_str(DUMMY_ID).unwrap(),
        "accounts_storage_deposit",
        &json!({}).to_string().into_bytes(),
        near_sdk_sim::DEFAULT_GAS / 2,
        near_sdk::env::storage_byte_cost() * 1_000,
    );
    user.call(
        AccountId::from_str(FT_ID).unwrap(),
        "storage_deposit",
        &json!({
            "account_id": user.account_id()
        })
        .to_string()
        .into_bytes(),
        near_sdk_sim::DEFAULT_GAS / 2,
        near_sdk::env::storage_byte_cost() * 125, // attached deposit
    )
    .assert_success();
}

pub fn init_with_macros(
    ft_total_supply: u128,
) -> (UserAccount, ContractAccount<DummyContract>, ContractAccount<FTContract>, UserAccount) {
    let root = init_simulator(None);
    // uses default values for deposit and gas
    let dummy = deploy!(
        // Contract Proxy
        contract: DummyContract,
        // Contract account id
        contract_id: DUMMY_ID,
        // Bytes of contract
        bytes: &DUMMY_BYTES,
        // User deploying the contract,
        signer_account: root,
        // init method
        init_method: new()
    );

    let ft = deploy!(
        // Contract Proxy
        contract: FTContract,
        // Contract account id
        contract_id: FT_ID,
        // Bytes of contract
        bytes: &FT_BYTES,
        // User deploying the contract,
        signer_account: root,
        // init method
        init_method: new_default_meta(root.account_id(), ft_total_supply.into())
    );

    let alice = root.create_user(AccountId::from_str("alice").unwrap(), to_yocto("100"));
    register_user(&alice);
    register_user(&root);

    root.call(
        AccountId::from_str(FT_ID).unwrap(),
        "storage_deposit",
        &json!({
            "account_id": dummy.account_id()
        })
        .to_string()
        .into_bytes(),
        near_sdk_sim::DEFAULT_GAS / 2,
        near_sdk::env::storage_byte_cost() * 125, // attached deposit
    )
    .assert_success();

    (root, dummy, ft, alice)
}
