use serde::Deserialize;
use serde::Serialize;

/// Frequency of time-based visual blink feedback.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum TimeBasedBlink {
    /// No time-based blink.
    #[default]
    Off,
    /// Blink once per hour.
    Hourly,
    /// Blink every fifteen minutes.
    EveryFifteenMinutes,
    /// Blink every five minutes.
    EveryFiveMinutes,
    /// Blink every minute.
    EveryMinute,
}

impl TimeBasedBlink {
    /// All variants in display order.
    pub const ALL: [TimeBasedBlink; 5] = [
        TimeBasedBlink::Off,
        TimeBasedBlink::Hourly,
        TimeBasedBlink::EveryFifteenMinutes,
        TimeBasedBlink::EveryFiveMinutes,
        TimeBasedBlink::EveryMinute,
    ];

    /// Short label suitable for slider tick marks.
    pub const fn label(self) -> &'static str {
        match self {
            TimeBasedBlink::Off => "Off",
            TimeBasedBlink::Hourly => "1h",
            TimeBasedBlink::EveryFifteenMinutes => "15m",
            TimeBasedBlink::EveryFiveMinutes => "5m",
            TimeBasedBlink::EveryMinute => "1m",
        }
    }

    /// Numeric index for slider positioning (0-based).
    pub const fn index(self) -> usize {
        match self {
            TimeBasedBlink::Off => 0,
            TimeBasedBlink::Hourly => 1,
            TimeBasedBlink::EveryFifteenMinutes => 2,
            TimeBasedBlink::EveryFiveMinutes => 3,
            TimeBasedBlink::EveryMinute => 4,
        }
    }

    /// Convert a numeric index back to a variant.
    pub fn from_index(index: usize) -> Option<Self> {
        Self::ALL.get(index).copied()
    }

    /// Check whether a blink should trigger at the given minute-of-hour.
    ///
    /// Returns `true` if the current minute aligns with the configured frequency.
    pub fn should_blink(self, minute: u32) -> bool {
        match self {
            TimeBasedBlink::Off => false,
            TimeBasedBlink::Hourly => minute == 0,
            TimeBasedBlink::EveryFifteenMinutes => minute % 15 == 0,
            TimeBasedBlink::EveryFiveMinutes => minute % 5 == 0,
            TimeBasedBlink::EveryMinute => true,
        }
    }
}
