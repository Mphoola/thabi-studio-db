# 0034 — Settings page, and the dev gallery removed

**Status:** accepted · **Date:** 2026-10-16

## Settings page

Settings outgrew a dialog. It is now a page in the main area
(`src/components/settings/SettingsPage.vue`), opened with ⌘, / Settings…
like before. The explorer sidebar hides while it is open. Sections are listed
on the left and shown one at a time; each groups its rows into titled cards
(`SettingsGroup`, `SettingRow`):

- **General:** appearance.
- **Security:** app lock (password, idle lock) and recovery (codes,
  questions). See `0033`.
- **Editor:** font (size, family) and typing (indentation, keyword case,
  wrapping, Vim).
- **Queries & Results:** timeout and rows kept; time zone for values.
- **History:** keep history, how long, clear.

Done or Escape returns to the view that was open before. Escape inside one of
the page's own dialogs closes only that dialog. The last section shown is kept
while the app runs (`useLayout().settingsSection`).

## Removed

The dev-only Component Gallery (`src/components/dev/`, `0003` item 8) and the
"Show Component Gallery" and "Show Welcome View" commands. Both were for
testing. "Show Welcome View" duplicated "Show Connections"; the connections
page itself (`WelcomeView.vue`) stays.
