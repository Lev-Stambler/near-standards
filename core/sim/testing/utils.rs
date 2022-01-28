use std::str::FromStr;

use dummy::ContractContract as DummyContract;
use ft::ContractContract as FTContract;
use multi_token::ContractContract as MTContract;
use multi_token_standard::TokenType;
use near_sdk::serde_json::json;
use near_sdk::{json_types::U128, AccountId};
use near_sdk_sim::{
    call, deploy, init_simulator, to_yocto, ContractAccount, UserAccount, DEFAULT_GAS,
    STORAGE_AMOUNT,
};
use nft::{ContractContract as NFTContract, DEFAULT_META};

// Load in contract bytes at runtime
near_sdk_sim::lazy_static_include::lazy_static_include_bytes! {
    DUMMY_BYTES => "res/dummy.wasm",
    FT_BYTES => "res/ft.wasm",
    NFT_BYTES => "res/nft.wasm",
    MT_BYTES => "res/multi_token.wasm",
}

pub const DUMMY_ID: &str = "dummy";
pub const FT_ID: &str = "ft";
pub const NFT_ID: &str = "nft";
pub const NFT_TOKEN_ID: &str = "nft1";
pub const MT_ID: &str = "mt";

pub const MT_NFT_ID: &str = "MT-nft";
pub const MT_FT_ID: &str = "MT-ft";

// Register the given `user` with FT contract
pub fn register_user(user: &near_sdk_sim::UserAccount, is_root: bool) {
    user.call(
        AccountId::from_str(DUMMY_ID).unwrap(),
        "accounts_storage_deposit",
        &json!({
            "registration_only": false,
        })
        .to_string()
        .into_bytes(),
        near_sdk_sim::DEFAULT_GAS / 2,
        near_sdk::env::storage_byte_cost() * 1_000,
    );

    if !is_root {
        user.call(
            AccountId::from_str(MT_ID).unwrap(),
            "storage_deposit",
            &json!({
                "account_id": user.account_id(),
                "token_ids": vec![MT_NFT_ID.to_string(), MT_FT_ID.to_string()],
                "registration_only": false,
            })
            .to_string()
            .into_bytes(),
            near_sdk_sim::DEFAULT_GAS / 2,
            near_sdk::env::storage_byte_cost() * 2_000,
        )
        .assert_success();
    }

    user.call(
        AccountId::from_str(FT_ID).unwrap(),
        "storage_deposit",
        &json!({
            "account_id": user.account_id(),
            "registration_only": false,
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
) -> (
    UserAccount,
    ContractAccount<DummyContract>,
    ContractAccount<FTContract>,
    ContractAccount<NFTContract>,
    ContractAccount<MTContract>,
    UserAccount,
) {
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

    let mt = deploy!(
        // Contract Proxy
        contract: MTContract,
        // Contract account id
        contract_id: MT_ID,
        // Bytes of contract
        bytes: &MT_BYTES,
        // User deploying the contract,
        signer_account: root,
        // init method
        init_method: new(root.account_id())
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
    let nft = deploy!(
        // Contract Proxy
        contract: NFTContract,
        // Contract account id
        contract_id: NFT_ID,
        // Bytes of contract
        bytes: &NFT_BYTES,
        // User deploying the contract,
        signer_account: root,
        // init method
        init_method: new_default_meta(root.account_id())
    );

    let alice = root.create_user(AccountId::from_str("alice").unwrap(), to_yocto("100"));

    // Create the mt ft
    call!(
        root,
        mt.mint(
            MT_FT_ID.to_string(),
            TokenType::Ft,
            Some(ft_total_supply.into()),
            root.account_id(),
            None
        ),
        deposit = near_sdk::env::storage_byte_cost() * 1_000
    )
    .assert_success();
    // Create the mt nft
    call!(
        root,
        mt.mint(MT_NFT_ID.to_string(), TokenType::Nft, None, root.account_id(), None),
        deposit = near_sdk::env::storage_byte_cost() * 1_000
    )
    .assert_success();

    // Create the mt ft
    call!(
        root,
        mt.mint(
            MT_FT_ID.to_string(),
            TokenType::Ft,
            Some(ft_total_supply.into()),
            root.account_id(),
            None
        ),
        deposit = near_sdk::env::storage_byte_cost() * 1_000
    )
    .assert_success();

    call!(
        root,
        nft.nft_mint(NFT_TOKEN_ID.to_string(), root.account_id(), DEFAULT_META),
        deposit = near_sdk::env::storage_byte_cost() * 1_000
    )
    .assert_success();

    register_user(&root, true);
    register_user(&alice, false);

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

    // Register the dummy contract with MT
    root.call(
        AccountId::from_str(MT_ID).unwrap(),
        "storage_deposit",
        &json!({
                "account_id": dummy.account_id(),
                "token_ids": vec![MT_NFT_ID.to_string(), MT_FT_ID.to_string()],
                "registration_only": false,
        })
        .to_string()
        .into_bytes(),
        near_sdk_sim::DEFAULT_GAS / 2,
        near_sdk::env::storage_byte_cost() * 325, // attached deposit
    )
    .assert_success();

    // Mint an NFT

    (root, dummy, ft, nft, mt, alice)
}
