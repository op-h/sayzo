# Changelog

All notable changes to Sayzo are documented here. The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and the project uses [Semantic Versioning](https://semver.org/).

## [0.3.2] - 2026-10-30

### Fixed
- The British English voices (Emma, Isabella, Alice, Lily, George, Fable, Daniel and Lewis) read nothing. The speech engine was asked for an "en-gb" voice that its phoneme data doesn't have, so every sentence failed silently. They now speak normally.
- The dashboard could open as a blank white window, or come back blank after being minimized. Sayzo now makes it redraw every time it's opened or restored, and any moment before it's drawn shows the theme's background instead of a white sheet.

## [0.3.1] - 2026-10-28

### Fixed
- After Sayzo had been running for a while, **Draw on screen** and **Read a screen area** could stop opening, with no message, until Sayzo was restarted. This happened when Windows updated the WebView2 runtime in the background. Sayzo now stays on the WebView2 version it started with, and if a window still can't open, it restarts itself and opens what you asked for.
- If the drawing toolbar failed to open, an invisible drawing layer could be left covering the screen.

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
- Drawing opens in **mouse mode**, so your screen stays live; pick a tool to draw, click the same tool again to go back to the mouse.
- Drag either bar from any empty space, not just the grip; both bars are a little bigger.
- **The drawing toolbar is linked to the voice bar:** it docks right under it (or above it, near the bottom of the screen), follows it when you move it, re-centres when you minimize or expand it, and the bar's pen button turns blue while drawing. Drag the toolbar away to undock it.
- The voice bar stays on top of the drawing layer.
- New six-dot grip: wider-spaced, fuller dots with hover and press feedback.
- Both bars size themselves to their contents, with even spacing on every side.

### Fixed
- Clicking the drawing area could cover the toolbar, so you couldn't pick another tool.
- A click on the toolbar could be ignored when the drawing had just changed.
- Drawing shortcuts didn't work while an Arabic (or any non-English) keyboard layout was active.
- Typing in a text box could trigger tool shortcuts instead of typing.
- Popups opened off screen when the toolbar was dragged to the top.
- The minimized bar had a wide empty area on the right; the drawing toolbar had a 2 px gap at the top.
- A white box flashed on screen for a moment when drawing or the screen-area picker opened.

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

[0.3.2]: https://github.com/op-h/sayzo/releases/tag/v0.3.2
[0.3.1]: https://github.com/op-h/sayzo/releases/tag/v0.3.1
[0.3.0]: https://github.com/op-h/sayzo/releases/tag/v0.3.0
[0.2.1]: https://github.com/op-h/sayzo/releases/tag/v0.2.1
[0.2.0]: https://github.com/op-h/sayzo/releases/tag/v0.2.0
[0.1.1]: https://github.com/op-h/sayzo/releases/tag/v0.1.1
[0.1.0]: https://github.com/op-h/sayzo/releases/tag/v0.1.0
