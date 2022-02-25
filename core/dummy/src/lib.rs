use near_account::{
    impl_near_accounts_plugin, AccountDeposits, AccountInfoTrait, Accounts, NewInfo,
};
use near_internal_balances_plugin::impl_near_balance_plugin;

use near_contract_standards::storage_management::StorageManagement;
use near_internal_balances_plugin::token_id::TokenId;
use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::collections::{LazyOption, UnorderedMap};
use near_sdk::json_types::U128;
use near_sdk::{
    assert_one_yocto, env, log, near_bindgen, AccountId, Balance, PanicOnDefault, PromiseOrValue,
};

#[derive(BorshDeserialize, BorshSerialize)]
pub struct AccountInfo {
    pub message: String,
    pub internal_balance: UnorderedMap<TokenId, Balance>,
}

impl NewInfo for AccountInfo {
    fn default_from_account_id(account_id: AccountId) -> Self {
        Self {
            message: "".to_string(),
            internal_balance: UnorderedMap::new(format!("{}-bal", account_id).as_bytes()),
        }
    }
}
#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct Contract {
    pub accounts: Accounts<AccountInfo>,
}

impl_near_accounts_plugin!(Contract, accounts, AccountInfo);
impl_near_balance_plugin!(Contract, accounts, AccountInfo, internal_balance);

#[near_bindgen]
impl Contract {
    /// Initializes the contract with the given total supply owned by the given `owner_id` with
    /// the given fungible token metadata.
    #[init]
    pub fn new() -> Self {
        Contract { accounts: Accounts::new() }
    }

    #[payable]
    pub fn write_message(&mut self, set_own_message: String) {
        assert_one_yocto();
        let caller = env::predecessor_account_id();
        let account = &mut self.accounts.get_account_checked(&caller);
        account.info.message = set_own_message;
        self.accounts.insert_account_check_storage(&caller, account);
    }

    pub fn get_message(&self, account_id: AccountId) -> String {
        let account = self.accounts.get_account(&account_id.into());
        account.map(|a| a.info.message).unwrap_or("".to_string())
    }
}
