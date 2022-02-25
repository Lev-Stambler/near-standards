#[cfg(test)]
pub mod test_utils {
    use near_account::NewInfo;
    use near_sdk::{borsh::{self, BorshSerialize, BorshDeserialize}, collections::UnorderedMap, Balance, AccountId};

    use crate::{TokenId, BalanceInfo, core_impl::AccountInfoTrait};

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

    // impl near_account::AccountInfoTrait for Info {}
    // impl AccountInfoTrait for Info {}
}
