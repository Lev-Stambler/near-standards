#[macro_export]
macro_rules! impl_near_balance_plugin {
    ($contract_struct: ident, $accounts: ident, $info_struct: ident, $balance_map: ident) => {
        use $crate::{
            BalanceInfo, NearFTInternalBalance,
            SudoInternalBalanceHandlers,
        };

        pub use $crate::InternalBalanceHandlers;

        impl $crate::BalanceInfo for $info_struct {
            fn get_balance(&self, token_id: &$crate::token_id::TokenId) -> Balance {
                self.$balance_map.get(token_id).unwrap_or(0)
            }

            fn set_balance(&mut self, token_id: &$crate::token_id::TokenId, balance: Balance) {
                self.$balance_map.insert(token_id, &balance);
            }
        }

        impl $crate::core_impl::AccountInfoTrait for $info_struct {}

        impl SudoInternalBalanceHandlers for $contract_struct {
            fn subtract_balance(
                &mut self,
                account_id: &AccountId,
                token_id: &TokenId,
                amount: Balance,
            ) {
                $crate::core_impl::subtract_balance(
                    &mut self.$accounts,
                    account_id,
                    token_id,
                    amount,
                )
            }

            fn get_storage_cost_for_one_balance(&mut self, token_id: TokenId) -> Balance {
                $crate::core_impl::get_storage_cost_for_one_balance(&mut self.$accounts, token_id)
            }

            fn increase_balance(
                &mut self,
                account_id: &AccountId,
                token_id: &TokenId,
                amount: Balance,
            ) {
                $crate::core_impl::increase_balance(
                    &mut self.$accounts,
                    account_id,
                    token_id,
                    amount,
                )
            }

            fn get_balance_internal(&self, account_id: &AccountId, token_id: &TokenId) -> Balance {
                self.$accounts
                    .get_account(&account_id)
                    .map(|a| $crate::core_impl::internal_balance_get_balance(&a, &token_id))
                    .unwrap_or(0)
            }

            fn internal_balance_transfer_internal(
                &mut self,
                recipient: AccountId,
                token_id: TokenId,
                amount: u128,
                message: Option<String>,
            ) {
                $crate::core_impl::internal_balance_transfer(
                    &mut self.$accounts,
                    &recipient,
                    &token_id,
                    amount,
                    message,
                )
            }
        }

        #[near_bindgen]
        impl InternalBalanceHandlers for $contract_struct {
            fn ft_on_transfer(
                &mut self,
                sender_id: AccountId,
                amount: String,
                msg: String,
            ) -> String {
                $crate::ft::ft_on_transfer(&mut self.$accounts, sender_id, amount, msg)
            }

            fn nft_on_transfer(
                &mut self,
                sender_id: AccountId,
                previous_owner_id: AccountId,
                token_id: String,
                msg: String,
            ) -> bool {
                $crate::nft::nft_on_transfer(
                    &mut self.$accounts,
                    sender_id,
                    previous_owner_id,
                    token_id,
                    msg,
                )
            }

            fn internal_balance_get_balance(&self, account_id: AccountId, token_id: TokenId) -> U128 {
                let bal = self
                    .$accounts
                    .get_account(&account_id.into())
                    .map(|a| $crate::core_impl::internal_balance_get_balance(&a, &token_id.into()))
                    .unwrap_or(0);
                U128::from(bal)
            }

            /// A private contract function which resolves the ft transfer by updating the amount used in the balances
            /// @returns the amount used
            #[private]
            fn resolve_internal_withdraw_call(
                &mut self,
                account_id: AccountId,
                token_id: $crate::token_id::TokenId,
                amount: U128,
                is_ft_call: bool,
            ) -> U128 {
                $crate::core_impl::resolve_internal_withdraw_call(
                    &mut self.$accounts,
                    &account_id.into(),
                    token_id,
                    amount,
                    is_ft_call,
                )
            }

            #[payable]
            fn internal_balance_transfer(
                &mut self,
                recipient: AccountId,
                token_id: $crate::token_id::TokenId,
                amount: U128,
                message: Option<String>,
            ) {
                self.internal_balance_transfer_internal(recipient.into(), token_id, amount.into(), message)
            }

            #[payable]
            fn internal_balance_withdraw_to(
                &mut self,
                amount: U128,
                token_id: $crate::token_id::TokenId,
                recipient: Option<AccountId>,
                msg: Option<String>,
            ) {
                $crate::core_impl::internal_balance_withdraw_to(
                    &mut self.$accounts,
                    amount.into(),
                    &token_id,
                    recipient,
                    msg,
                )
            }
        }
        impl NearFTInternalBalance for $contract_struct {}
    };
}
