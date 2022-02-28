#[macro_export]
macro_rules! impl_near_accounts_plugin {
    ($contract_struct: ident, $accounts: ident, $info_struct: ident) => {
        impl AccountInfoTrait for $info_struct {}
        use $crate::NearAccountPlugin;

        #[near_sdk::near_bindgen]
        impl $crate::NearAccountPlugin for $contract_struct {
            #[payable]
            fn accounts_storage_deposit(
                &mut self,
                account_id: Option<near_sdk::AccountId>,
                registration_only: Option<bool>,
            ) -> near_contract_standards::storage_management::StorageBalance {
                self.$accounts.storage_deposit(account_id, registration_only)
            }

            #[payable]
            fn accounts_storage_withdraw(
                &mut self,
                amount: Option<near_sdk::json_types::U128>,
            ) -> near_contract_standards::storage_management::StorageBalance {
                self.$accounts.storage_withdraw(amount)
            }

            #[payable]
            fn accounts_storage_unregister(&mut self, force: Option<bool>) -> bool {
                self.$accounts.storage_unregister(force)
            }

            fn accounts_storage_balance_bounds(
                &self,
            ) -> near_contract_standards::storage_management::StorageBalanceBounds {
                self.$accounts.storage_balance_bounds()
            }

            fn accounts_storage_balance_of(
                &self,
                account_id: near_sdk::AccountId,
            ) -> Option<near_contract_standards::storage_management::StorageBalance> {
                self.$accounts.storage_balance_of(account_id)
            }

            fn accounts_near_balance_of(
                &self,
                account_id: near_sdk::AccountId,
            ) -> Option<near_contract_standards::storage_management::StorageBalance> {
                self.$accounts.storage_balance_of(account_id)
            }
        }
    };
}
