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

use crate::{Exchange, Runtime, XYK};
use super::{create_pool, register_asset, update_balance, AccountId, AssetId, Balance, Price};

use frame_benchmarking::{account, BenchmarkError};
use frame_support::{
	dispatch::DispatchResult,
	traits::OnFinalize,
};
use frame_system::RawOrigin;
use orml_benchmarking::runtime_benchmarks;
use orml_traits::MultiCurrency;
use sp_runtime::traits::{BlakeTwo256, Hash};
use primitives::constants::currency::DOLLARS;

const INITIAL_ASSET_BALANCE: Balance = 1_000_000_000_000_000;
const MAX_INTENTIONS_IN_BLOCK: u32 = 1000;
const SEED: u32 = 0;

const SELL_INTENTION_AMOUNT: Balance = 1_000_000_000;
const SELL_INTENTION_LIMIT: Balance = 1;
const BUY_INTENTION_AMOUNT: Balance = 1_000_000_000;
const BUY_INTENTION_LIMIT: Balance = 2_000_000_000;

fn create_funded_account(name: &'static str, index: u32, assets: &[AssetId]) -> AccountId {
	let account_id: AccountId = account(name, index, SEED);

	for asset_id in assets {
		update_balance(*asset_id, &account_id, INITIAL_ASSET_BALANCE);
	}

	account_id
}

fn feed_intentions(asset_a: AssetId, asset_b: AssetId, number: u32, amounts: &[u32]) -> DispatchResult {
	for idx in 0..number / 2 {
		let user = create_funded_account("user", idx + 2, &[asset_a, asset_b]);

		Exchange::sell(
			RawOrigin::Signed(user.clone()).into(),
			asset_a,
			asset_b,
			amounts[idx as usize] as u128,
			SELL_INTENTION_LIMIT,
			false,
		)?;

		let buyer = create_funded_account("user", idx + number + 1, &[asset_a, asset_b]);

		Exchange::buy(
			RawOrigin::Signed(buyer.clone()).into(),
			asset_a,
			asset_b,
			amounts[idx as usize] as u128,
			amounts[idx as usize] as u128 * 2u128,
			false,
		)?;
	}

	Ok(())
}

fn validate_finalize(asset_a: AssetId, _asset_b: AssetId, number: u32, amounts: &[u32]) -> DispatchResult {
	for idx in 0..number / 2 {
		let user: AccountId = account("user", idx + 2, SEED);
		assert_eq!(
			<Runtime as pallet_xyk::Config>::Currency::free_balance(asset_a, &user),
			INITIAL_ASSET_BALANCE - amounts[idx as usize] as u128
		);

		let buyer: AccountId = account("user", idx + number + 1, SEED);
		assert_eq!(
			<Runtime as pallet_xyk::Config>::Currency::free_balance(asset_a, &buyer),
			INITIAL_ASSET_BALANCE + amounts[idx as usize] as u128
		);
	}

	Ok(())
}

runtime_benchmarks! {
	{ Runtime, pallet_exchange }

	known_overhead_for_on_finalize {
		let t: u32 = 5;
	}: {  Exchange::on_finalize(t); }
	verify {
	}

	sell_intention {
		let nbr_intentions_appended: u32  = MAX_INTENTIONS_IN_BLOCK;

		let asset_a = register_asset(b"ASSETA".to_vec(), 1_000_000).map_err(|_| BenchmarkError::Stop("Failed to register asset"))?;
		let asset_b = register_asset(b"ASSETB".to_vec(), 1_000_000).map_err(|_| BenchmarkError::Stop("Failed to register asset"))?;

		let caller = create_funded_account("caller", 1, &[asset_a, asset_b]);

		let amount : Balance =  DOLLARS;
		let limit : Balance =  DOLLARS;

		create_pool(caller.clone(), asset_a, asset_b, amount, Price::from(10));

		feed_intentions(asset_a, asset_b, nbr_intentions_appended, &INTENTION_AMOUNTS)?;

		assert_eq!(Exchange::get_intentions_count((asset_a, asset_b)), nbr_intentions_appended);

	}: {  Exchange::sell(RawOrigin::Signed(caller.clone()).into(), asset_a, asset_b, amount ,limit, false)? }
	verify{
		assert_eq!(Exchange::get_intentions_count((asset_a, asset_b)), nbr_intentions_appended + 1);
	}

	buy_intention {
		let asset_a = register_asset(b"ASSETA".to_vec(), 1_000_000).map_err(|_| BenchmarkError::Stop("Failed to register asset"))?;
		let asset_b = register_asset(b"ASSETB".to_vec(), 1_000_000).map_err(|_| BenchmarkError::Stop("Failed to register asset"))?;

		let caller = create_funded_account("caller", 1, &[asset_a, asset_b]);

		let amount : Balance = DOLLARS;
		let limit : Balance = DOLLARS;

		let nbr_intentions_appended: u32  = MAX_INTENTIONS_IN_BLOCK;

		create_pool(caller.clone(), asset_a, asset_b, amount, Price::from(1));

		feed_intentions(asset_a, asset_b, nbr_intentions_appended, &INTENTION_AMOUNTS)?;

		assert_eq!(Exchange::get_intentions_count((asset_a, asset_b)), nbr_intentions_appended);

	}: {  Exchange::buy(RawOrigin::Signed(caller.clone()).into(), asset_a, asset_b, amount / 10 ,limit, false)? }
	verify{
		assert_eq!(Exchange::get_intentions_count((asset_a, asset_b)), nbr_intentions_appended + 1);
	}

	on_finalize {
		let t in 0 .. MAX_INTENTIONS_IN_BLOCK; // Intention component

		let asset_a = register_asset(b"ASSETA".to_vec(), 1_000_000).map_err(|_| BenchmarkError::Stop("Failed to register asset"))?;
		let asset_b = register_asset(b"ASSETB".to_vec(), 1_000_000).map_err(|_| BenchmarkError::Stop("Failed to register asset"))?;

		let caller = create_funded_account("caller", 1, &[asset_a, asset_b]);

		let amount : Balance = 100_000_000_000_000;

		// First generate random amounts
		// This is basically used to generate intentions with different amounts
		// it is because algorithm does sort the intention by amount, so we need something not sorted./
		let random_seed = BlakeTwo256::hash(b"Sixty-nine");

		create_pool(caller, asset_a, asset_b, amount, Price::from(1));

		feed_intentions(asset_a, asset_b, t, &INTENTION_AMOUNTS)?;

		assert_eq!(Exchange::get_intentions_count((asset_a, asset_b)), t);

	}: {  Exchange::on_finalize(t); }
	verify {
		assert_eq!(Exchange::get_intentions_count((asset_a, asset_b)), 0);
		validate_finalize(asset_a, asset_b, t, &INTENTION_AMOUNTS)?;
	}

	on_finalize_buys_no_matches {
		let t in 0 .. 100; // Intention component

		let asset_a = register_asset(b"ASSETA".to_vec(), 1_000_000).map_err(|_| BenchmarkError::Stop("Failed to register asset"))?;
		let asset_b = register_asset(b"ASSETB".to_vec(), 1_000_000).map_err(|_| BenchmarkError::Stop("Failed to register asset"))?;

		let caller = create_funded_account("caller", 1, &[asset_a, asset_b]);

		let amount : Balance = 100_000_000_000_000;

		create_pool(caller, asset_a, asset_b, amount, Price::from(1));

		for idx in 0 .. t {
			let user = create_funded_account("user", idx + 100, &[asset_a, asset_b]);
			Exchange::buy(
				RawOrigin::Signed(user.clone()).into(),
				asset_a,
				asset_b,
				BUY_INTENTION_AMOUNT,
				BUY_INTENTION_LIMIT,
				false,
			)?;
		}

		assert_eq!(Exchange::get_intentions_count((asset_a, asset_b)), t);

	}: {  Exchange::on_finalize(t); }
	verify {
		assert_eq!(Exchange::get_intentions_count((asset_a, asset_b)), 0);
		for idx in 0..t  {
			let user: AccountId = account("user", idx + 100, SEED);
			assert_eq!(<Runtime as pallet_xyk::Config>::Currency::free_balance(asset_a, &user), INITIAL_ASSET_BALANCE + SELL_INTENTION_AMOUNT);
		}
	}

	on_finalize_sells_no_matches {
		let t in 0 .. 100; // Intention component

		let asset_a = register_asset(b"ASSETA".to_vec(), 1_000_000).map_err(|_| BenchmarkError::Stop("Failed to register asset"))?;
		let asset_b = register_asset(b"ASSETB".to_vec(), 1_000_000).map_err(|_| BenchmarkError::Stop("Failed to register asset"))?;

		let caller = create_funded_account("caller", 1, &[asset_a, asset_b]);

		let amount : Balance = 100_000_000_000_000;

		create_pool(caller, asset_a, asset_b, amount, Price::from(10));

		for idx in 0 .. t {
			let user = create_funded_account("user", idx + 100, &[asset_a, asset_b]);
			Exchange::sell(
				RawOrigin::Signed(user.clone()).into(),
				asset_a,
				asset_b,
				SELL_INTENTION_AMOUNT,
				SELL_INTENTION_LIMIT,
				false,
			)?;
		}

		assert_eq!(Exchange::get_intentions_count((asset_a, asset_b)), t);

	}: {  Exchange::on_finalize(t); }
	verify {
		assert_eq!(Exchange::get_intentions_count((asset_a, asset_b)), 0);
		for idx in 0..t  {
			let user: AccountId = account("user", idx + 100, SEED);
			assert_eq!(<Runtime as pallet_xyk::Config>::Currency::free_balance(asset_a, &user), INITIAL_ASSET_BALANCE - SELL_INTENTION_AMOUNT);
		}
	}

	sell_extrinsic {
		let asset_a = register_asset(b"ASSETA".to_vec(), 1_000_000).map_err(|_| BenchmarkError::Stop("Failed to register asset"))?;
		let asset_b = register_asset(b"ASSETB".to_vec(), 1_000_000).map_err(|_| BenchmarkError::Stop("Failed to register asset"))?;

		let creator = create_funded_account("caller", 100, &[asset_a, asset_b]);
		let seller = create_funded_account("seller", 101, &[asset_a, asset_b]);

		let amount : Balance = 10_000_000_000;
		let min_bought : Balance = 1_000;
		let discount = false;

		create_pool(creator, asset_a, asset_b, amount, Price::from(1));

	}: { XYK::sell(RawOrigin::Signed(seller.clone()).into(), asset_a, asset_b, 1_000_000_000, min_bought, false)?; }
	verify {
		assert_eq!(<Runtime as pallet_xyk::Config>::Currency::free_balance(asset_a, &seller), 999_999_000_000_000);
		assert_eq!(<Runtime as pallet_xyk::Config>::Currency::free_balance(asset_b, &seller), 1000000907272729);
	}

	on_finalize_for_one_sell_extrinsic {
		let asset_a = register_asset(b"ASSETA".to_vec(), 1_000_000).map_err(|_| BenchmarkError::Stop("Failed to register asset"))?;
		let asset_b = register_asset(b"ASSETB".to_vec(), 1_000_000).map_err(|_| BenchmarkError::Stop("Failed to register asset"))?;

		let creator = create_funded_account("caller", 100, &[asset_a, asset_b]);
		let seller = create_funded_account("seller", 101, &[asset_a, asset_b]);

		let amount : Balance = 10_000_000_000;
		let discount = false;

		create_pool(creator, asset_a, asset_b, amount, Price::from(1));

		Exchange::sell(
			RawOrigin::Signed(seller.clone()).into(),
			asset_a,
			asset_b,
			SELL_INTENTION_AMOUNT,
			SELL_INTENTION_LIMIT,
			false,
		)?;

		assert_eq!(Exchange::get_intentions_count((asset_a, asset_b)), 1);

	}: {  Exchange::on_finalize(1u32); }
	verify {
		assert_eq!(Exchange::get_intentions_count((asset_a, asset_b)), 0);
		assert_eq!(<Runtime as pallet_xyk::Config>::Currency::free_balance(asset_a, &seller), 999_999_000_000_000);
		assert_eq!(<Runtime as pallet_xyk::Config>::Currency::free_balance(asset_b, &seller), 1000000907272729);
	}

	buy_extrinsic {
		let asset_a = register_asset(b"ASSETA".to_vec(), 1_000_000).map_err(|_| BenchmarkError::Stop("Failed to register asset"))?;
		let asset_b = register_asset(b"ASSETB".to_vec(), 1_000_000).map_err(|_| BenchmarkError::Stop("Failed to register asset"))?;

		let creator = create_funded_account("caller", 100, &[asset_a, asset_b]);
		let buyer = create_funded_account("buyer", 101, &[asset_a, asset_b]);

		let amount : Balance = 10_000_000_000;
		let max_sold: Balance = 2_000_000_000;
		let discount = false;

		create_pool(creator, asset_a, asset_b, amount, Price::from(1));

	}: { XYK::buy(RawOrigin::Signed(buyer.clone()).into(), asset_a, asset_b, 1_000_000_000, max_sold, false)?; }
	verify {
		assert_eq!(<Runtime as pallet_xyk::Config>::Currency::free_balance(asset_a, &buyer), 1000001000000000);
		assert_eq!(<Runtime as pallet_xyk::Config>::Currency::free_balance(asset_b, &buyer), 999998886666666);
	}

	on_finalize_for_one_buy_extrinsic {
		let t:u32 = 5;

		let asset_a = register_asset(b"ASSETA".to_vec(), 1_000_000).map_err(|_| BenchmarkError::Stop("Failed to register asset"))?;
		let asset_b = register_asset(b"ASSETB".to_vec(), 1_000_000).map_err(|_| BenchmarkError::Stop("Failed to register asset"))?;

		let creator = create_funded_account("caller", 100, &[asset_a, asset_b]);
		let buyer = create_funded_account("buyer", 101, &[asset_a, asset_b]);

		let amount : Balance = 10_000_000_000;
		let max_sold: Balance = 2_000_000_000;
		let discount = false;

		create_pool(creator, asset_a, asset_b, amount, Price::from(1));

		Exchange::buy(
			RawOrigin::Signed(buyer.clone()).into(),
			asset_a,
			asset_b,
			1_000_000_000,
			max_sold,
			false,
		)?;

		assert_eq!(Exchange::get_intentions_count((asset_a, asset_b)), 1);

	}: {  Exchange::on_finalize(t); }
	verify {
		assert_eq!(Exchange::get_intentions_count((asset_a, asset_b)), 0);
		assert_eq!(<Runtime as pallet_xyk::Config>::Currency::free_balance(asset_a, &buyer), 1000001000000000);
		assert_eq!(<Runtime as pallet_xyk::Config>::Currency::free_balance(asset_b, &buyer), 999998886666666);
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use orml_benchmarking::impl_benchmark_test_suite;

	fn new_test_ext() -> sp_io::TestExternalities {
		frame_system::GenesisConfig::default()
			.build_storage::<crate::Runtime>()
			.unwrap()
			.into()
	}

	impl_benchmark_test_suite!(new_test_ext(),);
}

pub const INTENTION_AMOUNTS: [u32; 1000] = [
	16632, 35979, 81119, 57522, 5693, 2576, 69100, 9570, 60695, 52377, 60305, 28056, 22327, 5495, 1453, 54753, 24070,
	19693, 63859, 8695, 17029, 88735, 2056, 59702, 33969, 12853, 86656, 53372, 16402, 6204, 14548, 16947, 39328, 90622,
	21390, 2483, 96858, 81070, 51984, 98238, 81843, 50046, 9583, 72790, 68617, 28207, 49282, 3888, 62416, 56705, 34981,
	85433, 35540, 81771, 18365, 10098, 64972, 96213, 40397, 5337, 86376, 64127, 83208, 29291, 86729, 6633, 84484,
	77150, 93673, 95659, 11130, 37542, 5922, 42414, 47076, 42272, 20817, 1017, 65304, 19828, 21687, 95466, 13339,
	38831, 65485, 19709, 100448, 30771, 4983, 99950, 27527, 59189, 96281, 33234, 9210, 33503, 85251, 24844, 90971,
	19902, 5657, 99352, 95985, 95077, 22289, 4794, 76402, 63558, 48745, 28485, 83418, 57292, 7999, 55717, 2573, 27310,
	17660, 35318, 95621, 90470, 51724, 30712, 61414, 59354, 74483, 33200, 15832, 9046, 55658, 90237, 76948, 94924,
	4989, 98015, 21228, 60174, 78447, 46564, 92237, 7495, 6291, 84548, 6724, 46581, 42274, 34910, 23420, 100064, 2722,
	53591, 12209, 20532, 57305, 1301, 92267, 53844, 22871, 72195, 84389, 48418, 28641, 65082, 11012, 53460, 50083,
	32029, 73700, 94710, 15802, 64829, 44311, 82058, 57846, 57274, 45543, 26546, 95897, 28804, 100821, 7163, 50381,
	57196, 82760, 19162, 92867, 42282, 72970, 65778, 16794, 37027, 11879, 33239, 77717, 85750, 6464, 4589, 45378,
	54583, 33679, 31904, 66781, 15031, 31105, 32148, 12587, 71278, 17308, 33890, 30367, 76961, 98537, 70151, 71278,
	98615, 46826, 60714, 7041, 22210, 94422, 83894, 99556, 12028, 60571, 93188, 11723, 80063, 98073, 8940, 96109,
	41070, 16398, 15116, 64945, 29197, 9007, 65456, 17424, 67230, 83748, 59392, 21683, 9719, 14331, 79755, 55418,
	80950, 99294, 99364, 53959, 70418, 7332, 33381, 71112, 73138, 52471, 64464, 99486, 28652, 55450, 100646, 26897,
	93552, 25540, 92190, 20481, 90173, 69253, 5751, 75374, 59148, 67788, 8500, 51360, 81961, 50901, 92356, 63942,
	31110, 53209, 21610, 2237, 85707, 80042, 39585, 94960, 15874, 80596, 41702, 95307, 72807, 52997, 65633, 35596,
	98488, 34385, 17279, 50263, 30478, 89522, 21723, 81306, 61360, 27218, 68819, 94707, 100378, 53167, 78548, 95137,
	61336, 72510, 86988, 15700, 24013, 67766, 16652, 45105, 10513, 26666, 40190, 40886, 54296, 6693, 8809, 93282,
	98093, 40016, 56603, 58041, 74217, 31022, 9059, 36036, 24419, 61313, 51807, 66865, 72821, 49998, 72484, 98417,
	52482, 77676, 14461, 50207, 14447, 38198, 52357, 43533, 2810, 93069, 59533, 81597, 67125, 6226, 17807, 46456,
	90919, 65776, 18173, 87063, 95293, 76115, 19836, 25491, 21125, 70300, 72821, 49489, 26119, 63389, 58352, 84340,
	77125, 32157, 69456, 33274, 99572, 84012, 34246, 11315, 73639, 90716, 38754, 83543, 36028, 41527, 23141, 2195,
	90149, 37075, 60810, 50260, 90095, 87354, 33032, 75541, 24645, 94985, 74077, 10566, 91902, 95354, 27190, 89743,
	52180, 70104, 7808, 3703, 78772, 42704, 11244, 78132, 61673, 93986, 82515, 68458, 14673, 59017, 36819, 45752,
	50893, 64823, 65510, 23340, 66085, 81861, 73696, 99567, 65182, 70751, 62825, 21479, 46503, 80570, 23654, 52733,
	28496, 16201, 64035, 36303, 80383, 45347, 4964, 59884, 97285, 42565, 65966, 62495, 45117, 81108, 25671, 36917,
	96692, 44457, 92339, 48324, 2375, 17179, 32661, 76254, 48617, 53591, 20112, 43653, 17268, 63307, 50573, 53750,
	68756, 70267, 22909, 96213, 24137, 17266, 57127, 21126, 56633, 30556, 95171, 55043, 93997, 16747, 84811, 7437,
	54097, 98599, 63594, 56461, 57158, 69095, 6298, 19094, 11402, 51610, 99084, 77844, 68896, 69378, 18056, 61379,
	30447, 96122, 63476, 40961, 81569, 54395, 60763, 2401, 27842, 5852, 77156, 30845, 33866, 38290, 68956, 27844,
	16492, 66863, 54523, 10525, 49038, 80560, 1809, 17512, 55476, 64058, 88722, 24365, 34282, 3393, 9709, 37369, 95246,
	54585, 36195, 83016, 13202, 25806, 13733, 89764, 40238, 34512, 67427, 57887, 91659, 32894, 53343, 67129, 83774,
	54472, 81820, 34166, 78473, 71640, 28164, 38972, 61346, 72226, 2544, 48359, 82224, 24696, 82291, 27573, 14224,
	3345, 26753, 42019, 66751, 6435, 85160, 57693, 34190, 23899, 50433, 90966, 48024, 79804, 94886, 53645, 58097, 4955,
	17710, 75783, 68023, 35342, 42709, 20340, 14194, 88921, 40479, 19809, 78043, 65547, 80662, 27292, 43014, 68916,
	40663, 82225, 56969, 93943, 94069, 33802, 53637, 55230, 71039, 28537, 98536, 84093, 90059, 62228, 56439, 3738,
	86920, 51590, 1346, 18875, 74918, 55028, 68096, 67013, 42738, 44450, 46604, 23026, 32446, 27699, 23528, 46959,
	43394, 9712, 57430, 8442, 28173, 20771, 78700, 70390, 100104, 24551, 2456, 17222, 82471, 42915, 74132, 10582,
	27546, 6654, 14573, 76713, 75732, 5940, 27056, 51558, 2046, 20098, 67034, 57882, 100132, 60000, 99723, 86290,
	10533, 21991, 42412, 91964, 30858, 24290, 97202, 98194, 17508, 45566, 98115, 29462, 46802, 49815, 54842, 22939,
	31065, 43569, 36056, 8263, 1881, 11203, 51802, 35812, 66901, 68322, 68843, 26453, 43986, 20030, 7762, 69412,
	100304, 87239, 30879, 59093, 65866, 47918, 47569, 16196, 85937, 99572, 49230, 23054, 88228, 61035, 77578, 77986,
	91012, 24734, 53580, 16077, 82527, 2288, 24036, 43200, 65140, 16209, 11013, 72746, 46101, 62181, 27422, 85776,
	43468, 47012, 6123, 58767, 8570, 71187, 69092, 6552, 9408, 43701, 96923, 30538, 99078, 79500, 54839, 1428, 86908,
	12585, 63330, 9011, 77704, 62523, 55056, 77129, 78084, 49656, 29780, 38891, 13544, 98055, 38163, 92397, 43495,
	31857, 49942, 75365, 100562, 77573, 53596, 90816, 3868, 84812, 4089, 96455, 10865, 5941, 43883, 16509, 74714,
	33257, 56539, 29620, 13175, 22355, 76156, 90752, 9585, 71664, 25697, 79449, 23134, 86809, 69387, 97027, 39391,
	94370, 4829, 48195, 99864, 77133, 29275, 100318, 94132, 9960, 69764, 62162, 60213, 64578, 55372, 90445, 96001,
	17529, 50674, 92425, 41782, 42369, 57322, 18586, 65417, 85476, 15028, 88641, 67411, 64975, 43254, 69350, 44285,
	45727, 28651, 11259, 69078, 7252, 75106, 30647, 36698, 47826, 48857, 10854, 62240, 5694, 4464, 16275, 79739, 23210,
	23509, 31898, 91786, 82808, 24118, 2594, 6416, 62619, 95972, 39495, 36907, 55801, 66199, 93959, 23769, 21508,
	11970, 32751, 2461, 20836, 57289, 17959, 2369, 67080, 23734, 65489, 98048, 57542, 2217, 5343, 65785, 17021, 61037,
	71671, 11014, 68367, 1640, 28619, 61834, 24030, 76690, 28257, 75771, 91982, 83647, 1290, 50046, 93019, 58035,
	15838, 67833, 29256, 12284, 49289, 1117, 47086, 6000, 76178, 48988, 46661, 96370, 76048, 98778, 3445, 88113, 8471,
	22901, 22711, 62200, 40671, 68711, 72877, 48249, 93428, 38028, 66474, 41503, 68566, 17909, 72311, 36304, 19563,
	45532, 46486, 58282, 22859, 22359, 45521, 6509, 62128, 72955, 63291, 55312, 56427, 79898, 13355, 71804, 71015,
	30137, 85783, 95802, 89555, 87950, 21863, 23131, 14840, 63942, 52599, 10445, 40714, 28213, 99425, 94177, 55316,
	66160, 88433, 71057, 60371, 43351, 74621, 69724, 69878, 79892, 48071, 44284, 13989, 51216, 1154, 82803, 71961,
	37722, 35975, 8774, 5896, 80719, 37746, 45200, 65245, 42642, 46341, 97962, 33938, 22537, 97921, 43772, 62439,
	31821, 42232, 71898, 26641, 77655, 90261, 31221, 92571, 82241, 66541, 6326, 56434, 88576,
];
