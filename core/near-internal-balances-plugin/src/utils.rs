#[cfg(test)]
pub mod test_utils {
    use std::convert::TryFrom;

    use near_account::{NewInfo, Accounts, Account};
    use near_contract_standards::storage_management::StorageManagement;
    use near_sdk::{
        borsh::{self, BorshDeserialize, BorshSerialize},
        collections::UnorderedMap,
        testing_env, AccountId, Balance, test_utils::{VMContextBuilder, accounts},
    };

    use crate::{core_impl::AccountInfoTrait, BalanceInfo, TokenId};

    #[derive(BorshSerialize, BorshDeserialize)]
    pub struct Info {
        pub internal_balance: UnorderedMap<TokenId, Balance>,
    }

    impl NewInfo for Info {
        fn default_from_account_id(account_id: AccountId) -> Self {
            Self {
                internal_balance: UnorderedMap::new((format!("{}-bals-i", &account_id)).as_bytes()),
            }
        }
    }

    impl BalanceInfo for Info {
        fn get_balance(&self, token_id: &TokenId) -> Balance {
            self.internal_balance.get(token_id).unwrap_or(0)
        }

        fn set_balance(&mut self, token_id: &TokenId, balance: Balance) {
            self.internal_balance.insert(token_id, &balance);
        }

        fn get_all_tokens(&self) -> Vec<TokenId> {
            self.internal_balance.keys().collect()
        }
    }

    pub fn get_near_accounts(
        mut context: VMContextBuilder,
    ) -> (AccountId, AccountId, Accounts<Info>, Account<Info>, VMContextBuilder) {
        let mut near_accounts = Accounts::<Info>::new();
        let account: AccountId = accounts(0).into();
        let tok: AccountId = accounts(2).into();
        let min = near_accounts.storage_balance_bounds().min.0;
        testing_env!(context.attached_deposit(min * 10).build());
        near_accounts.storage_deposit(Some(AccountId::try_from(account.clone()).unwrap()), None);
        testing_env!(context.attached_deposit(1).build());
        let near_account = near_accounts.get_account_checked(&account);

        (account, tok, near_accounts, near_account, context)
    }
}
