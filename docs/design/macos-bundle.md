# A double-clickable Scrim.app (R4, 22 Oct 2026)

**What it's for:** opening Scrim on this Mac from Finder, the Dock or Spotlight, without a terminal, while the signed, notarised release (`packaging/macos/package.sh`) waits for a signing identity.

## `cargo xtask bundle-macos`

It builds the release app for this Mac and writes `target/bundle/Scrim.app`:

| Part | What |
|---|---|
| `Contents/MacOS/figcraft` | The windowed app (`cargo build --release -p figcraft`), built for the minimum macOS below, with `FIGCRAFT_BUILD_SHA` set to the commit (Fix Y3): `FIGCRAFT_BUILD_SHA` if it is set, else `git rev-parse HEAD`, as `packaging/env.sh` does for the release. Its `--version` says "Scrim <version> (<full sha>)"; after signing, the xtask runs it and refuses the bundle if the commit isn't there. |
| `Contents/Info.plist` | From the release template, `packaging/macos/Info.plist.in`, so the two never drift. Name Scrim, identifier `design.scrim.app`, `NSHighResolutionCapable`. The version is Cargo's `[workspace.package]` version (the short X.Y.Z, and the full one as `ScrimVersion`), plus the commit (`ScrimBuildCommit`: the full sha since Fix Y3, the same one the binary prints, as the release's `package.sh` writes it). The executable is `figcraft`. `LSMinimumSystemVersion` is the build's target: `MACOSX_DEPLOYMENT_TARGET`, else the compiler's default for this Mac (`rustc --print deployment-target`: 11.0 on Apple silicon). The same value is passed to the build. |
| `Contents/Resources/Scrim.icns` | Made with `iconutil` from the placeholder icon's PNGs. |
| Signature | Ad hoc (`codesign --force --sign - --timestamp=none`): the executable, then the bundle, then `codesign --verify --strict`. |

- **The icon's one swap point:** `assets/app-icon/scrim/` (`scrim-16.png` … `scrim-1024.png`), which `cargo xtask icon` writes from the placeholder drawn in code. When the real mark comes, replacing those PNGs (and `scrim.svg`) changes this bundle, the release DMG and the Linux and Windows icons together. Nothing else names the icon.
- **What it doesn't do:** it doesn't notarise, doesn't touch Gatekeeper's settings, and doesn't add or remove anyone's quarantine attribute. The release script stays the way to ship.

## Opening it

**On the Mac that built it:** a bundle you build yourself carries no quarantine attribute (checked: `xattr` lists only `com.apple.provenance`), so it opens with a double-click like any app. Its ad-hoc signature is valid on disk.

**On another Mac** (downloaded, AirDropped, copied from a shared drive), the copy is quarantined. Because the bundle is signed ad hoc and not notarised, Gatekeeper refuses the first double-click: macOS says it can't verify that "Scrim" is free of malware, and offers only Move to Trash or Done. To open it anyway, once:
- **macOS 14 and earlier:** Control-click (or right-click) Scrim in Finder, choose **Open**, then **Open** in the dialog.
- **macOS 15 and later:** the Control-click route was removed. After the first refused attempt, open **System Settings ▸ Privacy & Security**, scroll to Security, click **Open Anyway** next to the line about Scrim, and confirm with your password.

After that it opens normally. These steps follow Apple's support guidance for apps from unidentified developers; they weren't tried here. Making a quarantined copy to try them would mean adding a quarantine attribute and triggering Gatekeeper on this Mac, which this task doesn't do.

**Launch Services:** opening the bundle registers it, like any app, as the handler for `.figcraft` files and `figcraft://` links (its `Info.plist` says so). A copy made only for a test is unregistered with `lsregister -u` and deleted afterwards.

## Connect Claude Desktop from inside the bundle

- **The entry:** Connect writes the app's own executable, by its real path, with `mcp-bridge` (Fix P). Inside the bundle that is `…/Scrim.app/Contents/MacOS/figcraft mcp-bridge`, and Claude Desktop starts it the same way.
- **The bridge isn't an app:** `figcraft mcp-bridge` answers on stdio and never opens a window. Launch Services doesn't list its process, so there's no Dock icon and it can't come to the front.
- **When the app moves** (another folder, another copy), the entry still names the old path: the Claude Desktop card says **Out of date**, "Claude Desktop points to … this window is at …". **Re-register**, confirmed, writes the new path, keeping a backup of the file first.

## Verified live (`docs/scrim-ports/macos-bundle/`)

On a scratch `HOME` and a scratch settings folder (Scrim's rules 9), with the MCP port seeded to 7690:
- **Opened with `open -n`**, no terminal: Launch Services listed it as a foreground app, `design.scrim.app`.
- **Its MCP endpoint** answered `initialize` as `scrim` 0.7.0.
- **The entry Connect would write** is the bundle's `Contents/MacOS/figcraft` with `mcp-bridge`, into the scratch Claude Desktop file.
- **That bridge, started as Desktop starts it,** initialised, listed 25 tools, and a `create_node` reached the window. Launch Services never listed its process, and no app appeared while it ran. It exited 0 when its input closed.
- **Configured, then moved:** written into the scratch configuration it read Configured. A copy of the bundle opened from another folder read Out of date, naming the old path. The copy was then unregistered and deleted.
- **The person's own files were only read:** Claude Desktop's real configuration and Scrim's real settings had the same timestamp, size and checksum before and after.

## Not verified

- **Re-register live, and any window capture:** the screen was locked all night, and a locked Mac draws no window, so the run skipped both and its log says so. Re-register after a move is covered in the UI harness: `desktop_flow_tests.rs` › `a_bridge_from_another_copy_of_scrim_is_out_of_date_and_re_registered` presses it through its confirmation and checks the new path and the backup.
- **A real Claude Desktop launch** of the bundle's bridge; the script started the same command the same way.
- **A notarised build,** and Gatekeeper's dialogs on a quarantined copy (above).
- **Intel Macs:** the bundle is built for the Mac that runs the task (arm64 here). The release script makes the universal build.
