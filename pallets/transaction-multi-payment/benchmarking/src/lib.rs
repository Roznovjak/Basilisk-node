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

mod mock;

use sp_std::prelude::*;
use sp_std::vec;

use frame_benchmarking::{account, benchmarks};
use frame_system::RawOrigin;
use orml_traits::{MultiCurrencyExtended, MultiCurrency};
use pallet_transaction_multi_payment::Pallet as MultiPaymentModule;
use primitives::{Amount, AssetId, Balance, Price};

#[cfg(test)]
use orml_traits::MultiCurrency;

use frame_support::dispatch;
use pallet_xyk as xykpool;

pub struct Pallet<T: Config>(pallet_transaction_multi_payment::Pallet<T>);

pub trait Config:
	pallet_transaction_payment::Config + pallet_transaction_multi_payment::Config + xykpool::Config
{
}

const SEED: u32 = 0;
// const ASSET_ID: AssetId = 3;
// const HDX: AssetIdOf<T> = 0;

pub type AssetIdOf<T> =
<<T as pallet_transaction_multi_payment::Config>::MultiCurrency as MultiCurrency<<T as frame_system::Config>::AccountId>>::CurrencyId;

fn funded_account<T: Config>(name: &'static str, index: u32) -> T::AccountId
where
	T::MultiCurrency: MultiCurrencyExtended<T::AccountId, Balance = Balance, Amount = Amount>,
{
	let caller: T::AccountId = account(name, index, SEED);

	let HDX: AssetIdOf<T> = 0u32;
	let ASSET_ID: AssetIdOf<T> = 0u32;
	T::MultiCurrency::update_balance(ASSET_ID, &caller, 10_000_000_000_000).unwrap();
	T::MultiCurrency::update_balance(HDX, &caller, 10_000_000_000_000).unwrap();

	caller
}

fn initialize_pool<T: Config>(
	caller: T::AccountId,
	asset: AssetId,
	amount: Balance,
	price: Price,
) -> dispatch::DispatchResultWithPostInfo {
	let HDX: AssetIdOf<T> = 0;
	xykpool::Pallet::<T>::create_pool(RawOrigin::Signed(caller).into(), HDX, asset, amount, price)?;
	Ok(().into())
}

benchmarks! {
	swap_currency {
		let maker = funded_account::<T>("maker", 1);
		let ASSET_ID: AssetIdOf<T> = 0;
		initialize_pool::<T>(maker.clone(), ASSET_ID, 1_000_000_000_000, Price::from(1))?;
		MultiPaymentModule::<T>::add_new_member(&maker);
		MultiPaymentModule::<T>::add_currency(RawOrigin::Signed(maker).into(), ASSET_ID, Price::from(10))?;

		let caller = funded_account::<T>("caller", 2);
		MultiPaymentModule::<T>::set_currency(RawOrigin::Signed(caller.clone()).into(), ASSET_ID)?;

	}: { MultiPaymentModule::<T>::swap_currency(&caller, 1000)? }
	verify{
		assert_eq!(MultiPaymentModule::<T>::get_currency(caller.clone()), Some(ASSET_ID));
		#[cfg(test)]
		assert_eq!(T::MultiCurrency::free_balance(ASSET_ID, &caller), 9999689661666);
	}

	set_currency {
		let maker = funded_account::<T>("maker", 1);
		let ASSET_ID: AssetIdOf<T> = 0;
		initialize_pool::<T>(maker.clone(), ASSET_ID, 1_000_000_000_000, Price::from(1))?;
		MultiPaymentModule::<T>::add_new_member(&maker);
		MultiPaymentModule::<T>::add_currency(RawOrigin::Signed(maker).into(), ASSET_ID, Price::from(10))?;

		let caller = funded_account::<T>("caller", 123);

		let currency_id: AssetId = ASSET_ID;

	}: { MultiPaymentModule::<T>::set_currency(RawOrigin::Signed(caller.clone()).into(), currency_id)? }
	verify{
		assert_eq!(MultiPaymentModule::<T>::get_currency(caller), Some(currency_id));
	}

	add_currency {
		let caller = funded_account::<T>("maker", 1);
		MultiPaymentModule::<T>::add_new_member(&caller);

		let price = Price::from(10);

	}: { MultiPaymentModule::<T>::add_currency(RawOrigin::Signed(caller.clone()).into(), 10, price)? }
	verify {
		assert_eq!(MultiPaymentModule::<T>::currencies(10), Some(price));
	}

	remove_currency {
		let caller = funded_account::<T>("maker", 1);
		MultiPaymentModule::<T>::add_new_member(&caller);
		MultiPaymentModule::<T>::add_currency(RawOrigin::Signed(caller.clone()).into(), 10, Price::from(2))?;

		assert_eq!(MultiPaymentModule::<T>::currencies(10), Some(Price::from(2)));

	}: { MultiPaymentModule::<T>::remove_currency(RawOrigin::Signed(caller.clone()).into(), 10)? }
	verify {
		assert_eq!(MultiPaymentModule::<T>::currencies(10), None)
	}

	add_member{
		let member = funded_account::<T>("newmember", 10);
	}: { MultiPaymentModule::<T>::add_member(RawOrigin::Root.into(), member.clone())? }
	verify {
		assert_eq!(MultiPaymentModule::<T>::authorities(), vec![member]);
	}

	remove_member{
		let member = funded_account::<T>("newmember", 10);
		MultiPaymentModule::<T>::add_new_member(&member);
	}: { MultiPaymentModule::<T>::remove_member(RawOrigin::Root.into(), member.clone())? }
	verify {
		assert_eq!(MultiPaymentModule::<T>::authorities(), vec![]);
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::mock::{ExtBuilder, Test};
	use frame_support::assert_ok;

	#[test]
	fn test_benchmarks() {
		ExtBuilder::default().base_weight(5).build().execute_with(|| {
			assert_ok!(Pallet::<Test>::test_benchmark_swap_currency());
			assert_ok!(Pallet::<Test>::test_benchmark_set_currency());
			assert_ok!(Pallet::<Test>::test_benchmark_add_currency());
			assert_ok!(Pallet::<Test>::test_benchmark_remove_currency());
			assert_ok!(Pallet::<Test>::test_benchmark_add_member());
			assert_ok!(Pallet::<Test>::test_benchmark_remove_member());
		});
	}
}
