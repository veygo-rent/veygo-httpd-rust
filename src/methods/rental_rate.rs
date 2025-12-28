use chrono::{DateTime, Duration, TimeDelta, Utc};

pub fn calculate_billable_duration_hours ( raw_duration: TimeDelta ) -> f64 {
    // Tiered billing:
    // - First 8 hours are billed 1:1
    // - Hours after 8 up to the end of the first week (168 hours total) are billed at 0.25 per hour
    // - Hours after 168 are billed at 0.15 per hour
    let calculated_duration_hours: f64 = {
        let billable_duration_hours: f64 = raw_duration.num_minutes() as f64 / 60.0;
        
        let h = billable_duration_hours.max(0.0);

        // Tier 1: first 8 hours at 1x
        let tier1_hours = h.min(8.0);

        // Tier 2: from hour 9 up to hour 168 (i.e., next 160 hours) at 0.25x
        let tier2_hours = (h - 8.0).clamp(0.0, 160.0);

        // Tier 3: beyond 168 hours at 0.15x
        let tier3_hours = (h - 168.0).max(0.0);

        tier1_hours + (tier2_hours * 0.25) + (tier3_hours * 0.15)
    };
    
    calculated_duration_hours
}

pub fn calculate_duration_after_reward ( raw_duration: TimeDelta, reward_hours: f64 ) -> TimeDelta {
    if reward_hours <= 0.0 {
        return raw_duration;
    }
    // Subtract reward time safely (prevent negative billable duration due to rounding)
    let total_minutes: i64 = raw_duration.num_minutes().max(0);
    let mut reward_minutes: i64 = (reward_hours.max(0.0) * 60.0).round() as i64;
    if reward_minutes > total_minutes {
        reward_minutes = total_minutes;
    }
    
    Duration::minutes(total_minutes - reward_minutes)
}

pub fn billable_days_count ( raw_duration: TimeDelta ) -> i32 {
    let bill_hours = calculate_billable_duration_hours(raw_duration);
    (bill_hours / 24.0).ceil() as i32
}

pub fn calculate_late_hours (supposed: DateTime<Utc>, actual: DateTime<Utc> ) -> f64 {
    if supposed >= actual {
        0.0
    } else {
        calculate_billable_duration_hours(actual - supposed)
    }
}
