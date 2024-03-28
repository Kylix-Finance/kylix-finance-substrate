# Kylix Finance - The cross-chain Lending Dapp
### Kylix is a substrate Lending Dapp that implements Compound V2 style functionalities for lending and borrowing cross-chain assets 

[<img alt="github" src="https://img.shields.io/badge/github-davassi/davassi?style=for-the-badge&labelColor=555555&logo=github" height="20">](https://github.com/davassi/kylix-finance/)
[![unsafe forbidden](https://img.shields.io/badge/unsafe-forbidden-success.svg)](https://github.com/rust-secure-code/safety-dance/)
[![panic forbidden](https://img.shields.io/badge/panic-forbidden-success.svg)](https://github.com/dtolnay/no-panic)
[![Project Status: Active – The project has reached a kind of usable state and is being actively developed.](https://www.repostatus.org/badges/latest/active.svg)](https://www.repostatus.org/#active)

Kylix Finance is a non-custodial substrate Dapp that allows users to participate as depositors or borrowers, allowing them to lend and borrow assets on Polkadot. Borrowers can leverage their assets in an over-collateralised manner, while depositors can provide liquidity and earn interest as a stable passive income.

:warning: It is **not a production-ready substrate node**, but it is still a proof of concept. It is discouraged to use this code 'as-is' in a production runtime.

## User Flow

![Kylix flow](./polkalend.png)

## How does it work - Supply and Withdrawal

Kylix lets users borrow assets for a fee and lend them for interest. A borrower can instantly get a loan and start investing by providing some collateral. When the collateral falls below a specific value, the borrower must top it up to the required level to avoid liquidation. The collateral is unlocked when the borrower returns the loan plus a fee.

By depositing one of the listed assets, the lender will be able to receive lendTokens and earn lending fee income. lendToken is like a deposit certificate of an underlying asset that accrues interest from being borrowed on Kylix Finance. lendToken is redeemable at any time at a 1-to-1 rate with the underlying asset.

### Liquidation Protection - Borrow and Repay

A collateralized loan gives borrowers more time to use their funds in return for providing collateral. A borrower can provide a variety of crypto to back up their loans. With crypto being volatile, you will likely have a low loan-to-value ratio (LTV), such as 50%, for example. This figure means that your loan will only be half the value of your collateral. This difference provides moving room for the collateral’s value if it decreases. Once your collateral falls below the loan's or some other value, the funds are sold or transferred to the lender.

## Exposed Extrinsics

Kylix Finance currently exposes to the world 9 defined extrinsic:

<details>
<summary><h3>do_supply</h3></summary>

Create a new lending pool. Deposit initial liquidity (in the form of an asset). Create a new liquidity token. Mint & transfer to the caller accounts an amount of the liquidity token equal to `currency_amount`. Emits two events on success: `LiquidityPoolCreated` and `AddedLiquidity`.

#### Parameters:
 * `origin` – Origin for the call. Must be signed.
  * `liquidity_token_id` – ID of the liquidity token to be created. The asset with this ID must *not* exist.
  * `asset_a_id` – ID of the asset A traded on the created liquidity pool. The asset with this ID must exist.
  * `asset_b_id` – ID of the asset B traded on the created liquidity pool. The asset with this ID must exist.
  * `amount_a` – Initial amount of asset A to deposit in the pool. Must be greater than 0.


#### Errors:
* `LiquidityPoolAlreadyExisting` - Trying to recreate an existing liquidity pool
* `LiquidityPoolDoesNotExist` - Trying to add or remove liquidity from/to a non-existing liquidity pool

#### Tests
 * `create_new_liquidity_pool_success_test`
  * `create_the_same_liquidity_pool_twice_fail_test`

</details>

<details>
<summary><h3>do_withdraw</h3></summary>

#### Parameters:

#### Errors:

#### Tests
</details>

<details>
<summary><h3>do_borrow</h3></summary>

#### Parameters:
 
#### Errors:

#### Tests
</details>

<details>
<summary><h3>do_repay</h3></summary>


#### Parameters:
 
#### Errors:

#### Tests
</details>



 

## Getting Started

### Build

Use the following command to build the node without launching it:

```sh
cargo build --release
```

### Single-Node Development Chain

The following command starts a single-node development chain that doesn't persist state:

```sh
./target/release/node-template --dev
```

To purge the development chain's state, run the following command:

```sh
./target/release/node-template purge-chain --dev
```

To start the development chain with detailed logging, run the following command:

```sh
RUST_BACKTRACE=1 ./target/release/node-template -ldebug --dev
```

Development chains:

- Maintain state in a `tmp` folder while the node is running.
- Use the **Alice** and **Bob** accounts as default validator authorities.
- Use the **Alice** account as the default `sudo` account.
- Are preconfigured with a genesis state (`/node/src/chain_spec.rs`) that includes several prefunded development accounts.

To persist chain state between runs, specify a base path by running a command similar to the following:

```sh
// Create a folder to use as the db base path
$ mkdir my-chain-state

// Use of that folder to store the chain state
$ ./target/release/node-template --dev --base-path ./my-chain-state/

// Check the folder structure created inside the base path after running the chain
$ ls ./my-chain-state
chains
$ ls ./my-chain-state/chains/
dev
$ ls ./my-chain-state/chains/dev
db keystore network
```

### Connect with Polkadot-JS Apps Front-End

After you start the node template locally, you can interact with it using the hosted version of the [Polkadot/Substrate Portal](https://polkadot.js.org/apps/#/explorer?rpc=ws://localhost:9944) front-end by connecting to the local node endpoint.
A hosted version is also available on [IPFS (redirect) here](https://dotapps.io/) or [IPNS (direct) here](ipns://dotapps.io/?rpc=ws%3A%2F%2F127.0.0.1%3A9944#/explorer).
You can also find the source code and instructions for hosting your instance on the [polkadot-js/apps](https://github.com/polkadot-js/apps) repository.

### Future Improvements

0. TODO

## Contribution

Kylix Finance is a work in progress. If you have suggestions for features, or if you find any issues in the code, design, interface, etc, please feel free to share them on our [GitHub](https://github.com/davassi/polkalend-finance/issues).

I appreciate very much your feedback!
