# CouchDeck – Native Ryujinx Integration

Repository: https://github.com/saidsco/couchdeck-launcher

## Ziel

Implementiere eine native Integration des originalen Ryujinx-Emulators unter Linux.

Ausgangsbasis ist die bestehende CouchDeck-Architektur auf `main`. Kein Rewrite.

Der Benutzer hat Ryujinx bereits vollständig installiert und konfiguriert (liegt momentan auf dem Entwicklungsrechner lokal in ~/dat2/publish).

Folgende drei Anforderungen müssen erfüllt werden:

1. Ein emuliertes Spiel erscheint als eigene Kachel in CouchDeck.
2. Das Spiel startet möglichst nahtlos direkt im Vollbild, ohne zusätzliche Navigation durch Ryujinx.
3. Die aktive Emulatorsitzung kann mit einer konfigurierbaren Controller-Kombination gezielt und möglichst schonend beendet werden. Anschließend kehrt der Benutzer zu CouchDeck zurück.

## Phase 1 – Ryujinx erkennen und Direktstart

Ergänze eine Emulator-Konfiguration im Admin-Bereich.

- Pfad zur nativen Ryujinx-Installation auswählen.
- Standardmäßig das normale Ryujinx-Frontend verwenden.
- Unterstützte Startparameter vorab prüfen und dokumentieren.
- Optional `Ryujinx.Headless.SDL2` anbieten, wenn tatsächlich verfügbar und erfolgreich getestet.
- Spielpfad über den vorhandenen Controller-Dateiauswahldialog auswählen.
- Cover, Hintergrund und angezeigten Titel über die bestehende Artwork-Verwaltung bearbeiten.
- Für jedes importierte Spiel einen eigenen verwalteten `.desktop`-Eintrag erzeugen.

Übernehme keine Verantwortung für Installation, Firmware, Schlüssel oder Spieldateien des Emulators.

## Phase 2 – Managed Launch und Session Tracking

Erweitere `LaunchMethod` um eine rückwärtskompatible Variante für verwaltete Emulatorstarts.

Normale Desktop-Dateien müssen weiterhin ausschließlich über GIO gestartet werden.

Für verwaltete Ryujinx-Einträge darf CouchDeck den Emulator direkt über `std::process::Command` starten.

Die Startparameter werden aus validierten, strukturierten Daten zusammengesetzt – nicht durch Shell-Parsing beliebiger Desktop-Dateien. Generierte Desktop-Dateien und der verwaltete Startweg müssen dieselbe Konfiguration verwenden.

Entwickle einen Session Manager mit Prozessüberwachung und den Zuständen Starting, Running, Closing, Finished und Failed.

Beachte insbesondere:
- Startskripte, die Kindprozesse erzeugen.
- Keine doppelte Emulatorsitzung ohne explizite Behandlung.
- Prozessende und Startfehler zuverlässig unterscheiden.
- Keine unkontrollierte Suche oder Terminierung fremder Ryujinx-Prozesse.
- Keine Blockierung des UI-Threads.

## Phase 3 – Beenden über Controller

Erweitere die bestehende gilrs-Eingabeschicht um einen separaten Session-Hotkey.

Vorschlag: Start + Select für zwei Sekunden halten.

Anforderungen:
- Kombination konfigurierbar und deaktivierbar.
- Nur bei einer aktiven, verwalteten Sitzung auslösen.
- Normale Controller-Navigation bleibt während des Spiels deaktiviert.
- Keine wiederholte Auslösung bei weiterhin gehaltenen Tasten.
- Controller-Trennung und erneute Verbindung korrekt behandeln.
- Prüfen, ob die Kombination in Spielen störende Eingaben verursacht.

Implementiere eine Strategie für das geordnete Schließen:

1. Emulator-eigene Schließschnittstelle nutzen, falls vorhanden und nachgewiesen.
2. Alternativ eine gezielte normale Fensterschließanforderung verwenden.
3. Tatsächliches Sitzungsende abwarten.
4. Bei Fehlschlag einen ausdrücklich bestätigten Notfallabbruch anbieten.

Für KDE Plasma 6 unter Wayland die KWin-Scripting-API prüfen. Ausschließlich eindeutig der gestarteten Sitzung zugeordnete Fenster dürfen geschlossen werden.

Kein blindes `pkill`, kein root-Zugriff und keine unbestätigte zwangsweise Prozessbeendigung.

## Phase 4 – Nahtlose Rückkehr

Nach bestätigtem Sitzungsende soll CouchDeck wieder in den Vordergrund wechseln.

Verwende die bereits vorhandene Fensteraktivierung aus CouchDeck v0.4.x, statt eine parallele Aktivierungslogik zu entwickeln.

Kein Fokusdiebstahl während des laufenden Spiels.

Unter Wayland sind Einschränkungen des Compositors explizit zu behandeln.

Nach Rückkehr soll die zuvor ausgewählte Spielkachel weiterhin fokussiert sein.

## Kompatibilität und Sicherheit

- Bestehende Profile, Kinderprofile und Zeitlimits erhalten.
- Vor jedem Ryujinx-Start die zentrale Spielberechtigung prüfen.
- Bei ausgeschöpftem Zeitkontingent keine neuen Emulatorstarts zulassen.
- Das Sitzungsende darf keine Admin- oder Kinderrechte umgehen.
- Bestehende Steam-, Prism- und normale Desktop-Einträge dürfen nicht regressieren.
- Bestehende Bibliotheksdaten rückwärtskompatibel laden.
- Neue UI-Texte vollständig in Deutsch und Englisch lokalisieren.
- Keine automatisch heruntergeladenen Spiele, Firmware oder Schlüssel.
- Bestehende Ryujinx-Konfiguration nicht ungefragt verändern.

## Tests und Abnahme

Ergänze Unit- und Integrationstests für:

- Command-Line-Generierung mit Leerzeichen und Sonderzeichen in Pfaden.
- Import mehrerer Spiele als getrennte Bibliothekseinträge.
- Fehlendes Ryujinx oder fehlende Spieldatei.
- Prozessstart, reguläres Ende, Absturz und fehlgeschlagenes Schließen.
- Doppelte Starts und bereits extern laufende Emulatorprozesse.
- Controller-Hotkey mit Haltezeit und Debouncing.
- Kinderprofile und gesperrte Spielstarts.
- Rückwärtskompatibilität vorhandener Datenmodelle.

Manuelle Abnahme unter Bazzite KDE/Wayland:

1. Spiel aus CouchDeck starten.
2. Ryujinx startet das korrekte Spiel im Vollbild.
3. Controller funktioniert im Spiel.
4. Beenden-Kombination wird erkannt.
5. Ausschließlich die verwaltete Sitzung wird beendet.
6. CouchDeck erscheint wieder mit korrekt gesetztem Fokus.
7. Normale Steam-, Prism- und Desktop-Spiele funktionieren weiterhin.

## Umsetzung

Arbeite zunächst einen funktionsfähigen Direktstart als Proof of Concept aus, anschließend Session Tracking und Controller-Beenden.

Halte alle Änderungen modular und testbar.

Führe `cargo fmt`, `cargo clippy`, `cargo test` und einen Release-Build aus. Dokumentiere nicht verifizierte Annahmen ausdrücklich.

Keine Änderungen an GitHub-Releases oder Versions-Tags ohne Freigabe.
