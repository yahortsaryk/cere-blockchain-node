//! Autogenerated weights for pallet_ddc_payouts
//!
//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 4.0.0-dev
//! DATE: 2023-12-20, STEPS: `50`, REPEAT: 20, LOW RANGE: `[]`, HIGH RANGE: `[]`
//! HOSTNAME: `bench`, CPU: `DO-Premium-AMD`
//! EXECUTION: Some(Wasm), WASM-EXECUTION: Interpreted, CHAIN: Some("dev"), DB CACHE: 1024

// Executed Command:
// ./target/release/cere
// benchmark
// pallet
// --chain=dev
// --execution=wasm
// --pallet=pallet-ddc-payouts
// --extrinsic=*
// --steps=50
// --repeat=20
// --template=./.maintain/frame-weight-template.hbs
// --output=pallets/ddc-payouts/src/weights.rs

#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused_parens)]
#![allow(unused_imports)]

use frame_support::{traits::Get, weights::{Weight, constants::RocksDbWeight}};
use sp_std::marker::PhantomData;

/// Weight functions needed for pallet_ddc_payouts.
pub trait WeightInfo {
	fn set_authorised_caller() -> Weight;
	fn begin_billing_report() -> Weight;
	fn begin_charging_customers() -> Weight;
	fn send_charging_customers_batch(b: u32, ) -> Weight;
	fn end_charging_customers() -> Weight;
	fn begin_rewarding_providers() -> Weight;
	fn send_rewarding_providers_batch(b: u32, ) -> Weight;
	fn end_rewarding_providers() -> Weight;
	fn end_billing_report() -> Weight;
}

/// Weights for pallet_ddc_payouts using the Substrate node and recommended hardware.
pub struct SubstrateWeight<T>(PhantomData<T>);
impl<T: frame_system::Config> WeightInfo for SubstrateWeight<T> {
	// Storage: DdcPayouts AuthorisedCaller (r:0 w:1)
	fn set_authorised_caller() -> Weight {
		Weight::from_ref_time(90_258_000_u64)
			.saturating_add(T::DbWeight::get().writes(1_u64))
	}
	// Storage: DdcPayouts AuthorisedCaller (r:1 w:0)
	// Storage: DdcPayouts ActiveBillingReports (r:1 w:1)
	fn begin_billing_report() -> Weight {
		Weight::from_ref_time(214_646_000_u64)
			.saturating_add(T::DbWeight::get().reads(2_u64))
			.saturating_add(T::DbWeight::get().writes(1_u64))
	}
	// Storage: DdcPayouts AuthorisedCaller (r:1 w:0)
	// Storage: DdcPayouts ActiveBillingReports (r:1 w:1)
	fn begin_charging_customers() -> Weight {
		Weight::from_ref_time(228_676_000_u64)
			.saturating_add(T::DbWeight::get().reads(2_u64))
			.saturating_add(T::DbWeight::get().writes(1_u64))
	}
	// Storage: DdcPayouts AuthorisedCaller (r:1 w:0)
	// Storage: DdcPayouts ActiveBillingReports (r:1 w:1)
	// Storage: DdcClusters ClustersGovParams (r:1 w:0)
	// Storage: DdcCustomers Ledger (r:1 w:1)
	// Storage: System Account (r:2 w:2)
	// Storage: DdcPayouts DebtorCustomers (r:1 w:1)
	/// The range of component `b` is `[1, 1000]`.
	fn send_charging_customers_batch(b: u32, ) -> Weight {
		Weight::from_ref_time(891_324_000_u64)
			// Standard Error: 3_864_375
			.saturating_add(Weight::from_ref_time(558_679_506_u64).saturating_mul(b as u64))
			.saturating_add(T::DbWeight::get().reads(7_u64))
			.saturating_add(T::DbWeight::get().reads((2_u64).saturating_mul(b as u64)))
			.saturating_add(T::DbWeight::get().writes(5_u64))
			.saturating_add(T::DbWeight::get().writes((2_u64).saturating_mul(b as u64)))
	}
	// Storage: DdcPayouts AuthorisedCaller (r:1 w:0)
	// Storage: DdcPayouts ActiveBillingReports (r:1 w:1)
	// Storage: DdcClusters ClustersGovParams (r:1 w:0)
	// Storage: System Account (r:3 w:3)
	// Storage: DdcClusters Clusters (r:1 w:0)
	// Storage: Staking Validators (r:2 w:0)
	// Storage: Staking Bonded (r:1 w:0)
	// Storage: Staking Ledger (r:1 w:0)
	// Storage: Staking Nominators (r:1 w:0)
	fn end_charging_customers() -> Weight {
		Weight::from_ref_time(1_691_550_000_u64)
			.saturating_add(T::DbWeight::get().reads(12_u64))
			.saturating_add(T::DbWeight::get().writes(4_u64))
	}
	// Storage: DdcPayouts AuthorisedCaller (r:1 w:0)
	// Storage: DdcPayouts ActiveBillingReports (r:1 w:1)
	fn begin_rewarding_providers() -> Weight {
		Weight::from_ref_time(234_686_000_u64)
			.saturating_add(T::DbWeight::get().reads(2_u64))
			.saturating_add(T::DbWeight::get().writes(1_u64))
	}
	// Storage: DdcPayouts AuthorisedCaller (r:1 w:0)
	// Storage: DdcPayouts ActiveBillingReports (r:1 w:1)
	// Storage: System Account (r:2 w:2)
	/// The range of component `b` is `[1, 1000]`.
	fn send_rewarding_providers_batch(b: u32, ) -> Weight {
		Weight::from_ref_time(565_710_000_u64)
			// Standard Error: 854_032
			.saturating_add(Weight::from_ref_time(408_429_599_u64).saturating_mul(b as u64))
			.saturating_add(T::DbWeight::get().reads(4_u64))
			.saturating_add(T::DbWeight::get().reads((1_u64).saturating_mul(b as u64)))
			.saturating_add(T::DbWeight::get().writes(3_u64))
			.saturating_add(T::DbWeight::get().writes((1_u64).saturating_mul(b as u64)))
	}
	// Storage: DdcPayouts AuthorisedCaller (r:1 w:0)
	// Storage: DdcPayouts ActiveBillingReports (r:1 w:1)
	fn end_rewarding_providers() -> Weight {
		Weight::from_ref_time(274_535_000_u64)
			.saturating_add(T::DbWeight::get().reads(2_u64))
			.saturating_add(T::DbWeight::get().writes(1_u64))
	}
	// Storage: DdcPayouts AuthorisedCaller (r:1 w:0)
	// Storage: DdcPayouts ActiveBillingReports (r:1 w:1)
	fn end_billing_report() -> Weight {
		Weight::from_ref_time(232_626_000_u64)
			.saturating_add(T::DbWeight::get().reads(2_u64))
			.saturating_add(T::DbWeight::get().writes(1_u64))
	}
}

// For backwards compatibility and tests
impl WeightInfo for () {
	// Storage: DdcPayouts AuthorisedCaller (r:0 w:1)
	fn set_authorised_caller() -> Weight {
		Weight::from_ref_time(90_258_000_u64)
			.saturating_add(RocksDbWeight::get().writes(1_u64))
	}
	// Storage: DdcPayouts AuthorisedCaller (r:1 w:0)
	// Storage: DdcPayouts ActiveBillingReports (r:1 w:1)
	fn begin_billing_report() -> Weight {
		Weight::from_ref_time(214_646_000_u64)
			.saturating_add(RocksDbWeight::get().reads(2_u64))
			.saturating_add(RocksDbWeight::get().writes(1_u64))
	}
	// Storage: DdcPayouts AuthorisedCaller (r:1 w:0)
	// Storage: DdcPayouts ActiveBillingReports (r:1 w:1)
	fn begin_charging_customers() -> Weight {
		Weight::from_ref_time(228_676_000_u64)
			.saturating_add(RocksDbWeight::get().reads(2_u64))
			.saturating_add(RocksDbWeight::get().writes(1_u64))
	}
	// Storage: DdcPayouts AuthorisedCaller (r:1 w:0)
	// Storage: DdcPayouts ActiveBillingReports (r:1 w:1)
	// Storage: DdcClusters ClustersGovParams (r:1 w:0)
	// Storage: DdcCustomers Ledger (r:1 w:1)
	// Storage: System Account (r:2 w:2)
	// Storage: DdcPayouts DebtorCustomers (r:1 w:1)
	/// The range of component `b` is `[1, 1000]`.
	fn send_charging_customers_batch(b: u32, ) -> Weight {
		Weight::from_ref_time(891_324_000_u64)
			// Standard Error: 3_864_375
			.saturating_add(Weight::from_ref_time(558_679_506_u64).saturating_mul(b as u64))
			.saturating_add(RocksDbWeight::get().reads(7_u64))
			.saturating_add(RocksDbWeight::get().reads((2_u64).saturating_mul(b as u64)))
			.saturating_add(RocksDbWeight::get().writes(5_u64))
			.saturating_add(RocksDbWeight::get().writes((2_u64).saturating_mul(b as u64)))
	}
	// Storage: DdcPayouts AuthorisedCaller (r:1 w:0)
	// Storage: DdcPayouts ActiveBillingReports (r:1 w:1)
	// Storage: DdcClusters ClustersGovParams (r:1 w:0)
	// Storage: System Account (r:3 w:3)
	// Storage: DdcClusters Clusters (r:1 w:0)
	// Storage: Staking Validators (r:2 w:0)
	// Storage: Staking Bonded (r:1 w:0)
	// Storage: Staking Ledger (r:1 w:0)
	// Storage: Staking Nominators (r:1 w:0)
	fn end_charging_customers() -> Weight {
		Weight::from_ref_time(1_691_550_000_u64)
			.saturating_add(RocksDbWeight::get().reads(12_u64))
			.saturating_add(RocksDbWeight::get().writes(4_u64))
	}
	// Storage: DdcPayouts AuthorisedCaller (r:1 w:0)
	// Storage: DdcPayouts ActiveBillingReports (r:1 w:1)
	fn begin_rewarding_providers() -> Weight {
		Weight::from_ref_time(234_686_000_u64)
			.saturating_add(RocksDbWeight::get().reads(2_u64))
			.saturating_add(RocksDbWeight::get().writes(1_u64))
	}
	// Storage: DdcPayouts AuthorisedCaller (r:1 w:0)
	// Storage: DdcPayouts ActiveBillingReports (r:1 w:1)
	// Storage: System Account (r:2 w:2)
	/// The range of component `b` is `[1, 1000]`.
	fn send_rewarding_providers_batch(b: u32, ) -> Weight {
		Weight::from_ref_time(565_710_000_u64)
			// Standard Error: 854_032
			.saturating_add(Weight::from_ref_time(408_429_599_u64).saturating_mul(b as u64))
			.saturating_add(RocksDbWeight::get().reads(4_u64))
			.saturating_add(RocksDbWeight::get().reads((1_u64).saturating_mul(b as u64)))
			.saturating_add(RocksDbWeight::get().writes(3_u64))
			.saturating_add(RocksDbWeight::get().writes((1_u64).saturating_mul(b as u64)))
	}
	// Storage: DdcPayouts AuthorisedCaller (r:1 w:0)
	// Storage: DdcPayouts ActiveBillingReports (r:1 w:1)
	fn end_rewarding_providers() -> Weight {
		Weight::from_ref_time(274_535_000_u64)
			.saturating_add(RocksDbWeight::get().reads(2_u64))
			.saturating_add(RocksDbWeight::get().writes(1_u64))
	}
	// Storage: DdcPayouts AuthorisedCaller (r:1 w:0)
	// Storage: DdcPayouts ActiveBillingReports (r:1 w:1)
	fn end_billing_report() -> Weight {
		Weight::from_ref_time(232_626_000_u64)
			.saturating_add(RocksDbWeight::get().reads(2_u64))
			.saturating_add(RocksDbWeight::get().writes(1_u64))
	}
}