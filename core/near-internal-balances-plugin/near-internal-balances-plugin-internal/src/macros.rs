#[macro_export]
macro_rules! impl_near_balance_plugin {
    ($contract_struct: ident, $accounts: ident, $info_struct: ident, $balance_map: ident) => {
        use $crate::{
            BalanceInfo, InternalBalanceHandlers, NearFTInternalBalance,
            SudoInternalBalanceHandlers,
        };

        impl $crate::BalanceInfo for $info_struct {
            fn get_balance(&self, token_id: &AccountId) -> Balance {
                self.$balance_map.get(token_id).unwrap_or(0)
            }

            fn set_balance(&mut self, token_id: &AccountId, balance: Balance) {
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

            fn get_storage_cost_for_one_balance(&mut self, token_id: &TokenId) -> Balance {
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
                    .map(|a| $crate::core_impl::get_balance(&a, &token_id))
                    .unwrap_or(0)
            }

            fn balance_transfer_internal(
                &mut self,
                recipient: AccountId,
                token_id: TokenId,
                amount: u128,
                message: Option<String>,
            ) {
                $crate::core_impl::balance_transfer(
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
            fn ft_on_transfer(&mut self, sender_id: String, amount: String, msg: String) -> String {
                $crate::core_impl::ft_on_transfer(&mut self.$accounts, sender_id, amount, msg)
            }

            fn get_balance(&self, account_id: ValidAccountId, token_id: ValidTokenId) -> U128 {
                let bal = self
                    .$accounts
                    .get_account(&account_id.into())
                    .map(|a| $crate::core_impl::get_balance(&a, &token_id.into()))
                    .unwrap_or(0);
                U128::from(bal)
            }

            /// A private contract function which resolves the ft transfer by updating the amount used in the balances
            /// @returns the amount used
            #[private]
            fn resolve_internal_ft_withdraw_call(
                &mut self,
                account_id: ValidAccountId,
                token_id: ValidAccountId,
                amount: U128,
                is_ft_call: bool,
            ) -> U128 {
                $crate::core_impl::resolve_internal_ft_withdraw_call(
                    &mut self.$accounts,
                    &account_id.into(),
                    token_id.into(),
                    amount,
                    is_ft_call,
                )
            }

            #[payable]
            fn balance_transfer(
                &mut self,
                recipient: ValidAccountId,
                token_id: ValidTokenId,
                amount: U128,
                message: Option<String>,
            ) {
                self.balance_transfer_internal(
                    recipient.into(),
                    token_id.into(),
                    amount.into(),
                    message,
                )
            }

            #[payable]
            fn withdraw_to(
                &mut self,
                amount: U128,
                token_id: ValidTokenId,
                recipient: Option<ValidAccountId>,
                msg: Option<String>,
            ) {
                $crate::core_impl::withdraw_to(
                    &mut self.$accounts,
                    amount.into(),
                    token_id.into(),
                    recipient.map(|r| r.into()),
                    msg,
                )
            }
        }
        impl NearFTInternalBalance for $contract_struct {}
    };
}
