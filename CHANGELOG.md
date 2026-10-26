# Changelog

All notable changes to Sayzo are documented here. The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and the project uses [Semantic Versioning](https://semver.org/).

## [0.3.0] - 2026-10-26

### Added
- **Draw on screen** (Ctrl+Alt+D, the pen button on the bar, or the tray menu). Pen with pressure, highlighter, laser pointer, line, arrow, rectangle, ellipse, text, numbered steps and eraser, over any app.
- **Smart shapes:** hold the pen still at the end of a stroke and a rough line, box or circle becomes a clean one.
- **Read tool:** draw a box around any text on screen and Sayzo reads it aloud.
- **Mouse mode** (M or Ctrl+Alt+D): click and scroll the apps underneath while your drawings stay on screen.
- Whiteboard and blackboard, fading ink, spotlight, hide/show drawings, undo/redo, clear.
- **Screenshot with drawings:** copied to the clipboard and saved to Pictures\Sayzo.
- Drawing toolbar in Sayzo's glass style, with colour and size picker, that can be collapsed to a small pill and dragged anywhere.
- **Minimize the floating bar** to a small pill that still shows playback controls while reading.

## [0.2.1] - 2026-10-22

### Fixed
- The app froze, and couldn't even be closed, when the **Read text from a screen area** button on the bar was clicked. Opening the dashboard from the bar is protected against the same problem.

## [0.2.0] - 2026-10-22

### Added
- **Add your own voices and languages.** Import any free voice from the sherpa-onnx collection (Piper, Coqui, MMS or Kokoro) as a `.tar.bz2`, `.zip` or folder from Dashboard → Voices. 50+ languages and many accents are available.
- **Five more built-in languages:** Spanish, French, Italian, Portuguese (Brazil) and Hindi.
- **Automatic language detection per sentence**, choosing among the languages you have voices for, so each language is read by its own voice.
- **Kareem**, a third built-in Arabic voice that's best for everyday dialect text.

### Fixed
- **Arabic pronunciation.** Missing vowels (tashkeel) are now restored offline with the CATT diacritizer before speaking. In our tests, formal Arabic word errors dropped from 18% to 2.5% with Miro, and dialect errors dropped by about a third.
- **Numbers in Arabic text** are now read as Arabic words, including Arabic-Indic digits, decimals and percentages.
- Persian and Urdu text is no longer read with Arabic vowels.

## [0.1.1] - 2026-10-21

### Fixed
- The Arabic sample text on the Home page still used the app's old name.

### Changed
- Added screenshots and a download link to the README.

## [0.1.0] - 2026-10-21

First public release.

### Added
- Floating glass bar with a dashboard (Home, Voices, History, Shortcuts, Settings).
- Offline neural voices: Kokoro-82M for English (27 voices) and Piper for Arabic (2 voices).
- Auto-read on select, a read-selection hotkey, and OCR screen-area reading.
- Per-sentence Arabic/English detection, with voice switching inside mixed sentences.
- Reading panel with sentence highlighting and click-to-jump.
- WAV export, reading history, tray menu, launch at startup, hide from screen sharing.
- Customizable global shortcuts, themes and transparency.

[0.3.0]: https://github.com/op-h/sayzo/releases/tag/v0.3.0
[0.2.1]: https://github.com/op-h/sayzo/releases/tag/v0.2.1
[0.2.0]: https://github.com/op-h/sayzo/releases/tag/v0.2.0
[0.1.1]: https://github.com/op-h/sayzo/releases/tag/v0.1.1
[0.1.0]: https://github.com/op-h/sayzo/releases/tag/v0.1.0
