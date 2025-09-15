///! Best-effort DateTime handling generalization for parsing feeds and handling
///! conversions between std, chrono, and tokio.
use super::*;

/// Datetime generalization for conversion between libraries.
/// This attempts to support millisecond resolution.
#[derive(
    Clone, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord,
)]
pub struct DateTime(chrono::DateTime<chrono::Utc>);

impl DateTime {
    /// Get the current DateTime.
    pub fn now() -> Self {
        DateTime(chrono::Utc::now())
    }

    /// Generate a DateTime at the unix epoch.
    pub fn epoch() -> Self {
        DateTime(chrono::DateTime::UNIX_EPOCH)
    }

    /// Check whether or not this timestamp has passed.
    pub fn has_passed(&self, duration: &Duration) -> bool {
        self.0 + duration.0 < DateTime::now().0
    }

    // pub fn to_std(&self) -> std::time::Instant {
    //     self.0.into()
    // }

    /// Convert to chrono::DateTime.
    pub fn to_chrono(&self) -> chrono::DateTime<chrono::Utc> {
        self.0.clone()
    }

    /// Convert to tokio::Instant.
    pub fn to_tokio(&self) -> tokio::time::Instant {
        let dur: chrono::Duration = self.0 - chrono::Utc::now();
        let millis: i64 = dur.num_milliseconds();
        if millis >= 0 {
            let dur = std::time::Duration::from_millis(millis as u64);
            tokio::time::Instant::now() + dur
        } else {
            let dur = std::time::Duration::from_millis(millis.abs() as u64);
            tokio::time::Instant::now() - dur
        }
    }

    /// Convert to ISO-8601 string.
    pub fn to_iso8601(&self) -> String {
        return self.0.format("%+").to_string();
    }

    // pub fn from_std(&self, dt: std::time::Instant) -> Self {
    //     Self(match chrono::DateTime::<chrono::Utc>::from_timestamp_millis(dt.duration_since(std::time::Instant)))
    // }

    /// Convert from chrono::DateTime.
    pub fn from_chrono(dt: chrono::DateTime<chrono::Utc>) -> Self {
        Self(dt)
    }
}

impl std::fmt::Display for DateTime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0.format("%Y-%m-%d %H:%M UTC").to_string())
    }
}

impl TryFrom<&str> for DateTime {
    type Error = ();
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let date = value;

        // rfc3339:
        if let Ok(parsed) =
            chrono::DateTime::<chrono::FixedOffset>::parse_from_rfc3339(
                date.as_ref(),
            )
        {
            return Ok(DateTime(parsed.to_utc()));
        }

        // rfc2822:
        if let Ok(parsed) =
            chrono::DateTime::<chrono::FixedOffset>::parse_from_rfc2822(
                date.as_ref(),
            )
        {
            return Ok(DateTime(parsed.to_utc()));
        }

        // iso8601 and variants:

        if let Ok(parsed) = chrono::NaiveDateTime::parse_from_str(
            date.as_ref(),
            "%Y-%m-%dT%H:%M:%SZ",
        ) {
            return Ok(DateTime(chrono::DateTime::from_naive_utc_and_offset(
                parsed,
                chrono::Utc,
            )));
        }

        if let Ok(parsed) = chrono::NaiveDateTime::parse_from_str(
            date.as_ref(),
            "%Y-%m-%dT%H:%MZ",
        ) {
            return Ok(DateTime(chrono::DateTime::from_naive_utc_and_offset(
                parsed,
                chrono::Utc,
            )));
        }

        if let Ok(parsed) =
            chrono::NaiveDate::parse_from_str(date.as_ref(), "%Y-%m-%d")
        {
            if let Some(parsed) = parsed.and_hms_opt(0, 0, 0) {
                return Ok(DateTime(
                    chrono::DateTime::from_naive_utc_and_offset(
                        parsed,
                        chrono::Utc,
                    ),
                ));
            }
        }

        Err(())
    }
}

impl TryFrom<&String> for DateTime {
    type Error = ();

    fn try_from(value: &String) -> Result<Self, Self::Error> {
        DateTime::try_from(value.as_str())
    }
}

impl std::ops::Sub<Duration> for DateTime {
    type Output = DateTime;
    fn sub(self, rhs: Duration) -> Self::Output {
        DateTime(self.0 - rhs.0)
    }
}

impl std::ops::Sub<DateTime> for DateTime {
    type Output = Duration;
    fn sub(self, rhs: DateTime) -> Self::Output {
        Duration(self.0 - rhs.0)
    }
}

impl std::ops::Add<Duration> for DateTime {
    type Output = DateTime;
    fn add(self, rhs: Duration) -> Self::Output {
        DateTime(self.0 + rhs.0)
    }
}

/// Duration generalization for conversion between libraries.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Duration(chrono::Duration);

impl Duration {
    /// Create a duration from seconds.
    pub fn from_seconds(sec: u64) -> Self {
        Self(chrono::Duration::seconds(sec as i64))
    }

    /// Convert duration to std.
    pub fn to_std(&self) -> std::time::Duration {
        match self.0.to_std() {
            Ok(dur) => dur,
            Err(_) => std::time::Duration::ZERO,
        }
    }

    /// Convert duration to chrono.
    pub fn to_chrono(&self) -> chrono::Duration {
        self.0.clone()
    }

    /// Convert duration to tokio.
    pub fn to_tokio(&self) -> tokio::time::Duration {
        tokio::time::Duration::from_secs(self.0.num_seconds() as u64)
    }

    /// Create duration from std.
    pub fn from_std(dur: std::time::Duration) -> Self {
        Self(match chrono::Duration::from_std(dur) {
            Ok(dur) => dur,
            Err(_) => chrono::Duration::zero(),
        })
    }

    /// Create duration from chrono.
    pub fn from_chrono(dur: chrono::Duration) -> Self {
        Self(dur)
    }

    /// Create duration from tokio.
    pub fn from_tokio(dur: tokio::time::Duration) -> Self {
        Self(chrono::Duration::seconds(dur.as_secs() as i64))
    }
}

/// Trait for formatting time as expected in the If-Modified-Since standard header.
pub trait IfModifiedSinceHeader {
    /// Create a timestamp following the If-Modified-Since header format.
    fn if_modified_since_time(&self) -> String;
}

impl IfModifiedSinceHeader for DateTime {
    fn if_modified_since_time(&self) -> String {
        use chrono::Datelike;
        use chrono::Timelike;

        let weekday = self.0.weekday().to_string();
        let day = format!("{:0>2}", self.0.day());
        let month = match self.0.month() {
            1 => "Jan",
            2 => "Feb",
            3 => "Mar",
            4 => "Apr",
            5 => "May",
            6 => "Jun",
            7 => "Jul",
            8 => "Aug",
            9 => "Sep",
            10 => "Oct",
            11 => "Nov",
            _ => "Dec",
        };
        let year = self.0.year();
        let hour = format!("{:0>2}", self.0.hour());
        let minute = format!("{:0>2}", self.0.minute());
        let second = format!("{:0>2}", self.0.second());
        let since = format!(
            "{}, {} {} {} {}:{}:{} GMT",
            weekday, day, month, year, hour, minute, second
        );
        since
    }
}
