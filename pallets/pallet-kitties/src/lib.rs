#![cfg_attr(not(feature = "std"), no_std)]

/// Edit this file to define custom logic or remove it if it is not needed.
/// Learn more about FRAME and the core library of Substrate FRAME pallets:
/// <https://docs.substrate.io/v3/runtime/frame>
pub use pallet::*;

// #[cfg(test)]
// mod mock;

// #[cfg(test)]
// mod tests;

// #[cfg(feature = "runtime-benchmarks")]
// mod benchmarking;

use frame_support::pallet_prelude::*;
use frame_system::pallet_prelude::*;
use frame_support::inherent::Vec;
use frame_support::dispatch::fmt;
use frame_support::traits::{Currency, Get};
use frame_support::{traits::Randomness};
use frame_support::sp_runtime::traits::{Hash}

use pallet_timestamp::{self as timestamp};
use pallet_kitty_limit::KittyLimit;

type BalanceOf<T> = <<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;
#[frame_support::pallet]
pub mod pallet {
	pub use super::*;

	#[derive(TypeInfo, Default, Encode, Decode)]
	#[scale_info(skip_type_params(T))]
	pub struct Kitty<T:Config> {
		id: u32,
		dna: Vec<u8>,
		price: BalanceOf<T>,
		gender: Gender,
		owner: T::AccountId,
		created_date: <T as pallet_timestamp::Config>::Moment,
	}
	pub type Id = u32;

	#[derive(TypeInfo, Encode ,Decode, Debug)]
	pub enum Gender {
		Male,
		Female,
	}

	impl Default for Gender{
		fn default()-> Self{
			Gender::Male
		}
	}

	/// Configure the pallet by specifying the parameters and types on which it depends.
	#[pallet::config]
	pub trait Config: frame_system::Config + timestamp::Config {
		/// Because this pallet emits events, it depends on the runtime's definition of an event.
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
		type Currency: Currency<Self::AccountId>;
		type KittyRandomness: Randomness<Self::Hash, Self::BlockNumber>;
		type KittyLimit:: Randomness<Self::Hash, Self::BlockNumber>;
	}

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(_);

	// The pallet's runtime storage items.
	// https://docs.substrate.io/v3/runtime/storage
	#[pallet::storage]
	// Learn more about declaring storage items:
	// https://docs.substrate.io/v3/runtime/storage#declaring-storage-items
	pub type KittyIndex<T> = StorageValue<_, Id, ValueQuery>;


	#[pallet::storage]
	// Learn more about declaring storage items:
	// https://docs.substrate.io/v3/runtime/storage#declaring-storage-items
	// Kitty Storaga
	pub(super) type KittyList<T: Config> = StorageMap<_, Blake2_128Concat, Id, Kitty<T>, OptionQuery>;

	#[pallet::storage]
	// Kitty is owning
	pub(super) type KittyOwner<T: Config> = StorageMap<_, Blake2_128Concat, T::AccountId, Vec<Kitty<T>>, OptionQuery>;

	// Pallets use events to inform users when important changes are made.
	// https://docs.substrate.io/v3/runtime/events-and-errors
	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Event documentation should end with an array that provides descriptive names for event
		/// parameters. [something, who]
		KittyStored(Vec<u8>, T::AccountId),
		TransferKittyStored(T::AccountId, u32, T::AccountId),
		SetLimitKittyStored,
	}

	// Errors inform users that something went wrong.
	#[pallet::error]
	pub enum Error<T> {
		/// Error names should be descriptive.
		NoneValue,
		/// Errors should have helpful documentation associated with them.
		StorageOverflow,
		/// Error don't exits kitty.
		NotExistKitty,
		NotOwner,
		OverKittyLimit,
	}

	// Dispatchable functions allows users to interact with the pallet and invoke state changes.
	// These functions materialize as "extrinsics", which are often compared to transactions.
	// Dispatchable functions must be annotated with a weight and must return a DispatchResult.
	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// An example dispatchable that takes a singles value as a parameter, writes the value to
		/// storage and emits an event. This function must be dispatched by a signed extrinsic.
		#[pallet::weight(10_000 + T::DbWeight::get().writes(1))]
		pub fn create_kitty(origin: OriginFor<T>, dna: Vec<u8>, price: u32 ) -> DispatchResult {
			// Check that the extrinsic was signed and get the signer.
			// This function will return an error if the extrinsic is not signed.
			// https://docs.substrate.io/v3/runtime/origins
			let who = ensure_signed(origin)?;

			// Check paramater
			let gender = Self::gen_gender(dna.clone())?;

			// get Kitty ID
			let mut current_id = <KittyIndex<T>>::get();
			// increa id
			current_id +=1;

			let kitty = Kitty {
				id: current_id,
				dna: dna.clone(),
				price: price,
				gender: gender,
				owner: who.clone(),
			};

			// Update storage.
			
			<KittyList<T>>::insert(current_id, &kitty);
			
			KittyIndex::<T>::put(current_id);

			// Update Amount of kitty owning
			let kitty_owning = <KittyOwner<T>>::get(who.clone());
			let mut kitty_owning_vec = match kitty_owning{
				None => Vec::new(),
				_	 =>	<KittyOwner<T>>::get(who.clone()).unwrap(),
			};
			kitty_owning_vec.push(kitty);
			<KittyOwner<T>>::insert(who.clone(), kitty_owning_vec);

			// Emit an event.
			Self::deposit_event(Event::KittyStored(dna, who));
			// Return a successful DispatchResultWithPostInfo
			Ok(())
		}

		#[pallet::weight(10_000 + T::DbWeight::get().writes(1))]
		pub fn transfer_kitty(origin: OriginFor<T>, kitty_id: u32, destination: T::AccountId) -> DispatchResult {
			// Check that the extrinsic was signed and get the signer.
			// This function will return an error if the extrinsic is not signed.
			// https://docs.substrate.io/v3/runtime/origins
			let who = ensure_signed(origin)?;
			log::info!("create_kitty: {:?}", T::KittyLimit::get() as usize);

			//generate random hash:
			let dna = Self::random_hash(&who);
			
			let kitty_op = <KittyList<T>>::get(kitty_id);
			ensure!(kitty_op.is_some(), Error::<T>::NotExistKitty);
			let mut kitty = kitty_op.unwrap();

			// Update storage.
			
			// Update Owner Kitty
			let old_owner = kitty.owner;
			kitty.owner = destination.clone();
			// Update Kitty list
			<KittyList<T>>::insert(kitty_id, &kitty);

			// Update Kitty_Owner storage
			// Old Owner
			let old_owner_kitty_op = <KittyOwner<T>>::get(&old_owner);
			let mut old_owner_kitty_vec = match old_owner_kitty_op{
				None => Vec::new(),
				_	 =>	old_owner_kitty_op.unwrap(),
			};
			// remove kitty in list owner
			if let Some(index) = old_owner_kitty_vec.iter().position(|value| value.id == kitty_id) {
				old_owner_kitty_vec.swap_remove(index);
			}
			<KittyOwner<T>>::insert(old_owner, old_owner_kitty_vec);

			let new_owner_kitty_op = <KittyOwner<T>>::get(&destination);
			let mut new_owner_kitty_vec = match new_owner_kitty_op{
				None => Vec::new(),
				_	 =>	new_owner_kitty_op.unwrap(),
			};
			new_owner_kitty_vec.push(kitty);
			<KittyOwner<T>>::insert(destination.clone(), new_owner_kitty_vec);

			// Emit an event.
			Self::deposit_event(Event::TransferKittyStored(who, kitty_id, destination));
			// Return a successful DispatchResultWithPostInfo
			Ok(())
		}

		#[pallet::weight(10_000 + T::DbWeight::get().writes(1))]
		pub fn set_limit_kitty(origin: OriginFor<T>, value: u32) -> DispatchResult {
			let _ = ensure_signed(origin)?;
			let _limit = T::KittyLimit::set(value);
			// Emit an event.
			Self::deposit_event(Event::SetLimitKittySuccess);

			Ok(())
		}

	}
}

// helper function
impl<T> Pallet<T> {
	fn gen_gender(dna: Vec<u8>) -> Result<Gender,Error<T>>{
		let mut res = Gender::Male;
		if dna.len() % 2 !=0 {
			res = Gender::Female;
		}
		Ok(res)
	}
}