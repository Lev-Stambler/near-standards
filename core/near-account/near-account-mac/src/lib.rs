use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput, ItemStruct};

#[proc_macro_derive(NearAccounts)]
pub fn near_accounts(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let input_struct = input.clone();

    let struct_: syn::DataStruct = match input.data {
        syn::Data::Struct(data) => data,
        _ => panic!("Usage of #[Derive(NearAccounts)] on a non-struct type"),
    };

    let (impl_generics, ty_generics, where_clause) = input_struct.generics.split_for_impl();
    let struct_name = &input_struct.ident;

    let stream = quote! {

        #[near_sdk::near_bindgen]
        impl #struct_name #ty_generics #where_clause {

            #[payable]
            pub fn accounts_storage_deposit(
                &mut self,
                account_id: Option<near_sdk::AccountId>,
                registration_only: Option<bool>,
            ) -> near_contract_standards::storage_management::StorageBalance {
                self.accounts.storage_deposit(account_id, registration_only)
            }

            #[payable]
            pub fn accounts_storage_withdraw(&mut self, amount: Option<near_sdk::json_types::U128>) -> near_contract_standards::storage_management::StorageBalance {
                self.accounts.storage_withdraw(amount)
            }

            #[payable]
            pub fn accounts_storage_unregister(&mut self, force: Option<bool>) -> bool {
                self.accounts.storage_unregister(force)
            }

            pub fn accounts_storage_balance_bounds(&self) -> near_contract_standards::storage_management::StorageBalanceBounds {
                self.accounts.storage_balance_bounds()
            }

            pub fn accounts_storage_balance_of(&self, account_id: near_sdk::AccountId) -> Option<near_contract_standards::storage_management::StorageBalance> {
                self.accounts.storage_balance_of(account_id)
            }

            pub fn accounts_near_balance_of(&self, account_id: near_sdk::AccountId) -> Option<near_contract_standards::storage_management::StorageBalance> {
                self.accounts.storage_balance_of(account_id)
            }
        }
    };
    TokenStream::from(stream)
}
