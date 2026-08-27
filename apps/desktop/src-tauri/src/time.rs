use std::time::{SystemTime, UNIX_EPOCH};

pub(crate) fn current_rfc3339_timestamp() -> Option<String> {
    let elapsed = SystemTime::now().duration_since(UNIX_EPOCH).ok()?;
    unix_timestamp_to_rfc3339(elapsed.as_secs(), elapsed.subsec_millis())
}

fn unix_timestamp_to_rfc3339(seconds: u64, milliseconds: u32) -> Option<String> {
    let days = i64::try_from(seconds / 86_400).ok()?;
    let seconds_of_day = seconds % 86_400;
    let z = days.checked_add(719_468)?;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let day_of_era = z - era * 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let mut year = year_of_era + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_prime = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_prime + 2) / 5 + 1;
    let month = month_prime + if month_prime < 10 { 3 } else { -9 };
    year += i64::from(month <= 2);
    if !(0..=9_999).contains(&year) || milliseconds > 999 {
        return None;
    }
    let hour = seconds_of_day / 3_600;
    let minute = (seconds_of_day % 3_600) / 60;
    let second = seconds_of_day % 60;
    Some(format!(
        "{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}.{milliseconds:03}Z"
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_utc_rfc3339() {
        assert_eq!(
            unix_timestamp_to_rfc3339(0, 0).as_deref(),
            Some("1970-01-01T00:00:00.000Z")
        );
        assert_eq!(
            unix_timestamp_to_rfc3339(86_400, 7).as_deref(),
            Some("1970-01-02T00:00:00.007Z")
        );
        assert_eq!(unix_timestamp_to_rfc3339(0, 1_000), None);
    }
}
