use super::*;
use crate::mock::*;
use frame_support::assert_ok;
use sp_runtime::traits::Zero;

#[test]
fn distribute_profit_should_work() {
	ExtBuilder::default()
		.existential_deposit(100)
		.system_ratio(Perbill::from_percent(20))
		.treasury_ratio(Perbill::from_percent(33))
		.operation_ratio(Perbill::from_percent(25))
		.collator_ratio(Perbill::from_percent(22))
		.min_liquidation_threshold(20_000_000_000)
		.profit_distribution_cycle(10)
		.build()
		.execute_with(|| {
			for n in 1..=ProfitDistributionCycle::<Test>::get() {
				frame_system::Pallet::<Test>::set_block_number(n.into());
			}

			assert_ok!(Liquidation::test_distribute_profit());
			let events = frame_system::Pallet::<Test>::events()
				.into_iter()
				.map(|record| record.event)
				.filter_map(|event| {
					if let RuntimeEvent::Liquidation(inner_event) = event {
						Some(inner_event)
					} else {
						None
					}
				})
				.collect::<Vec<_>>();

			assert!(events.iter().any(expected_event));
			assert_eq!(<TotalIncome<Test>>::get(), Zero::zero());
			assert_eq!(<TotalCost<Test>>::get(), Zero::zero());
			assert!(<CollatorRealGasCosts<Test>>::iter().next().is_none());
		});
}
