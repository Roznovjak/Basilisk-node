#![cfg_attr(not(feature = "std"), no_std)]
#![allow(clippy::unused_unit)]
#![allow(clippy::upper_case_acronyms)]

use codec::{Decode, Encode};
use frame_support::{
	dispatch::{DispatchError, DispatchResult, DispatchResultWithPostInfo},
	ensure,
	traits::{Currency, ExistenceRequirement},
};
use frame_system::ensure_signed;
use orml_utilities::with_transaction_result;
use sp_runtime::{
	traits::{StaticLookup, Zero},
	RuntimeDebug,
};
use sp_std::vec::Vec;
use weights::WeightInfo;

mod benchmarking;
pub mod weights;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

pub type Balance = u128;
pub type BalanceOf<T> = <<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;
pub type ClassData = Vec<u8>;
pub type TokenIdOf<T> = <T as orml_nft::Config>::TokenId;
pub type ClassIdOf<T> = <T as orml_nft::Config>::ClassId;

#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
#[derive(Encode, Decode, RuntimeDebug, Clone, PartialEq, Eq)]
pub struct TokenData {
	pub locked: bool,
}

// Re-export pallet items so that they can be accessed from the crate namespace.
pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_support::pallet_prelude::*;
	use frame_system::pallet_prelude::OriginFor;

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	/// Stores prices for each NFT class
	#[pallet::storage]
	#[pallet::getter(fn class_item_price)]
	pub type ClassItemPrice<T: Config> = StorageMap<_, Blake2_128Concat, ClassIdOf<T>, BalanceOf<T>, ValueQuery>;

	#[pallet::config]
	pub trait Config: frame_system::Config + orml_nft::Config<ClassData = ClassData, TokenData = TokenData> {
		type Currency: Currency<Self::AccountId>;
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
		type WeightInfo: WeightInfo;
		// This type is needed to convert from Currency to Balance
		type CurrencyBalance: From<Balance>
			+ Into<<Self::Currency as Currency<<Self as frame_system::Config>::AccountId>>::Balance>;
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::weight(<T as Config>::WeightInfo::create_class())]
		pub fn create_class(
			origin: OriginFor<T>,
			metadata: Vec<u8>,
			data: T::ClassData,
			price: BalanceOf<T>,
		) -> DispatchResultWithPostInfo {
			let sender = ensure_signed(origin)?;
			let class_id = orml_nft::Pallet::<T>::create_class(&sender, metadata, data)?;
			ClassItemPrice::<T>::insert(class_id, price);
			Self::deposit_event(Event::NFTTokenClassCreated(sender, class_id));
			Ok(().into())
		}

		#[pallet::weight(<T as Config>::WeightInfo::mint(*quantity))]
		pub fn mint(
			origin: OriginFor<T>,
			class_id: ClassIdOf<T>,
			metadata: Vec<u8>,
			token_data: TokenData,
			quantity: u32,
		) -> DispatchResultWithPostInfo {
			let sender = ensure_signed(origin)?;
			ensure!(quantity > Zero::zero(), Error::<T>::InvalidQuantity);
			let class_info = orml_nft::Pallet::<T>::classes(class_id).ok_or(Error::<T>::ClassNotFound)?;
			ensure!(sender == class_info.owner, Error::<T>::NotClassOwner);
			let data = token_data;
			for _ in 0..quantity {
				orml_nft::Pallet::<T>::mint(&sender, class_id, metadata.clone(), data.clone())?;
			}
			Self::deposit_event(Event::NFTTokenMinted(sender, class_id, quantity));
			Ok(().into())
		}

		#[pallet::weight(<T as Config>::WeightInfo::transfer())]
		pub fn transfer(
			origin: OriginFor<T>,
			dest: <T::Lookup as StaticLookup>::Source,
			token: (ClassIdOf<T>, TokenIdOf<T>),
		) -> DispatchResultWithPostInfo {
			let sender = ensure_signed(origin)?;
			let to: T::AccountId = T::Lookup::lookup(dest)?;
			ensure!(sender != to, Error::<T>::CannotSendToSelf);
			let token_info = orml_nft::Pallet::<T>::tokens(token.0, token.1).ok_or(Error::<T>::TokenNotFound)?;
			ensure!(sender == token_info.owner, Error::<T>::NotTokenOwner);
			ensure!(!token_info.data.locked, Error::<T>::TokenLocked);
			orml_nft::Pallet::<T>::transfer(&sender, &to, token)?;
			Self::deposit_event(Event::NFTTokenTransferred(sender, to, token.0, token.1));
			Ok(().into())
		}

		#[pallet::weight(<T as Config>::WeightInfo::burn())]
		pub fn burn(origin: OriginFor<T>, token: (T::ClassId, T::TokenId)) -> DispatchResultWithPostInfo {
			let sender = ensure_signed(origin)?;
			let token_info = orml_nft::Pallet::<T>::tokens(token.0, token.1).ok_or(Error::<T>::TokenNotFound)?;
			ensure!(sender == token_info.owner, Error::<T>::NotTokenOwner);
			ensure!(!token_info.data.locked, Error::<T>::TokenLocked);
			orml_nft::Pallet::<T>::burn(&sender, token)?;
			Self::deposit_event(Event::NFTTokenBurned(sender, token.0, token.1));
			Ok(().into())
		}

		#[pallet::weight(<T as Config>::WeightInfo::destroy_class())]
		pub fn destroy_class(origin: OriginFor<T>, class_id: ClassIdOf<T>) -> DispatchResultWithPostInfo {
			let sender = ensure_signed(origin)?;
			let class_info = orml_nft::Pallet::<T>::classes(class_id).ok_or(Error::<T>::ClassNotFound)?;
			ensure!(sender == class_info.owner, Error::<T>::NotClassOwner);
			ensure!(class_info.total_issuance == Zero::zero(), Error::<T>::NonZeroIssuance);
			orml_nft::Pallet::<T>::destroy_class(&sender, class_id)?;
			Self::deposit_event(Event::NFTTokenClassDestroyed(sender, class_id));
			Ok(().into())
		}

		#[pallet::weight(<T as Config>::WeightInfo::buy_from_pool())]
		pub fn buy_from_pool(origin: OriginFor<T>, token: (ClassIdOf<T>, TokenIdOf<T>)) -> DispatchResult {
			let sender = ensure_signed(origin)?;
			let class_info = orml_nft::Pallet::<T>::classes(token.0).ok_or(Error::<T>::ClassNotFound)?;
			let token_info = orml_nft::Pallet::<T>::tokens(token.0, token.1).ok_or(Error::<T>::TokenNotFound)?;
			let price = Self::class_item_price(token.0);
			ensure!(class_info.owner == token_info.owner, Error::<T>::TokenAlreadyHasAnOwner);
			ensure!(sender != token_info.owner, Error::<T>::CannotBuyOwnToken);
			ensure!(!token_info.data.locked, Error::<T>::TokenLocked);

			with_transaction_result(|| {
				orml_nft::Pallet::<T>::transfer(&token_info.owner, &sender, token)?;
				T::Currency::transfer(&sender, &token_info.owner, price, ExistenceRequirement::KeepAlive)?;
				Self::deposit_event(Event::NFTBoughtFromPool(class_info.owner, sender, token.0, token.1));
				Ok(())
			})
		}

		#[pallet::weight(<T as Config>::WeightInfo::sell_to_pool())]
		pub fn sell_to_pool(origin: OriginFor<T>, token: (ClassIdOf<T>, TokenIdOf<T>)) -> DispatchResult {
			let sender = ensure_signed(origin)?;
			let class_info = orml_nft::Pallet::<T>::classes(token.0).ok_or(Error::<T>::ClassNotFound)?;
			let token_info = orml_nft::Pallet::<T>::tokens(token.0, token.1).ok_or(Error::<T>::TokenNotFound)?;
			let price = Self::class_item_price(token.0);
			ensure!(class_info.owner != token_info.owner, Error::<T>::CannotSellPoolToken);
			ensure!(sender == token_info.owner, Error::<T>::NotTokenOwner);
			ensure!(!token_info.data.locked, Error::<T>::TokenLocked);

			with_transaction_result(|| {
				orml_nft::Pallet::<T>::transfer(&sender, &class_info.owner, token)?;
				T::Currency::transfer(&class_info.owner, &sender, price, ExistenceRequirement::KeepAlive)?;
				Self::deposit_event(Event::NFTSoldToPool(sender, class_info.owner, token.0, token.1));
				Ok(())
			})
		}
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<T::BlockNumber> for Pallet<T> {}

	#[pallet::event]
	#[pallet::generate_deposit(pub(crate) fn deposit_event)]
	pub enum Event<T: Config> {
		NFTTokenClassCreated(T::AccountId, T::ClassId),
		NFTTokenMinted(T::AccountId, T::ClassId, u32),
		NFTTokenMintedLockToggled(T::AccountId, T::ClassId, T::TokenId, bool),
		NFTTokenTransferred(T::AccountId, T::AccountId, T::ClassId, T::TokenId),
		NFTTokenBurned(T::AccountId, T::ClassId, T::TokenId),
		NFTTokenClassDestroyed(T::AccountId, T::ClassId),
		NFTBoughtFromPool(T::AccountId, T::AccountId, T::ClassId, T::TokenId),
		NFTSoldToPool(T::AccountId, T::AccountId, T::ClassId, T::TokenId),
	}

	#[pallet::error]
	pub enum Error<T> {
		/// The class does not exist
		ClassNotFound,
		/// The token does not exist
		TokenNotFound,
		/// Not the class owner
		NotClassOwner,
		/// Not the token owner
		NotTokenOwner,
		/// The token class is not empty
		NonZeroIssuance,
		/// Token is currently locked
		TokenLocked,
		/// Quantity has to be greater than zero
		InvalidQuantity,
		/// A token can not be transferred to self
		CannotSendToSelf,
		/// A user cannot buy already owned token
		CannotBuyOwnToken,
		/// Token has been already bought from a pool
		TokenAlreadyHasAnOwner,
		/// A token still owned by class owner
		CannotSellPoolToken,
	}
}

impl<T: Config> Pallet<T> {
	pub fn is_owner(account: &T::AccountId, token: (T::ClassId, T::TokenId)) -> bool {
		orml_nft::Pallet::<T>::is_owner(account, token)
	}

	pub fn is_locked(token: (T::ClassId, T::TokenId)) -> Result<bool, DispatchError> {
		let token_info = orml_nft::Pallet::<T>::tokens(token.0, token.1).ok_or(Error::<T>::TokenNotFound)?;
		Ok(token_info.data.locked)
	}

	pub fn toggle_lock(account: &T::AccountId, token_id: (T::ClassId, T::TokenId)) -> DispatchResult {
		let _class_info = orml_nft::Pallet::<T>::classes(token_id.0).ok_or(Error::<T>::ClassNotFound)?;
		orml_nft::Tokens::<T>::mutate_exists(token_id.0, token_id.1, |token| -> DispatchResult {
			if let Some(ref mut token) = token {
				ensure!(*account == token.owner, Error::<T>::NotTokenOwner);
				token.data.locked ^= true; // Toggle
						   // fix clone
				Self::deposit_event(Event::NFTTokenMintedLockToggled(
					account.clone(),
					token_id.0,
					token_id.1,
					token.data.locked,
				));
			}
			Ok(())
		})
	}
}
