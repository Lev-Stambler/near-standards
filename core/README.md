Near Standards Library
===================
## Library Overview
This library consists of two libraries which build on top of each other:
- `near-account`
- `near-internal-balance-plugin`

Both libraries deal with storing information related to different accounts in a smart contract.
`near-account` will keep a map of users to some struct. `near-account` will ensure 
that users pay for their storage via keeping track of storage. It also keeps track
of the total amount of Near deposited into the smart contract and will keep a `free Near`
field.

`near-internal-balances-plugin` deals with all things Tokens and is built off of `near-account`. It allows users to transfer 
tokens into and out of a smart contract. This means that users can "deposit" 
tokens into a smart contract and the smart contract then has ownership over the tokens.
But, the smart contract keeps track of the balances. This is basically what [Ref Finance](https://app.ref.finance/) does with tokens.

Currently, the library supports
- [NFTs (with the NEP 171 standard)](https://nomicon.io/Standards/NonFungibleToken/Core),
- [FTs (with the NEP 141 standard)](https://nomicon.io/Standards/FungibleToken/Core),
- Multi Tokens (also known as MT, found [on Github](https://github.com/near/NEPs/pull/245))

For more info on both libraries, please check out the docs.rs

- `near-account` docs TODO:
- `near-internal-balance-plugin` docs TODO:

## Building and Testing

To build run:
```bash
./build.sh
```


As with many Rust libraries and contracts, there are unit tests in 
`near-account` and `near-internal-balance-plugin`.

Additionally, this project has [simulation](https://www.near-sdk.io/testing/simulation-tests) tests in ]the `sim` directory. Simulation tests allow testing cross-contract calls, which is crucial to ensuring that the following functionality works properly:
- Storage deposits work successfully for users
- Free Near is kept track of for users
- the `ft_transfer_call`, `mt_transfer_call`, `nft_transfer_call` successfully deposit ft's, mt's, and nft's into the user's account balance
- `internal_balance_withdraw_to` withdraws tokens successfully from a user's account

These simulation tests are the reason this project has the file structure it does. Note that the root project has a `Cargo.toml` which sets it up as a workspace. `ft`, `nft`, and `mt` (found outside the core's directory) are all used for simulation testing purposes. `dummy` is also used for simulation tests.

You can run all tests with one command:

```bash
cargo test
```

If you want to run only simulation tests, you can use `cargo test simulate`, since all the simulation tests include "simulate" in their names.


## Notes

 - The maximum balance value is limited by U128 (`2**128 - 1`).
 - JSON calls should pass U128 as a base-10 string. E.g. "100".
 - This does not include escrow functionality, as `ft_transfer_call` provides a superior approach. An escrow system can, of course, be added as a separate contract or additional functionality within this contract.


## Contributing

When making changes to the files, remember to use `./build.sh` to compile all contracts and copy the output to the `res` folder. If you forget this, **the simulation tests will not use the latest versions**.

Note that if the `rust-toolchain` file in this repository changes, please make sure to update the `.gitpod.Dockerfile` to explicitly specify using that as default as well.
