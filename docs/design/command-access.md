# Command access: read or write (C1.3)

Settings ▸ Tool access holds `run_command` and `batch` to the same switches as the tools (`crates/engine/src/cmd/access.rs`, enforced in `crates/mcp/src/server.rs`). This page lists how every command is classified.

## How the classification was made

- **Before C1.3** a command carried one flag, `history` (it makes an undo step). That covered the 301 commands that change the document; the other 316 were unclassified.
- **Undoable commands write.** That's the registry's own flag; no list needed.
- **54 commands only answer: Read.** Each returns information and changes nothing: no document, file, selection, view, setting, clipboard or network.
- **The other 262 write.** They change the selection, the view or a panel, a setting, the clipboard, a file, a live session, a plugin, or reach the network.
- **Two calls to make, for you to overrule:**
  - Commands that only *fetch* over the network (`plugin.directory.list`, `collab.serverInfo`, `collab.files`) are classified Write: they reach outside the machine.
  - Changing only the selection or the view is classified Write: the `select` tool is not read-only either.
- **A test** (`access_tests.rs` › `every_registered_command_is_classified_once`) fails when a registered command is in neither list. An unclassified command is refused for agents (deny by default).

## Commands that do what a tool does

| Command | Tool |
|---|---|
| `node.create` | `create_node` |
| `node.set` | `set_properties` |
| `node.get` | `get_node` |
| `node.query` | `query_nodes` |
| `lint.run` | `lint_design` |
| `lint.setProfile` | `set_lint_profile` |
| `lint.ignore` | `ignore_lint` |
| `design.analyze` | `analyze_design` |
| `history.changes` | `get_changes` |
| `file.export` | `export` |
| `file.exportLayers` | `export` |
| `file.exportSvg` | `export` |
| `file.exportFramesPdf` | `export` |
| `file.open` | `open_file` |
| `file.openRecent` | `open_file` |
| `file.save` | `save_file` |
| `file.saveAs` | `save_file` |
| `edit.undo` | `undo` |
| `edit.redo` | `redo` |
| `command.batch` | `batch` |
| `select.set` | `select` |
| `select.none` | `select` |
| `select.all` | `select` |
| `select.add` | `select` |
| `select.remove` | `select` |
| `select.area` | `select` |
| `select.byName` | `select` |
| `select.children` | `select` |
| `select.inverse` | `select` |
| `select.matching` | `select` |
| `select.next` | `select` |
| `select.parent` | `select` |
| `select.previous` | `select` |
| `select.sameEffect` | `select` |
| `select.sameFill` | `select` |
| `select.sameFont` | `select` |
| `select.sameInstance` | `select` |
| `select.sameProperties` | `select` |
| `select.sameStroke` | `select` |
| `select.sameText` | `select` |

## Read (54)

- **ai:** `ai.findings` (Design lint findings by rule), `ai.status` (Agent connection)
- **annotation:** `annotation.categories` (Annotation categories), `annotation.list` (List annotations)
- **branch:** `branch.list` (List branches), `branch.review` (Review branch)
- **collab:** `collab.diff` (Show sync changes), `collab.peers` (People in this file)
- **comment:** `comment.list` (List comments), `comment.people` (People in comments)
- **component:** `component.list` (List components), `component.properties` (Component properties)
- **design:** `design.analyze` (Analyze design)
- **devResource:** `devResource.list` (List dev resources)
- **diagnostics:** `diagnostics.events` (Diagnostics)
- **doc:** `doc.list` (List documents)
- **frame:** `frame.presets` (List frame presets)
- **history:** `history.changes` (Changes since a revision)
- **image:** `image.presets` (Image presets)
- **inspect:** `inspect.assets` (Inspect assets), `inspect.code` (Code for layer), `inspect.componentCode` (Component code), `inspect.properties` (Inspect properties), `inspect.readyForDev` (Ready for dev)
- **instance:** `instance.exposed` (Exposed nested instances)
- **library:** `library.list` (List libraries), `library.updates` (Library updates), `library.usage` (Library usage)
- **lint:** `lint.run` (Lint design)
- **mcp:** `mcp.status` (MCP endpoint status), `mcp.tools` (MCP tool access)
- **measure:** `measure.distances` (Measure distances), `measure.padding` (Measure auto layout padding)
- **node:** `node.get` (Get layer), `node.query` (Find layers)
- **page:** `page.list` (List pages)
- **paint:** `paint.documentColors` (Document colours)
- **plugin:** `plugin.list` (List plugins)
- **proto:** `proto.evaluate` (Evaluate prototype expression)
- **select:** `select.layersAt` (Layers at a point)
- **selection:** `selection.colors` (Selection colors)
- **settings:** `settings.open` (Settings)
- **shapeBuilder:** `shapeBuilder.regions` (Shape builder regions)
- **style:** `style.list` (List styles)
- **template:** `template.list` (List templates)
- **text:** `text.fonts` (List fonts), `text.missingFonts` (Missing fonts)
- **toolbar:** `toolbar.state` (Toolbar)
- **variable:** `variable.list` (List variables)
- **vector:** `vector.edit.bounds` (Selected points' bounds), `vector.edit.state` (Vector edit state)
- **version:** `version.list` (List versions)
- **view:** `view.get` (Get view)
- **widget:** `widget.list` (List widgets)

## Write, outside the undo history (262)

- **ai:** `ai.copyConnectCommand`, `ai.selectChange`, `ai.selectFinding`, `ai.undoChange`
- **app:** `app.actions`, `app.preferences`
- **branch:** `branch.archive`, `branch.create`, `branch.merge`, `branch.open`, `branch.updateFromMain`
- **code:** `code.copy`, `code.copyStandaloneHtml`, `code.setFormat`
- **codeConnect:** `codeConnect.export`
- **collab:** `collab.cursor`, `collab.cursorChat`, `collab.files`, `collab.follow`, `collab.forgetFile`, `collab.host`, `collab.join`, `collab.leave`, `collab.manageFile`, `collab.openFile`, `collab.publish`, `collab.recoverFile`, `collab.registerLinks`, `collab.removePerson`, `collab.report`, `collab.serverInfo`, `collab.share`, `collab.spotlight`, `collab.stop`
- **command:** `command.batch`
- **comment:** `comment.link`, `comment.markRead`, `comment.next`, `comment.open`, `comment.zoom`
- **component:** `component.goToMain`, `component.playground`
- **dev:** `dev.compare`, `dev.focus`, `dev.unfocus`
- **diagnostics:** `diagnostics.clear`, `diagnostics.copy`, `diagnostics.retention`, `diagnostics.save`
- **doc:** `doc.home`, `doc.nextTab`, `doc.previousTab`, `doc.switch`
- **edit:** `edit.copy`, `edit.copyProperties`, `edit.pickColor`, `edit.redo`, `edit.setDefaultProperties`, `edit.undo`
- **file:** `file.close`, `file.duplicate`, `file.export`, `file.exportFramesPdf`, `file.exportLayers`, `file.exportSvg`, `file.new`, `file.newFromTemplate`, `file.open`, `file.openRecent`, `file.recover`, `file.revert`, `file.save`, `file.saveAs`, `file.saveVersion`, `file.versionHistory`
- **fonts:** `fonts.copyMissingNames`, `fonts.muteMissing`
- **guide:** `guide.select`
- **help:** `help.about`, `help.gettingStarted`, `help.page`, `help.shortcuts`
- **layers:** `layers.collapseAll`
- **libraries:** `libraries.open`, `libraries.publish`
- **library:** `library.publishToServer`
- **mcp:** `mcp.connect-claude-code`, `mcp.copy-address`, `mcp.copy-command`, `mcp.remove-old-entry`, `mcp.reregister`, `mcp.restart-server`, `mcp.restore-tools`, `mcp.run-connect`, `mcp.set-port`, `mcp.set-root-dir`, `mcp.set-tool`, `mcp.set-tool-group`, `mcp.toggle-auth`
- **page:** `page.select`
- **panel:** `panel.mode.animate`, `panel.mode.design`, `panel.tab.ai`, `panel.tab.code`, `panel.tab.design`, `panel.tab.prototype`
- **plugin:** `plugin.close`, `plugin.directory.install`, `plugin.directory.list`, `plugin.directory.publish`, `plugin.directory.unpublish`, `plugin.event`, `plugin.import`, `plugin.remove`, `plugin.run`, `plugin.ui`
- **plugins:** `plugins.manage`, `plugins.runLast`
- **pref:** `pref.accent`, `pref.appearance`, `pref.autoSave`, `pref.highlightHover`, `pref.keepTool`, `pref.language`, `pref.motion`, `pref.nudge`, `pref.numberKeysOpacity`, `pref.preserveUnsaved`, `pref.publishServer`, `pref.reduceMotion`, `pref.renameDuplicates`, `pref.showDimensions`, `pref.theme`, `pref.userName`
- **proto:** `proto.closePreview`, `proto.present`, `proto.presentOptions`, `proto.preview`
- **select:** `select.add`, `select.all`, `select.area`, `select.byName`, `select.children`, `select.inverse`, `select.matching`, `select.next`, `select.none`, `select.parent`, `select.previous`, `select.remove`, `select.sameEffect`, `select.sameFill`, `select.sameFont`, `select.sameInstance`, `select.sameProperties`, `select.sameStroke`, `select.sameText`, `select.set`
- **selection:** `selection.selectColor`
- **shapeBuilder:** `shapeBuilder.mode`
- **text:** `text.edit.begin`, `text.edit.caret`, `text.edit.end`, `text.edit.preedit`, `text.edit.selectLine`, `text.edit.selectWord`, `text.edit.selection`, `text.endPreview`, `text.preview`
- **tool:** `tool.arrow`, `tool.comment`, `tool.ellipse`, `tool.frame`, `tool.hand`, `tool.line`, `tool.move`, `tool.pen`, `tool.pencil`, `tool.pencilBrush`, `tool.place`, `tool.polygon`, `tool.rectangle`, `tool.scale`, `tool.scaleAnchor`, `tool.section`, `tool.set`, `tool.shapeBuilder`, `tool.slice`, `tool.star`, `tool.text`
- **toolbar:** `toolbar.group`, `toolbar.move`, `toolbar.reset`, `toolbar.toggle`, `toolbar.ungroup`
- **transform:** `transform.rotationOrigin`, `transform.setRotationOrigin`
- **variable:** `variable.export`
- **vector:** `vector.edit.begin`, `vector.edit.end`, `vector.edit.lasso`, `vector.edit.penEnd`, `vector.edit.select`, `vector.edit.selectAll`, `vector.edit.tool`
- **version:** `version.autosave`, `version.delete`, `version.duplicate`, `version.exitPreview`, `version.preview`, `version.rename`
- **view:** `view.assets`, `view.comments`, `view.contextMenu`, `view.design`, `view.designPrototype`, `view.devMode`, `view.frameOutlines`, `view.fullscreen`, `view.layers`, `view.layoutGrids`, `view.leftSidebar`, `view.maskOutlines`, `view.multiplayerCursors`, `view.nextFrame`, `view.nextPage`, `view.nextRegion`, `view.outlines`, `view.outlinesHiddenLayers`, `view.outlinesObjectBounds`, `view.outlinesSelection`, `view.pan`, `view.panelWidths`, `view.pixelGrid`, `view.pixelPreview`, `view.prevFrame`, `view.prevPage`, `view.previousRegion`, `view.proofSrgb`, `view.prototype`, `view.rulers`, `view.set`, `view.slices`, `view.snapGeometry`, `view.snapObjects`, `view.snapPixel`, `view.themeDark`, `view.themeLight`, `view.themeSystem`, `view.toggleUi`, `view.zoom100`, `view.zoom200`, `view.zoom50`, `view.zoomAt`, `view.zoomFit`, `view.zoomIn`, `view.zoomOut`, `view.zoomSelection`
- **widget:** `widget.click`, `widget.insert`
- **window:** `window.focus`, `window.new`
