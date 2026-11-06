# Releasing Scrim for macOS

`cargo xtask release-macos` builds Scrim for macOS as people download it.
- **The app:** universal (Apple silicon and Intel), signed with your Developer ID Application certificate, with the hardened runtime.
- **The image:** a signed `.dmg` holding the app and an Applications link.
- **Notarisation, twice:** Apple notarises the app and its ticket is stapled to it; then the image is made from the stapled app, and Apple notarises the image, whose ticket is stapled to it. A copy of the app dragged out of the image carries its own ticket, so it opens offline too.
- **The checksum:** a `.sha256` file beside the image.

Nothing is uploaded: publishing the release on GitHub stays a separate step that you do.

The code and the order of its steps are in `xtask/src/release_macos.rs`; the network it uses is described in [`network-audit.md`](network-audit.md).

## What you need once

### 1. A Developer ID Application certificate

You need an Apple Developer Program membership. A local development certificate (as Xcode makes for running apps on your own Mac) can't sign an app other Macs open, and the command refuses one.

1. In Xcode: Settings ▸ Accounts ▸ your team ▸ Manage Certificates ▸ + ▸ **Developer ID Application**. Or make it at developer.apple.com ▸ Certificates, download it and double-click it.
2. Check that the login keychain has it, with its private key:

   ```sh
   security find-identity -v -p codesigning
   ```

   One line must read `"Developer ID Application: <Your Name> (<TEAMID>)"`. That exact text is the identity you pass to the command.

### 2. A notarisation profile in the Keychain

`notarytool` keeps your Apple ID, team and an app-specific password in the Keychain under a name. The command uses that name and never sees the password.

1. Make an app-specific password at account.apple.com ▸ Sign-In and Security ▸ App-Specific Passwords.
2. Store the profile (it asks for the password; type it there, never in a file or a script):

   ```sh
   xcrun notarytool store-credentials scrim-notary --apple-id <your Apple ID> --team-id <TEAMID>
   ```

3. Check it: `xcrun notarytool history --keychain-profile scrim-notary` lists your past submissions, or none.

## Making a release

From a clean checkout whose commit is pushed (the app names the commit it was built from, and the command checks both):

```sh
# The plan, nothing run:
cargo xtask release-macos --dry-run --identity "Developer ID Application: <Your Name> (<TEAMID>)"

# Signed and on a disk image, not notarised (to check the build first):
cargo xtask release-macos --skip-notarize --identity "Developer ID Application: <Your Name> (<TEAMID>)"

# The release:
cargo xtask release-macos --identity "Developer ID Application: <Your Name> (<TEAMID>)"
```

Instead of `--identity`, you can set `SCRIM_SIGN_IDENTITY`. `--notary-profile <name>` uses another Keychain profile (default `scrim-notary`).

The command refuses to sign:
- with no identity named;
- with an identity that isn't exactly one listed by `security find-identity`;
- with anything but a Developer ID Application identity.

It never picks one for you.

### What it does, and how long it takes

| Stage | What | Time measured on 5 Nov 2026 (Apple silicon, 4 build jobs) |
|---|---|---|
| Preconditions | Clean tree, commit pushed, Rust targets (added with rustup when missing), tools, identity, the minimum macOS (11.0) the same in the build and the Info.plist | seconds |
| Build | `figcraft` (the app) and `scrim` (the CLI and MCP bridge), release mode, for `aarch64-apple-darwin` and `x86_64-apple-darwin` in one cargo command | 584 s from a cold cache; seconds when nothing changed |
| Universal | `lipo -create` for each, checked with `lipo -archs`; `figcraft --version` must name the commit | seconds |
| Assemble | `Scrim.app` as `cargo xtask bundle-macos` makes it: the Info.plist (the `.scrim` document type and its UTI, the version, the commit, the maintainers' address), the icon, both binaries | seconds |
| Sign | Inside out (`scrim`, `figcraft`, then the bundle), each with the hardened runtime and a secure timestamp, never `--deep`; then `codesign --verify --deep --strict` and `codesign -dvv` | 10 s (each signature asks Apple's timestamp server) |
| Notarise the app | The signed app zipped (`ditto -c -k --keepParent`), then `xcrun notarytool submit <zip> --keychain-profile scrim-notary --wait` | the first Apple wait: 59 s on 6 Nov 2026 (Apple's queue decides: usually a minute or a few, sometimes an hour) |
| Staple the app | `stapler staple Scrim.app`, then `stapler validate Scrim.app` | seconds |
| Disk image | The stapled `Scrim.app` and an Applications link (`hdiutil create -srcfolder`, HFS+, compressed), and signed. The image is never made from an app that isn't stapled: staging it needs the ticket stapling writes. (`hdiutil makehybrid`, used before, gave the binaries a Finder-info attribute when copied out of the image, which strict signature checks refuse.) | about 20 s |
| Notarise the image | `xcrun notarytool submit <dmg> --keychain-profile scrim-notary --wait` | the second Apple wait: 68 s on 6 Nov 2026 |
| Staple the image | `stapler staple`, then `stapler validate`, on the image | seconds |
| Verify | `spctl` on the app (execute) and the image (open, primary signature); `stapler validate` on the app; the image mounted read-only, the app copied out of it with `ditto`, and that copy checked as a person's would be (`stapler validate`, `spctl`, `codesign --verify --deep --strict`); `codesign --verify --deep --strict` on the app | seconds |
| Checksum | `<image>.sha256`, in the form `shasum -a 256 -c` reads | seconds |

The whole run on 6 Nov 2026 took 592 s: the build for a new commit 451 s, the two Apple waits 127 s, the rest about 15 s. Allow 10 to 15 minutes, more when Apple's queue is busy.

The output is in `~/.cache/scrim-target/release/` (with the target folder this repository uses), which is also cargo's own release folder:
- `Scrim-<version>-universal.dmg`;
- `Scrim-<version>-universal.dmg.sha256`;
- `Scrim.app` (the stapled app, as on the image).

The work folder is `release-macos/` beside it.

Running the command again starts over: it clears the previous image, checksum and app first. Submitting an image again is fine; Apple simply notarises it again.

### No entitlements

The app is signed with the hardened runtime and no entitlements (`packaging/macos/entitlements.plist` says why):
- it isn't sandboxed;
- it loads no code from outside the app: plugins run in an interpreter, `wasmi`, with no just-in-time compiler;
- it draws through Metal, which is part of macOS.

The release check launches the signed app to confirm this.

### Why the app is notarised on its own first

A ticket can't be stapled to an app inside an image without changing the image, which breaks the image's signature. So the app is notarised and stapled before the image is made from it, and the image is notarised after:
- **When someone opens the image,** macOS checks the image's ticket.
- **When they open the app they copied out of it,** the app carries its own ticket, so the check needs no connection, even on a Mac that is offline.

The release checks the second point itself: it mounts the image, copies the app out as a person would, and validates the copy's ticket.

That means two submissions and two waits for Apple instead of one; with `--skip-notarize`, neither.

## Reading a notarisation log

When Apple doesn't accept a submission (the app's zip or the image), the command fetches its log and prints it, then stops (it doesn't try again, and after a refused app no image is made). You can fetch it yourself with the submission id it prints:

```sh
xcrun notarytool log <submission id> --keychain-profile scrim-notary
xcrun notarytool history --keychain-profile scrim-notary   # every submission and its status
```

The log is JSON. Look at `issues`: each has a `path` (the file inside the image) and a `message`. The common ones:

| Message | Meaning |
|---|---|
| The binary is not signed with a valid Developer ID certificate | Signed with something else (ad hoc, a development certificate), or the certificate expired. |
| The signature does not include a secure timestamp | Signed without `--timestamp`. The command always adds it, so this means the timestamp server couldn't be reached during signing: run it again. |
| The executable does not have the hardened runtime enabled | Signed without `--options runtime`. |
| The signature of the binary is invalid | The file changed after signing. |

## Gatekeeper: when a Mac won't open Scrim

| What macOS says | Why | What to do |
|---|---|---|
| "Scrim can't be opened because Apple cannot check it for malicious software" | The app isn't notarised (a `--skip-notarize` build, or one built from source). | For a build you trust: Control-click the app ▸ Open ▸ Open, or System Settings ▸ Privacy & Security ▸ Open Anyway. A release from this command carries its ticket, so it opens offline too. |
| "Scrim is from an unidentified developer" | The app isn't signed with a Developer ID. | The same as above, for a build you trust. A release from this command is always signed. |
| "Scrim is damaged and can't be opened. You should move it to the Trash." | The signature doesn't match the files: the app was changed after signing, or a download was cut short. On recent macOS it is also what an unsigned app shows when it carries the quarantine mark. | Download it again and check the checksum (below). Don't strip the quarantine mark with `xattr` to get past this on a release download: it would hide a real problem. For an unsigned build you made yourself, `xattr -d com.apple.quarantine /Applications/Scrim.app` removes the mark. |

## Checking a download

In the folder with the image and its checksum file:

```sh
shasum -a 256 -c Scrim-<version>-universal.dmg.sha256
# Scrim-<version>-universal.dmg: OK

spctl --assess --type open --context context:primary-signature -vv Scrim-<version>-universal.dmg
# … accepted, source=Notarized Developer ID
```

Once the app is in Applications:

```sh
spctl --assess --type execute -vv /Applications/Scrim.app
# /Applications/Scrim.app: accepted, source=Notarized Developer ID
codesign -dvv /Applications/Scrim.app 2>&1 | grep -E 'Authority=Developer ID Application|TeamIdentifier|flags'
```

## The other macOS packaging

`packaging/macos/package.sh` is the script GitHub Actions runs for tagged releases (`.github/workflows/release.yml`). It signs and notarises with secrets stored in the repository instead of the Keychain, and also ships the CLI as its own zip. `release-macos` is the one to run on your Mac. Both fill in the same Info.plist template.
