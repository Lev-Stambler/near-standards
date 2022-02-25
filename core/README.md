Near Standards Library
===================
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
