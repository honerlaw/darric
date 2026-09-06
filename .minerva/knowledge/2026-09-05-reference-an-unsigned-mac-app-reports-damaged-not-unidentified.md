# An unsigned macOS app downloaded through a browser reports "damaged", not "unidentified developer"

**Date**: 2026-09-05
**Type**: reference
**Summary**: on Apple Silicon a quarantined unsigned app fails with "Darric is damaged and can't be opened" offering only Move to Trash — not the milder unidentified-developer prompt — and right-click → Open does not clear it; only removing the quarantine attribute does
**Context**: .minerva/work/2026-09-05-release-on-merge

## Context

darric's release builds are unsigned: no Apple Developer certificate exists for the project, so
`release.yml` publishes the `.dmg` files as `tauri build` produces them.

## Finding

The widely-repeated remedy for an unsigned Mac app — _right-click → Open, then confirm the
"unidentified developer" dialog_ — describes a **signed but un-notarized** app. A completely
unsigned app on Apple Silicon, carrying the quarantine attribute a browser download sets, gets a
different and much worse dialog:

> **"Darric" is damaged and can't be opened. You should move it to the Trash.**

There is no Open button, no override in the dialog, and nothing indicating the file is intact. A
user who sees it concludes the download is corrupt and retries or gives up — the failure reads as
a broken artifact rather than a security policy. Right-click → Open does not produce an override
path here.

The actual remedy is to remove the attribute after copying the app into place:

```sh
xattr -d com.apple.quarantine /Applications/Darric.app
```

`tauri build` applies an ad-hoc signature, and it does not help: an ad-hoc signature is not a
Developer ID signature and is not notarization, so quarantine still triggers this dialog.

## Implications

- Any README or release note for an unsigned macOS build must give the `xattr` command
  specifically. "Right-click → Open" is wrong for this case and sends the user in a circle.
- Judge the cost of not signing by this dialog, not by the unidentified-developer one. The gap
  between them is most of the argument for paying for a Developer ID.
- The same applies to the `.app` inside the `.dmg` and to any zip — quarantine is set by the
  downloading application, not by the bundle format.

## Related

- [[2026-09-05-reference-macos-13-is-retired-and-macos-15-intel-is-the-last-x86-64-image]] — the other operational fact governing what darric can ship
