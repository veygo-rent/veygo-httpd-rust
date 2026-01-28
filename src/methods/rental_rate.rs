use chrono::{DateTime, Duration, TimeDelta, Utc};
use rust_decimal::prelude::*;

pub fn calculate_billable_duration_hours ( raw_duration: TimeDelta ) -> Decimal {
    // Tiered billing:
    // - First 8 hours are billed 1:1
    // - Hours after 8 up to the end of the first week (168 hours total) are billed at 0.25 per hour
    // - Hours after 168 are billed at 0.15 per hour
    let calculated_duration_hours: Decimal = {
        let billable_duration_hours: Decimal = Decimal::new(raw_duration.num_minutes(), 0) / Decimal::new(60, 0);

        if billable_duration_hours <= Decimal::zero() {
            return Decimal::zero()
        }

        // Tier 1: first 8 hours at 1x
        let tier1_hours = billable_duration_hours.min(Decimal::new(8, 0));

        // Tier 2: from hour 9 up to hour 168 (i.e., next 160 hours) at 0.25x
        let tier2_hours = (billable_duration_hours - Decimal::new(8, 0)).clamp(Decimal::zero(), Decimal::new(160, 0));

        // Tier 3: beyond 168 hours at 0.15x
        let tier3_hours = (billable_duration_hours - Decimal::new(168, 0)).max(Decimal::zero());

        tier1_hours + (tier2_hours * Decimal::new(25, 2)) + (tier3_hours * Decimal::new(15, 2))
    };
    
    calculated_duration_hours
}

pub fn calculate_duration_after_reward ( raw_duration: TimeDelta, reward_hours: Decimal ) -> TimeDelta {
    if reward_hours <= Decimal::new(0, 0) {
        return raw_duration;
    }
    // Subtract reward time safely (prevent negative billable duration due to rounding)
    let total_minutes: i64 = raw_duration.num_minutes().max(0);
    let mut reward_minutes: i64 = (reward_hours * Decimal::new(60, 0)).round().to_i64().unwrap();
    if reward_minutes > total_minutes {
        reward_minutes = total_minutes;
    }
    
    Duration::minutes(total_minutes - reward_minutes)
}

pub fn billable_days_count ( raw_duration: TimeDelta ) -> i32 {
    let bill_hours = calculate_billable_duration_hours(raw_duration);
    (bill_hours / Decimal::new(24, 0)).ceil().to_i32().unwrap()
}

pub fn calculate_late_hours (supposed: DateTime<Utc>, actual: DateTime<Utc> ) -> Decimal {
    if supposed >= actual {
        Decimal::new(0, 0)
    } else {
        // Calculate the difference in hours
        let diff = supposed - actual;
        let late_hours = Decimal::new(diff.num_minutes(), 0) / Decimal::new(60, 0);
        late_hours
    }
}
