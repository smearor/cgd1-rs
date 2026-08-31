# Alarms

The CGD1 supports up to 16 independent alarm slots (indexed 0–15). Each alarm has a time, day-of-week repeat mask, and snooze setting.

## Alarm Structure

### AlarmEntry (5 bytes)

```
[Enabled] [HH] [MM] [Days] [Snooze]
```

| Field | Bytes | Description |
|---|---|---|
| Enabled | 1 | `0x01` = on, `0x00` = off |
| HH | 1 | Hour (0–23) |
| MM | 1 | Minute (0–59) |
| Days | 1 | Day bitmask (see below) |
| Snooze | 1 | `0x01` = on, `0x00` = off |

An empty/unused slot has all bytes set to `0xFF`: `FF FF FF FF FF`.

### DayMask Bitmask

| Bit | Value | Day |
|---|---|---|
| 0 | `0x01` | Monday |
| 1 | `0x02` | Tuesday |
| 2 | `0x04` | Wednesday |
| 3 | `0x08` | Thursday |
| 4 | `0x10` | Friday |
| 5 | `0x20` | Saturday |
| 6 | `0x40` | Sunday |
| — | `0x00` | Once (no repeat) |

Common patterns:

| Name | Value | Days |
|---|---|---|
| Every day | `0x7F` | Mon–Sun |
| Weekdays | `0x3E` | Mon–Fri |
| Weekends | `0x41` | Sat–Sun |

## Protocol

### Set Alarm

Send `07 05 [ID] [Enabled] [HH] [MM] [Days] [Snooze]` to Data Write.

ACK: `04 ff 05 00 00` (success)

### Delete Alarm

Overwrite the slot with `FF` values: `07 05 [ID] FF FF FF FF FF`

ACK: `04 ff 05 00 00` (success)

### Read Alarms

Send `01 06` to Data Write.

Response: The device sends 6 packets on Data Notify, each carrying 3 alarm entries:

```
11 06 [Base Index] [Entry 1 (5B)] [Entry 2 (5B)] [Entry 3 (5B)]
```

Each packet is 18 bytes. All 16 slots are returned (empty slots have `FF FF FF FF FF`).

## CLI Usage

### Read all alarms

```bash
cgd1 alarm-list AA:BB:CC:DD:EE:FF
```

### Set an alarm

```bash
cgd1 alarm-set AA:BB:CC:DD:EE:FF 3 07:30 --repeat 3e
```

| Argument | Description |
|---|---|
| `address` | Device MAC address |
| `slot` | Slot index 0–15 |
| `time` | Alarm time in HH:MM format |
| `--repeat` | Day mask as hex (default: `7f` = every day) |
| `--no-snooze` | Disable snooze for this alarm |

### Delete an alarm

```bash
cgd1 alarm-delete AA:BB:CC:DD:EE:FF 3
```

## Library API

```rust
use cgd1_rs::{AlarmSlotIndex, ClockTime, DayMask};

// Set an alarm
device.set_alarm(
    AlarmSlotIndex::new(3)?,
    ClockTime::new(7, 30)?,
    DayMask::WEEKDAYS,
    true,  // enabled
    true,  // snooze
).await?;

// Read all alarms
let slots = device.read_alarms().await?;
for slot in &slots {
    if slot.is_empty() {
        continue;
    }
    println!(
        "Slot {}: {:02}:{:02} repeat={:#04x} enabled={} snooze={}",
        slot.index(),
        slot.entry().hour(),
        slot.entry().minute(),
        slot.entry().day_mask(),
        slot.entry().enabled(),
        slot.entry().snooze(),
    );
}

// Delete an alarm
device.delete_alarm(AlarmSlotIndex::new(3)?).await?;
```

## DayMask Constants

The `DayMask` newtype provides common constants:

| Constant | Value | Description |
|---|---|---|
| `DayMask::ONCE` | `0x00` | No repeat (one-shot) |
| `DayMask::EVERY_DAY` | `0x7F` | Monday through Sunday |
| `DayMask::WEEKDAYS` | `0x3E` | Monday through Friday |
| `DayMask::WEEKENDS` | `0x41` | Saturday and Sunday |
