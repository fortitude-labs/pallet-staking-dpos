#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet_session::Pallet as SessionPallet;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

pub(crate) const LOG_TARGET: &str = "runtime::dpos";

// syntactic sugar for logging.
#[macro_export]
macro_rules! log {
	($level:tt, $patter:expr $(, $values:expr)* $(,)?) => {
		log::$level!(
			target: crate::LOG_TARGET,
			concat!("[{:?}] 💸 ", $patter), <frame_system::Pallet<T>>::block_number() $(, $values)*
		)
	};
}

#[frame_support::pallet]
pub mod pallet {
	use frame_support::pallet_prelude::*;
	use frame_support::traits::{
		Currency, LockIdentifier, LockableCurrency, ReservableCurrency, WithdrawReasons,
	};
	use frame_system::{ensure_root, pallet_prelude::*};
	use sp_staking::SessionIndex;
	use sp_std::vec::Vec;

	const STAKING_ID: LockIdentifier = *b"staking ";

	/// The balance type of this pallet.
	pub type BalanceOf<T> = <T as Config>::CurrencyBalance;

	/// Configure the pallet by specifying the parameters and types on which it depends.
	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// The staking balance.
		type Currency: LockableCurrency<
				Self::AccountId,
				Moment = BlockNumberFor<Self>,  // usually a block number
				Balance = Self::CurrencyBalance,
			> + ReservableCurrency<Self::AccountId>;
		/// Just the `Currency::Balance` type; we have this item to allow us to constrain it to `From<u64>`.
		type CurrencyBalance: sp_runtime::traits::AtLeast32BitUnsigned
			+ codec::FullCodec
			+ Copy
			+ MaybeSerializeDeserialize
			+ sp_std::fmt::Debug
			+ Default
			+ From<u64>
			+ TypeInfo
			+ MaxEncodedLen;

		/// Because this pallet emits events, it depends on the runtime's definition of an event.
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

		#[pallet::constant]
		type MinimumValidatorCount: Get<u32>;

		#[pallet::constant]
		type MaximumValidatorCount: Get<u32>;
	}

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	/// Map from all locked "stash" accounts to the controller account.
	#[pallet::storage]
	#[pallet::getter(fn bonded)]
	pub type Bonded<T: Config> = StorageMap<_, Twox64Concat, T::AccountId, BalanceOf<T>>;

	/// DoubleMap from all the staked per user.
	#[pallet::storage]
	#[pallet::getter(fn user_staked)]
	pub type UserStaked<T: Config> =
		StorageDoubleMap<_, Twox64Concat, T::AccountId, Twox64Concat, T::AccountId, BalanceOf<T>>;

	/// Map from all stake for validators.
	#[pallet::storage]
	#[pallet::getter(fn staked)]
	pub type Staked<T: Config> = StorageMap<_, Twox64Concat, T::AccountId, BalanceOf<T>>;

	/// Minimum number of validators.
	#[pallet::storage]
	#[pallet::getter(fn minimum_validator_count)]
	pub type MinimumValidatorCount<T> =
		StorageValue<_, u32, ValueQuery, <T as Config>::MinimumValidatorCount>;

	/// Maximum number of validators.
	#[pallet::storage]
	#[pallet::getter(fn maximum_validator_count)]
	pub type MaximumValidatorCount<T> =
		StorageValue<_, u32, ValueQuery, <T as Config>::MaximumValidatorCount>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// An account has bonded this amount.
		Bonded(T::AccountId, BalanceOf<T>),
		/// An account has unbonded
		Unbonded(T::AccountId),
		// An user voted
		Voted(T::AccountId, T::AccountId, BalanceOf<T>),
		// An user unvoted
		Unvoted(T::AccountId, T::AccountId),
	}

	// Errors inform users that something went wrong.
	#[pallet::error]
	pub enum Error<T> {
		/// Not a stash account.
		NotStash,
		/// Stash is already bonded.
		AlreadyBonded,
		/// Cannot have a validator or nominator role, with value less than the minimum defined by
		/// governance (see `MinValidatorBond` and `MinNominatorBond`). If unbonding is the
		/// intention, `chill` first to remove one's role as validator/nominator.
		InsufficientBond,
		BadState,
		/// Invalid number of validators.
		InvalidNumberOfValidators,
		AlreadyVoted,
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::call_index(0)]  // Assigning an explicit call index
		#[pallet::weight(T::DbWeight::get().reads_writes(1, 1).saturating_add(10_000.into()))]
		pub fn bond(
			origin: OriginFor<T>,
			#[pallet::compact] value: BalanceOf<T>,
		) -> DispatchResult {
			let stash = ensure_signed(origin)?;

			if <Bonded<T>>::contains_key(&stash) {
				return Err(Error::<T>::AlreadyBonded.into());
			}

			// Reject a bond which is considered to be _dust_.
			if value < T::Currency::minimum_balance() {
				return Err(Error::<T>::InsufficientBond.into());
			}

			frame_system::Pallet::<T>::inc_consumers(&stash).map_err(|_| Error::<T>::BadState)?;

			let stash_balance = T::Currency::free_balance(&stash);
			let value = value.min(stash_balance);
			T::Currency::set_lock(STAKING_ID, &stash, value, WithdrawReasons::all());

			<Bonded<T>>::insert(&stash, value);

			Self::deposit_event(Event::<T>::Bonded(stash, value));

			Ok(())
		}

		#[pallet::call_index(1)]  // Assigning an explicit call index
		#[pallet::weight(T::DbWeight::get().reads_writes(1, 1).saturating_add(10_000.into()))]
		pub fn unbond(origin: OriginFor<T>) -> DispatchResult {
			let stash = ensure_signed(origin)?;

			if None == <Bonded<T>>::take(&stash) {
				return Err(Error::<T>::NotStash.into());
			}

			T::Currency::remove_lock(STAKING_ID, &stash);

			frame_system::Pallet::<T>::dec_consumers(&stash);

			Self::deposit_event(Event::<T>::Unbonded(stash));

			Ok(())
		}

		#[pallet::call_index(2)]  // Assigning an explicit call index
		#[pallet::weight(T::DbWeight::get().reads_writes(1, 1).saturating_add(10_000.into()))]
		pub fn set_minimum_validator_count(origin: OriginFor<T>, value: u32) -> DispatchResult {
			ensure_root(origin)?;
			if value == 0 || value > <MaximumValidatorCount<T>>::get() {
				return Err(Error::<T>::InvalidNumberOfValidators.into());
			}
			<MinimumValidatorCount<T>>::set(value);
			Ok(())
		}

		#[pallet::call_index(3)]  // Assigning an explicit call index
		#[pallet::weight(T::DbWeight::get().reads_writes(1, 1).saturating_add(10_000.into()))]
		pub fn set_maximum_validator_count(origin: OriginFor<T>, value: u32) -> DispatchResult {
			ensure_root(origin)?;
			if value < <MinimumValidatorCount<T>>::get() {
				return Err(Error::<T>::InvalidNumberOfValidators.into());
			}
			<MaximumValidatorCount<T>>::set(value);
			Ok(())
		}

		#[pallet::call_index(4)]  // Assigning an explicit call index
		#[pallet::weight(T::DbWeight::get().reads_writes(1, 1).saturating_add(10_000.into()))]
		pub fn vote(
			origin: OriginFor<T>,
			target: T::AccountId,
			value: BalanceOf<T>,
		) -> DispatchResult {
			let voter = ensure_signed(origin)?;

			if <UserStaked<T>>::contains_key(&voter, &target) {
				return Err(Error::<T>::AlreadyVoted.into());
			}

			T::Currency::reserve(&voter, value)?;

			<UserStaked<T>>::insert(&voter, &target, &value);

			let stake = <Staked<T>>::get(&target);
			match stake {
				Some(x) => <Staked<T>>::insert(&target, x + value),
				None => <Staked<T>>::insert(&target, value),
			}

			Self::deposit_event(Event::<T>::Voted(voter, target, value));

			Ok(())
		}

		#[pallet::call_index(5)]  // Assigning an explicit call index
		#[pallet::weight(T::DbWeight::get().reads_writes(1, 1).saturating_add(10_000.into()))]
		pub fn unvote(origin: OriginFor<T>, target: T::AccountId) -> DispatchResult {
			let voter = ensure_signed(origin)?;

			if !<UserStaked<T>>::contains_key(&voter, &target) {
				return Ok(());
			}

			let staked = <UserStaked<T>>::take(&voter, &target).expect("alredy checked");

			T::Currency::unreserve(&voter, staked);

			let stake = <Staked<T>>::get(&target).expect("already checked");
			<Staked<T>>::insert(&target, stake - staked);

			Self::deposit_event(Event::<T>::Unvoted(voter, target));

			Ok(())
		}
	}

	impl<T: Config> pallet_session::SessionManager<T::AccountId> for Pallet<T> {
		fn new_session(new_index: SessionIndex) -> Option<Vec<T::AccountId>> {
			log!(debug, "planning new session {}", new_index);

			let min_validator_count = <MinimumValidatorCount<T>>::get();
			let max_validator_count = <MaximumValidatorCount<T>>::get();

			let mut validators: Vec<(T::AccountId, BalanceOf<T>)> = <Bonded<T>>::iter().collect();
			if validators.len() < min_validator_count as usize {
				log!(
					warn,
					"validators count {} less than the minimum {} ... skip",
					validators.len(),
					min_validator_count
				);
				return None;
			}

			for i in validators.iter_mut() {
				let stake = <Staked<T>>::get(&i.0);
				match stake {
					Some(x) => i.1 = i.1 + x,
					None => (),
				};
			}

			validators.sort_by(|a, b| b.1.cmp(&a.1));
			validators.truncate(max_validator_count as usize);

			let mut winners: Vec<T::AccountId> = Vec::new();
			for i in validators {
				winners.push(i.0);
			}

			Some(winners)
		}

		fn end_session(end_index: SessionIndex) {
			log!(debug, "ending session {}", end_index);
		}

		fn start_session(start_index: SessionIndex) {
			log!(debug, "starting session {}", start_index);
		}
	}
}
