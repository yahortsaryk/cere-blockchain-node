//! # DDC Staking Pallet
//!
//! The DDC Staking pallet is used to manage funds at stake by DDC network maintainers.
//!
//! - [`Config`]
//! - [`Call`]
//! - [`Pallet`]
//!
//! ## GenesisConfig
//!
//! The DDC Staking pallet depends on the [`GenesisConfig`]. The
//! `GenesisConfig` is optional and allow to set some initial stakers in DDC.
#![feature(is_some_and)]
#![cfg_attr(not(feature = "std"), no_std)]
#![recursion_limit = "256"]

#[cfg(feature = "runtime-benchmarks")]
pub mod benchmarking;
#[cfg(any(feature = "runtime-benchmarks", test))]
pub mod testing_utils;

#[cfg(test)]
pub(crate) mod mock;
#[cfg(test)]
mod tests;

pub mod weights;
use core::fmt::Debug;

use codec::{Decode, Encode, HasCompact};
pub use ddc_primitives::{ClusterId, NodePubKey, NodeType};
use ddc_traits::{
	cluster::{ClusterCreator, ClusterVisitor, ClusterVisitorError},
	node::{NodeCreator, NodeVisitor},
	staking::{StakerCreator, StakingVisitor, StakingVisitorError},
};
#[cfg(feature = "std")]
use frame_support::assert_ok;
use frame_support::{
	pallet_prelude::*,
	parameter_types,
	traits::{Currency, DefensiveSaturating, LockIdentifier, LockableCurrency, WithdrawReasons},
	BoundedVec,
};
use frame_system::pallet_prelude::*;
pub use pallet::*;
use scale_info::TypeInfo;
use sp_runtime::{
	traits::{AtLeast32BitUnsigned, CheckedAdd, CheckedSub, Saturating, StaticLookup, Zero},
	RuntimeDebug, SaturatedConversion,
};
use sp_std::prelude::*;

use crate::weights::WeightInfo;

const DDC_STAKING_ID: LockIdentifier = *b"ddcstake"; // DDC maintainer's stake

/// The balance type of this pallet.
pub type BalanceOf<T> =
	<<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

parameter_types! {
	/// A limit to the number of pending unlocks an account may have in parallel.
	pub MaxUnlockingChunks: u32 = 32;
}

/// Just a Balance/BlockNumber tuple to encode when a chunk of funds will be unlocked.
#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, TypeInfo, MaxEncodedLen)]
#[scale_info(skip_type_params(T))]
pub struct UnlockChunk<Balance, BlockNumber>
where
	Balance: HasCompact + MaxEncodedLen,
	BlockNumber: HasCompact + MaxEncodedLen,
{
	/// Amount of funds to be unlocked.
	#[codec(compact)]
	value: Balance,
	/// Block number at which point it'll be unlocked.
	#[codec(compact)]
	block: BlockNumber,
}

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, TypeInfo, MaxEncodedLen)]
#[scale_info(skip_type_params(T))]
pub struct StakingLedger<AccountId, Balance, T>
where
	Balance: HasCompact + MaxEncodedLen,
	T: Config,
{
	/// The stash account whose balance is actually locked and at stake.
	pub stash: AccountId,
	/// The total amount of the stash's balance that we are currently accounting for.
	/// It's just `active` plus all the `unlocking` balances.
	#[codec(compact)]
	pub total: Balance,
	/// The total amount of the stash's balance that will be at stake in any forthcoming
	/// rounds.
	#[codec(compact)]
	pub active: Balance,
	/// Block number at which chilling will be allowed.
	pub chilling: Option<T::BlockNumber>,
	/// Any balance that is becoming free, which may eventually be transferred out of the stash
	/// (assuming it doesn't get slashed first). It is assumed that this will be treated as a first
	/// in, first out queue where the new (higher value) blocks get pushed on the back.
	pub unlocking: BoundedVec<UnlockChunk<Balance, T::BlockNumber>, MaxUnlockingChunks>,
}

impl<
		AccountId,
		Balance: HasCompact + Copy + Saturating + AtLeast32BitUnsigned + Zero + MaxEncodedLen + Debug,
		T: Config,
	> StakingLedger<AccountId, Balance, T>
{
	/// Initializes the default object using the given stash.
	pub fn default_from(stash: AccountId) -> Self {
		Self {
			stash,
			total: Zero::zero(),
			active: Zero::zero(),
			chilling: Default::default(),
			unlocking: Default::default(),
		}
	}

	/// Remove entries from `unlocking` that are sufficiently old and reduce the
	/// total by the sum of their balances.
	fn consolidate_unlocked(self, current_block: T::BlockNumber) -> Self {
		let mut total = self.total;
		let unlocking: BoundedVec<_, _> = self
			.unlocking
			.into_iter()
			.filter(|chunk| {
				if chunk.block > current_block {
					true
				} else {
					total = total.saturating_sub(chunk.value);
					false
				}
			})
			.collect::<Vec<_>>()
			.try_into()
			.expect(
				"filtering items from a bounded vec always leaves length less than bounds. qed",
			);

		Self { stash: self.stash, total, active: self.active, chilling: self.chilling, unlocking }
	}
}

#[frame_support::pallet]
pub mod pallet {
	use ddc_traits::{cluster::ClusterManager, node::NodeVisitorError};

	use super::*;

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: frame_system::Config {
		type Currency: LockableCurrency<Self::AccountId, Moment = Self::BlockNumber>;

		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

		/// Weight information for extrinsics in this pallet.
		type WeightInfo: WeightInfo;

		type ClusterVisitor: ClusterVisitor<Self>;

		type ClusterCreator: ClusterCreator<Self, BalanceOf<Self>>;

		type ClusterManager: ClusterManager<Self>;

		type NodeVisitor: NodeVisitor<Self>;

		type NodeCreator: NodeCreator<Self>;
	}

	/// Map from all locked "stash" accounts to the controller account.
	#[pallet::storage]
	#[pallet::getter(fn bonded)]
	pub type Bonded<T: Config> = StorageMap<_, Twox64Concat, T::AccountId, T::AccountId>;

	/// Map from all (unlocked) "controller" accounts to the info regarding the staking.
	#[pallet::storage]
	#[pallet::getter(fn ledger)]
	pub type Ledger<T: Config> =
		StorageMap<_, Blake2_128Concat, T::AccountId, StakingLedger<T::AccountId, BalanceOf<T>, T>>;

	/// The map of (wannabe) Storage nodes participants stash keys to the DDC cluster ID they
	/// wish to participate into.
	#[pallet::storage]
	#[pallet::getter(fn storages)]
	pub type Storages<T: Config> = StorageMap<_, Twox64Concat, T::AccountId, ClusterId>;

	/// Map from DDC node ID to the node operator stash account.
	#[pallet::storage]
	#[pallet::getter(fn nodes)]
	pub type Nodes<T: Config> = StorageMap<_, Twox64Concat, NodePubKey, T::AccountId>;

	/// Map from operator stash account to DDC node ID.
	#[pallet::storage]
	#[pallet::getter(fn providers)]
	pub type Providers<T: Config> = StorageMap<_, Twox64Concat, T::AccountId, NodePubKey>;

	/// Map of Storage node provider stash accounts that aim to leave a cluster
	#[pallet::storage]
	#[pallet::getter(fn leaving_storages)]
	pub type LeavingStorages<T: Config> = StorageMap<_, Twox64Concat, T::AccountId, ClusterId>;

	#[pallet::genesis_config]
	pub struct GenesisConfig<T: Config> {
		#[allow(clippy::type_complexity)]
		pub storages: Vec<(T::AccountId, T::AccountId, NodePubKey, BalanceOf<T>, ClusterId)>,
	}

	#[cfg(feature = "std")]
	impl<T: Config> Default for GenesisConfig<T> {
		fn default() -> Self {
			GenesisConfig { storages: Default::default() }
		}
	}

	#[pallet::genesis_build]
	impl<T: Config> GenesisBuild<T> for GenesisConfig<T> {
		fn build(&self) {
			// Add initial storage network participants
			for &(ref stash, ref controller, ref node, balance, cluster) in &self.storages {
				assert!(
					T::Currency::free_balance(stash) >= balance,
					"Stash do not have enough balance to participate in storage network."
				);
				assert_ok!(Pallet::<T>::bond(
					T::RuntimeOrigin::from(Some(stash.clone()).into()),
					T::Lookup::unlookup(controller.clone()),
					node.clone(),
					balance,
				));
				assert_ok!(Pallet::<T>::store(
					T::RuntimeOrigin::from(Some(controller.clone()).into()),
					cluster,
				));
			}
		}
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(crate) fn deposit_event)]
	pub enum Event<T: Config> {
		/// An account has bonded this amount. \[stash, amount\]
		///
		/// NOTE: This event is only emitted when funds are bonded via a dispatchable. Notably,
		/// it will not be emitted for staking rewards when they are added to stake.
		Bonded(T::AccountId, BalanceOf<T>),
		/// An account has unbonded this amount. \[stash, amount\]
		Unbonded(T::AccountId, BalanceOf<T>),
		/// An account has called `withdraw_unbonded` and removed unbonding chunks worth `Balance`
		/// from the unlocking queue. \[stash, amount\]
		Withdrawn(T::AccountId, BalanceOf<T>),
		/// An account has stopped participating as DDC network participant.
		/// \[stash\]
		Chilled(T::AccountId),
		/// An account has declared desire to stop participating in DDC network soon.
		/// \[stash, cluster, block\]
		ChillSoon(T::AccountId, ClusterId, T::BlockNumber),
		/// An account that started participating as DDC network participant.
		/// \[stash\]
		Activated(T::AccountId),
		/// An account that started unbonding tokens below the minimum value set for the cluster
		/// his DDC node is assigned to \[stash\]
		LeaveSoon(T::AccountId),
		/// An account that unbonded tokens below the minimum value set for the cluster his
		/// DDC node was assigned to \[stash\]
		Left(T::AccountId),
	}

	#[pallet::error]
	pub enum Error<T> {
		/// Not a controller account.
		NotController,
		/// Not a stash account.
		NotStash,
		/// Stash is already bonded.
		AlreadyBonded,
		/// Controller or node is already paired.
		AlreadyPaired,
		/// Cannot have a DDC network participant, with the size less than defined by
		/// governance (see `BondSize`). If unbonding is the intention, `chill` first to remove
		/// one's role as activated DDC node.
		InsufficientBond,
		/// Can not schedule more unlock chunks.
		NoMoreChunks,
		/// Internal state has become somehow corrupted and the operation cannot continue.
		BadState,
		/// An account already declared a desire to participate in the network with a certain role
		/// and to take another role it should call `chill` first.
		AlreadyInRole,
		/// Action is allowed at some point of time in future not reached yet.
		TooEarly,
		/// Origin of the call is not a controller of the stake associated with the provided node.
		NotNodeController,
		/// No stake found associated with the provided node.
		NodeHasNoStake,
		/// No cluster found
		NoCluster,
		/// No cluster governance params found for cluster
		NoClusterGovParams,
		/// Conditions for fast chill are not met, try the regular `chill` from
		FastChillProhibited,
		/// Storing operation is called for non-Storage node
		StoringProhibited,
		/// Arithmetic overflow occurred
		ArithmeticOverflow,
		/// Arithmetic underflow occurred
		ArithmeticUnderflow,
		/// Attempt to associate stake with non-existing node
		NodeIsNotFound,
		/// Action is prohibited for a node provider stash account that is in the process of
		/// leaving a cluster
		NodeIsLeaving,
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Take the origin account as a stash and lock up `value` of its balance. `controller` will
		/// be the account that controls it.
		///
		/// `value` must be more than the `minimum_balance` specified by `T::Currency`.
		///
		/// The dispatch origin for this call must be _Signed_ by the stash account.
		///
		/// Emits `Bonded`.
		#[pallet::weight(T::WeightInfo::bond())]
		pub fn bond(
			origin: OriginFor<T>,
			controller: <T::Lookup as StaticLookup>::Source,
			node: NodePubKey,
			#[pallet::compact] value: BalanceOf<T>,
		) -> DispatchResult {
			let stash = ensure_signed(origin)?;

			if <Bonded<T>>::contains_key(&stash) {
				Err(Error::<T>::AlreadyBonded)?
			}

			let controller = T::Lookup::lookup(controller)?;

			if <Ledger<T>>::contains_key(&controller) {
				Err(Error::<T>::AlreadyPaired)?
			}

			// Reject a bond which is considered to be _dust_.
			if value < T::Currency::minimum_balance() {
				Err(Error::<T>::InsufficientBond)?
			}

			// Reject a bond with a known DDC node.
			if Nodes::<T>::contains_key(&node) || Providers::<T>::contains_key(&stash) {
				Err(Error::<T>::AlreadyPaired)?
			}

			// Checks that the node is registered in the network
			ensure!(T::NodeVisitor::exists(&node), Error::<T>::NodeIsNotFound);

			frame_system::Pallet::<T>::inc_consumers(&stash).map_err(|_| Error::<T>::BadState)?;

			Nodes::<T>::insert(&node, &stash);
			Providers::<T>::insert(&stash, &node);

			// You're auto-bonded forever, here. We might improve this by only bonding when
			// you actually store/serve and remove once you unbond __everything__.
			<Bonded<T>>::insert(&stash, &controller);

			let stash_balance = T::Currency::free_balance(&stash);
			let value = value.min(stash_balance);
			Self::deposit_event(Event::<T>::Bonded(stash.clone(), value));
			let item = StakingLedger {
				stash,
				total: value,
				active: value,
				chilling: Default::default(),
				unlocking: Default::default(),
			};
			Self::update_ledger(&controller, &item);
			Ok(())
		}

		/// Schedule a portion of the stash to be unlocked ready for transfer out after the bond
		/// period ends. If this leaves an amount actively bonded less than
		/// T::Currency::minimum_balance(), then it is increased to the full amount.
		///
		/// The dispatch origin for this call must be _Signed_ by the controller, not the stash.
		///
		/// Once the unlock period is done, you can call `withdraw_unbonded` to actually move
		/// the funds out of management ready for transfer.
		///
		/// No more than a limited number of unlocking chunks (see `MaxUnlockingChunks`)
		/// can co-exists at the same time. In that case, [`Call::withdraw_unbonded`] need
		/// to be called first to remove some of the chunks (if possible).
		///
		/// If a user encounters the `InsufficientBond` error when calling this extrinsic,
		/// they should call `chill` first in order to free up their bonded funds.
		///
		/// Emits `Unbonded`.
		///
		/// See also [`Call::withdraw_unbonded`].
		#[pallet::weight(T::WeightInfo::unbond())]
		pub fn unbond(
			origin: OriginFor<T>,
			#[pallet::compact] value: BalanceOf<T>,
		) -> DispatchResult {
			let controller = ensure_signed(origin)?;
			let mut ledger = Self::ledger(&controller).ok_or(Error::<T>::NotController)?;

			ensure!(
				ledger.unlocking.len() < MaxUnlockingChunks::get() as usize,
				Error::<T>::NoMoreChunks,
			);

			let mut value = value.min(ledger.active);

			if !value.is_zero() {
				ledger.active =
					ledger.active.checked_sub(&value).ok_or(Error::<T>::ArithmeticUnderflow)?;

				// Avoid there being a dust balance left in the staking system.
				if ledger.active < T::Currency::minimum_balance() {
					value =
						value.checked_add(&ledger.active).ok_or(Error::<T>::ArithmeticOverflow)?;
					ledger.active = Zero::zero();
				}

				let min_active_bond = if let Some(cluster_id) = Self::storages(&ledger.stash) {
					let bond_size =
						T::ClusterVisitor::get_bond_size(&cluster_id, NodeType::Storage)
							.map_err(Into::<Error<T>>::into)?;
					bond_size.saturated_into::<BalanceOf<T>>()
				} else {
					// If node is not assigned to a cluster or node is chilling, allow to unbond
					// any available amount.
					Zero::zero()
				};

				// Make sure that the user maintains enough active bond for their role in the
				// cluster. If a user runs into this error, they should chill first.
				ensure!(ledger.active >= min_active_bond, Error::<T>::InsufficientBond);

				let node_pub_key =
					<Providers<T>>::get(&ledger.stash).ok_or(Error::<T>::BadState)?;

				let unbonding_delay = if T::NodeVisitor::exists(&node_pub_key) {
					let node_cluster_id = T::NodeVisitor::get_cluster_id(&node_pub_key)
						.map_err(Into::<Error<T>>::into)?;

					if let Some(cluster_id) = node_cluster_id {
						let bonding_params = T::ClusterVisitor::get_bonding_params(&cluster_id)
							.map_err(Into::<Error<T>>::into)?;

						let min_bond_size = match node_pub_key {
							NodePubKey::StoragePubKey(_) => bonding_params.storage_bond_size,
						};

						// If provider is trying to unbond after chilling and aims to leave the
						// cluster eventually, we keep its stake till the end of unbonding period.
						if ledger.active < min_bond_size.saturated_into::<BalanceOf<T>>() {
							match node_pub_key {
								NodePubKey::StoragePubKey(_) =>
									LeavingStorages::<T>::insert(ledger.stash.clone(), cluster_id),
							};

							Self::deposit_event(Event::<T>::LeaveSoon(ledger.stash.clone()));
						};

						match node_pub_key {
							NodePubKey::StoragePubKey(_) => bonding_params.storage_unbonding_delay,
						}
					} else {
						// If node is not a member of any cluster, allow immediate unbonding.
						T::BlockNumber::from(0u32)
					}
				} else {
					// If node was deleted, allow immediate unbonding.
					T::BlockNumber::from(0u32)
				};

				// block number + configuration -> no overflow
				let block = <frame_system::Pallet<T>>::block_number() + unbonding_delay;
				if let Some(chunk) =
					ledger.unlocking.last_mut().filter(|chunk| chunk.block == block)
				{
					// To keep the chunk count down, we only keep one chunk per block. Since
					// `unlocking` is a FiFo queue, if a chunk exists for `block` we know that it
					// will be the last one.
					chunk.value = chunk.value.defensive_saturating_add(value)
				} else {
					ledger
						.unlocking
						.try_push(UnlockChunk { value, block })
						.map_err(|_| Error::<T>::NoMoreChunks)?;
				};

				Self::update_ledger(&controller, &ledger);

				Self::deposit_event(Event::<T>::Unbonded(ledger.stash, value));
			}
			Ok(())
		}

		/// Remove any unlocked chunks from the `unlocking` queue from our management.
		///
		/// This essentially frees up that balance to be used by the stash account to do
		/// whatever it wants.
		///
		/// The dispatch origin for this call must be _Signed_ by the controller.
		///
		/// Emits `Withdrawn`.
		///
		/// See also [`Call::unbond`].
		#[pallet::weight(T::WeightInfo::withdraw_unbonded())]
		pub fn withdraw_unbonded(origin: OriginFor<T>) -> DispatchResult {
			let controller = ensure_signed(origin)?;
			let mut ledger = Self::ledger(&controller).ok_or(Error::<T>::NotController)?;
			let (stash, old_total) = (ledger.stash.clone(), ledger.total);
			let node_pub_key = <Providers<T>>::get(stash.clone()).ok_or(Error::<T>::BadState)?;

			ledger = ledger.consolidate_unlocked(<frame_system::Pallet<T>>::block_number());

			if ledger.unlocking.is_empty() && ledger.active < T::Currency::minimum_balance() {
				// This account must have called `unbond()` with some value that caused the active
				// portion to fall below existential deposit + will have no more unlocking chunks
				// left. We can now safely remove all staking-related information.
				Self::kill_stash(&stash)?;
				// Remove the lock.
				T::Currency::remove_lock(DDC_STAKING_ID, &stash);
			} else {
				// This was the consequence of a partial unbond. just update the ledger and move on.
				Self::update_ledger(&controller, &ledger);
			};

			// `old_total` should never be less than the new total because
			// `consolidate_unlocked` strictly subtracts balance.
			if ledger.total < old_total {
				// Already checked that this won't overflow by entry condition.
				let value =
					old_total.checked_sub(&ledger.total).ok_or(Error::<T>::ArithmeticUnderflow)?;
				Self::deposit_event(Event::<T>::Withdrawn(stash.clone(), value));

				// If provider aimed to leave the cluster and the unbonding period ends, remove
				// the node from the cluster
				if let Some(cluster_id) = <LeavingStorages<T>>::get(&stash) {
					// Cluster manager could remove the node from cluster by this moment already, so
					// it is ok to ignore result.
					let _ = T::ClusterManager::remove_node(&cluster_id, &node_pub_key);

					<LeavingStorages<T>>::remove(&stash);

					Self::deposit_event(Event::<T>::Left(stash));
				}
			}

			Ok(())
		}

		/// Declare the desire to participate in storage network for the origin controller. Also
		/// works to cancel a previous "chill".
		///
		/// `cluster` is the ID of the DDC cluster the participant wishes to join.
		///
		/// The dispatch origin for this call must be _Signed_ by the controller, not the stash. The
		/// bond size must be greater than or equal to the `StorageBondSize`.
		#[pallet::weight(T::WeightInfo::store())]
		pub fn store(origin: OriginFor<T>, cluster_id: ClusterId) -> DispatchResult {
			let controller = ensure_signed(origin)?;

			T::ClusterVisitor::ensure_cluster(&cluster_id).map_err(Into::<Error<T>>::into)?;

			let ledger = Self::ledger(&controller).ok_or(Error::<T>::NotController)?;
			// Retrieve the respective bond size from Cluster Visitor
			let bond_size = T::ClusterVisitor::get_bond_size(&cluster_id, NodeType::Storage)
				.map_err(Into::<Error<T>>::into)?;
			ensure!(
				ledger.active >= bond_size.saturated_into::<BalanceOf<T>>(),
				Error::<T>::InsufficientBond
			);
			let stash = &ledger.stash;

			// Only Storage node can perform storing (i.e. saving content)
			let node_pub_key = <Providers<T>>::get(stash).ok_or(Error::<T>::BadState)?;
			ensure!(
				matches!(node_pub_key, NodePubKey::StoragePubKey(_)),
				Error::<T>::StoringProhibited
			);

			// Is it an attempt to cancel a previous "chill"?
			if let Some(current_cluster) = Self::storages(stash) {
				// Switching the cluster is prohibited. The user should chill first.
				ensure!(current_cluster == cluster_id, Error::<T>::AlreadyInRole);
				// Cancel previous "chill" attempts
				Self::reset_chilling(&controller);
				return Ok(())
			} else {
				// Can't participate in new Storage network if provider hasn't left the previous
				// cluster yet
				ensure!(!LeavingStorages::<T>::contains_key(stash), Error::<T>::NodeIsLeaving);
			}

			Self::do_add_storage(stash, cluster_id);
			Self::deposit_event(Event::<T>::Activated(stash.clone()));

			Ok(())
		}

		/// Declare no desire to either participate in DDC network.
		///
		/// Only in case the delay for the role _origin_ maintains in the cluster is set to zero in
		/// cluster settings, it removes the participant immediately. Otherwise, it requires at
		/// least two invocations to effectively remove the participant. The first invocation only
		/// updates the [`Ledger`] to note the block number at which the participant may "chill"
		/// (current block + the delay from the cluster settings). The second invocation made at the
		/// noted block (or any further block) will remove the participant from the list of DDC
		/// network participants. If the cluster settings updated significantly decreasing
		/// the delay, one may invoke it again to decrease the block at with the participant may
		/// "chill". But it never increases the block at which the participant may "chill" even when
		/// the cluster settings updated increasing the delay.
		///
		/// The dispatch origin for this call must be _Signed_ by the controller, not the stash.
		///
		/// Emits `ChillSoon`, `Chill`.
		#[pallet::weight(T::WeightInfo::chill())]
		pub fn chill(origin: OriginFor<T>) -> DispatchResult {
			let controller = ensure_signed(origin)?;
			let ledger = Self::ledger(&controller).ok_or(Error::<T>::NotController)?;
			let current_block = <frame_system::Pallet<T>>::block_number();

			// Extract delay from the cluster settings.
			let (cluster, delay) = if let Some(cluster) = Self::storages(&ledger.stash) {
				let chill_delay = T::ClusterVisitor::get_chill_delay(&cluster, NodeType::Storage)
					.map_err(Into::<Error<T>>::into)?;
				(cluster, chill_delay)
			} else {
				return Ok(()) // node is already chilling or leaving the cluster
			};

			if delay == T::BlockNumber::from(0u32) {
				// No delay is set, so we can chill right away.
				Self::chill_stash(&ledger.stash);
				return Ok(())
			}

			let can_chill_from = current_block.defensive_saturating_add(delay);
			match ledger.chilling {
				None => {
					// No previous declarations of desire to chill. Note it to allow chilling soon.
					Self::chill_stash_soon(&ledger.stash, &controller, cluster, can_chill_from);
					return Ok(())
				},
				Some(chilling) if can_chill_from < chilling => {
					// Time to chill is not reached yet, but it is allowed to chill earlier. Update
					// to allow chilling sooner.
					Self::chill_stash_soon(&ledger.stash, &controller, cluster, can_chill_from);
					return Ok(())
				},
				Some(chilling) if chilling > current_block => Err(Error::<T>::TooEarly)?,
				Some(_) => (),
			}

			// It's time to chill.
			Self::chill_stash(&ledger.stash);
			Self::reset_chilling(&controller); // for future chilling

			Ok(())
		}

		/// (Re-)set the controller of a stash.
		///
		/// Effects will be felt at the beginning of the next block.
		///
		/// The dispatch origin for this call must be _Signed_ by the stash, not the controller.
		#[pallet::weight(T::WeightInfo::set_controller())]
		pub fn set_controller(
			origin: OriginFor<T>,
			controller: <T::Lookup as StaticLookup>::Source,
		) -> DispatchResult {
			let stash = ensure_signed(origin)?;
			let old_controller = Self::bonded(&stash).ok_or(Error::<T>::NotStash)?;
			let controller = T::Lookup::lookup(controller)?;
			if <Ledger<T>>::contains_key(&controller) {
				Err(Error::<T>::AlreadyPaired)?
			}
			if controller != old_controller {
				<Bonded<T>>::insert(&stash, &controller);
				if let Some(l) = <Ledger<T>>::take(&old_controller) {
					<Ledger<T>>::insert(&controller, l);
				}
			}
			Ok(())
		}

		/// (Re-)set the DDC node of a node operator stash account. Requires to chill first.
		///
		/// The dispatch origin for this call must be _Signed_ by the stash, not the controller.
		#[pallet::weight(T::WeightInfo::set_node())]
		pub fn set_node(origin: OriginFor<T>, new_node: NodePubKey) -> DispatchResult {
			let stash = ensure_signed(origin)?;

			if let Some(existing_node_stash) = Nodes::<T>::get(&new_node) {
				if existing_node_stash != stash {
					Err(Error::<T>::AlreadyPaired)?
				}
			}

			// Remove previously owned node from storage
			if let Some(current_node) = Providers::<T>::get(&stash) {
				<Nodes<T>>::remove(current_node);
			}

			// Ensure only one node per stash.
			ensure!(!<Storages<T>>::contains_key(&stash), Error::<T>::AlreadyInRole);

			// Ensure that provider is not about leaving the cluster as it may cause the removal
			// of an unexpected node after unbonding.
			ensure!(!<LeavingStorages<T>>::contains_key(&stash), Error::<T>::NodeIsLeaving);

			<Nodes<T>>::insert(new_node.clone(), stash.clone());
			<Providers<T>>::insert(stash, new_node);

			Ok(())
		}

		/// Allow cluster node candidate to chill in the next block.
		///
		/// The dispatch origin for this call must be _Signed_ by the controller.
		#[pallet::weight(10_000)]
		pub fn fast_chill(origin: OriginFor<T>) -> DispatchResult {
			let controller = ensure_signed(origin)?;

			let stash = <Ledger<T>>::get(&controller).ok_or(Error::<T>::NotController)?.stash;
			let node_pub_key = <Providers<T>>::get(&stash).ok_or(Error::<T>::BadState)?;
			let node_stash = <Nodes<T>>::get(&node_pub_key).ok_or(Error::<T>::BadState)?;
			ensure!(stash == node_stash, Error::<T>::NotNodeController);

			let cluster_id = <Storages<T>>::get(&stash).ok_or(Error::<T>::NodeHasNoStake)?;

			let is_cluster_node = T::ClusterManager::contains_node(&cluster_id, &node_pub_key);
			ensure!(!is_cluster_node, Error::<T>::FastChillProhibited);

			// block number + 1 => no overflow
			let can_chill_from =
				<frame_system::Pallet<T>>::block_number() + T::BlockNumber::from(1u32);
			Self::chill_stash_soon(&stash, &controller, cluster_id, can_chill_from);

			Ok(())
		}
	}

	impl<T: Config> Pallet<T> {
		/// Update the ledger for a controller.
		///
		/// This will also update the stash lock.
		fn update_ledger(
			controller: &T::AccountId,
			ledger: &StakingLedger<T::AccountId, BalanceOf<T>, T>,
		) {
			T::Currency::set_lock(
				DDC_STAKING_ID,
				&ledger.stash,
				ledger.total,
				WithdrawReasons::all(),
			);
			<Ledger<T>>::insert(controller, ledger);
		}

		/// Chill a stash account.
		fn chill_stash(stash: &T::AccountId) {
			let chilled_as_storage = Self::do_remove_storage(stash);
			if chilled_as_storage {
				Self::deposit_event(Event::<T>::Chilled(stash.clone()));
			}
		}

		/// Note a desire of a stash account to chill soon.
		pub fn chill_stash_soon(
			stash: &T::AccountId,
			controller: &T::AccountId,
			cluster: ClusterId,
			can_chill_from: T::BlockNumber,
		) {
			Ledger::<T>::mutate(controller, |maybe_ledger| {
				if let Some(ref mut ledger) = maybe_ledger {
					ledger.chilling = Some(can_chill_from)
				}
			});
			Self::deposit_event(Event::<T>::ChillSoon(stash.clone(), cluster, can_chill_from));
		}

		/// Remove all associated data of a stash account from the staking system.
		///
		/// Assumes storage is upgraded before calling.
		///
		/// This is called:
		/// - after a `withdraw_unbonded()` call that frees all of a stash's bonded balance.
		/// - through `reap_stash()` if the balance has fallen to zero (through slashing).
		fn kill_stash(stash: &T::AccountId) -> DispatchResult {
			let controller = <Bonded<T>>::get(stash).ok_or(Error::<T>::NotStash)?;

			<Bonded<T>>::remove(stash);
			<Ledger<T>>::remove(&controller);

			if let Some(node_pub_key) = <Providers<T>>::take(stash) {
				<Nodes<T>>::remove(node_pub_key);
			};

			Self::do_remove_storage(stash);

			frame_system::Pallet::<T>::dec_consumers(stash);

			Ok(())
		}

		/// This function will add a storage network participant to the `Storages` storage map.
		///
		/// If the storage network participant already exists, their cluster will be updated.
		pub fn do_add_storage(who: &T::AccountId, cluster: ClusterId) {
			Storages::<T>::insert(who, cluster);
		}

		/// This function will remove a storage network participant from the `Storages` map.
		///
		/// Returns true if `who` was removed from `Storages`, otherwise false.
		pub fn do_remove_storage(who: &T::AccountId) -> bool {
			Storages::<T>::take(who).is_some()
		}

		/// Reset the chilling block for a controller.
		pub fn reset_chilling(controller: &T::AccountId) {
			Ledger::<T>::mutate(controller, |maybe_ledger| {
				if let Some(ref mut ledger) = maybe_ledger {
					ledger.chilling = None
				}
			});
		}
	}

	impl<T: Config> StakerCreator<T, BalanceOf<T>> for Pallet<T> {
		fn bond_stake_and_participate(
			stash: T::AccountId,
			controller: T::AccountId,
			node: NodePubKey,
			value: BalanceOf<T>,
			cluster_id: ClusterId,
		) -> DispatchResult {
			Nodes::<T>::insert(&node, &stash);
			Providers::<T>::insert(&stash, &node);
			<Bonded<T>>::insert(&stash, &controller);
			let stash_balance = T::Currency::free_balance(&stash);
			let value = value.min(stash_balance);
			Self::deposit_event(Event::<T>::Bonded(stash.clone(), value));
			let item = StakingLedger {
				stash: stash.clone(),
				total: value,
				active: value,
				chilling: Default::default(),
				unlocking: Default::default(),
			};
			Self::update_ledger(&controller, &item);
			match node {
				NodePubKey::StoragePubKey(_node) => Self::do_add_storage(&stash, cluster_id),
			}

			Ok(())
		}
	}

	impl<T: Config> StakingVisitor<T> for Pallet<T> {
		fn has_activated_stake(
			node_pub_key: &NodePubKey,
			cluster_id: &ClusterId,
		) -> Result<bool, StakingVisitorError> {
			let stash =
				<Nodes<T>>::get(node_pub_key).ok_or(StakingVisitorError::NodeStakeDoesNotExist)?;
			let maybe_storage_in_cluster = Storages::<T>::get(&stash);

			let has_activated_stake: bool = maybe_storage_in_cluster
				.is_some_and(|staking_cluster| staking_cluster == *cluster_id);

			Ok(has_activated_stake)
		}

		fn has_stake(node_pub_key: &NodePubKey) -> bool {
			<Nodes<T>>::get(node_pub_key).is_some()
		}

		fn has_chilling_attempt(node_pub_key: &NodePubKey) -> Result<bool, StakingVisitorError> {
			let stash =
				<Nodes<T>>::get(node_pub_key).ok_or(StakingVisitorError::NodeStakeDoesNotExist)?;
			let controller =
				<Bonded<T>>::get(&stash).ok_or(StakingVisitorError::NodeStakeIsInBadState)?;

			let is_chilling_attempt = <Ledger<T>>::get(&controller)
				.ok_or(StakingVisitorError::NodeStakeIsInBadState)?
				.chilling
				.is_some();

			Ok(is_chilling_attempt)
		}
	}

	impl<T> From<ClusterVisitorError> for Error<T> {
		fn from(error: ClusterVisitorError) -> Self {
			match error {
				ClusterVisitorError::ClusterDoesNotExist => Error::<T>::NoCluster,
				ClusterVisitorError::ClusterGovParamsNotSet => Error::<T>::NoClusterGovParams,
			}
		}
	}

	impl<T> From<NodeVisitorError> for Error<T> {
		fn from(error: NodeVisitorError) -> Self {
			match error {
				NodeVisitorError::NodeDoesNotExist => Error::<T>::NodeIsNotFound,
			}
		}
	}
}
