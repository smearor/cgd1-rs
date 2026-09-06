# App
app-title = Wecker CGD1

# Menu
menu-reset-token = Token zurücksetzen
menu-info = Info

# Dropdown
dropdown-no-devices = Keine Geräte

# Toggle buttons
toggle-alarms = Wecker
toggle-display = Anzeige
toggle-region = Region
toggle-sensors = Sensoren
toggle-audio = Audio

# Tooltips
tooltip-scan = Nach Geräten suchen
tooltip-connect = Verbinden / Trennen
tooltip-toggle-alarms = Weck-Editor ein-/ausblenden
tooltip-toggle-display = Anzeige-Editor ein-/ausblenden
tooltip-toggle-region = Regions-Editor ein-/ausblenden
tooltip-toggle-sensors = Sensor-Übersicht ein-/ausblenden
tooltip-toggle-audio = Audio-Editor ein-/ausblenden

# Battery
battery-tooltip-unknown = -- %
battery-tooltip = { $level } %

# Status messages
status-disconnected = Getrennt
status-scanning = Suche...
status-no-devices-found = Keine Geräte gefunden
status-devices-found = { $count } Gerät(e) gefunden
status-no-device-selected = Kein Gerät ausgewählt
status-connecting = Verbinde...
status-connected = Verbunden mit { $addr }
status-connect-failed = Verbindung fehlgeschlagen: { $error }
status-connect-failed-new-token = { $error }. Gerät muss auf Werkseinstellungen zurückgesetzt werden, um einen neuen Token zu akzeptieren.
status-connect-task-failed = Verbindungsaufgabe fehlgeschlagen
status-reconnecting = Erneut verbinden...
status-device-reconnected = Gerät erneut verbunden
status-device-unresponsive = Gerät reagiert nicht, trenne...
status-selected = Ausgewählt: { $addr }
status-available = Verfügbar: { $addr }
status-alarm-triggered = Wecker { $slot } ausgelöst
status-scan-failed = Suche fehlgeschlagen: { $error }
status-scan-task-failed = Suchaufgabe fehlgeschlagen

# Dialog: no device
dialog-no-device-title = Kein Gerät ausgewählt
dialog-no-device-body = Wählen Sie zuerst ein Gerät aus der Dropdown-Liste.

# Dialog: reset token
dialog-reset-token-title = Auth-Token zurücksetzen?
dialog-reset-token-body =
    Dies löscht den gespeicherten Token für { $addr }.
    Das Gerät muss auf Werkseinstellungen zurückgesetzt werden, um einen neuen Token zu akzeptieren.

    Trennen Sie zuerst, falls aktuell verbunden.

# Info dialog
info-dialog-title = Info - Wecker CGD1
info-app-name = Wecker CGD1
info-github-link = github.com/smearor/cgd1-rs
info-license-text =
    Lizenziert unter der MIT-Lizenz

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
info-close = Schließen

# Alarm editor
alarm-read-from-device = Vom Gerät lesen
tooltip-enable-alarm = Wecker aktivieren
tooltip-snooze = Schlummern
tooltip-once = Einmal
tooltip-set = Setzen
tooltip-delete = Löschen

# Day labels
day-mo = Mo
day-tu = Di
day-we = Mi
day-th = Do
day-fr = Fr
day-sa = Sa
day-su = So

# Weekday short names
weekday-mon = Mo
weekday-tue = Di
weekday-wed = Mi
weekday-thu = Do
weekday-fri = Fr
weekday-sat = Sa
weekday-sun = So

# Common status messages
status-no-device-connected = Kein Gerät verbunden
status-reading-settings = Lese Einstellungen...
status-writing-settings = Schreibe Einstellungen...
status-read-failed = Lesen fehlgeschlagen: { $error }
status-read-task-failed = Leseaufgabe fehlgeschlagen
status-write-failed = Schreiben fehlgeschlagen: { $error }
status-write-task-failed = Schreibaufgabe fehlgeschlagen

# Alarm editor status
status-invalid-time = Ungültige Zeit
status-invalid-time-error = Ungültige Zeit: { $error }
status-setting-alarm = Setze Wecker #{ $slot }...
status-alarm-set = Wecker #{ $slot } gesetzt
status-set-failed = Setzen fehlgeschlagen: { $error }
status-set-task-failed = Setzaufgabe fehlgeschlagen
status-deleting-alarm = Lösche Wecker #{ $slot }...
status-alarm-deleted = Wecker #{ $slot } gelöscht
status-delete-failed = Löschen fehlgeschlagen: { $error }
status-delete-task-failed = Löschaufgabe fehlgeschlagen
status-reading-alarms = Lese Wecker...
status-loaded-alarms = { $count } Wecker geladen

# Display editor
frame-display = Anzeige
frame-night-mode = Nachtmodus
label-brightness = Helligkeit
label-screen-timeout = Bildschirm-Timeout
label-blink-on-connect = Blinken beim Verbinden
label-time-based-blink = Zeitbasiertes Blinken
label-night-mode = Nachtmodus
label-night-brightness = Nacht-Helligkeit
label-night-start = Nacht-Beginn
label-night-end = Nacht-Ende
tooltip-blink-on-connect = Dreifaches Blinken als visuelles Feedback beim Verbinden
tooltip-read-display = Anzeige-Einstellungen vom Gerät lesen
tooltip-write-display = Anzeige-Einstellungen auf Gerät schreiben
button-read = Lesen
button-write = Schreiben
status-display-settings-loaded = Anzeige-Einstellungen geladen
status-display-settings-written = Anzeige-Einstellungen geschrieben
status-invalid-brightness = Ungültige Helligkeit: { $error }
status-invalid-night-brightness = Ungültige Nacht-Helligkeit: { $error }
status-invalid-night-start = Ungültiger Nacht-Beginn: { $error }
status-invalid-night-start-time = Ungültige Nacht-Beginn-Zeit
status-invalid-night-end = Ungültiges Nacht-Ende: { $error }
status-invalid-night-end-time = Ungültige Nacht-Ende-Zeit
status-invalid-screen-duration = Ungültige Bildschirm-Dauer: { $error }

# Region editor
frame-regional = Regional
label-time-format = Zeitformat
label-temperature = Temperatur
label-language = Sprache
label-timezone = Zeitzone
toggle-24h = 24h
toggle-12h = 12h
toggle-english = English
toggle-chinese = 中文
button-sync-from-system = Vom System synchronisieren
tooltip-sync-from-system = Zeitzone aus lokaler Systemuhr setzen
tooltip-read-region = Regions-Einstellungen vom Gerät lesen
tooltip-write-region = Regions-Einstellungen auf Gerät schreiben
info-tz-dst = Das Gerät hat keine Sommerzeit-Logik. Die Zeitzone wird beim Verbinden automatisch synchronisiert. Nach einer Sommerzeit-Umstellung neu verbinden.
status-region-settings-loaded = Regions-Einstellungen geladen
status-region-settings-written = Regions-Einstellungen geschrieben
status-invalid-timezone = Ungültige Zeitzone: { $error }
status-tz-synced = Zeitzone an System angepasst (UTC{ $offset })
status-tz-not-in-list = System-Zeitzone UTC{ $offset } nicht in Liste

# Sensor overview
button-refresh = Aktualisieren
column-mac-address = MAC-Adresse
column-temp = Temp
column-humidity = Luftfeucht
column-battery = Akku

# Audio editor
frame-active-ringtone = Aktiver Klingelton
frame-custom-upload = Eigener Upload
label-ringtone = Klingelton
label-volume = Lautstärke
label-target-slot = Ziel-Slot
toggle-slot-a = Slot A
toggle-slot-b = Slot B
button-preview = Vorschau (Beep)
button-apply = Anwenden
button-upload = Hochladen
button-select-audio-file = Audiodatei wählen…
tooltip-read-ringtone = Aktuellen Klingelton vom Gerät lesen
tooltip-preview = Test-Beep mit aktueller Lautstärke abspielen
info-ringtone = Wählt Klingelton und Lautstärke. Eingebaute Klingeltöne werden auf das Gerät hochgeladen (die Firmware speichert die Klingelton-Auswahl nicht über die Einstellungen).
info-upload-format = 8-Bit PCM, 8 kHz, mono, max. ~12 Sekunden. Immer zwischen Slots abwechseln.
label-no-file-selected = Keine Datei ausgewählt
filter-audio-files = Audiodateien
file-chooser-title = Audiodatei wählen
button-cancel = Abbrechen
button-open = Öffnen
status-settings-loaded = Einstellungen geladen
status-playing-preview = Spiele Vorschau-Beep...
status-preview-played = Vorschau abgespielt
status-preview-failed = Vorschau fehlgeschlagen: { $error }
status-preview-task-failed = Vorschau-Aufgabe fehlgeschlagen
status-uploading-ringtone = Lade Klingelton-Audio auf Gerät...
status-writing-ringtone = Schreibe Klingelton in Einstellungen...
status-ringtone-applied = Klingelton angewendet
status-apply-failed = Anwenden fehlgeschlagen: { $error }
status-apply-task-failed = Anwenden-Aufgabe fehlgeschlagen
status-uploading-to = Lade hoch zu { $name }...
status-upload-complete = Upload abgeschlossen
status-upload-failed = Upload fehlgeschlagen: { $error }
status-upload-task-failed = Upload-Aufgabe fehlgeschlagen
status-invalid-volume = Ungültige Lautstärke: { $error }
status-read-custom-ringtone-failed = Eigener Klingelton konnte nicht gelesen werden: { $error }

# Time-based blink labels
label-off = Aus
label-hourly = 1h
label-15m = 15m
label-5m = 5m
label-1m = 1m
