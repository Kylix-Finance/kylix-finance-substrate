# LENDING

## Overview

The Lending pallet oversees lending pools and treasury activities. This pallet is designed to emulate the protocol of Compound V2, where assets from all users are aggregated through a pool-based approach. The interest rates for lending operations are dynamically adjusted based on the supply and demand. Furthermore, a new token is minted for each lending position, allowing for the transfer of ownership.

## Pallet overview

### Config types

  - `PalletId`: Type representing the pallet identifier.
  - `RuntimeEvent`: Type representing runtime events.
  - `NativeBalance`: Type to access the Balances Pallet, supporting various operations such as inspection, mutation, holding, and freezing.
  - `Fungibles`: Type to access the Assets Pallet, supporting inspection, mutation, and creation of fungible assets.
  - `WeightInfo`: Weight information for extrinsics in this pallet.
  - `Time`: Type representing moment time.


### Storage types

This pallet uses the following storage items:

* `LendingPoolStorage`: `StorageMap` that uses `AssetPool` as key and `LendingPool` as a value.

* `UnderlyingAssetStorage`:  `StorageMap` that uses `AssetIdOf` to  as key and `UnderlyingAsset` as a value.

* `MinMaxExchangeRate`: `StorageValue` that keeps the starting and maximum `Rate` allowed in a market.

* `SupplyIndexStorage`: `StorageMap` that uses tuple of `(AccountOf<T>, AssetIdOf<T>)` as key and `SupplyIndex` as value.

* `Borrows`: `StorageMap` that has `(AccountOf<T>, AssetIdOf<T>, AssetIdOf<T>)` as key and stores `UserBorrow` as value.

* `AssetPrices`: `StorageMap` that has `(AssetIdOf<T>, AssetIdOf<T>)` as a key and stores `FixedU128` as value.

# Extrinsics

This pallet provides the following extrinsics:

**Create Lending Pool**
========================

The `create_lending_pool` function allows users to create a new lending pool and supply it with liquidity. This function is used to create a new reserve and add liquidity to it, given an asset and its amount.

**Functionality**

* Creates a new lending pool if it does not already exist
* Adds the provided liquidity to the pool
* Returns LP tokens to the user in a ratio-based manner

**Arguments**

* `origin`: The origin caller of this function (must be signed by the user creating the lending pool)
* `id`: The pool id provided by the user
* `asset`: The identifier for the type of asset being supplied
* `balance`: The amount of `asset` being supplied

**Error Handling**

This function will return an error in the following scenarios:

* If the origin is not signed (i.e., the function was not called by a user)
* If the provided assets do not exist
* If `amount` is 0 or less
* If adding liquidity to the pool fails due to arithmetic overflows or underflows

**Events**

If the function succeeds, it will trigger the following events:

* `LendingPoolAdded(who, asset_a)` if a new lending pool was created
* `DepositSupplied(who, asset_a, amount_a)` after the liquidity has been successfully added

**Activate Lending Pool**
=========================

The `activate_lending_pool` function enables a user to activate a non-empty lending pool, allowing supply operations to be performed.

**Functionality**

* Activates a lending pool that is not empty
* Enables supply operations for the activated pool
* Otherwise, only withdrawals are allowed

**Arguments**

* `origin`: The origin caller of this function (must be signed by the user creating the lending pool)
* `asset`: The identifier for the type of asset being provided

**Error Handling**

This function will return an error in the following scenarios:

* If the origin is not signed (i.e., the function was not called by a user)
* If the provided assets do not exist
* If the pool does not exist
* If the pool is already activated
* If the pool is empty

**Events**

If the function succeeds, it will trigger the following event:

* `LendingPoolActivated(who, asset_a)` if the lending pool was successfully activated

**Supply Liquidity**
=====================

The `supply` function allows a user to supply liquidity to a lending pool.

**Functionality**

* Supplies liquidity to a lending pool
* Enables the user to provide an amount of a specific asset to the pool

**Arguments**

* `origin`: The origin caller of this function (must be signed by the user providing liquidity)
* `asset`: The identifier for the type of asset being supplied
* `balance`: The amount of `asset` being supplied

**Error Handling**

This function will return an error in the following scenarios:

* If the origin is not signed (i.e., the function was not called by a user)
* If the provided assets do not exist
* If the pool does not exist
* If the pool is not active
* If the user has not enough liquidity to supply
* If the balance amount to supply is not valid
* If adding liquidity to the pool fails due to arithmetic overflows or underflows

**Events**

If the function succeeds, it will trigger the following event:

* `DepositSupplied(who, asset, balance)` if the lending pool has been successfully supplied.

**Withdraw Liquidity**
=====================

The `withdraw` function allows a user to withdraw liquidity from a lending pool.

**Functionality**

* Withdraws liquidity from a lending pool
* Enables the user to remove a specific amount of an asset from the pool

**Arguments**

* `origin`: The origin caller of this function (must be signed by the user withdrawing liquidity)
* `asset`: The identifier for the type of asset being withdrawn
* `balance`: The amount of `asset` being withdrawn

**Error Handling**

This function will return an error in the following scenarios:

* If the origin is not signed (i.e., the function was not called by a user)
* If the provided assets do not exist
* If the pool does not exist
* If the pool is not active
* If the user has not enough liquidity to withdraw
* If the balance amount to withdraw is not valid
* If withdrawing liquidity from the pool fails due to arithmetic overflows or underflows

**Events**

If the function succeeds, it will trigger the following event:

* `DepositWithdrawn(who, balance)` if the liquidity was successfully withdrawn from the lending pool.

**Borrow Liquidity**
=====================

The `borrow` function allows a user to borrow liquidity from a lending pool.

**Functionality**

* Borrows liquidity from a lending pool
* Enables the user to receive a specific amount of an asset from the pool

**Arguments**

* `origin`: The origin caller of this function (must be signed by the user borrowing liquidity)
* `asset`: The identifier for the type of asset being borrowed
* `balance`: The amount of `asset` being borrowed

**Error Handling**

This function will return an error in the following scenarios:

* If the origin is not signed (i.e., the function was not called by a user)
* If the provided assets do not exist
* If the pool does not exist
* If the pool is not active
* If the user has not enough liquidity to borrow
* If the balance amount to borrow is not valid
* If borrowing liquidity from the pool fails due to arithmetic overflows or underflows

**Events**

If the function succeeds, it will trigger the following event:

* `DepositBorrowed(who, balance)` if the liquidity was successfully borrowed from the lending pool.

**Repay Liquidity**
=====================

The `repay` function allows a user to repay liquidity to a lending pool.

**Functionality**

* Repays liquidity to a lending pool
* Enables the user to return a specific amount of an asset to the pool

**Arguments**

* `origin`: The origin caller of this function (must be signed by the user repaying liquidity)
* `asset`: The identifier for the type of asset being repaid
* `balance`: The amount of `asset` being repaid

**Error Handling**

This function will return an error in the following scenarios:

* If the origin is not signed (i.e., the function was not called by a user)
* If the provided assets do not exist
* If the pool does not exist
* If the pool is not active
* If the user has not enough liquidity to repay
* If the balance amount to repay is not valid
* If repaying liquidity to the pool fails due to arithmetic overflows or underflows

**Events**

If the function succeeds, it will trigger the following event:

* `DepositRepaid(who, balance)` if the liquidity was successfully repaid to the lending pool.

**Claim Rewards**
================

The `claim_rewards` function allows a user to claim their rewards.

**Functionality**

* Claims rewards for a user
* Triggers an event to notify the system that rewards have been claimed

**Arguments**

* `origin`: The origin caller of this function (must be signed by the user claiming rewards)
* `balance`: The amount of rewards to be claimed

**Return Value**

* `DispatchResult`: Returns `Ok(())` if the function succeeds, or an error if it fails.

**Events**

If the function succeeds, it will trigger the following event:

* `RewardsClaimed { who, balance }`: Notifies the system that rewards have been claimed by a user.

**Deactivate Lending Pool**
==========================

The `deactivate_lending_pool` function allows a user to deactivate a lending pool that is not empty.

**Functionality**

* Deactivates a lending pool that is not empty
* Prevents supply operations from being performed, allowing only withdrawals

**Arguments**

* `origin`: The origin caller of this function (must be signed by the user deactivating the lending pool)
* `asset`: The identifier for the type of asset associated with the lending pool

**Error Handling**

This function will return an error in the following scenarios:

* If the origin is not signed (i.e., the function was not called by a user)
* If the provided assets do not exist
* If the pool does not exist
* If the pool is already deactivated
* If the pool is empty

**Events**

If the function succeeds, it will trigger the following event:

* `LendingPoolDeactivated(who, asset_a)` if the lending pool was successfully deactivated.

**Update Pool Rate Model**
==========================

The `update_pool_rate_model` function allows a user to update the rate model of a lending pool.

**Functionality**

* Updates the rate model of a lending pool
* Enables a user to modify the rate model associated with a lending pool

**Arguments**

* `origin`: The origin caller of this function (must be signed by the user updating the rate model)
* `asset`: The identifier for the type of asset associated with the lending pool

**Error Handling**

This function will return an error in the following scenarios:

* If the origin is not signed (i.e., the function was not called by a user)

**Events**

If the function succeeds, it will trigger the following event:

* `LendingPoolRateModelUpdated { who, asset }`: Notifies the system that the rate model of a lending pool was updated by a user.

**Update Pool Kink**
=====================

The `update_pool_kink` function allows a user to update the kink of a lending pool.

**Functionality**

* Updates the kink of a lending pool
* Enables a user to modify the kink associated with a lending pool

**Arguments**

* `origin`: The origin caller of this function (must be signed by the user updating the kink)
* `asset`: The identifier for the type of asset associated with the lending pool

**Error Handling**

This function will return an error in the following scenarios:

* If the origin is not signed (i.e., the function was not called by a user)

**Events**

If the function succeeds, it will trigger the following event:

* `LendingPoolKinkUpdated { who, asset }`: Notifies the system that the kink of a lending pool was updated by a user.

**Set Asset Price**
=====================

The `set_asset_price` function allows a user to set the price of one asset in terms of another asset.

**Functionality**

* Sets the relative price of one asset (`asset_1`) in terms of another asset (`asset_2`)
* Enables users to specify the price of an asset in terms of another asset

**Parameters**

* `origin`: The transaction origin (must be a signed extrinsic)
* `asset_1`: The identifier for the first asset (the asset whose price is being set)
* `asset_2`: The identifier for the second asset (the asset relative to which the price is measured)
* `price`: The price of `asset_1` in terms of `asset_2` (must be a non-zero value)

**Errors**

* `InvalidAssetPrice`: This error is thrown if the `price` parameter is zero.
* 
**Events**

* `AssetPriceAdded { asset_1, asset_2, price }`: This event is emitted after the price is successfully set. It contains the asset identifiers and the new price.


# Events

This pallet emits the following events:

### DepositSupplied

 **Description**: Signals that a user has supplied assets to the lending pool.
 **Fields**:
  - `who`: Account ID of the user who supplied the assets.
  - `asset`: ID of the asset supplied.
  - `balance`: Amount of the asset supplied.

### DepositWithdrawn

**Description**: Indicates that a user has withdrawn assets from the lending pool.
**Fields**:
  - `who`: Account ID of the user who withdrew the assets.
  - `balance`: Amount of the assets withdrawn.

### DepositBorrowed

**Description**: Denotes that a user has borrowed assets from the lending pool.
**Fields**:
  - `who`: Account ID of the user who borrowed the assets.
  - `balance`: Amount of the assets borrowed.

### DepositRepaid

**Description**: Indicates that a user has repaid borrowed assets to the lending pool.
**Fields**:
  - `who`: Account ID of the user who repaid the assets.
  - `balance`: Amount of the assets repaid.

### RewardsClaimed

**Description**: Indicates that a user has claimed rewards from the lending pool.
**Fields**:
  - `who`: Account ID of the user who claimed the rewards.
  - `balance`: Amount of rewards claimed.

### LendingPoolAdded

**Description**: Signals the addition of a new lending pool.
**Fields**:
  - `who`: Account ID of the user who added the lending pool.
  - `asset`: ID of the asset associated with the lending pool.

### LendingPoolRemoved

**Description**: Signals the removal of a lending pool.
**Fields**:
  - `who`: Account ID of the user who removed the lending pool.

### LendingPoolActivated

**Description**: Indicates that a lending pool has been activated.
**Fields**:
  - `who`: Account ID of the user who activated the lending pool.
  - `asset`: ID of the asset associated with the activated lending pool.

### LendingPoolDeactivated

**Description**: Denotes the deactivation of a lending pool.
**Fields**:
  - `who`: Account ID of the user who deactivated the lending pool.
  - `asset`: ID of the asset associated with the deactivated lending pool.

### LendingPoolRateModelUpdated

**Description**: Signals the update of the rate model for a lending pool.
**Fields**:
  - `who`: Account ID of the user who updated the rate model.
  - `asset`: ID of the asset associated with the updated rate model.

### LendingPoolKinkUpdated

**Description**: Indicates the update of the kink for a lending pool.
**Fields**:
  - `who`: Account ID of the user who updated the kink.
  - `asset`: ID of the asset associated with the updated kink.

### LPTokenMinted

**Description**: Denotes the minting of LP tokens for a user.
**Fields**:
  - `who`: Account ID of the user who minted LP tokens.
  - `asset`: ID of the asset associated with the minted LP tokens.
  - `balance`: Amount of LP tokens minted.

### AssetPriceAdded

**Description**: Signals the addition of the price of an asset.
**Fields**:
  - `asset_1`: ID of the first asset in the pair.
  - `asset_2`: ID of the second asset in the pair.
  - `price`: Fixed price of the asset pair.

# Errors

This pallet uses the following error types:

### LendingPoolDoesNotExist
- Indicates that the lending pool does not exist.

### LendingPoolAlreadyExists

- Indicates that the lending pool already exists.

### LendingPoolAlreadyActivated

- Indicates that the lending pool is already activated.

### LendingPoolAlreadyDeactivated

- Indicates that the lending pool is already deactivated.

### LendingPoolNotActive

- Indicates that the lending pool is not active or has been deprecated.

### InvalidLiquiditySupply

- Indicates that the balance amount to supply is not valid.

### InvalidLiquidityWithdrawal

- Indicates that the balance amount to withdraw is not valid.

### NotEnoughLiquiditySupply

- Indicates that the user does not have enough liquidity to supply.

### NotEnoughElegibleLiquidityToWithdraw

- Indicates that the user wants to withdraw more than allowed.

### LendingPoolIsEmpty

- Indicates that the lending pool is empty.

### OverflowError

- Indicates the classic overflow error.

### IdAlreadyExists

- Indicates that the ID already exists.

### NotEnoughCollateral

- Indicates that the user does not have enough collateral assets.

### LoanDoesNotExists

- Indicates that the loan being repaid does not exist.

### InvalidAssetPrice

- Indicates that the price of the asset cannot be zero.

### AssetPriceNotSet
- Indicates that the price of the asset is not available


# Licensing

This pallet is licensed under the terms of the Apache License (Version 2.0).

## Contributing

Kylix Finance is a work in progress. If you have suggestions for features, or if you find any issues in the code, design, interface, etc, please feel free to share them on our [GitHub](https://github.com/Kylix-Finance/kylix-finance-substrate/issues) or reach us on Discord:

## Changelog

[Insert changelog entries, including changes, fixes, and updates]
