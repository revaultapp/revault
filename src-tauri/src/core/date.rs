/// Converts Unix epoch seconds to a civil date (year, month, day).
/// Based on Howard Hinnant's civil_from_days algorithm:
/// https://howardhinnant.github.io/date_algorithms.html
pub fn civil_from_secs(secs: u64) -> (i32, u8, u8) {
    let days = secs as i64 / 86400;
    let z = days + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = (z - era * 146097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y as i32, m as u8, d as u8)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn civil_from_epoch_start() {
        // Unix epoch: 1970-01-01 00:00:00 UTC
        let (y, m, d) = civil_from_secs(0);
        assert_eq!(y, 1970);
        assert_eq!(m, 1);
        assert_eq!(d, 1);
    }

    #[test]
    fn civil_from_secs_leap_year() {
        // 2024 is a leap year - February 29 exists
        // 2024-02-29 00:00:00 UTC = 1709164800
        let (y, m, d) = civil_from_secs(1709164800);
        assert_eq!(y, 2024);
        assert_eq!(m, 2);
        assert_eq!(d, 29);
    }

    #[test]
    fn civil_from_secs_non_leap_year() {
        // 2023-03-01 00:00:00 UTC = 1677628800
        let (y, m, d) = civil_from_secs(1677628800);
        assert_eq!(y, 2023);
        assert_eq!(m, 3);
        assert_eq!(d, 1);
    }
}
