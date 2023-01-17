#![cfg_attr(not(feature = "std"), no_std)]

/// Edit this file to define custom logic or remove it if it is not needed.
/// Learn more about FRAME and the core library of Substrate FRAME pallets:
/// <https://docs.substrate.io/v3/runtime/frame>

pub use pallet::*;

use codec::{Decode, Encode};
use frame_support::{dispatch, ensure, traits::{
		Currency, 
		ExistenceRequirement,
		IsSubType, 
		tokens::{
			WithdrawReasons
		}
	}, weights::{
		DispatchInfo, 
		PostDispatchInfo
	}
};
use frame_support::sp_runtime::transaction_validity::ValidTransaction;
use frame_system::{
	self as system, 
	ensure_signed
};
use substrate_fixed::types::U64F64;
use sp_runtime::{
	traits::{
		Dispatchable, 
		DispatchInfoOf, 
		SignedExtension, 
	},
	transaction_validity::{
        TransactionValidityError, 
		TransactionValidity
    }
};

use sp_std::vec::Vec;
use sp_std::vec;
use sp_std::marker::PhantomData;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

/// ************************************************************
///	-Subtensor-Imports
/// ************************************************************
mod weights;
mod serving;
mod step;
mod registration;
mod staking;

#[frame_support::pallet]
pub mod pallet {
	use sp_core::{U256};
	use frame_support::IterableStorageMap;
	use frame_support::{pallet_prelude::*, Printable, traits::{Currency}};
	use frame_system::pallet_prelude::*;
	use sp_std::vec::Vec;
	use sp_std::vec;

	/// Configure the pallet by specifying the parameters and types on which it depends.
	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// Because this pallet emits events, it depends on the runtime's definition of an event.
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

		/// --- Currency type that will be used to place deposits on neurons
		type Currency: Currency<Self::AccountId> + Send + Sync;
		
		/// --- The transaction fee in RAO per byte
		type TransactionByteFee: Get<BalanceOf<Self>>;

		/// Debug is on
		#[pallet::constant]
		type SDebug: Get<u64>;

		/// Rho constant
		#[pallet::constant]
		type InitialRho: Get<u64>;

		/// Kappa constant
		#[pallet::constant]
		type InitialKappa: Get<u64>;

		/// Default Batch size.
		#[pallet::constant]
		type InitialValidatorBatchSize: Get<u64>;

		/// Default Batch size.
		#[pallet::constant]
		type InitialValidatorSequenceLen: Get<u64>;

		/// Default Epoch length.
		#[pallet::constant]
		type InitialValidatorEpochLen: Get<u64>;

		/// Default Reset length.
		#[pallet::constant]
		type InitialValidatorEpochsPerReset: Get<u64>;

		/// Max UID constant.
		#[pallet::constant]
		type InitialMaxAllowedUids: Get<u64>;

		/// Initial min allowed weights.
		#[pallet::constant]
		type InitialMinAllowedWeights: Get<u64>;

		/// Initial allowed max min weight ratio
		#[pallet::constant]
		type InitialMaxAllowedMaxMinRatio: Get<u64>;

		/// Initial max weight limit.
		#[pallet::constant]
		type InitialMaxWeightLimit: Get<u32>;

		/// Initial stake pruning denominator
		#[pallet::constant]
		type InitialStakePruningDenominator: Get<u64>;

		/// Initial stake pruning min
		#[pallet::constant]
		type InitialStakePruningMin: Get<u64>;

		/// Initial incentive pruning denominator
		#[pallet::constant]
		type InitialIncentivePruningDenominator: Get<u64>;

		/// Initial foundation distribution
		#[pallet::constant]
		type InitialFoundationDistribution: Get<u64>;

		/// Immunity Period Constant.
		#[pallet::constant]
		type InitialImmunityPeriod: Get<u64>;

		/// Blocks per step.
		#[pallet::constant]
		type InitialBlocksPerStep: Get<u64>;

		/// Blocks per era.
		#[pallet::constant]
		type InitialBondsMovingAverage: Get<u64>;
		
		/// SelfOwnership constant
		#[pallet::constant]
		type SelfOwnership: Get<u64>;

		/// Activity constant
		#[pallet::constant]
		type InitialActivityCutoff: Get<u64>;

		/// Initial registration difficulty.
		#[pallet::constant]
		type InitialIssuance: Get<u64>;

		/// Initial registration difficulty.
		#[pallet::constant]
		type InitialDifficulty: Get<u64>;

		/// Minimum registration difficulty
		#[pallet::constant]
		type MinimumDifficulty: Get<u64>;

		/// Maximum registration difficulty
		#[pallet::constant]
		type MaximumDifficulty: Get<u64>;

		/// Initial adjustment interval.
		#[pallet::constant]
		type InitialAdjustmentInterval: Get<u64>;

		/// Initial max registrations per block.
		#[pallet::constant]
		type InitialMaxRegistrationsPerBlock: Get<u64>;

		/// Initial target registrations per interval.
		#[pallet::constant]
		type InitialTargetRegistrationsPerInterval: Get<u64>;

		///// u8 where value (x) represents x * 10^-2
		/// Initial scaling law power.
		#[pallet::constant]
		type InitialScalingLawPower: Get<u8>;

		/// Initial synergy scaling law power.
		#[pallet::constant]
		type InitialSynergyScalingLawPower: Get<u8>;

		/// Initial validator exclude quantile.
		#[pallet::constant]
		type InitialValidatorExcludeQuantile: Get<u8>;

		/// Initial validator context pruning length.
		#[pallet::constant]
		type InitialValidatorPruneLen: Get<u64>;

		/// Initial validator logits divergence penalty/threshold.
		#[pallet::constant]
		type InitialValidatorLogitsDivergence: Get<u64>;
	}

	pub type AccountIdOf<T> = <T as frame_system::Config>::AccountId;
	pub type NeuronMetadataOf<T> = NeuronMetadata<AccountIdOf<T>>;
	pub type BalanceOf<T> = <<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

	#[derive(Encode, Decode, Default, TypeInfo)]
    pub struct NeuronMetadata<AccountId> {

		/// ---- The endpoint's code version.
        pub version: u32,

        /// ---- The endpoint's u128 encoded ip address of type v6 or v4.
        pub ip: u128,

        /// ---- The endpoint's u16 encoded port.
        pub port: u16,

        /// ---- The endpoint's ip type, 4 for ipv4 and 6 for ipv6.
        pub ip_type: u8,

        /// ---- The endpoint's unique identifier.
        pub uid: u32,

        /// ---- The neuron modality. Modalities specify which datatype
        /// the neuron endpoint can process. This information is non
        /// verifiable. However, neurons should set this correctly
        /// in order to be detected by others with this datatype.
        /// The initial modality codes are:
        /// TEXT: 0
        /// IMAGE: 1
        /// TENSOR: 2
        pub modality: u8,

        /// ---- The associated hotkey account.
        /// Registration and changing weights can be made by this
        /// account.
        pub hotkey: AccountId,

        /// ---- The associated coldkey account.
        /// Staking and unstaking transactions must be made by this account.
        /// The hotkey account (in the Neurons map) has permission to call
        /// subscribe and unsubscribe.
        pub coldkey: AccountId,

		/// ---- Is this neuron active in the incentive mechanism.
		pub active: u32,

		/// ---- Block number of last chain update.
		pub last_update: u64,

		/// ---- Transaction priority.
		pub priority: u64,

		/// ---- The associated stake in this account.
		pub stake: u64,

		/// ---- The associated rank in this account.
		pub rank: u64,

		/// ---- The associated trust in this account.
		pub trust: u64,

		/// ---- The associated consensus in this account.
		pub consensus: u64,

		/// ---- The associated incentive in this account.
		pub incentive: u64,

		/// ---- The associated dividends in this account.
		pub dividends: u64,

		/// ---- The associated emission last block for this account.
		pub emission: u64,

		/// ---- The associated bond ownership.
		pub bonds: Vec<(u32,u64)>,

		/// ---- The associated weights ownership.
		pub weights: Vec<(u32,u32)>,
    }

	#[pallet::pallet]
	#[pallet::without_storage_info]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(_);

	/// ************************************************************
	///	*---- Storage Objects
	/// ************************************************************
	
	// --- Number of peers.
	#[pallet::storage]
	pub type N<T> = StorageValue<
		_, 
		u32, 
		ValueQuery
	>;

	#[pallet::storage]
	pub type TotalStake<T> = StorageValue<
		_, 
		u64, 
		ValueQuery
	>;

	#[pallet::storage]
	pub type TotalEmission<T> = StorageValue<
		_, 
		u64, 
		ValueQuery
	>;

	#[pallet::storage]
	pub type TotalBondsPurchased<T> = StorageValue<
		_, 
		u64, 
		ValueQuery
	>;

	#[pallet::type_value] 
	pub fn DefaultRho<T: Config>() -> u64 { T::InitialRho::get() }
	#[pallet::storage]
	pub type Rho<T> = StorageValue<
		_, 
		u64, 
		ValueQuery,
		DefaultRho<T>
	>;

	#[pallet::type_value] 
	pub fn DefaultKappa<T: Config>() -> u64 { T::InitialKappa::get() }
	#[pallet::storage]
	pub type Kappa<T> = StorageValue<
		_, 
		u64, 
		ValueQuery,
		DefaultKappa<T>
	>;

	#[pallet::type_value] 
	pub fn DefaultIncentivePruningDenominator<T: Config>() -> u64 { T::InitialIncentivePruningDenominator::get() }
	#[pallet::storage]
	pub type IncentivePruningDenominator<T> = StorageValue<
		_, 
		u64, 
		ValueQuery,
		DefaultIncentivePruningDenominator<T>
	>;

	#[pallet::type_value] 
	pub fn DefaultStakePruningDenominator<T: Config>() -> u64 { T::InitialStakePruningDenominator::get() }
	#[pallet::storage]
	pub type StakePruningDenominator<T> = StorageValue<
		_, 
		u64, 
		ValueQuery,
		DefaultStakePruningDenominator<T>
	>;

	#[pallet::type_value] 
	pub fn DefaultStakePruningMin<T: Config>() -> u64 { T::InitialStakePruningMin::get() }
	#[pallet::storage]
	pub type StakePruningMin<T> = StorageValue<
		_, 
		u64, 
		ValueQuery,
		DefaultStakePruningMin<T>
	>;

	#[pallet::type_value] 
	pub fn DefaultValidatorEpochLen<T: Config>() -> u64 { T::InitialValidatorEpochLen::get() }
	#[pallet::storage]
	pub type ValidatorEpochLen<T> = StorageValue<
		_, 
		u64, 
		ValueQuery,
		DefaultValidatorEpochLen<T>
	>;

	#[pallet::type_value] 
	pub fn DefaultValidatorEpochsPerReset<T: Config>() -> u64 { T::InitialValidatorEpochsPerReset::get() }
	#[pallet::storage]
	pub type ValidatorEpochsPerReset<T> = StorageValue<
		_, 
		u64, 
		ValueQuery,
		DefaultValidatorEpochsPerReset<T>
	>;

	#[pallet::type_value] 
	pub fn DefaultValidatorBatchSize<T: Config>() -> u64 { T::InitialValidatorBatchSize::get() }
	#[pallet::storage]
	pub type ValidatorBatchSize<T> = StorageValue<
		_, 
		u64, 
		ValueQuery,
		DefaultValidatorBatchSize<T>
	>;

	#[pallet::type_value] 
	pub fn DefaultValidatorSequenceLen<T: Config>() -> u64 { T::InitialValidatorSequenceLen::get() }
	#[pallet::storage]
	pub type ValidatorSequenceLength<T> = StorageValue<
		_, 
		u64, 
		ValueQuery,
		DefaultValidatorSequenceLen<T>
	>;

	#[pallet::type_value] 
	pub fn DefaultMaxAllowedUids<T: Config>() -> u64 { T::InitialMaxAllowedUids::get() }
	#[pallet::storage]
	pub type MaxAllowedUids<T> = StorageValue<
		_, 
		u64, 
		ValueQuery,
		DefaultMaxAllowedUids<T>
	>;


	#[pallet::type_value] 
	pub fn DefaultMinAllowedWeights<T: Config>() -> u64 { T::InitialMinAllowedWeights::get() }
	#[pallet::storage]
	pub type MinAllowedWeights<T> = StorageValue<
		_, 
		u64, 
		ValueQuery,
		DefaultMinAllowedWeights<T>
	>;

	#[pallet::type_value] 
	pub fn DefaultMaxAllowedMaxMinRatio<T: Config>() -> u64 { T::InitialMaxAllowedMaxMinRatio::get() }
	#[pallet::storage]
	pub type MaxAllowedMaxMinRatio<T> = StorageValue<
		_, 
		u64, 
		ValueQuery,
		DefaultMaxAllowedMaxMinRatio<T>
	>;

	#[pallet::type_value] 
	pub fn DefaultMaxWeightLimit<T: Config>() -> u32 { T::InitialMaxWeightLimit::get() }
	#[pallet::storage]
	pub type MaxWeightLimit<T> = StorageValue<
		_, 
		u32, 
		ValueQuery,
		DefaultMaxWeightLimit<T>
	>;

	#[pallet::type_value] 
	pub fn DefaultImmunityPeriod<T: Config>() -> u64 { T::InitialImmunityPeriod::get() }
	#[pallet::storage]
	pub type ImmunityPeriod<T> = StorageValue<
		_, 
		u64, 
		ValueQuery,
		DefaultImmunityPeriod<T>
	>;

	#[pallet::type_value] 
	pub fn DefaultTotalIssuance<T: Config>() -> u64 { T::InitialIssuance::get() }
	#[pallet::storage]
	pub type TotalIssuance<T> = StorageValue<
		_, 
		u64, 
		ValueQuery,
		DefaultTotalIssuance<T>
	>;

	#[pallet::type_value] 
	pub fn DefaultBlocksSinceLastStep<T: Config>() -> u64 { 0 }
	#[pallet::storage]
	pub type BlocksSinceLastStep<T> = StorageValue<
		_, 
		u64, 
		ValueQuery,
		DefaultBlocksSinceLastStep<T>
	>;

	#[pallet::type_value] 
	pub fn DefaultBlocksPerStep<T: Config>() -> u64 { T::InitialBlocksPerStep::get() }
	#[pallet::storage]
	pub type BlocksPerStep<T> = StorageValue<
		_, 
		u64, 
		ValueQuery,
		DefaultBlocksPerStep<T>
	>;

	#[pallet::type_value] 
	pub fn DefaultBondsMovingAverage<T: Config>() -> u64 { T::InitialBondsMovingAverage::get() }
	#[pallet::storage]
	pub type BondsMovingAverage<T> = StorageValue<
		_, 
		u64, 
		ValueQuery,
		DefaultBondsMovingAverage<T>
	>;

	#[pallet::type_value] 
	pub fn DefaultDifficulty<T: Config>() -> u64 { T::InitialDifficulty::get() }
	#[pallet::storage]
	pub type Difficulty<T> = StorageValue<
		_, 
		u64, 
		ValueQuery,
		DefaultDifficulty<T>
	>;

	#[pallet::type_value] 
	pub fn DefaultActivityCutoff<T: Config>() -> u64 { T::InitialActivityCutoff::get() }
	#[pallet::storage]
	pub type ActivityCutoff<T> = StorageValue<
		_, 
		u64, 
		ValueQuery,
		DefaultActivityCutoff<T>
	>;

	#[pallet::type_value] 
	pub fn DefaultAdjustmentInterval<T: Config>() -> u64 { T::InitialAdjustmentInterval::get() }
	#[pallet::storage]
	pub type AdjustmentInterval<T> = StorageValue<
		_, 
		u64, 
		ValueQuery,
		DefaultAdjustmentInterval<T>
	>;

	#[pallet::type_value] 
	pub fn DefaultTargetRegistrationsPerInterval<T: Config>() -> u64 { T::InitialTargetRegistrationsPerInterval::get() }
	#[pallet::storage]
	pub type TargetRegistrationsPerInterval<T> = StorageValue<
		_, 
		u64, 
		ValueQuery,
		DefaultTargetRegistrationsPerInterval<T>
	>;

	#[pallet::type_value] 
	pub fn DefaultMaxRegistrationsPerBlock<T: Config>() -> u64 { T::InitialMaxRegistrationsPerBlock::get() }
	#[pallet::storage]
	pub type MaxRegistrationsPerBlock<T> = StorageValue<
		_, 
		u64, 
		ValueQuery,
		DefaultMaxRegistrationsPerBlock<T>
	>;

	#[pallet::type_value] 
	pub fn DefaultFoundationDistribution<T: Config>() -> u64 { T::InitialFoundationDistribution::get() }
	#[pallet::storage]
	pub type FoundationDistribution<T> = StorageValue<
		_, 
		u64, 
		ValueQuery,
		DefaultFoundationDistribution<T>
	>;

	#[pallet::type_value] 
	pub fn DefaultScalingLawPower<T: Config>() -> u8 { T::InitialScalingLawPower::get() }
	#[pallet::storage]
	pub type ScalingLawPower<T> = StorageValue<
		_, 
		u8, 
		ValueQuery,
		DefaultScalingLawPower<T>
	>;

	#[pallet::type_value] 
	pub fn DefaultSynergyScalingLawPower<T: Config>() -> u8{ T::InitialSynergyScalingLawPower::get() }
	#[pallet::storage]
	pub type SynergyScalingLawPower<T> = StorageValue<
		_, 
		u8, 
		ValueQuery,
		DefaultSynergyScalingLawPower<T>
	>;

	#[pallet::type_value] 
	pub fn DefaultValidatorExcludeQuantile<T: Config>() -> u8 { T::InitialValidatorExcludeQuantile::get() }
	#[pallet::storage]
	pub type ValidatorExcludeQuantile<T> = StorageValue<
		_, 
		u8, 
		ValueQuery,
		DefaultValidatorExcludeQuantile<T>
	>;

	#[pallet::type_value] 
	pub fn DefaultValidatorPruneLen<T: Config>() -> u64 { T::InitialValidatorPruneLen::get() }
	#[pallet::storage]
	pub type ValidatorPruneLen<T> = StorageValue<
		_, 
		u64, 
		ValueQuery,
		DefaultValidatorPruneLen<T>
	>;

	#[pallet::type_value] 
	pub fn DefaultValidatorLogitsDivergence<T: Config>() -> u64 { T::InitialValidatorLogitsDivergence::get() }
	#[pallet::storage]
	pub type ValidatorLogitsDivergence<T> = StorageValue<
		_, 
		u64, 
		ValueQuery,
		DefaultValidatorLogitsDivergence<T>
	>;

	/// #[pallet::type_value] 
	/// pub fn DefaultFoundationAccount<T: Config>() -> u64 { T::InitialFoundationAccount::get() }
	#[pallet::storage]
	pub(super) type FoundationAccount<T:Config> = StorageValue<
		_, 
		T::AccountId, 
		OptionQuery
	>;

	#[pallet::storage]
	pub type LastDifficultyAdjustmentBlock<T> = StorageValue<
		_, 
		u64, 
		ValueQuery
	>;

	#[pallet::storage]
	pub type LastMechansimStepBlock<T> = StorageValue<
		_, 
		u64, 
		ValueQuery
	>;

	#[pallet::storage]
	pub type RegistrationsThisInterval<T> = StorageValue<
		_, 
		u64, 
		ValueQuery
	>;

	#[pallet::storage]
	pub type RegistrationsThisBlock<T> = StorageValue<
		_, 
		u64, 
		ValueQuery
	>;


	/// ---- Maps from hotkey to uid.
	#[pallet::storage]
	#[pallet::getter(fn hotkey)]
    pub(super) type Hotkeys<T:Config> = StorageMap<
		_, 
		Blake2_128Concat, 
		T::AccountId, 
		u32, 
		ValueQuery
	>;

	#[pallet::storage]
	#[pallet::getter(fn usedwork)]
    pub(super) type UsedWork<T:Config> = StorageMap<
		_, 
		Identity, 
		Vec<u8>, 
		u64,
		ValueQuery
	>;

	/// ---- Maps from uid to neuron.
	#[pallet::storage]
    #[pallet::getter(fn uid)]
    pub(super) type Neurons<T:Config> = StorageMap<
		_, 
		Identity, 
		u32, 
		NeuronMetadataOf<T>, 
		OptionQuery
	>;

	/// ---- Maps from uid to uid as a set which we use to record uids to prune at next epoch.
	#[pallet::storage]
	#[pallet::getter(fn uid_to_prune)]
    pub(super) type NeuronsToPruneAtNextEpoch<T:Config> = StorageMap<
		_, 
		Identity, 
		u32, 
		u32, 
		ValueQuery,
	>;

	#[pallet::type_value] 
	pub fn DefaultBlockAtRegistration<T: Config>() -> u64 { 0 }
	#[pallet::storage]
	#[pallet::getter(fn block_at_registration)]
    pub(super) type BlockAtRegistration<T:Config> = StorageMap<
		_, 
		Identity, 
		u32, 
		u64, 
		ValueQuery,
		DefaultBlockAtRegistration<T>
	>;

	/// ************************************************************
	///	-Genesis-Configuration
	/// ************************************************************
	/// ---- Genesis Configuration (Mostly used for testing.)
    #[pallet::genesis_config]
    pub struct GenesisConfig {
        pub stake: Vec<(u64, u64)>,
    }

	#[cfg(feature = "std")]
	impl Default for GenesisConfig {
		fn default() -> Self {
			Self {
				stake: Default::default(),
			}
		}
	}
    
    #[pallet::genesis_build]
    impl<T:Config> GenesisBuild<T> for GenesisConfig {
        fn build(&self) {		
		}
	}


	#[cfg(feature = "std")]
	impl GenesisConfig {
		/// Direct implementation of `GenesisBuild::build_storage`.
		///
		/// Kept in order not to break dependency.
		pub fn build_storage<T: Config>(&self) -> Result<sp_runtime::Storage, String> {
			<Self as GenesisBuild<T>>::build_storage(self)
		}

		/// Direct implementation of `GenesisBuild::assimilate_storage`.
		///
		/// Kept in order not to break dependency.
		pub fn assimilate_storage<T: Config>(
			&self,
			storage: &mut sp_runtime::Storage
		) -> Result<(), String> {
			<Self as GenesisBuild<T>>::assimilate_storage(self, storage)
		}
	}

	

	// Pallets use events to inform users when important changes are made.
	// https://docs.substrate.io/v3/runtime/events-and-errors
	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Event documentation should end with an array that provides descriptive names for event
		/// parameters. [something, who]
		SomethingStored(u32, T::AccountId),
		/// ---- Event created when a caller successfully set's their weights
		/// on the chain.
		WeightsSet(T::AccountId),

		/// --- Event created when a new neuron account has been registered to 
		/// the chain.
		NeuronRegistered(u32),

		/// --- Event created when the axon server information is added to the network.
		AxonServed(u32),

		/// --- Event created during when stake has been transfered from 
		/// the coldkey onto the hotkey staking account.
		StakeAdded(T::AccountId, u64),

		/// --- Event created when stake has been removed from 
		/// the staking account into the coldkey account.
		StakeRemoved(T::AccountId, u64),

		/// --- Event created when the difficulty has been set.
		DifficultySet(u64),

		/// --- Event created when default blocks per step has been set.
		BlocksPerStepSet(u64),

		/// --- Event created when bonds moving average set.
		BondsMovingAverageSet(u64),

		/// --- Event created when the difficulty adjustment interval has been set.
		AdjustmentIntervalSet(u64),

		/// --- Event created when the activity cuttoff has been set.
		ActivityCuttoffSet(u64),

		/// --- Event created when the target registrations per interval has been set.
		TargetRegistrationsPerIntervalSet(u64),

		/// --- Event created when mechanism rho has been set.
		RhoSet(u64),

		/// --- Event created when mechanism kappa has been set.
		KappaSet(u64),

		/// --- Event created when max allowed uids has been set.
		MaxAllowedUidsSet(u64),

		/// --- Event created when min allowed weights has been set.
		MinAllowedWeightsSet(u64),

		/// --- Event created when the max allowed max min ration has been set.
		MaxAllowedMaxMinRatioSet( u64 ),

		/// --- Event created when the max weight limit has been set.
		MaxWeightLimitSet( u32 ),

		/// --- Event created when the incentive pruning denominator has been set.
		IncentivePruningDenominatorSet( u64 ),

		/// --- Event created when the stake pruning denominator has been set.
		StakePruningDenominatorSet( u64 ),

		/// --- Event created when the stake pruning min has been set.
		StakePruningMinSet( u64 ),

		/// --- Event created when the foundation account has been set.
		FoundationAccountSet( T::AccountId ),

		/// --- Event created when the foundation distribution has been set.
		FoundationDistributionSet( u64 ),

		/// --- Event created when the scaling law power has been set.
		ScalingLawPowerSet( u8 ),

		/// --- Event created when the synergy scaling law power has been set.
		SynergyScalingLawPowerSet( u8 ),

		/// --- Event created when the validator exclude quantile has been set.
		ValidatorExcludeQuantileSet( u8 ),

		/// --- Event created when the validator pruning length has been set.
		ValidatorPruneLenSet( u64 ),

		/// --- Event created when the validator logits divergence value has been set.
		ValidatorLogitsDivergenceSet( u64 ),

		/// --- Event created when the validator default epoch length has been set.
		ValidatorEpochLenSet(u64),

		/// --- Event created when the validator default epoch per reset has been set.
		ValidatorEpochsPerResetSet(u64),

		/// --- Event created when the batch size has been set.
		ValidatorBatchSizeSet(u64),

		/// --- Event created when the sequence length has been set.
		ValidatorSequenceLengthSet(u64),

		/// --- Event created when the immunity period has been set.
		ImmunityPeriodSet(u64),

		/// --- Event thrown when bonds have been reset.
		ResetBonds()
	}

	// Errors inform users that something went wrong.
	#[pallet::error]
	pub enum Error<T> {
		/// Error names should be descriptive.
		NoneValue,
		/// Errors should have helpful documentation associated with them.
		StorageOverflow,
		 /// ---- Thrown when the user tries to serve an axon which is not of type
	    /// 4 (IPv4) or 6 (IPv6).
		InvalidIpType,

		/// --- Thrown when an invalid IP address is passed to the serve function.
		InvalidIpAddress,

		/// --- Thrown when an invalid modality attempted on serve.
		/// Currently the chain only accepts modality TEXT = 0.
		InvalidModality,

		/// ---- Thrown when the caller attempts to set the weight keys
		/// and values but these vectors have different size.
		WeightVecNotEqualSize,

		/// ---- Thrown when the caller attempts to set weights with duplicate uids
		/// in the weight matrix.
		DuplicateUids,

		/// ---- Thrown when a caller attempts to set weight to at least one uid that
		/// does not exist in the metagraph.
		InvalidUid,

		/// ---- Thrown if the supplied pow hash block is in the future or negative
		InvalidWorkBlock,

		/// ---- Thrown if the supplied pow hash block does not meet the network difficulty.
		InvalidDifficulty,

		/// ---- Thrown if the supplied pow hash seal does not match the supplied work.
		InvalidSeal,

		/// ---- Thrown when registrations this block exceeds allowed number.
		ToManyRegistrationsThisBlock,

		/// ---- Thrown when the caller requests setting or removing data from
		/// a neuron which does not exist in the active set.
		NotRegistered,

		/// ---- Thrown when the caller requests registering a neuron which 
		/// already exists in the active set.
		AlreadyRegistered,

		/// ---- Thrown when a stake, unstake or subscribe request is made by a coldkey
		/// which is not associated with the hotkey account. 
		/// See: fn add_stake and fn remove_stake.
		NonAssociatedColdKey,

		/// ---- Thrown when the caller requests removing more stake then there exists 
		/// in the staking account. See: fn remove_stake.
		NotEnoughStaketoWithdraw,

		///  ---- Thrown when the caller requests adding more stake than there exists
		/// in the cold key account. See: fn add_stake
		NotEnoughBalanceToStake,

		/// ---- Thrown when the caller tries to add stake, but for some reason the requested
		/// amount could not be withdrawn from the coldkey account
		BalanceWithdrawalError,

		/// ---- Thrown when the dispatch attempts to convert between a u64 and T::balance 
		/// but the call fails.
		CouldNotConvertToBalance,

		/// ---- Thrown when the dispatch attempts to set weights on chain with fewer elements 
		/// than are allowed.
		NotSettingEnoughWeights,

		/// ---- Thrown when the dispatch attempts to set weights on chain with where the normalized
		/// max value is more than MaxAllowedMaxMinRatio.
		MaxAllowedMaxMinRatioExceeded,

		/// ---- Thrown when the dispatch attempts to set weights on chain with where any normalized
		/// weight is more than MaxWeightLimit.
		MaxWeightExceeded,

		/// ---- Thrown when the caller attempts to use a repeated work.
		WorkRepeated,

		/// ---- Thrown when the caller attempts to set a storage value outside of its allowed range.
		StorageValueOutOfRange,
	}

	impl<T: Config> Printable for Error<T> {
        fn print(&self) {
            match self {
                Error::AlreadyRegistered => "The node with the supplied public key is already registered".print(),
                Error::NotRegistered  => "The node with the supplied public key is not registered".print(),
                Error::WeightVecNotEqualSize => "The vec of keys and the vec of values are not of the same size".print(),
                Error::NonAssociatedColdKey => "The used cold key is not associated with the hot key acccount".print(),
				Error::StorageValueOutOfRange => "The supplied storage value is outside of its allowed range".print(),
                _ => "Invalid Error Case".print(),
            }
        }
    }

	/// ************************************************************
	/// -Block-Hooks
	/// ************************************************************
	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {

		/// ---- Called on the initialization of this pallet. (the order of on_finalize calls is determined in the runtime)
		///
		/// # Args:
		/// 	* 'n': (T::BlockNumber):
		/// 		- The number of the block we are initializing.
		fn on_initialize( _n: BlockNumberFor<T> ) -> Weight {
			
			// Only run the block step every `blocks_per_step`.
			// Initially `blocks_since_last_step + 1` is 0 but increments until it reaches `blocks_per_step`.
			// We use the >= here in the event that we lower get_blocks per step and these qualities never meet.
			if Self::get_blocks_since_last_step() + 1 >= Self::get_blocks_per_step() {

				// Compute the amount of emission we perform this step.
				// Note that we use blocks_since_last_step here instead of block_per_step incase this is lowered
				// This would mint more tao than is allowed.
				let emission_this_step:u64 = ( Self::get_blocks_since_last_step() + 1 ) * Self::get_block_emission();

				// Apply emission step based on mechanism and updates values.
				Self::mechanism_step( emission_this_step );

				// Reset counter down to 0, this ensures that if `blocks_per_step=1` we will do an emission on every block.
				// If `blocks_per_step=2` we will skip the next block, since (0+1) !>= 2, add one to the counter, and then apply the next
				// token increment where (1+1) >= 2.
				Self::set_blocks_since_last_step( 0 );

			} else {
				// Increment counter.
				Self::set_blocks_since_last_step( Self::get_blocks_since_last_step() + 1 );
			}

			// Make a difficulty update.
			Self::update_difficulty();
			
			return 0;
		}
	}
    

	// Dispatchable functions allows users to interact with the pallet and invoke state changes.
	// These functions materialize as "extrinsics", which are often compared to transactions.
	// Dispatchable functions must be annotated with a weight and must return a DispatchResult.
	#[pallet::call]
	impl<T: Config> Pallet<T> {
        /// --- Sets the caller weights for the incentive mechanism. The call can be
		/// made from the hotkey account so is potentially insecure, however, the damage
		/// of changing weights is minimal if caught early. This function includes all the
		/// checks that the passed weights meet the requirements. Stored as u32s they represent
		/// rational values in the range [0,1] which sum to 1 and can be interpreted as
		/// probabilities. The specific weights determine how inflation propagates outward
		/// from this peer. 
		/// 
		/// Note: The 32 bit integers weights should represent 1.0 as the max u32.
		/// However, the function normalizes all integers to u32_max anyway. This means that if the sum of all
		/// elements is larger or smaller than the amount of elements * u32_max, all elements
		/// will be corrected for this deviation. 
		/// 
		/// # Args:
		/// 	* `origin`: (<T as frame_system::Config>Origin):
		/// 		- The caller, a hotkey who wishes to set their weights.
		/// 
		/// 	* `uids` (Vec<u32>):
		/// 		- The edge endpoint for the weight, i.e. j for w_ij.
		///
		/// 	* 'weights' (Vec<u32>):
		/// 		- The u32 integer encoded weights. Interpreted as rational
		/// 		values in the range [0,1]. They must sum to in32::MAX.
		///
		/// # Event:
		/// 	* WeightsSet;
		/// 		- On successfully setting the weights on chain.
		///
		/// # Raises:
		/// 	* 'WeightVecNotEqualSize':
		/// 		- If the passed weights and uids have unequal size.
		///
		/// 	* 'WeightSumToLarge':
		/// 		- When the calling coldkey is not associated with the hotkey account.
		///
		/// 	* 'InsufficientBalance':
		/// 		- When the amount to stake exceeds the amount of balance in the
		/// 		associated colkey account.
		///
        #[pallet::weight((0, DispatchClass::Normal, Pays::No))]
		pub fn set_weights(
			origin:OriginFor<T>, 
			dests: Vec<u32>, 
			weights: Vec<u32>
		) -> DispatchResult {
			Self::do_set_weights(origin, dests, weights)
		}
		
		/// --- Adds stake to a neuron account. The call is made from the
		/// coldkey account linked in the neurons's NeuronMetadata.
		/// Only the associated coldkey is allowed to make staking and
		/// unstaking requests. This protects the neuron against
		/// attacks on its hotkey running in production code.
		///
		/// # Args:
		/// 	* 'origin': (<T as frame_system::Config>Origin):
		/// 		- The caller, a coldkey signature associated with the hotkey account.
		///
		/// 	* 'hotkey' (T::AccountId):
		/// 		- The hotkey account to add stake to.
		///
		/// 	* 'ammount_staked' (u64):
		/// 		- The ammount to transfer from the balances account of the cold key
		/// 		into the staking account of the hotkey.
		///
		/// # Event:
		/// 	* 'StakeAdded':
		/// 		- On the successful staking of funds.
		///
		/// # Raises:
		/// 	* 'NotRegistered':
		/// 		- If the hotkey account is not active (has not subscribed)
		///
		/// 	* 'NonAssociatedColdKey':
		/// 		- When the calling coldkey is not associated with the hotkey account.
		///
		/// 	* 'InsufficientBalance':
		/// 		- When the amount to stake exceeds the amount of balance in the
		/// 		associated colkey account.
		///
		#[pallet::weight((0, DispatchClass::Normal, Pays::No))]
		pub fn add_stake(
			origin:OriginFor<T>, 
			hotkey: T::AccountId, 
			ammount_staked: u64
		) -> DispatchResult {
			Self::do_add_stake(origin, hotkey, ammount_staked)
		}

		/// ---- Remove stake from the staking account. The call must be made
		/// from the coldkey account attached to the neuron metadata. Only this key
		/// has permission to make staking and unstaking requests.
		///
		/// # Args:
		/// 	* 'origin': (<T as frame_system::Config>Origin):
		/// 		- The caller, a coldkey signature associated with the hotkey account.
		///
		/// 	* 'hotkey' (T::AccountId):
		/// 		- The hotkey account to withdraw stake from.
		///
		/// 	* 'ammount_unstaked' (u64):
		/// 		- The ammount to transfer from the staking account into the balance
		/// 		of the coldkey.
		///
		/// # Event:
		/// 	* 'StakeRemoved':
		/// 		- On successful withdrawl.
		///
		/// # Raises:
		/// 	* 'NonAssociatedColdKey':
		/// 		- When the calling coldkey is not associated with the hotkey account.
		///
		/// 	* 'NotEnoughStaketoWithdraw':
		/// 		- When the amount to unstake exceeds the quantity staked in the
		/// 		associated hotkey staking account.
		///
		#[pallet::weight((0, DispatchClass::Normal, Pays::No))]
		pub fn remove_stake(
			origin:OriginFor<T>, 
			hotkey: T::AccountId, 
			ammount_unstaked: u64
		) -> DispatchResult {
			Self::do_remove_stake(origin, hotkey, ammount_unstaked)
		}

		/// ---- Serves or updates axon information for the neuron associated with the caller. If the caller
		/// already registered the metadata is updated. If the caller is not registered this call throws NotRegsitered.
		///
		/// # Args:
		/// 	* 'origin': (<T as frame_system::Config>Origin):
		/// 		- The caller, a hotkey associated of the registered neuron.
		///
		/// 	* 'ip' (u128):
		/// 		- The u64 encoded IP address of type 6 or 4.
		///
		/// 	* 'port' (u16):
		/// 		- The port number where this neuron receives RPC requests.
		///
		/// 	* 'ip_type' (u8):
		/// 		- The ip type one of (4,6).
		///
		/// 	* 'modality' (u8):
		/// 		- The neuron modality type.
		///
		/// # Event:
		/// 	* 'AxonServed':
		/// 		- On subscription of a new neuron to the active set.
		///
		#[pallet::weight((0, DispatchClass::Normal, Pays::No))]
		pub fn serve_axon (
			origin:OriginFor<T>, 
			version: u32, 
			ip: u128, 
			port: u16, 
			ip_type: u8, 
			modality: u8 
		) -> DispatchResult {
			Self::do_serve_axon( origin, version, ip, port, ip_type, modality )
		}

		/// ---- Registers a new neuron to the graph. 
		///
		/// # Args:
		/// 	* 'origin': (<T as frame_system::Config>Origin):
		/// 		- The caller, registration key as found in RegistrationKey::get(0);
		///
		/// 	* 'block_number' (u64):
		/// 		- Block number of hash to attempt.
		///
		/// 	* 'nonce' (u64):
		/// 		- Hashing nonce as a u64.
		///
		/// 	* 'work' (Vec<u8>):
		/// 		- Work hash as list of bytes.
		/// 
		/// 	* 'hotkey' (T::AccountId,):
		/// 		- Hotkey to register.
		/// 
		/// 	* 'coldkey' (T::AccountId,):
		/// 		- Coldkey to register.
		///
		/// # Event:
		/// 	* 'NeuronRegistered':
		/// 		- On subscription of a new neuron to the active set.
		///
		#[pallet::weight((0, DispatchClass::Normal, Pays::No))]
		pub fn register( 
				origin:OriginFor<T>, 
				block_number: u64, 
				nonce: u64, 
				work: Vec<u8>,
				hotkey: T::AccountId, 
				coldkey: T::AccountId 
		) -> DispatchResult {
			Self::do_registration(origin, block_number, nonce, work, hotkey, coldkey)
		}
		/// ---- SUDO ONLY FUNCTIONS
		///
		/// # Args:
		/// 	* 'origin': (<T as frame_system::Config>Origin):
		/// 		- The caller, must be sudo.
		///
		/// ONE OF:
		/// 	* 'adjustment_interval' (u64):
		/// 	* 'activity_cutoff' (u64):
		/// 	* 'difficulty' (u64):
		///
		/// # Events:
		///		* 'DifficultySet'
		/// 	* 'AdjustmentIntervalSet'
		///		* 'ActivityCuttoffSet'
		///		* 'TargetRegistrationsPerIntervalSet'
		///
		/// 
		#[pallet::weight((0, DispatchClass::Operational, Pays::No))]
		pub fn sudo_set_blocks_per_step ( 
			origin:OriginFor<T>, 
			blocks_per_step: u64 
		) -> DispatchResult {
			ensure_root( origin )?;
			BlocksPerStep::<T>::set( blocks_per_step );
			Self::deposit_event( Event::BlocksPerStepSet( blocks_per_step ) );
			Ok(())
		}

		#[pallet::weight((0, DispatchClass::Operational, Pays::No))]
		pub fn sudo_set_bonds_moving_average ( 
			origin:OriginFor<T>, 
			bonds_moving_average: u64 
		) -> DispatchResult {
			ensure_root( origin )?;
			BondsMovingAverage::<T>::set( bonds_moving_average );
			Self::deposit_event( Event::BondsMovingAverageSet( bonds_moving_average ) );
			Ok(())
		}

		#[pallet::weight((0, DispatchClass::Operational, Pays::No))]
		pub fn sudo_set_difficulty ( 
			origin:OriginFor<T>, 
			difficulty: u64 
		) -> DispatchResult {
			ensure_root( origin )?;
			Difficulty::<T>::set( difficulty );
			Self::deposit_event( Event::DifficultySet( difficulty ) );
			Ok(())
		}

		#[pallet::weight((0, DispatchClass::Operational, Pays::No))]
		pub fn sudo_set_adjustment_interval ( 
			origin:OriginFor<T>, 
			adjustment_interval: u64 
		) -> DispatchResult {
			ensure_root( origin )?;
			AdjustmentInterval::<T>::set( adjustment_interval );
			Self::deposit_event( Event::AdjustmentIntervalSet( adjustment_interval ) );
			Ok(())
		}

		#[pallet::weight((0, DispatchClass::Operational, Pays::No))]
		pub fn sudo_set_activity_cutoff ( 
			origin:OriginFor<T>, 
			activity_cutoff: u64 
		) -> DispatchResult {
			ensure_root( origin )?;
			ActivityCutoff::<T>::set( activity_cutoff );
			Self::deposit_event( Event::ActivityCuttoffSet( activity_cutoff ) );
			Ok(())
		}

		#[pallet::weight((0, DispatchClass::Operational, Pays::No))]
		pub fn sudo_target_registrations_per_interval ( 
			origin:OriginFor<T>, 
			target_registrations_per_interval: u64 
		) -> DispatchResult {
			ensure_root( origin )?;
			TargetRegistrationsPerInterval::<T>::set( target_registrations_per_interval );
			Self::deposit_event( Event::TargetRegistrationsPerIntervalSet( target_registrations_per_interval ) );
			Ok(())
		}

		#[pallet::weight((0, DispatchClass::Operational, Pays::No))]
		pub fn sudo_set_rho ( 
			origin:OriginFor<T>, 
			rho: u64 
		) -> DispatchResult {
			ensure_root( origin )?;
			Rho::<T>::set( rho );
			Self::deposit_event( Event::RhoSet( rho ) );
			Ok(())
		}

		#[pallet::weight((0, DispatchClass::Operational, Pays::No))]
		pub fn sudo_set_kappa ( 
			origin:OriginFor<T>, 
			kappa: u64 
		) -> DispatchResult {
			ensure_root( origin )?;
			Kappa::<T>::set( kappa );
			Self::deposit_event( Event::KappaSet( kappa ) );
			Ok(())
		}

		#[pallet::weight((0, DispatchClass::Operational, Pays::No))]
		pub fn sudo_set_max_allowed_uids ( 
			origin:OriginFor<T>, 
			max_allowed_uids: u64 
		) -> DispatchResult {
			ensure_root( origin )?;
			MaxAllowedUids::<T>::set( max_allowed_uids );
			Self::deposit_event( Event::MaxAllowedUidsSet( max_allowed_uids ) );
			Ok(())
		}

		#[pallet::weight((0, DispatchClass::Operational, Pays::No))]
		pub fn sudo_set_min_allowed_weights ( 
			origin:OriginFor<T>, 
			min_allowed_weights: u64 
		) -> DispatchResult {
			ensure_root( origin )?;
			MinAllowedWeights::<T>::set( min_allowed_weights );
			Self::deposit_event( Event::MinAllowedWeightsSet( min_allowed_weights ) );
			Ok(())
		}

		#[pallet::weight((0, DispatchClass::Operational, Pays::No))]
		pub fn sudo_set_max_allowed_max_min_ratio ( 
			origin:OriginFor<T>, 
			max_allowed_max_min_ratio: u64 
		) -> DispatchResult {
			ensure_root( origin )?;
			MaxAllowedMaxMinRatio::<T>::set( max_allowed_max_min_ratio );
			Self::deposit_event( Event::MaxAllowedMaxMinRatioSet( max_allowed_max_min_ratio ) );
			Ok(())
		}

		#[pallet::weight((0, DispatchClass::Operational, Pays::No))]
		pub fn sudo_set_max_weight_limit ( 
			origin:OriginFor<T>, 
			max_weight_limit: u32 
		) -> DispatchResult {
			ensure_root( origin )?;
			MaxWeightLimit::<T>::set( max_weight_limit );
			Self::deposit_event( Event::MaxWeightLimitSet( max_weight_limit ) );
			Ok(())
		}

		#[pallet::weight((0, DispatchClass::Operational, Pays::No))]
		pub fn sudo_set_validator_batch_size ( 
			origin:OriginFor<T>, 
			validator_batch_size: u64 
		) -> DispatchResult {
			ensure_root( origin )?;
			ValidatorBatchSize::<T>::set( validator_batch_size );
			Self::deposit_event( Event::ValidatorBatchSizeSet( validator_batch_size ) );
			Ok(())
		}

		#[pallet::weight((0, DispatchClass::Operational, Pays::No))]
		pub fn sudo_set_validator_sequence_length ( 
			origin:OriginFor<T>, 
			validator_sequence_length: u64 
		) -> DispatchResult {
			ensure_root( origin )?;
			ValidatorSequenceLength::<T>::set( validator_sequence_length );
			Self::deposit_event( Event::ValidatorSequenceLengthSet( validator_sequence_length ) );
			Ok(())
		}

		#[pallet::weight((0, DispatchClass::Operational, Pays::No))]
		pub fn sudo_set_validator_epoch_len ( 
			origin:OriginFor<T>, 
			validator_epoch_len : u64 
		) -> DispatchResult {
			ensure_root( origin )?;
			ValidatorEpochLen::<T>::set( validator_epoch_len );
			Self::deposit_event( Event::ValidatorEpochLenSet( validator_epoch_len ) );
			Ok(())
		}

		#[pallet::weight((0, DispatchClass::Operational, Pays::No))]
		pub fn sudo_set_validator_epochs_per_reset ( 
			origin:OriginFor<T>, 
			validator_epochs_per_reset : u64 
		) -> DispatchResult {
			ensure_root( origin )?;
			ValidatorEpochsPerReset::<T>::set( validator_epochs_per_reset );
			Self::deposit_event( Event::ValidatorEpochsPerResetSet( validator_epochs_per_reset ) );
			Ok(())
		}

		#[pallet::weight((0, DispatchClass::Operational, Pays::No))]
		pub fn sudo_set_incentive_pruning_denominator( 
			origin:OriginFor<T>, 
			incentive_pruning_denominator: u64 
		) -> DispatchResult {
			ensure_root( origin )?;
			IncentivePruningDenominator::<T>::set( incentive_pruning_denominator );
			Self::deposit_event( Event::IncentivePruningDenominatorSet( incentive_pruning_denominator ));
			Ok(())
		}
		
		#[pallet::weight((0, DispatchClass::Operational, Pays::No))]
		pub fn sudo_set_stake_pruning_denominator( 
			origin:OriginFor<T>, 
			stake_pruning_denominator: u64 
		) -> DispatchResult {
			ensure_root( origin )?;
			StakePruningDenominator::<T>::set( stake_pruning_denominator );
			Self::deposit_event( Event::StakePruningDenominatorSet( stake_pruning_denominator ));
			Ok(())
		}

		#[pallet::weight((0, DispatchClass::Operational, Pays::No))]
		pub fn sudo_set_stake_pruning_min( 
			origin:OriginFor<T>, 
			stake_pruning_min: u64 
		) -> DispatchResult {
			ensure_root( origin )?;
			StakePruningMin::<T>::set( stake_pruning_min );
			Self::deposit_event( Event::StakePruningMinSet( stake_pruning_min ));
			Ok(())
		}

		#[pallet::weight((0, DispatchClass::Operational, Pays::No))]
		pub fn sudo_set_immunity_period ( 
			origin:OriginFor<T>, 
			immunity_period: u64 
		) -> DispatchResult {
			ensure_root( origin )?;
			ImmunityPeriod::<T>::set( immunity_period );
			Self::deposit_event( Event::ImmunityPeriodSet( immunity_period ) );
			Ok(())
		}

		#[pallet::weight((0, DispatchClass::Operational, Pays::No))]
		pub fn sudo_reset_bonds ( 
			origin:OriginFor<T>
		) -> DispatchResult {
			ensure_root( origin )?;
			Self::reset_bonds();
			Self::deposit_event( Event::ResetBonds() );
			Ok(())
		}

		#[pallet::weight((0, DispatchClass::Operational, Pays::No))]
		pub fn sudo_set_scaling_law_power( 
			origin:OriginFor<T>, 
			scaling_law_power: u8 
		) -> DispatchResult {
			ensure_root( origin )?;
			ensure!( scaling_law_power <= 100, Error::<T>::StorageValueOutOfRange  ); // The power must be between 0 and 100 => 0% and 100%
			ScalingLawPower::<T>::set( scaling_law_power );
			Self::deposit_event( Event::ScalingLawPowerSet( scaling_law_power ));
			Ok(())
		}

		#[pallet::weight((0, DispatchClass::Operational, Pays::No))]
		pub fn sudo_set_synergy_scaling_law_power( 
			origin:OriginFor<T>, 
			synergy_scaling_law_power: u8 
		) -> DispatchResult {
			ensure_root( origin )?;
			ensure!( synergy_scaling_law_power <= 100, Error::<T>::StorageValueOutOfRange ); // The power must be between 0 and 100 => 0% and 100%
		    SynergyScalingLawPower::<T>::set( synergy_scaling_law_power );
			Self::deposit_event( Event::SynergyScalingLawPowerSet( synergy_scaling_law_power ));
			Ok(())
		}

		#[pallet::weight((0, DispatchClass::Operational, Pays::No))]
		pub fn sudo_set_validator_exclude_quantile( 
			origin:OriginFor<T>, 
			validator_exclude_quantile: u8 
		) -> DispatchResult {
			ensure_root( origin )?;
			ensure!( validator_exclude_quantile <= 100, Error::<T>::StorageValueOutOfRange ); // The quantile must be between 0 and 100 => 0% and 100%
		    ValidatorExcludeQuantile::<T>::set( validator_exclude_quantile );
			Self::deposit_event( Event::ValidatorExcludeQuantileSet( validator_exclude_quantile ));
			Ok(())
		}

		#[pallet::weight((0, DispatchClass::Operational, Pays::No))]
		pub fn sudo_set_validator_prune_len( 
			origin:OriginFor<T>, 
			validator_prune_len: u64 
		) -> DispatchResult {
			ensure_root( origin )?;
		    ValidatorPruneLen::<T>::set( validator_prune_len );
			Self::deposit_event( Event::ValidatorPruneLenSet( validator_prune_len ));
			Ok(())
		}

		#[pallet::weight((0, DispatchClass::Operational, Pays::No))]
		pub fn sudo_set_validator_logits_divergence( 
			origin:OriginFor<T>, 
			validator_logits_divergence: u64 
		) -> DispatchResult {
			ensure_root( origin )?;
		    ValidatorLogitsDivergence::<T>::set( validator_logits_divergence );
			Self::deposit_event( Event::ValidatorLogitsDivergenceSet( validator_logits_divergence ));
			Ok(())
		}
	}

	// ---- Subtensor helper functions.
	impl<T: Config> Pallet<T> {

		// TURN ON DEBUG
		pub fn debug() -> bool {
			return T::SDebug::get() == 1
		}

		// Adjustable Constants.
		// -- Blocks per step.
		pub fn get_blocks_since_last_step( ) -> u64 {
			BlocksSinceLastStep::<T>::get()
		}
		pub fn set_blocks_since_last_step( blocks_since_last_step: u64 ) {
			BlocksSinceLastStep::<T>::set( blocks_since_last_step );
		}
		pub fn get_blocks_per_step( ) -> u64 {
			BlocksPerStep::<T>::get()
		}
		pub fn set_blocks_per_step( blocks_per_step: u64 ) {
			BlocksPerStep::<T>::set( blocks_per_step );
		}
		pub fn get_bonds_moving_average( ) -> u64 {
			BondsMovingAverage::<T>::get()
		}
		pub fn set_bonds_moving_average( bonds_moving_average: u64 ) {
			BondsMovingAverage::<T>::set( bonds_moving_average );
		}
		// -- Difficulty.
		pub fn get_difficulty( ) -> U256 {
			return U256::from( Self::get_difficulty_as_u64() );
		}
		pub fn get_difficulty_as_u64( ) -> u64 {
			Difficulty::<T>::get()
		}
		pub fn set_difficulty_from_u64( difficulty: u64 ) {
			Difficulty::<T>::set( difficulty );
		}
		// -- Activity cuttoff
		pub fn get_activity_cutoff( ) -> u64 {
			return ActivityCutoff::<T>::get();
		}
		pub fn set_activity_cutoff( cuttoff: u64 ) {
			ActivityCutoff::<T>::set( cuttoff );
		}
		// -- Adjustment Interval.
		pub fn get_adjustment_interval() -> u64 {
			AdjustmentInterval::<T>::get()
		}
		pub fn set_adjustment_interval( interval: u64 ) {
			AdjustmentInterval::<T>::put( interval );
		}
		// -- Target registrations per interval.
		pub fn get_target_registrations_per_interval() -> u64 {
			TargetRegistrationsPerInterval::<T>::get()
		}
		pub fn set_target_registrations_per_interval( target: u64 ) {
			TargetRegistrationsPerInterval::<T>::put( target );
		}
		pub fn get_max_registratations_per_block( ) -> u64 {
			MaxRegistrationsPerBlock::<T>::get()
		}
		pub fn set_max_registratations_per_block( max_registrations: u64 ){
			MaxRegistrationsPerBlock::<T>::put( max_registrations );
		}
		// -- Minimum difficulty
		pub fn get_minimum_difficulty( ) -> u64 {
			return T::MinimumDifficulty::get();
		}
		// -- Maximum difficulty
		pub fn get_maximum_difficulty( ) -> u64 {
			return T::MaximumDifficulty::get();
		}
		// -- Get Block emission.
		pub fn get_block_emission( ) -> u64 {
			return 1000000000;
		}
		// -- Get step consensus temperature (rho)
		pub fn get_rho( ) -> u64 {
			return Rho::<T>::get();
		}
		pub fn set_rho( rho: u64 ) {
			Rho::<T>::put( rho );
		}
		pub fn get_incentive_pruning_denominator( ) -> u64 {
			return IncentivePruningDenominator::<T>::get();
		}
		pub fn set_incentive_pruning_denominator( incentive_pruning_denominator: u64 ) {
			IncentivePruningDenominator::<T>::put( incentive_pruning_denominator );
		}
		pub fn get_stake_pruning_denominator( ) -> u64 {
			return StakePruningDenominator::<T>::get();
		}
		pub fn set_stake_pruning_denominator( stake_pruning_denominator: u64 ) {
			StakePruningDenominator::<T>::put( stake_pruning_denominator );
		}
		pub fn get_stake_pruning_min( ) -> u64 {
			return StakePruningMin::<T>::get();
		}
		pub fn set_stake_pruning_min( stake_pruning_min: u64 ) {
			StakePruningMin::<T>::put( stake_pruning_min );
		}

		pub fn get_validator_sequence_length( ) -> u64 {
			return ValidatorSequenceLength::<T>::get();
		}
		pub fn set_validator_sequence_length( validator_sequence_length: u64 ) {
			ValidatorSequenceLength::<T>::put( validator_sequence_length );
		}
		pub fn get_validator_batch_size( ) -> u64 {
			return ValidatorBatchSize::<T>::get();
		}
		pub fn set_validator_batch_size( validator_batch_size: u64 ) {
			ValidatorBatchSize::<T>::put( validator_batch_size );
		}
		pub fn get_validator_epoch_len( ) -> u64 {
			return ValidatorEpochLen::<T>::get();
		}
		pub fn set_validator_epoch_len( validator_epoch_len: u64 ) {
			ValidatorEpochLen::<T>::put( validator_epoch_len );
		}
		pub fn get_validator_epochs_per_reset( ) -> u64 {
			return ValidatorEpochsPerReset::<T>::get();
		}
		pub fn set_validator_epochs_per_reset( validator_epochs_per_reset: u64 ) {
			ValidatorEpochsPerReset::<T>::put( validator_epochs_per_reset );
		}

		pub fn get_scaling_law_power( ) -> u8 {
			return ScalingLawPower::<T>::get();
		}
		pub fn set_scaling_law_power( scaling_law_power: u8 ) {
			ScalingLawPower::<T>::put( scaling_law_power );
		}

		pub fn get_synergy_scaling_law_power( ) -> u8 {
			return SynergyScalingLawPower::<T>::get();
		}
		pub fn set_synergy_scaling_law_power( synergy_scaling_law_power: u8 ) {
			SynergyScalingLawPower::<T>::put( synergy_scaling_law_power );
		}

		pub fn get_validator_exclude_quantile( ) -> u8 {
			return ValidatorExcludeQuantile::<T>::get();
		}
		pub fn set_validator_exclude_quantile( validator_exclude_quantile: u8 ) {
			ValidatorExcludeQuantile::<T>::put( validator_exclude_quantile );
		}

		pub fn get_validator_prune_len( ) -> u64 {
			return ValidatorPruneLen::<T>::get();
		}
		pub fn set_validator_prune_len( validator_prune_len: u64 ) {
			ValidatorPruneLen::<T>::put( validator_prune_len );
		}

		pub fn get_validator_logits_divergence( ) -> u64 {
			return ValidatorLogitsDivergence::<T>::get();
		}
		pub fn set_validator_logits_divergence( validator_logits_divergence: u64 ) {
			ValidatorLogitsDivergence::<T>::put( validator_logits_divergence );
		}

		// -- Get step consensus shift (1/kappa)
		pub fn get_kappa( ) -> u64 {
			return Kappa::<T>::get();
		}
		pub fn set_kappa( kappa: u64 ) {
			Kappa::<T>::put( kappa );
		}
		// -- Get self ownership proportion denominator
		pub fn get_self_ownership( ) -> u64 {
			return T::SelfOwnership::get();
		}
		pub fn get_last_mechanism_step_block( ) -> u64 {
			return LastMechansimStepBlock::<T>::get();
		}
		pub fn get_max_allowed_uids( ) -> u64 {
			return MaxAllowedUids::<T>::get();
		}
		pub fn set_max_allowed_uids( max_allowed_uids: u64 ) {
			MaxAllowedUids::<T>::put( max_allowed_uids );
		}
		pub fn get_min_allowed_weights( ) -> u64 {
			return MinAllowedWeights::<T>::get();
		}
		pub fn set_min_allowed_weights( min_allowed_weights: u64 ) {
			MinAllowedWeights::<T>::put( min_allowed_weights );
		}
		pub fn get_max_allowed_max_min_ratio( ) -> u64 {
			return MaxAllowedMaxMinRatio::<T>::get();
		}
		pub fn set_max_allowed_max_min_ratio( max_allowed_max_min_ratio: u64 ) {
			MaxAllowedMaxMinRatio::<T>::put( max_allowed_max_min_ratio );
		}
		pub fn get_max_weight_limit( ) -> u32 {
			return MaxWeightLimit::<T>::get();
		}
		pub fn set_max_weight_limit( max_weight_limit: u32 ) {
			MaxWeightLimit::<T>::put( max_weight_limit );
		}
		pub fn get_immunity_period( ) -> u64 {
			return ImmunityPeriod::<T>::get();
		}
		pub fn set_immunity_period( immunity_period: u64 ) {
			ImmunityPeriod::<T>::put( immunity_period );
		}

		// Variable Parameters
		pub fn get_registrations_this_interval( ) -> u64 {
			RegistrationsThisInterval::<T>::get()
		}
		pub fn get_registrations_this_block( ) -> u64 {
			RegistrationsThisBlock::<T>::get()
		}
		pub fn get_total_stake( ) -> u64 {
			return TotalStake::<T>::get();
		}
		pub fn get_total_issuance( ) -> u64 {
			return TotalIssuance::<T>::get();
		}
		pub fn get_initial_total_issuance( ) -> u64 {
			return T::InitialIssuance::get();
		}
		pub fn get_lastupdate( ) -> Vec<u64> {
			let mut result: Vec<u64> = vec![ 0; Self::get_neuron_count() as usize ];
			for ( uid_i, neuron_i ) in <Neurons<T> as IterableStorageMap<u32, NeuronMetadataOf<T>>>::iter() {
				result[ uid_i as usize ] = neuron_i.last_update;
			}
			return result
		}
		pub fn get_stake( ) -> Vec<u64> {
			let mut result: Vec<u64> = vec![ 0; Self::get_neuron_count() as usize ];
			for ( uid_i, neuron_i ) in <Neurons<T> as IterableStorageMap<u32, NeuronMetadataOf<T>>>::iter() {
				result[ uid_i as usize ] = neuron_i.stake;
			}
			return result
		}
		pub fn get_ranks( ) -> Vec<u64> {
			let mut result: Vec<u64> = vec![ 0; Self::get_neuron_count() as usize ];
			for ( uid_i, neuron_i ) in <Neurons<T> as IterableStorageMap<u32, NeuronMetadataOf<T>>>::iter() {
				result[ uid_i as usize ] = neuron_i.rank;
			}
			return result
		}
		pub fn get_trust( ) -> Vec<u64> {
			let mut result: Vec<u64> = vec![ 0; Self::get_neuron_count() as usize ];
			for ( uid_i, neuron_i ) in <Neurons<T> as IterableStorageMap<u32, NeuronMetadataOf<T>>>::iter() {
				result[ uid_i as usize ] = neuron_i.trust;
			}
			return result
		}
		pub fn get_consensus( ) -> Vec<u64> {
			let mut result: Vec<u64> = vec![ 0; Self::get_neuron_count() as usize ];
			for ( uid_i, neuron_i ) in <Neurons<T> as IterableStorageMap<u32, NeuronMetadataOf<T>>>::iter() {
				result[ uid_i as usize ] = neuron_i.consensus;
			}
			return result
		}
		pub fn get_incentive( ) -> Vec<u64> {
			let mut result: Vec<u64> = vec![ 0; Self::get_neuron_count() as usize ];
			for ( uid_i, neuron_i ) in <Neurons<T> as IterableStorageMap<u32, NeuronMetadataOf<T>>>::iter() {
				result[ uid_i as usize ] = neuron_i.incentive;
			}
			return result
		}
		pub fn get_dividends( ) -> Vec<u64> {
			let mut result: Vec<u64> = vec![ 0; Self::get_neuron_count() as usize ];
			for ( uid_i, neuron_i ) in <Neurons<T> as IterableStorageMap<u32, NeuronMetadataOf<T>>>::iter() {
				result[ uid_i as usize] = neuron_i.dividends;
			}
			return result
		}
		pub fn get_emission( ) -> Vec<u64> {
			let mut result: Vec<u64> = vec![ 0; Self::get_neuron_count() as usize ];
			for ( uid_i, neuron_i ) in <Neurons<T> as IterableStorageMap<u32, NeuronMetadataOf<T>>>::iter() {
				result[ uid_i as usize ] = neuron_i.emission;
			}
			return result
		}
		pub fn get_active( ) -> Vec<u32> {
			let mut result: Vec<u32> = vec![ 0; Self::get_neuron_count() as usize ];
			for ( uid_i, neuron_i ) in <Neurons<T> as IterableStorageMap<u32, NeuronMetadataOf<T>>>::iter() {
				result[ uid_i as usize] = neuron_i.active;
			}
			return result
		}
		pub fn get_bonds_for_neuron( neuron: &NeuronMetadataOf<T> ) -> Vec<u64>  {
			let mut bonds: Vec<u64> = vec![ 0; Self::get_neuron_count() as usize ];
			for (uid_j, bonds_ij) in neuron.bonds.iter(){
				bonds[ *uid_j as usize ] = *bonds_ij;
			}
			return bonds
		}
		pub fn get_bonds( ) -> Vec<Vec<u64>>  {
			let mut bonds: Vec<Vec<u64>> = vec![ vec![]; Self::get_neuron_count() as usize ];
			for ( uid_i, neuron_i ) in <Neurons<T> as IterableStorageMap<u32, NeuronMetadataOf<T>>>::iter() {
				bonds[ uid_i as usize ] = Self::get_bonds_for_neuron( &neuron_i );
			}
			return bonds
		}
		pub fn get_weights_for_neuron( neuron: &NeuronMetadataOf<T> ) -> Vec<u32>  {
			let mut weights: Vec<u32> = vec![ 0; Self::get_neuron_count() as usize ];
			for (uid_j, weights_ij) in neuron.weights.iter(){
				weights[ *uid_j as usize ] = *weights_ij;
			}
			return weights
		}
		pub fn get_weights( ) -> Vec<Vec<u32>>  {
			let mut weights: Vec<Vec<u32>> = vec![ vec![]; Self::get_neuron_count() as usize ];
			for ( uid_i, neuron_i ) in <Neurons<T> as IterableStorageMap<u32, NeuronMetadataOf<T>>>::iter() {
				weights[ uid_i as usize ] = Self::get_weights_for_neuron( &neuron_i );
			}
			return weights
		}		

		// Setters
		pub fn set_stake_from_vector( stake: Vec<u64> ) {
			let mut total_stake: u64 = 0;
			for uid_i in 0..Self::get_neuron_count() {
				let mut neuron = Neurons::<T>::get(uid_i).unwrap();
				neuron.stake = stake[ uid_i as usize ];
				Neurons::<T>::insert( uid_i, neuron );
				total_stake += stake[ uid_i as usize ];
			}
			TotalStake::<T>::set( total_stake );
		}
		pub fn set_last_update_from_vector( last_update: Vec<u64> ) {
			for uid_i in 0..Self::get_neuron_count() {
				let mut neuron = Neurons::<T>::get(uid_i).unwrap();
				neuron.last_update = last_update[ uid_i as usize ];
				Neurons::<T>::insert( uid_i, neuron );
			}
		}
		pub fn set_weights_from_matrix( weights: Vec<Vec<u32>> ) {
			for uid_i in 0..Self::get_neuron_count() {
				let mut sparse_weights: Vec<(u32, u32)> = vec![];
				for uid_j in 0..Self::get_neuron_count() {
					let weight_ij: u32 = weights[uid_i as usize][uid_j as usize];
					if weight_ij != 0 {
						sparse_weights.push( (uid_j, weight_ij) );
					}
				}
				let mut neuron = Neurons::<T>::get(uid_i).unwrap();
				neuron.weights = sparse_weights;
				Neurons::<T>::insert( uid_i, neuron );
			}
		}

		pub fn set_bonds_from_matrix( bonds: Vec<Vec<u64>> ) {
			for uid_i in 0..Self::get_neuron_count() {
				let mut sparse_bonds: Vec<(u32, u64)> = vec![];
				for uid_j in 0..Self::get_neuron_count() {
					let bond_ij: u64 = bonds[uid_i as usize][uid_j as usize];
					sparse_bonds.push( (uid_j, bond_ij) );
				}
				let mut neuron = Neurons::<T>::get(uid_i).unwrap();
				neuron.bonds = sparse_bonds;
				Neurons::<T>::insert( uid_i, neuron );
			}
		}
	
		// Helpers.
		// --- Returns Option if the u64 converts to a balance
		// use .unwarp if the result returns .some().
		pub fn u64_to_balance(input: u64) -> Option<<<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance>
		{
			input.try_into().ok()
		}

		// --- Returns true if the account-id has an active
		// account on chain.
		pub fn add_hotkey_to_active_set(hotkey_id: &T::AccountId, uid: u32) {
			Hotkeys::<T>::insert(&hotkey_id, uid);
		}

		// --- Returns true if the account-id has an active
		// account on chain.
		pub fn is_hotkey_active(hotkey_id: &T::AccountId) -> bool {
			return Hotkeys::<T>::contains_key(&hotkey_id);
		}

		// --- Returns false if the account-id has an active
		// account on chain.
		pub fn is_not_active(hotkey_id: &T::AccountId) -> bool {
			return !Self::is_hotkey_active(hotkey_id);
		}

		// --- Returns true if the uid is to be prunned at the next epoch.
		pub fn will_be_prunned ( uid:u32 ) -> bool {
			return NeuronsToPruneAtNextEpoch::<T>::contains_key( uid );
		}

		// --- Returns true if the uid is active, i.e. there
		// is a staking, last_update, and neuron account associated
		// with this uid.
		pub fn is_uid_active(uid: u32) -> bool {
			return Neurons::<T>::contains_key(uid);
		}

		// --- Returns hotkey associated with the hotkey account.
		// This should be called in conjunction with is_hotkey_active
		// to ensure this function does not throw an error.
		pub fn get_uid_for_hotkey(hotkey_id: &T::AccountId) -> u32{
			return Hotkeys::<T>::get(&hotkey_id);
		}
		pub fn get_neuron_for_uid ( uid: u32 ) -> NeuronMetadataOf<T> {
			return Neurons::<T>::get( uid ).unwrap();
		}

		// --- Returns the neuron associated with the passed hotkey.
		// The function makes a double mapping from hotkey -> uid -> neuron.
		pub fn get_neuron_for_hotkey(hotkey_id: &T::AccountId) -> NeuronMetadataOf<T> {
			let uid = Self::get_uid_for_hotkey(hotkey_id);
			return Self::get_neuron_for_uid(uid);
		}

		// --- Returns the next available network uid.
		// uids increment up to u64:MAX, this allows the chain to
		// have 18,446,744,073,709,551,615 peers before an overflow.
		pub fn get_neuron_count() -> u32 {
			let uid = N::<T>::get();
			uid
		}

		// --- Returns the next available network uid and increments uid.
		pub fn get_next_uid() -> u32 {
			let uid = N::<T>::get();
			assert!(uid < u32::MAX);  // The system should fail if this is ever reached.
			N::<T>::put(uid + 1);
			uid
		}

		// --- Returns a vanilla transaction fee for transactions as rao.
		pub fn calculate_transaction_fee(len: u64) -> u64 {
			return len * 100;
		}

		// --- Returns the transaction priority for setting weights.
		pub fn get_priority_set_weights( hotkey: &T::AccountId, len: u64 ) -> u64 {
			if Hotkeys::<T>::contains_key( hotkey ) {
				let uid = Hotkeys::<T>::get( hotkey );
				let neuron = Neurons::<T>::get( uid ).unwrap();
				// Multiply here by 1_000_000 since len may divide all log values to zero.
				// a peer with 1 tao will have priority 29 000 000 000 after 1 epoch.
				// with 10 tao 33 000 000 000
				// with 100 tao 36 000 000 000
				// with 1000 tao 39 000 000 000
				// with 10000 tao 43 000 000 000
				// division by len will always return a non zero value with which to differentiate. 
				return neuron.priority * 1_000_000 / len;
			} else{
				return 0;
			}
		}

	}
}

/************************************************************
	CallType definition
************************************************************/
#[derive(Debug, PartialEq)]
pub enum CallType {
    SetWeights,
    AddStake,
    RemoveStake,
    Register,
    Serve,
	Other,
}
impl Default for CallType {
    fn default() -> Self {
        CallType::Other
    }
}


/************************************************************
	SubtensorSignedExtension definition
************************************************************/

#[derive(Encode, Decode, Clone, Eq, PartialEq, scale_info::TypeInfo)]
#[scale_info(skip_type_params(T))]
pub struct SubtensorSignedExtension<T: Config + Send + Sync>(pub PhantomData<T>);

impl<T: Config + Send + Sync> SubtensorSignedExtension<T> where
    T::Call: Dispatchable<Info=DispatchInfo, PostInfo=PostDispatchInfo>,
    <T as frame_system::Config>::Call: IsSubType<Call<T>>,
{
    pub fn new() -> Self {
        Self(Default::default())
	}
    pub fn get_priority_vanilla() -> u64 {
        // Just return a rediculously high priority. This means that all extrinsics except
        // the set_weights function will have a priority over the set_weights calls.
        return u64::max_value();
    }
	pub fn get_priority_set_weights( who: &T::AccountId, len: u64 ) -> u64 {
		// Return the non vanilla priority for a set weights call.
        return Pallet::<T>::get_priority_set_weights( who, len );
    }
}

impl<T: Config + Send + Sync> sp_std::fmt::Debug for SubtensorSignedExtension<T> {
    fn fmt(&self, f: &mut sp_std::fmt::Formatter) -> sp_std::fmt::Result {
        write!(f, "SubtensorSignedExtension")
    }
}

impl<T: Config + Send + Sync> SignedExtension for SubtensorSignedExtension<T>
    where
        T::Call: Dispatchable<Info=DispatchInfo, PostInfo=PostDispatchInfo>,
        <T as frame_system::Config>::Call: IsSubType<Call<T>>,
{
	const IDENTIFIER: &'static str = "SubtensorSignedExtension";

    type AccountId = T::AccountId;
    type Call = <T as frame_system::Config>::Call;
    //<T as frame_system::Trait>::Call;
    type AdditionalSigned = ();
    type Pre = (CallType, u64, Self::AccountId);
    fn additional_signed(&self) -> Result<Self::AdditionalSigned, TransactionValidityError> { Ok(()) }

    fn validate(
        &self,
        who: &Self::AccountId,
        call: &Self::Call,
        _info: &DispatchInfoOf<Self::Call>,
        len: usize,
    ) -> TransactionValidity {
        match call.is_sub_type() {
            Some(Call::set_weights{..}) => {
				let priority: u64 = Self::get_priority_set_weights(who, len as u64);
                Ok(ValidTransaction {
                    priority: priority,
                    longevity: 1,
                    ..Default::default()
                })
            }
            Some(Call::add_stake{..}) => {
                Ok(ValidTransaction {
                    priority: Self::get_priority_vanilla(),
                    ..Default::default()
                })
            }
            Some(Call::remove_stake{..}) => {
                Ok(ValidTransaction {
                    priority: Self::get_priority_vanilla(),
                    ..Default::default()
                })
            }
            Some(Call::register{..}) => {
                Ok(ValidTransaction {
                    priority: Self::get_priority_vanilla(),
                    ..Default::default()
                })
            }
            _ => {
                Ok(ValidTransaction {
                    priority: Self::get_priority_vanilla(),
                    ..Default::default()
                })
            }
        }
    }

    // NOTE: Add later when we put in a pre and post dispatch step.
    fn pre_dispatch(
        self,
        who: &Self::AccountId,
        call: &Self::Call,
        _info: &DispatchInfoOf<Self::Call>,
        _len: usize,
    ) -> Result<Self::Pre, TransactionValidityError> {

        match call.is_sub_type() {
            Some(Call::add_stake{..}) => {
				let transaction_fee = 0;
                Ok((CallType::AddStake, transaction_fee, who.clone()))
            }
            Some(Call::remove_stake{..}) => {
				let transaction_fee = 0;
                Ok((CallType::RemoveStake, transaction_fee, who.clone()))
            }
			Some(Call::set_weights{..}) => {
				let transaction_fee = 0;
                Ok((CallType::SetWeights, transaction_fee, who.clone())) 
            }
			Some(Call::register{..}) => {
                let transaction_fee = 0;
                Ok((CallType::Register, transaction_fee, who.clone()))
            }
            Some(Call::serve_axon{..}) => {
                let transaction_fee = 0;
                Ok((CallType::Serve, transaction_fee, who.clone()))
            }
            _ => {
				let transaction_fee = 0;
                Ok((CallType::Other, transaction_fee, who.clone()))
            }
        }
    }
}
