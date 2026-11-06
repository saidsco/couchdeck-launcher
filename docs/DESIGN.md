# UI-/Datenkonzept

## Hauptansicht
- Oben links Projektname, oben rechts Profilavatar/-name und Menü.
- Mitte: Cover-Grid (3–6 Spalten abhängig von DPI/Viewport). Fokus deutlich mit Rahmen und geringfügiger Vergrößerung; Crop je Bild. Navigation bleibt vollständig über D-Pad erreichbar.
- Unten: reduzierte Controller-Hinweise, keine Status-/Shop-/Promotionsfläche.
- Hintergrund: optionales globales oder spielspezifisches Bild; dunkler Gradient für Lesbarkeit. Statischer Ersatz ohne Bild.
- Keine Animations-/Repaint-Schleife im Ruhezustand.

## Zustandsmaschine (Vorschlag)
`ProfileSelection -> Library -> GameDetails (optional) -> Launching -> Library`.
Aus jeder geeigneten Ansicht `Menu`, `AdminPin`, `AdminPanel`, `ArtworkPicker`, `ImportPreview`, `ErrorDialog` möglich. Bei Startsignal nicht den Prozess als 'running' anzeigen, solange keine sichere Erkennung existiert.

## Bibliothekseintrag
Verweise auf den *ursprünglichen* Desktop-Entry-Pfad oder einen explizit generierten **eigenen** Eintrag; Titel, Theme-Icon, eigenes Cover, Hintergrund, Position, Sichtbarkeit/Profil-Favoriten getrennt speichern. Nicht im Desktop-File Bildmetadaten missbrauchen.

## Importprozess
1. XDG-Anwendungen auflisten und `.desktop`-Dateiauswahl anbieten.
2. App-Name, Herkunft und `Exec` vor Freigabe anzeigen.
3. Bei Flatpak-Steam-Fehlern informativ auf alternatives generiertes Desktop-Entry hinweisen, keine automatische Änderung.
4. Originalquelle referenzieren oder explizit verwaltete Kopie erzeugen; never silent overwrite.
5. Cover als separates Asset auswählen, nicht `Icon=` patchen.
6. Import sofort im Grid, Speicherung restartfest.

## Account-/Sicherheitsgrenze
Profile sind **virtuelle Profile im selben Linux-Benutzerkonto**. Keine Abschottung zwischen Linux-Prozessen. PIN verhindert UI-Zufallsänderungen, nicht Dateisystem-Manipulation. Für echte Berechtigungen separate Linux-Accounts verwenden.

## Internationalisierung (v0.3)
- FTL-Ressourcen für Deutsch und Englisch sind ins Binary eingebettet; Englisch ist Fallback.
- Automatische Auswahl aus `LANGUAGE`/`LC_ALL`/`LC_MESSAGES`/`LANG`; manuelle Auswahl `auto`/`de`/`en` in den Einstellungen wirkt ohne Neustart.
- Technische Diagnosetexte aus dem Desktop-Parser können in der ersten Ausbaustufe noch deutsch bleiben.

## Profil-Hintergründe (v0.3)
- Cover, Spielhintergrund, globales Hintergrundbild und Profil-Hintergrund sind getrennte Asset-Typen.
- Profil-Hintergrund wird nicht in die `.desktop`-Datei geschrieben; relative Pfade liegen unter `$XDG_DATA_HOME/couchdeck/assets/backgrounds/`.
- Fallback: Profil → globales Hintergrundbild. In der Bibliothek hat ein fokussiertes Spiel Vorrang vor dem Profil-Hintergrund.

## Kinderprofile und Zeitbudget (v0.3)
- Zeitmessung ist an das laufende CouchDeck-Profil gebunden, nicht an einen erkannten Spielprozess.
- `timekeeping.json` speichert pro Profil Datum, genutzte Sekunden, Zusatzminuten und Warnflags; Tageswechsel setzt die Tageswerte zurück.
- Zentrale Startprüfung `check_start_allowed` blockiert neue Starts; laufende Spiele werden nie zwangsweise beendet.
- ParentAuth prüft jede Admin-PIN eines Nicht-Kind-Admin-Profils einmalig und entsperrt den Admin-Bereich nicht dauerhaft.
- Zusatzzeit ist nur für den aktuellen Kalendertag gültig und verändert das reguläre Tageslimit nicht.
- App-interne Sperren: Profilwechsel, Minimieren, Beenden, Systemaktionen; Best-Effort für WM-Close/Minimize. Kein Schutz gegen Prozess-Kill, TTY-Wechsel oder Dateiänderungen im selben Linux-Konto.
