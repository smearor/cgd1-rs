# App
app-title = Alarm Clock CGD1

# Menu
menu-reset-token = Reset Token
menu-info = Info

# Dropdown
dropdown-no-devices = No devices

# Toggle buttons
toggle-alarms = Alarms
toggle-display = Display
toggle-region = Region
toggle-sensors = Sensors
toggle-audio = Audio

# Tooltips
tooltip-scan = Scan for devices
tooltip-connect = Connect / Disconnect
tooltip-toggle-alarms = Show / hide alarm editor
tooltip-toggle-display = Show / hide display editor
tooltip-toggle-region = Show / hide region editor
tooltip-toggle-sensors = Show / hide sensor overview
tooltip-toggle-audio = Show / hide audio editor

# Battery
battery-tooltip-unknown = -- %
battery-tooltip = { $level } %

# Status messages
status-disconnected = Disconnected
status-scanning = Scanning...
status-no-devices-found = No devices found
status-devices-found = { $count } device(s) found
status-no-device-selected = No device selected
status-connecting = Connecting...
status-connected = Connected to { $addr }
status-connect-failed = Connect failed: { $error }
status-connect-failed-new-token = { $error }. Device may need factory reset to accept a new token.
status-connect-task-failed = Connect task failed
status-reconnecting = Reconnecting...
status-device-reconnected = Device reconnected
status-device-unresponsive = Device unresponsive, disconnecting...
status-selected = Selected: { $addr }
status-available = Available: { $addr }
status-alarm-triggered = Alarm { $slot } triggered
status-scan-failed = Scan failed: { $error }
status-scan-task-failed = Scan task failed

# Dialog: no device
dialog-no-device-title = No device selected
dialog-no-device-body = Select a device in the dropdown first.

# Dialog: reset token
dialog-reset-token-title = Reset auth token?
dialog-reset-token-body =
    This deletes the stored token for { $addr }.
    The device must be factory reset to accept a new token.

    Disconnect first if currently connected.

# Info dialog
info-dialog-title = Info - Alarm Clock CGD1
info-app-name = Alarm Clock CGD1
info-github-link = github.com/smearor/cgd1-rs
info-license-text =
    Licensed under the MIT License

    Copyright (c) 2024 smearor

    Permission is hereby granted, free of charge, to any person obtaining a copy
    of this software and associated documentation files (the "Software"), to deal
    in the Software without restriction, including without limitation the rights
    to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
    copies of the Software, and to permit persons to whom the Software is
    furnished to do so, subject to the following conditions:

    The above copyright notice and this permission notice shall be included in
    all copies or substantial portions of the Software.

    THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND.
info-close = Close

# Alarm editor
alarm-read-from-device = Read from Device
tooltip-enable-alarm = Enable alarm
tooltip-snooze = Snooze
tooltip-once = Once
tooltip-set = Set
tooltip-delete = Del

# Day labels
day-mo = Mo
day-tu = Tu
day-we = We
day-th = Th
day-fr = Fr
day-sa = Sa
day-su = Su

# Weekday short names
weekday-mon = Mon
weekday-tue = Tue
weekday-wed = Wed
weekday-thu = Thu
weekday-fri = Fri
weekday-sat = Sat
weekday-sun = Sun

# Common status messages
status-no-device-connected = No device connected
status-reading-settings = Reading settings...
status-writing-settings = Writing settings...
status-read-failed = Read failed: { $error }
status-read-task-failed = Read task failed
status-write-failed = Write failed: { $error }
status-write-task-failed = Write task failed

# Alarm editor status
status-invalid-time = Invalid time
status-invalid-time-error = Invalid time: { $error }
status-setting-alarm = Setting alarm #{ $slot }...
status-alarm-set = Alarm #{ $slot } set
status-set-failed = Set failed: { $error }
status-set-task-failed = Set task failed
status-deleting-alarm = Deleting alarm #{ $slot }...
status-alarm-deleted = Alarm #{ $slot } deleted
status-delete-failed = Delete failed: { $error }
status-delete-task-failed = Delete task failed
status-reading-alarms = Reading alarms...
status-loaded-alarms = Loaded { $count } alarm(s)

# Display editor
frame-display = Display
frame-night-mode = Night Mode
label-brightness = Brightness
label-screen-timeout = Screen Timeout
label-blink-on-connect = Blink on Connect
label-time-based-blink = Time-Based Blink
label-night-mode = Night Mode
label-night-brightness = Night Brightness
label-night-start = Night Start
label-night-end = Night End
tooltip-blink-on-connect = Triple-blink visual feedback when connecting
tooltip-read-display = Read display settings from device
tooltip-write-display = Write display settings to device
button-read = Read
button-write = Write
status-display-settings-loaded = Display settings loaded
status-display-settings-written = Display settings written
status-invalid-brightness = Invalid brightness: { $error }
status-invalid-night-brightness = Invalid night brightness: { $error }
status-invalid-night-start = Invalid night start: { $error }
status-invalid-night-start-time = Invalid night start time
status-invalid-night-end = Invalid night end: { $error }
status-invalid-night-end-time = Invalid night end time
status-invalid-screen-duration = Invalid screen duration: { $error }

# Region editor
frame-regional = Regional
label-time-format = Time Format
label-temperature = Temperature
label-language = Language
label-timezone = Timezone
toggle-24h = 24h
toggle-12h = 12h
toggle-english = English
toggle-chinese = 中文
button-sync-from-system = Sync from System
tooltip-sync-from-system = Set timezone from the computer's local clock
tooltip-read-region = Read region settings from device
tooltip-write-region = Write region settings to device
info-tz-dst = The device has no DST logic. Timezone is auto-synced on connect. Reconnect after a DST change to update.
status-region-settings-loaded = Region settings loaded
status-region-settings-written = Region settings written
status-invalid-timezone = Invalid timezone: { $error }
status-tz-synced = Timezone set to match system (UTC{ $offset })
status-tz-not-in-list = System timezone UTC{ $offset } not in list

# Sensor overview
button-refresh = Refresh
column-mac-address = MAC Address
column-temp = Temp
column-humidity = Humidity
column-battery = Battery

# Audio editor
frame-active-ringtone = Active Ringtone
frame-custom-upload = Custom Upload
label-ringtone = Ringtone
label-volume = Volume
label-target-slot = Target Slot
toggle-slot-a = Slot A
toggle-slot-b = Slot B
button-preview = Preview (Beep)
button-apply = Apply
button-upload = Upload
button-select-audio-file = Select Audio File…
tooltip-read-ringtone = Read current ringtone from device
tooltip-preview = Play a test beep at current volume
info-ringtone = Selects the ringtone and volume. Built-in ringtones are uploaded to the device (the firmware does not persist ringtone selection via settings).
info-upload-format = 8-bit PCM, 8 kHz, mono, max ~12 seconds. Always alternate slots between uploads.
label-no-file-selected = No file selected
filter-audio-files = Audio files
file-chooser-title = Select Audio File
button-cancel = Cancel
button-open = Open
status-settings-loaded = Settings loaded
status-playing-preview = Playing preview beep...
status-preview-played = Preview played
status-preview-failed = Preview failed: { $error }
status-preview-task-failed = Preview task failed
status-uploading-ringtone = Uploading ringtone audio to device...
status-writing-ringtone = Writing ringtone to settings...
status-ringtone-applied = Ringtone applied
status-apply-failed = Apply failed: { $error }
status-apply-task-failed = Apply task failed
status-uploading-to = Uploading to { $name }...
status-upload-complete = Upload complete
status-upload-failed = Upload failed: { $error }
status-upload-task-failed = Upload task failed
status-invalid-volume = Invalid volume: { $error }
status-read-custom-ringtone-failed = Failed to read custom ringtone: { $error }

# Time-based blink labels
label-off = Off
label-hourly = 1h
label-15m = 15m
label-5m = 5m
label-1m = 1m
