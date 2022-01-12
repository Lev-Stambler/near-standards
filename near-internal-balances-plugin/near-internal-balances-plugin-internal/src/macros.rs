#[macro_export]
macro_rules! impl_near_balance_plugin {
    ($contract_struct: ident, $accounts: ident, $info_struct: ident, $balance_map: ident) => {
        use $crate::{
            BalanceInfo, InternalBalanceFungibleTokenHandlers, SudoInternalBalanceFungibleToken, NearFTInternalBalance
        };

        impl BalanceInfo for $info_struct {
            fn get_balance(&self, token_id: &AccountId) -> Balance {
                // TODO: allow for custom balance field
                self.$balance_map.get(token_id).unwrap_or(0)
            }

            fn set_balance(&mut self, token_id: &AccountId, balance: Balance) {
                self.$balance_map.insert(token_id, &balance);
            }
        }

        impl SudoInternalBalanceFungibleToken for $contract_struct {
            fn subtract_balance(
                &mut self,
                account_id: &AccountId,
                token_id: &AccountId,
                amount: Balance,
            ) {
                $crate::core_impl::subtract_balance(
                    &mut self.$accounts,
                    account_id,
                    token_id,
                    amount,
                )
            }

            fn increase_balance(
                &mut self,
                account_id: &AccountId,
                token_id: &AccountId,
                amount: Balance,
            ) {
                $crate::core_impl::increase_balance(
                    &mut self.$accounts,
                    account_id,
                    token_id,
                    amount,
                )
            }

            fn get_ft_balance_internal(
                &self,
                account_id: &AccountId,
                token_id: &AccountId,
            ) -> Balance {
                self.$accounts
                    .get_account(&account_id)
                    .map(|a| $crate::core_impl::get_ft_balance(&a, &token_id))
                    .unwrap_or(0)
            }
        }

        #[near_bindgen]
        impl InternalBalanceFungibleTokenHandlers for $contract_struct {
            fn ft_on_transfer(&mut self, sender_id: String, amount: String, msg: String) -> String {
                $crate::core_impl::ft_on_transfer(&mut self.$accounts, sender_id, amount, msg)
            }

            fn get_ft_balance(&self, account_id: ValidAccountId, token_id: ValidAccountId) -> U128 {
                let bal = self
                    .$accounts
                    .get_account(&account_id.into())
                    .map(|a| $crate::core_impl::get_ft_balance(&a, &token_id.into()))
                    .unwrap_or(0);
                U128::from(bal)
            }

            /// A private contract function which resolves the ft transfer by updating the amount used in the balances
            /// @returns the amount used
            #[private]
            fn resolve_internal_ft_transfer_call(
                &mut self,
                account_id: ValidAccountId,
                token_id: ValidAccountId,
                amount: U128,
                is_ft_call: bool,
            ) -> U128 {
                $crate::core_impl::resolve_internal_ft_transfer_call(
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
                token_id: ValidAccountId,
                amount: U128,
                message: Option<String>,
            ) {
                $crate::core_impl::balance_transfer(
                    &mut self.$accounts,
                    &recipient.into(),
                    &token_id.into(),
                    amount.into(),
                    message,
                )
            }

            #[payable]
            fn withdraw_to(
                &mut self,
                amount: U128,
                token_id: ValidAccountId,
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
        impl NearFTInternalBalance for $contract_struct {
        }
    };
}
