/*!
Multi Token implementation with JSON serialization.
NOTES:
  - The maximum balance value is limited by U128 (2**128 - 1).
  - JSON calls should pass U128 as a base-10 string. E.g. "100".
  - The contract tracks the change in storage before and after the call. If the storage increases,
    the contract requires the caller of the contract to attach enough deposit to the function call
    to cover the storage cost.
    This is done to prevent a denial of service attack on the contract by taking all available storage.
    If the storage decreases, the contract will issue a refund for the cost of the released storage.
    The unused tokens from the attached deposit are also refunded, so it's safe to
    attach more deposit than required.
  - To prevent the deployed contract from being modified or deleted, it should not have any access
    keys on its account.
*/
use multi_token_standard::metadata::MultiTokenMetadata;
use multi_token_standard::MultiToken;
use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::collections::{LazyOption, LookupMap};
use near_sdk::json_types::U128;
use near_sdk::{
    env, log, near_bindgen, require, AccountId, Balance, BorshStorageKey, PanicOnDefault, Promise,
    PromiseOrValue,
};

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct Contract {
    token: MultiToken,
}

#[derive(BorshDeserialize, BorshSerialize, BorshStorageKey)]
enum StorageKey {
    MultiTokenOwner,
    MultiTokenMetadata,
    MultiTokenSupply,
}

#[near_bindgen]
impl Contract {
    #[init]
    pub fn new(owner_id: AccountId) -> Self {
        assert!(!env::state_exists(), "Already initialized");
        Self {
            token: MultiToken::new(
                StorageKey::MultiTokenOwner,
                owner_id,
                Some(StorageKey::MultiTokenMetadata),
                StorageKey::MultiTokenSupply,
            ),
        }
    }

    fn refund_deposit(&self, storage_used: u64) {
        let required_cost = env::storage_byte_cost() * Balance::from(storage_used);
        let attached_deposit = env::attached_deposit();
        assert!(
            required_cost <= attached_deposit,
            "Must attach {} yoctoNEAR to cover storage",
            required_cost,
        );
        let refund = attached_deposit - required_cost;
        if refund > 1 {
            Promise::new(env::predecessor_account_id()).transfer(refund);
        }
    }

    #[payable]
    pub fn mint(
        &mut self,
        token_id: TokenId,
        token_type: TokenType,
        amount: Option<U128>,
        token_owner_id: AccountId,
        token_metadata: Option<MultiTokenMetadata>,
    ) {
        let initial_storage_usage = env::storage_usage();
        require!(env::predecessor_account_id() == self.token.owner_id, "Unauthorized");
        // TODO: shouldn't this be is_none?
        // if self.token.token_metadata_by_id.is_some() && token_metadata.is_none() {
        //     env::panic_str("Must provide metadata");
        // }

        // Every token must have a token type and every NFT type cannot be re-minted
        match self.token.token_type_index.get(&token_id) {
            Some(TokenType::Ft) => {
                require!(
                    token_type == TokenType::Ft,
                    "Type must be of FT time tokenId already exists"
                )
            }
            Some(TokenType::Nft) => {
                env::panic_str("Attempting to mint already minted NFT");
            }
            None => {
                self.token.token_type_index.insert(&token_id, &token_type);
            }
        }

        let owner_id: AccountId = token_owner_id;
        // Core behavior: every token must have an owner
        match token_type {
            TokenType::Ft => {
                if amount.is_none() {
                    env::panic_str("Amount must be specified for Ft type tokens");
                }
                // advance the prefix index before insertion
                let amt = u128::from(amount.unwrap());
                //create LookupMap for balances
                match self.token.ft_owners_by_id.get(&token_id) {
                    Some(mut balances) => {
                        let current_bal = balances.get(&owner_id).unwrap_or(0);
                        // TODO not quite safe
                        if amt == 0 {
                            env::panic_str("error: amount should be greater than 0")
                        }
                        balances.insert(&owner_id, &(current_bal + amt));
                        let supply = self.token.ft_token_supply_by_id.get(&token_id).unwrap();
                        self.token.ft_token_supply_by_id.insert(&token_id, &(supply + amt));
                    }
                    None => {
                        let mut balances = self.token.internal_new_ft_balances();
                        // insert amount into balances
                        balances.insert(&owner_id, &amt);
                        self.token.ft_owners_by_id.insert(&token_id, &balances);
                        self.token.ft_token_supply_by_id.insert(&token_id, &amt);
                    }
                }
            }
            TokenType::Nft => {
                self.token.nft_owner_by_id.insert(&token_id, &owner_id);
            }
        }
        // Metadata extension: Save metadata, keep variable around to return later.
        // Note that check above already panicked if metadata extension in use but no metadata
        // provided to call.
        self.token.token_metadata_by_id.as_mut().and_then(|by_id| {
            by_id.insert(
                &token_id,
                &token_metadata.as_ref().unwrap_or(&MultiTokenMetadata {
                    spec: "aa".to_string(),   // required, essentially a version like "mt-1.0.0"
                    name: "aa".to_string(),   // required, ex. "Mosaics"
                    symbol: "aa".to_string(), // required, ex. "MOSIAC"
                    icon: None,               // Data URL
                    base_uri: None, // Centralized gateway known to have reliable access to decentralized storage assets referenced by `reference` or `media` URLs
                    decimals: Some(12), // precision decimals for tokens that need this information
                    reference: None, // URL to a JSON file with more info
                    reference_hash: None, // Base64-encoded sha256 hash of JSON from reference field. Required if `reference` is included.
                    title: None,          // ex. "Arch Nemesis: Mail Carrier" or "Parcel #5055"
                    description: None,    // free-form description
                    media: None, // URL to associated media, preferably to decentralized, content-addressed storage
                    media_hash: None, // Base64-encoded sha256 hash of content referenced by the `media` field. Required if `media` is included.
                    copies: None, // number of copies of this set of metadata in existence when token was minted.
                    issued_at: None, // ISO 8601 datetime when token was issued or minted
                    expires_at: None, // ISO 8601 datetime when token expires
                    starts_at: None, // ISO 8601 datetime when token starts being valid
                    updated_at: None, // ISO 8601 datetime when token was last updated
                    extra: None, // anything extra the NFT wants to store on-chain. Can be stringified JSON.
                }),
            )
        });
        // Return any extra attached deposit not used for storage
        self.refund_deposit(env::storage_usage() - initial_storage_usage);
    }
}
multi_token_standard::impl_multi_token_core!(Contract, token);
multi_token_standard::impl_multi_token_storage!(Contract, token);
