// This file is part of Basilisk-node.

// Copyright (C) 2020-2021  Intergalactic, Limited (GIB).
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::pallet_prelude::Weight;
use frame_support::sp_runtime::traits::Zero;
use primitives::{
	asset::AssetPair,
	traits::{OnCreatePoolHandler, OnTradeHandler, AMMTransfer},
	AssetId, Balance,
};
use sp_std::marker::PhantomData;
use sp_std::prelude::*;
use sp_std::convert::TryInto;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

mod types;
pub use types::*;

pub mod weights;
use weights::WeightInfo;

mod benchmarking; // TODO: rebenchmark

// Re-export pallet items so that they can be accessed from the crate namespace.
pub use pallet::*;
use sp_runtime::DispatchError;

/// Unique identifier for an asset pair.
/// AMM pools derive their own unique identifiers for asset pairs,
/// but this one is meant to not be bounded to one particular AMM pool.
pub type AssetPairId = Vec<u8>;

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_support::pallet_prelude::*;

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: frame_system::Config {
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

		/// Weight information for the extrinsics.
		type WeightInfo: WeightInfo;
	}

	#[pallet::error]
	pub enum Error<T> {
		/// Calculation error occurred while calculating average price
		PriceComputationError,

		/// An unexpected overflow occurred
		UpdateDataOverflow,
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(crate) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Pool was registered. [AssetPair]
		PoolRegistered(AssetPair),
	}

	/// The number of assets registered and handled by this pallet.
	#[pallet::storage]
	#[pallet::getter(fn num_of_assets)]
	pub type TrackedAssetsCount<T: Config> = StorageValue<_, u32, ValueQuery>;

	/// Sorted array of newly registered assets.
	/// All assets are processed and removed from the storage at the end of a block.
	/// Trades start to be processed from the next block.
	/// All trades in the same block as the asset registration are ignored.
	#[pallet::storage]
	#[pallet::getter(fn new_assets)]
	pub type NewAssets<T: Config> = StorageValue<_, Vec<AssetPairId>, ValueQuery>;

	/// Processed or partially processed data generated by trades.
	/// Data generated by trades are processed sequentially.
	/// Each new entry is combined with the previous value to produce new intermediate value.
	/// The last entry creates the resulting average price and volume.
	#[pallet::storage]
	#[pallet::getter(fn price_accumulator)]
	pub type PriceDataAccumulator<T: Config> = StorageMap<_, Blake2_128Concat, AssetPairId, PriceEntry, ValueQuery>;

	/// The last ten average values corresponding to the last ten blocks.
	#[pallet::storage]
	#[pallet::getter(fn price_data_ten)]
	pub type PriceDataTen<T: Config> = StorageValue<_, Vec<(AssetPairId, BucketQueue)>, ValueQuery>;

	/// The last ten average values corresponding to the last hundred blocks.
	/// Each average value corresponds to an interval of length ten blocks.
	#[pallet::storage]
	#[pallet::getter(fn price_data_hundred)]
	pub type PriceDataHundred<T: Config> = StorageMap<_, Blake2_128Concat, AssetPairId, BucketQueue, ValueQuery>;

	/// The last ten average values corresponding to the last thousand blocks.
	/// Each average value corresponds to an interval of length hundred blocks.
	#[pallet::storage]
	#[pallet::getter(fn price_data_thousand)]
	pub type PriceDataThousand<T: Config> = StorageMap<_, Blake2_128Concat, AssetPairId, BucketQueue, ValueQuery>;

	#[pallet::hooks]
	impl<T: Config> Hooks<T::BlockNumber> for Pallet<T> {
		fn on_initialize(_n: T::BlockNumber) -> Weight {
			T::WeightInfo::on_initialize_multiple_tokens_all_bucket_levels(Self::num_of_assets())
		}

		fn on_finalize(_n: T::BlockNumber) {
			// update average values in the storage
			Self::update_data();

			// clear the price buffer
			PriceDataAccumulator::<T>::remove_all(None);

			// add newly registered assets
			let _ = TrackedAssetsCount::<T>::try_mutate(|value| -> Result<(), DispatchError> {
				*value = value.checked_add(Self::new_assets().len().try_into().map_err(|_| Error::<T>::PriceComputationError)?).ok_or(Error::<T>::PriceComputationError)?;
				Ok(())
			// We don't want to throw an error here because this method is used in different extrinsics.
			// We also do not expect to have more than 2^32 assets registered.
			}).map_err(|_| panic!("Max number of assets reached!"));


			for new_asset in Self::new_assets().iter() {
				PriceDataTen::<T>::append((new_asset, BucketQueue::default()));
			}
			NewAssets::<T>::kill();
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {}
}

impl<T: Config> Pallet<T> {
	pub fn on_create_pool(asset_pair: AssetPair) {
		let data = PriceDataTen::<T>::get();
		if !data.iter().any(|bucket_tuple| bucket_tuple.0 == asset_pair.name()) {
			let _ = NewAssets::<T>::try_mutate(|new_assets| -> Result<(), ()> {
				// Keep the NewAssets vector sorted. It makes it easy to find duplicates.
				match new_assets.binary_search(&asset_pair.name()) {
					Ok(_pos) => Err(()),	// new asset is already in vector
					Err(pos) => {
						new_assets.insert(pos, asset_pair.name());
						Self::deposit_event(Event::PoolRegistered(asset_pair));
						Ok(())
					},
				}
			}).map_err(|_| {});

		}
	}

	pub fn on_trade(asset_pair: AssetPair, price_entry: PriceEntry) {
		let _ = PriceDataAccumulator::<T>::mutate(asset_pair.name(), |previous_price_entry| {
			let maybe_new_price_entry = previous_price_entry.calculate_new_price_entry(&price_entry);
			// Invalid values are ignored and not added to the queue.
			if let Some(new_price_entry) = maybe_new_price_entry {
				*previous_price_entry = new_price_entry;
			}
		});
	}

	fn update_data() {
		PriceDataTen::<T>::mutate(|data_ten| {
			for (asset_pair_id, data) in data_ten.iter_mut() {
				let maybe_price = PriceDataAccumulator::<T>::try_get(asset_pair_id);
				let result = if let Ok(price_entry) = maybe_price {
					PriceInfo {
						avg_price: price_entry.price,
						volume: price_entry.trade_amount,
					}
				} else {
					data.get_last()
				};

				data.update_last(result);
			}
		});

		let now = <frame_system::Pallet<T>>::block_number();

		// check if it's time to update "hundred" values
		if (now % T::BlockNumber::from(BUCKET_SIZE)) == T::BlockNumber::from(BUCKET_SIZE - 1) {
			for element_from_ten in PriceDataTen::<T>::get().iter() {
				PriceDataHundred::<T>::mutate(element_from_ten.0.clone(), |data| {
					data.update_last(element_from_ten.1.calculate_average());
				});
			}
		}

		// check if it's time to update "thousand" values
		if (now % T::BlockNumber::from(BUCKET_SIZE.pow(2))) == T::BlockNumber::from(BUCKET_SIZE.pow(2) - 1) {
			for element_from_hundred in PriceDataHundred::<T>::iter() {
				PriceDataThousand::<T>::mutate(element_from_hundred.0.clone(), |data| {
					data.update_last(element_from_hundred.1.calculate_average());
				});
			}
		}
	}
}

pub struct PriceOracleHandler<T>(PhantomData<T>);
impl<T: Config> OnCreatePoolHandler<AssetPair> for PriceOracleHandler<T> {
	fn on_create_pool(asset_pair: AssetPair) {
		Pallet::<T>::on_create_pool(asset_pair);
	}
}

impl<T: Config> OnTradeHandler<T::AccountId, AssetId, AssetPair, Balance> for PriceOracleHandler<T> {
	fn on_trade(amm_transfer: &AMMTransfer<T::AccountId, AssetId, AssetPair, Balance>, liq_amount: Balance) {
		let (price, amount) = if let Some(price_tuple) = amm_transfer.normalize_price() {
			price_tuple
		} else {
			// We don't want to throw an error here because this method is used in different extrinsics.
			// Invalid prices are ignored and not added to the queue.
			return;
		};

		// We assume that zero values are not valid.
		// Zero values are ignored and not added to the queue.
		if price.is_zero() || amount.is_zero() || liq_amount.is_zero() {
			return;
		}

		let price_entry = PriceEntry {
			price,
			trade_amount: amount,
			liquidity_amount: liq_amount,
		};

		Pallet::<T>::on_trade(amm_transfer.assets, price_entry);
	}

	fn on_trade_weight() -> Weight {
		T::WeightInfo::on_initialize_one_token() - T::WeightInfo::on_initialize_no_entry()
	}
}
