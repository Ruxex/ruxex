# Ruxex Upstream Import Reconstruction Plan

This is a generated intermediate plan reconstructed from the per-UTC-date audit ledgers. It is audit-only: no upstream change has been implemented by generating this file.

## Coverage

- Upstream fork point: `e269f2164cbb9f499e4f22301c393500e2a831f3`
- Audited range represented here: `2026-09-19` through `2026-10-23` UTC
- Audited commits: 1626
- Full upstream commit messages remain verbatim in the linked date ledgers; this plan carries the exact hash, date, subject, and ledger link needed to reconstruct each bundle.

### Decision totals

| Decision | Count |
| --- | ---: |
| `Import proactively` | 909 |
| `Deferred` | 337 |
| `User review required` | 30 |
| `Do not import` | 350 |

## Batch totals

| UTC batch | Commits | Import | Deferred | Review | Do not import |
| --- | ---: | ---: | ---: | ---: | ---: |
| 2026-09-19–2026-09-23 | 341 | 164 | 115 | 0 | 62 |
| 2026-09-24–2026-09-30 | 352 | 218 | 54 | 3 | 77 |
| 2026-10-01–2026-10-07 | 335 | 173 | 55 | 11 | 96 |
| 2026-10-08–2026-10-14 | 256 | 158 | 44 | 4 | 50 |
| 2026-10-15–2026-10-21 | 238 | 140 | 52 | 6 | 40 |
| 2026-10-22–2026-10-23 | 104 | 56 | 17 | 6 | 25 |

## Reconstructed bundles

The status is conservative: any bundle containing a review item is waiting for user decision; otherwise a bundle containing deferred work waits for the related upstream series or an explicit re-audit trigger. A bundle with only proactive work is a candidate for implementation, subject to tests and prerequisite ordering.

### B02 — App Server, thread history, and lifecycle

- Status: `Waiting for user decision`
- Members: 341
- Proactive: 248; Deferred: 79; Review: 14
- Implementation order: adapt or add tests first, manually adapt production code to Ruxex, run affected unit tests and relevant E2E tests, then record the Ruxex commit.

| Date | Upstream commit | Subject | Decision |
| --- | --- | --- | --- |
| 2026-09-19 | [`05de93520542`](./2026-09-19.md#05de93520542d5d1a3f3af76ad446607a12d98fd) | Expose declared onboarding skills in plugin details (#46544) | `Deferred` |
| 2026-09-19 | [`1537497e52f3`](./2026-09-19.md#1537497e52f3284253884472c3bd796ade5fc02b) | Allow compatible feature overrides when starting the shared daemon (#46529) | `Deferred` |
| 2026-09-19 | [`3bb0a530d1cd`](./2026-09-19.md#3bb0a530d1cdabe1bbd35b3f44d72f851f0a93f4) | Add system proxy fallback for login and startup requests (#46562) | `Import proactively` |
| 2026-09-19 | [`78d4d983d338`](./2026-09-19.md#78d4d983d3380fa92aa17fbe9e4f9b737b6a5223) | Gate reasoning effort updates on explicit model support (#46530) | `Import proactively` |
| 2026-09-19 | [`888be42a20c5`](./2026-09-19.md#888be42a20c5a727214d898c4e335ac2e27161af) | Support explicit provider model catalog URLs (#46561) | `Deferred` |
| 2026-09-19 | [`98a8d4ea9cab`](./2026-09-19.md#98a8d4ea9cab33510c1577619a3c5fcf27baac98) | Advertise the control socket's WebSocket message size limit (#46548) | `Deferred` |
| 2026-09-19 | [`b948cfa88da3`](./2026-09-19.md#b948cfa88da330b338d6f49ede5d2e58fdedf62b) | Rename plugin MCP and app extension APIs (#46564) | `Deferred` |
| 2026-09-19 | [`c6f5d9e9b5d9`](./2026-09-19.md#c6f5d9e9b5d9ab8085ad5d3a7e28717d3a6a19dc) | Return `LiveAgent` records from agent listing (#46553) | `Deferred` |
| 2026-09-19 | [`cd2f9ca69239`](./2026-09-19.md#cd2f9ca692397e3092315f5b6d39273383e882c9) | Fix active-turn environment selection lookup (#46557) | `Import proactively` |
| 2026-09-19 | [`e24ac448b37b`](./2026-09-19.md#e24ac448b37bc6a29e6f2ce8dad61ddaaabf09dc) | Allow thread instruction providers to share updates with subagents (#46577) | `Deferred` |
| 2026-09-19 | [`e416eadf7f99`](./2026-09-19.md#e416eadf7f990d96bc1b538c02ddcdb062d5542c) | Encapsulate rollout budget accounting in `LocalAgentControl` (#46559) | `Import proactively` |
| 2026-09-19 | [`f81a6331b4fc`](./2026-09-19.md#f81a6331b4fc904447b784185e2289625b86d81a) | Set explicit turn triggers for exec and TUI requests (#46569) | `Import proactively` |
| 2026-09-20 | [`6ba6b73657d0`](./2026-09-20.md#6ba6b73657d0590382fc2ae2ad2bad6ea268aff5) | Render dynamic tool activity and preserve TUI history ordering (#46731) | `Deferred` |
| 2026-09-20 | [`c45ea25ffb72`](./2026-09-20.md#c45ea25ffb72d5f7324489d824d0c677283aa0b4) | Allow subagents to request MCP elicitation input (#46877) | `Deferred` |
| 2026-09-21 | [`23a6b705bac7`](./2026-09-21.md#23a6b705bac7b849517946b3f1098380e214fd5c) | Add diagnostics for rollout compression metadata failures (#47026) | `Import proactively` |
| 2026-09-21 | [`3ead67394691`](./2026-09-21.md#3ead67394691548e5c2f1be4cad649907f075e66) | Defer extension tool history materialization until first access (#47035) | `Import proactively` |
| 2026-09-21 | [`3fd5160cd6c7`](./2026-09-21.md#3fd5160cd6c78f2051bb54359d53f09207171733) | Honor the configured product SKU in remote plugin requests (#47116) | `Deferred` |
| 2026-09-21 | [`40eeb6e8a89e`](./2026-09-21.md#40eeb6e8a89ef421c25d4c40e06fa1d40ce66b4f) | Recover corrupt message-board storage during thread deletion (#47042) | `Import proactively` |
| 2026-09-21 | [`429afbcaa97a`](./2026-09-21.md#429afbcaa97a9397e59976afdc0c38a15db64956) | Extract per-thread enrichment into a dedicated async helper (#47083) | `Import proactively` |
| 2026-09-21 | [`4a8cfe6bb7a4`](./2026-09-21.md#4a8cfe6bb7a4ac90ca38f932627a214d7b10567b) | Pass thread IDs to attachment uploads (#47121) | `Import proactively` |
| 2026-09-21 | [`6a601c00da85`](./2026-09-21.md#6a601c00da854de09253952aaf2222f17c7c02ef) | Preserve originating turn metadata for yielded skill calls (#47106) | `Import proactively` |
| 2026-09-21 | [`73129716c28b`](./2026-09-21.md#73129716c28bd56d49a8b02544934dcc8a961dc2) | Delete persisted message boards with their root threads (#47029) | `Deferred` |
| 2026-09-21 | [`75db67bbc6a1`](./2026-09-21.md#75db67bbc6a18200b52698c8870adaedd7746dea) | Enforce current provider requirements for the model catalog (#46917) | `Deferred` |
| 2026-09-21 | [`772abc9425d4`](./2026-09-21.md#772abc9425d4bf4c2802456365785378811e7d3d) | Preserve and expose thread item lifecycle timestamps (#47114) | `Deferred` |
| 2026-09-21 | [`8a62cc4dc1a4`](./2026-09-21.md#8a62cc4dc1a498cfdbee31ad58652f25e680273d) | Align message-board tools with the multi-agent namespace (#47028) | `Deferred` |
| 2026-09-21 | [`a6fdb11eda99`](./2026-09-21.md#a6fdb11eda992b706c5e6a21ae5fd8c7376f8e06) | Preserve metadata in native user verification requests (#46952) | `Import proactively` |
| 2026-09-21 | [`ac985376787f`](./2026-09-21.md#ac985376787fcf69a9dcd4ef43c4eb1752f230d0) | Track cumulative MCP attribution across requests and thread history (#47081) | `Deferred` |
| 2026-09-21 | [`b4b30dd5eb99`](./2026-09-21.md#b4b30dd5eb9994ddc51f915ecd74e9a8c1d93248) | Centralize JSON-RPC response serialization through payloads (#47089) | `Import proactively` |
| 2026-09-21 | [`d583e73c4d12`](./2026-09-21.md#d583e73c4d1204f1e9f654ef87065e9f0dd07ac7) | Fetch local message board thread summaries in one query (#47055) | `Deferred` |
| 2026-09-21 | [`e51aacad602d`](./2026-09-21.md#e51aacad602dec4108db0d2f62bb7f40c6ced56c) | Include attributed previews in agent message board notifications (#47075) | `Deferred` |
| 2026-09-21 | [`f3037cafd32b`](./2026-09-21.md#f3037cafd32b2097dae718eb86bcc9182dc0a5d1) | Refresh host-supplied cloud skills at turn startup (#47074) | `Deferred` |
| 2026-09-22 | [`04fc75adbe67`](./2026-09-22.md#04fc75adbe67a612a1cb0fc469533f24b24fa499) | Add conditional turn interruption that preserves pending input (#47340) | `Import proactively` |
| 2026-09-22 | [`2c2a42e65de0`](./2026-09-22.md#2c2a42e65de077c5518ea5b4c3999633ef6a12fc) | Restrict MCP OAuth authorization endpoints to HTTP(S) (#47326) | `Import proactively` |
| 2026-09-22 | [`3b50349e1a80`](./2026-09-22.md#3b50349e1a80930e6e9dcf6d9377d65ef55ef052) | Add explicit app and account targets to MCP resource reads (#47248) | `Deferred` |
| 2026-09-22 | [`559264d92e44`](./2026-09-22.md#559264d92e4462e887d7508c705599f33daf4f1e) | Preserve `invalid_prompt` as a distinct error classification (#47353) | `Import proactively` |
| 2026-09-22 | [`6cfe29984965`](./2026-09-22.md#6cfe29984965abfb5f7a5f86048f655d45d028a7) | Preserve image generation request IDs on failure (#47327) | `Import proactively` |
| 2026-09-22 | [`9124e23e2cd5`](./2026-09-22.md#9124e23e2cd5411fcd996724f37cb66ea074c17d) | Move plugin recommendations into developer context (#47349) | `Import proactively` |
| 2026-09-22 | [`a1f40f3f1326`](./2026-09-22.md#a1f40f3f1326eff7c81b860a4f4e27c1be186e10) | Expose hosted plugin extensions in app-server summaries (#47263) | `Deferred` |
| 2026-09-22 | [`b23324986d03`](./2026-09-22.md#b23324986d03ae02d041c00d8e15bee6c1437b46) | Expose HTTP origins in MCP server status (#47159) | `Deferred` |
| 2026-09-22 | [`b58b05e062b8`](./2026-09-22.md#b58b05e062b8ce7354f641f1e0ee059cfc5b7e01) | Suppress agent message board notifications to the post author (#47236) | `Deferred` |
| 2026-09-22 | [`b781733f958e`](./2026-09-22.md#b781733f958ed55282c5ed343e3a6fef7139498c) | Separate thread preparation from execution persistence (#47199) | `Import proactively` |
| 2026-09-22 | [`bf7c87a7b7bd`](./2026-09-22.md#bf7c87a7b7bdb1b65cbfdca06d15713d3aa20b01) | Ignore stale thread-close notifications after resuming a thread (#47155) | `Import proactively` |
| 2026-09-22 | [`c117207a6f1f`](./2026-09-22.md#c117207a6f1f948ac7fcdd5e784d75fc4e9d13e1) | Drop descendant channel posts from retained remote compaction v2 history (#47249) | `Import proactively` |
| 2026-09-22 | [`c46aa58a7baa`](./2026-09-22.md#c46aa58a7baaec125612523f27f931d1ad22f8cd) | Centralize agent resume handling in `LocalAgentControl` (#47244) | `Import proactively` |
| 2026-09-22 | [`d3e584093222`](./2026-09-22.md#d3e584093222018fc27919403aea149ccfd3bb38) | Subscribe authors to message-board channels created by posting (#47259) | `Deferred` |
| 2026-09-22 | [`e7bbc79f482a`](./2026-09-22.md#e7bbc79f482acf285e50a09a9f898aeb2ce3881c) | Resume model context from the latest compaction boundary (#47365) | `Import proactively` |
| 2026-09-22 | [`fdd89e78ac33`](./2026-09-22.md#fdd89e78ac33f02e65dabe60567cdfeba2e954d4) | Return agent snapshots from `close_agent` (#47252) | `Deferred` |
| 2026-09-23 | [`3032b387fa20`](./2026-09-23.md#3032b387fa20fd782030587a305a27e6d02a7adc) | Preserve history search during background draft recovery (#47423) | `Import proactively` |
| 2026-09-23 | [`370cd7cba87a`](./2026-09-23.md#370cd7cba87aeef8cf39bb194e51829b210c26a6) | Allow environment selection updates during an active turn (#47435) | `Import proactively` |
| 2026-09-23 | [`408a77dc1a1c`](./2026-09-23.md#408a77dc1a1cf95413df26b5185623cf245161d7) | Clear pending async question notifications when turns end (#47424) | `Import proactively` |
| 2026-09-23 | [`40eac3ce8a0c`](./2026-09-23.md#40eac3ce8a0c10cbcb9db910d529355eb2f8fc09) | Add explicit background control to image generation (#47484) | `Deferred` |
| 2026-09-23 | [`5bfb04b24767`](./2026-09-23.md#5bfb04b247672223743ff7a1900f55b98d4a3a31) | Attach inherited rollout history to diagnostic reports (#47653) | `Import proactively` |
| 2026-09-23 | [`8215b81920d3`](./2026-09-23.md#8215b81920d3fe5f414768f18efd6823a9c4f45e) | Avoid the shutdown timeout in the lagged-event test (#47696) | `Import proactively` |
| 2026-09-23 | [`8a3c4ea3b5a7`](./2026-09-23.md#8a3c4ea3b5a7c0e92cf24dae46ec87629a26bb7f) | Apply inherited environment settings at turn boundaries (#47428) | `Import proactively` |
| 2026-09-23 | [`8f1490eabe3e`](./2026-09-23.md#8f1490eabe3ec27e92c46fd1b10400be3c910581) | Stream daemon executable hashing off the async runtime (#47591) | `Import proactively` |
| 2026-09-23 | [`acd5ed9207cd`](./2026-09-23.md#acd5ed9207cd65239f67182829b585b09c7d19fb) | Wait for idle before injecting remote compaction test history (#47571) | `Import proactively` |
| 2026-09-23 | [`b4b055cfc8fc`](./2026-09-23.md#b4b055cfc8fccc0040d13aa4304cf9ba61c2e27e) | Materialize rollout persistence for pending inter-agent messages (#47691) | `Import proactively` |
| 2026-09-23 | [`ce7df3623757`](./2026-09-23.md#ce7df362375772367bbba3deb9fb41632e96be1a) | Overlap startup WebSocket preconnect with tool discovery (#47635) | `Import proactively` |
| 2026-09-23 | [`d6c4c6aea404`](./2026-09-23.md#d6c4c6aea404128bbd467e843a874c702838b084) | Classify rollout read failures by reason and progress (#47565) | `Import proactively` |
| 2026-09-23 | [`fdeaa6c1ecad`](./2026-09-23.md#fdeaa6c1ecad21ae5e154e5873d166488b1fbd5e) | Allow history and notes without experimental context capability (#47625) | `Deferred` |
| 2026-09-24 | [`12cb14f7b70f`](./2026-09-24.md#12cb14f7b70f57aaf643d10ca072c44b555992d1) | Default local threads to paginated history (#47900) | `Deferred` |
| 2026-09-24 | [`16b20547f6b1`](./2026-09-24.md#16b20547f6b1feecbf41b89057d704ce1c293ab0) | Stabilize thread resume and memory dual-write tests (#47817) | `Import proactively` |
| 2026-09-24 | [`1d87af5faa75`](./2026-09-24.md#1d87af5faa75c2c09785cd088353d3236333673f) | Honor network policy in history notes and image generation extensions (#47742) | `Import proactively` |
| 2026-09-24 | [`3d6428510fee`](./2026-09-24.md#3d6428510fee6f52d8993ebacf4818bb42ba0cde) | Warn users when SQLite diagnostic log writes fail (#47887) | `Import proactively` |
| 2026-09-24 | [`3e27195f2de0`](./2026-09-24.md#3e27195f2de00dc975b1db03440ade31b889d9b7) | Parameterize the turn-start originator header test (#47918) | `Import proactively` |
| 2026-09-24 | [`4083a68f8837`](./2026-09-24.md#4083a68f88375bb0bc90a41b8c454d9e2d7c5281) | Allow four concurrent threads in the multi-agent resume test (#47824) | `Import proactively` |
| 2026-09-24 | [`4b97832cfb6c`](./2026-09-24.md#4b97832cfb6c23a82fc7372abd34e71a98f43517) | Use `127.0.0.1` for local login redirects (#47927) | `Import proactively` |
| 2026-09-24 | [`5d80031172f5`](./2026-09-24.md#5d80031172f57453516d2ad492665abc7a03d919) | Parameterize the thread initialization analytics test by originator (#47945) | `Import proactively` |
| 2026-09-24 | [`62197a65e52f`](./2026-09-24.md#62197a65e52fe10495a75113cdb0bf03832bef2b) | Add diagnostic reasons to MCP attribution errors (#47899) | `Import proactively` |
| 2026-09-24 | [`694d8d45bd3d`](./2026-09-24.md#694d8d45bd3d440fa4f4cbf22667ac6c17a13758) | Remove GPT-5.4 from bundled catalogs and preserve migration prompts (#47932) | `Import proactively` |
| 2026-09-24 | [`731d4a5ca842`](./2026-09-24.md#731d4a5ca842739acdeb1e594834aed5ad9c82e2) | Allow full-access Windows setup to provision through registered Core (#47922) | `Import proactively` |
| 2026-09-24 | [`83b56bc5ad94`](./2026-09-24.md#83b56bc5ad9489a47a0eb3edc3e5852f597125be) | Support client secrets for pre-registered MCP OAuth clients (#47891) | `Deferred` |
| 2026-09-24 | [`a285066737c2`](./2026-09-24.md#a285066737c28399b83a0e6b0884c777059fb701) | Preserve diagnostic logs when SQLite logging fails (#47886) | `Import proactively` |
| 2026-09-24 | [`a83ba6124944`](./2026-09-24.md#a83ba61249443a9a0f911d452a8f54e3c7ebb1b8) | Centralize typed app-server response decoding (#47755) | `Import proactively` |
| 2026-09-24 | [`a8a4f14087fd`](./2026-09-24.md#a8a4f14087fd4922cf827049f9fce22cbbc6fb7c) | Fix attestation routing during thread startup (#47912) | `Import proactively` |
| 2026-09-24 | [`b15718acfc5e`](./2026-09-24.md#b15718acfc5e79dff8cc8999eada7c80df9c85cb) | Wait for login completion in recommended plugin tests (#47828) | `Import proactively` |
| 2026-09-24 | [`f5f08c54cb7a`](./2026-09-24.md#f5f08c54cb7a774594d3579c5731ea3e87f01c48) | Route resume prewarm through the cached WebSocket session (#47709) | `Deferred` |
| 2026-09-24 | [`fbd0c0d96ca0`](./2026-09-24.md#fbd0c0d96ca0fbc13360b6fd844e45053af7d1ee) | Add direct replies for thread settings updates (#47937) | `Import proactively` |
| 2026-09-25 | [`0b430baeb43d`](./2026-09-25.md#0b430baeb43dc11aeb136fff51f8348bca7f0230) | Use request notifications in the cloud config loader lifetime test (#48130) | `Import proactively` |
| 2026-09-25 | [`44a9bfa1456e`](./2026-09-25.md#44a9bfa1456e51269fa824fa2f93b435e3e800ed) | Preserve usage limit windows in turn and compaction analytics (#48174) | `Import proactively` |
| 2026-09-25 | [`5f3180c79374`](./2026-09-25.md#5f3180c79374bd9440db733e23ead8bc9f62d64b) | Preserve model and access program pairs during compaction (#48224) | `Import proactively` |
| 2026-09-25 | [`7596ab19a661`](./2026-09-25.md#7596ab19a661598b045916810d4b22e8607bb15b) | Keep archived threads with empty previews visible in listings (#48199) | `Import proactively` |
| 2026-09-25 | [`8ae55c863db2`](./2026-09-25.md#8ae55c863db26d417e83390c5854f1144114276b) | Respect configured authentication in the thread manager sample (#48004) | `Import proactively` |
| 2026-09-25 | [`8b78f4796605`](./2026-09-25.md#8b78f4796605bda8e31329537f5fef036e405e66) | Allow Windows daemon launches with residual job membership (#48157) | `Import proactively` |
| 2026-09-25 | [`97da50c1aa41`](./2026-09-25.md#97da50c1aa4169cb6bab68d7e4c9c7e7cd2c842f) | Expose current environment selections for a running turn (#47970) | `Import proactively` |
| 2026-09-25 | [`c7e80f873f67`](./2026-09-25.md#c7e80f873f67dbef58206b9d4f3c60e9d556eb16) | Preserve executor MCP credential boundaries across reconnects (#48143) | `Import proactively` |
| 2026-09-25 | [`ca60d6fa72aa`](./2026-09-25.md#ca60d6fa72aa060eed4b841c6a7b231b57f15dbe) | Skip message-board notification previews when there are no recipients (#48072) | `Deferred` |
| 2026-09-25 | [`d7b07d45517a`](./2026-09-25.md#d7b07d45517a793acfba4cbf8de697d723cceb46) | Test same-cell permission grants and strict review in code mode (#48017) | `Import proactively` |
| 2026-09-25 | [`de9e78e3e7ca`](./2026-09-25.md#de9e78e3e7caed0fdd75d20ae617faa646dfef3c) | Add item anchors to `thread/items/list` pagination (#48151) | `User review required` |
| 2026-09-25 | [`dfdb40cd0b72`](./2026-09-25.md#dfdb40cd0b72dfba3293db5c7c441232e8ef1a60) | Prevent Windows daemon launches from retaining launcher stdio (#48272) | `Import proactively` |
| 2026-09-26 | [`12de0e395d33`](./2026-09-26.md#12de0e395d3313bc564190d983cb4f5acf0be713) | Preserve WebSocket continuations when steering a turn (#48508) | `Import proactively` |
| 2026-09-26 | [`7f6c0f9387a0`](./2026-09-26.md#7f6c0f9387a0a60f396f61cc58f6b38bc98f2473) | Refresh the TUI welcome screen for new sessions (#48513) | `Import proactively` |
| 2026-09-26 | [`a6bd19261c30`](./2026-09-26.md#a6bd19261c30ce0a0225fe90e646822d29916f11) | Fall back to embedded mode under restrictive Windows launchers (#48491) | `Import proactively` |
| 2026-09-26 | [`c9e25207073a`](./2026-09-26.md#c9e25207073a88f1a3a4a885991b9143799a084f) | Preserve tool metadata for OpenAI provider endpoint overrides (#48344) | `Import proactively` |
| 2026-09-26 | [`ca41ed337fde`](./2026-09-26.md#ca41ed337fde50dce0b9fee4cd894d93699b3175) | Show turn tips while working and after completion in the TUI (#48352) | `Deferred` |
| 2026-09-26 | [`e72da2b53805`](./2026-09-26.md#e72da2b53805894878023d01949a25a082e0a5cb) | Stabilize skill catalogs across executor availability changes (#48353) | `Import proactively` |
| 2026-09-27 | [`32f578485143`](./2026-09-27.md#32f578485143354d1c321840a3e990aabdbaca9c) | Preserve MCP app resource URIs without defaulting display mode (#48764) | `Import proactively` |
| 2026-09-27 | [`3f4668da20a5`](./2026-09-27.md#3f4668da20a56907f0a8c546c8cffa16e5e03f28) | Add history-aware prewarming for idle threads (#48812) | `Deferred` |
| 2026-09-27 | [`99f7758a5777`](./2026-09-27.md#99f7758a577740f32df3aad53502e948142758a4) | Show short turn durations in TUI completion footers (#48807) | `Deferred` |
| 2026-09-27 | [`ea647275560f`](./2026-09-27.md#ea647275560ffbcd4cf240d048eab44aa66ba9ae) | Add single-server MCP status discovery with thread connection reuse (#48783) | `Deferred` |
| 2026-09-27 | [`fdbce2080c66`](./2026-09-27.md#fdbce2080c6681c57ecd66aad001d6504251650d) | Fix Unix socket connections through long symlink paths (#48772) | `Import proactively` |
| 2026-09-28 | [`06971ec9aad0`](./2026-09-28.md#06971ec9aad037d7c32b7466031fbb8b3b407103) | Prevent message-board notifications from reopening final answers (#48982) | `Deferred` |
| 2026-09-28 | [`126071639380`](./2026-09-28.md#12607163938054d7e90fa1d2a1c63772dece6689) | Cache parsed plugin manifests across plugin workflows (#49099) | `Import proactively` |
| 2026-09-28 | [`46fdd5ef3973`](./2026-09-28.md#46fdd5ef39735f4159cdcf0ec5e85c10521494e5) | Avoid SQLite stalls from connection setup and stderr span logging (#49032) | `Import proactively` |
| 2026-09-28 | [`81d5405882d5`](./2026-09-28.md#81d5405882d5a8cfba9df39382526d54037f7078) | Allow archiving threads before their first turn (#48828) | `Deferred` |
| 2026-09-28 | [`88e9a8329d17`](./2026-09-28.md#88e9a8329d17bd141e4f3f0e7c78d07a7baa8b23) | Track app-server running turns incrementally (#49084) | `Import proactively` |
| 2026-09-28 | [`c0d26949be41`](./2026-09-28.md#c0d26949be4144c751894ae96e28d3db2208b764) | Avoid full metadata rewrites for thread timestamp updates (#48983) | `Import proactively` |
| 2026-09-29 | [`0b1b78a4f169`](./2026-09-29.md#0b1b78a4f1694e2b9e393d385c7b82ca714ca08a) | Preserve thread overrides and cloud policy validity during config reloads (#49269) | `Import proactively` |
| 2026-09-29 | [`136391a23ec4`](./2026-09-29.md#136391a23ec456be08d68b1e6695946b731c19af) | Resume unsent TUI input after reconnecting (#49105) | `Import proactively` |
| 2026-09-29 | [`18194bfd3534`](./2026-09-29.md#18194bfd3534ca567d886eac454028dafaa68b6c) | Restrict capability roots to captured turn environments (#49280) | `Import proactively` |
| 2026-09-29 | [`458f7046a522`](./2026-09-29.md#458f7046a52232f5c01dd8bc65dfbce8d5e79fa8) | Treat explicit provider model catalogs as authoritative (#49135) | `Import proactively` |
| 2026-09-29 | [`8bd5a136ffab`](./2026-09-29.md#8bd5a136ffabf5cd4868f9eb8aff298d1f25d8ef) | Add recovery guidance to content-filter retries (#49119) | `Import proactively` |
| 2026-09-29 | [`a44afa527cf1`](./2026-09-29.md#a44afa527cf11114aed19ec893491bfc8d3931bb) | Track credential storage outcomes and redact sensitive errors (#49384) | `Import proactively` |
| 2026-09-29 | [`a7660cd15490`](./2026-09-29.md#a7660cd15490875b8c22f66e577da115ed927fe3) | Trace turn phases and correlate accepted input with turns (#49262) | `Import proactively` |
| 2026-09-29 | [`c2837d8ecefa`](./2026-09-29.md#c2837d8ecefacc1c1ad59039809ea42a3a976341) | Batch metadata reads when resolving thread names (#49305) | `Import proactively` |
| 2026-09-29 | [`d5e6526362f6`](./2026-09-29.md#d5e6526362f6efa322ca091b61eda495f16ffb30) | Scan the session index backwards for batch thread name lookups (#49297) | `Import proactively` |
| 2026-09-29 | [`f53f5a6fed66`](./2026-09-29.md#f53f5a6fed662bd6f352d08066925c871a061182) | Expose original error details to turn lifecycle contributors (#49138) | `Import proactively` |
| 2026-09-30 | [`3620b2caf8af`](./2026-09-30.md#3620b2caf8afecfcedd5c5c9520c0d239c880f26) | Detect SQLite corruption during startup and preserve recovery backups (#49701) | `Import proactively` |
| 2026-09-30 | [`49be2c7ab029`](./2026-09-30.md#49be2c7ab029434dc9545cf85a7a41fd5d3beb1a) | Deliver remote message board notifications to active turns (#49686) | `Deferred` |
| 2026-09-30 | [`4f699cd64227`](./2026-09-30.md#4f699cd64227f42ff1094b0d3704ff8c3d05c96d) | Move session index I/O off async runtime threads (#49708) | `Import proactively` |
| 2026-09-30 | [`5aa92804d255`](./2026-09-30.md#5aa92804d255dcaefe169dd7febb4006a2474e32) | Batch rollout listing scans on cancellable blocking workers (#49694) | `Import proactively` |
| 2026-09-30 | [`8c3612fb638d`](./2026-09-30.md#8c3612fb638d356579c446de68421288164484dc) | Use `memrchr` to find newlines in reverse JSONL scans (#49444) | `Import proactively` |
| 2026-09-30 | [`90abcfac0266`](./2026-09-30.md#90abcfac02665ad882853a04155591cd863b2ca7) | Add experimental thread prediction protocol types (#49480) | `Deferred` |
| 2026-09-30 | [`92bc601ad605`](./2026-09-30.md#92bc601ad60542c92bf0bb1e7a2eb70b84ac49d2) | Reuse unchanged history snapshots when resuming threads (#49600) | `Import proactively` |
| 2026-09-30 | [`9ef9cb1d9fc6`](./2026-09-30.md#9ef9cb1d9fc6013f6c1994346e0ee93ad9e6f986) | Complete turn abort callbacks before emitting terminal events (#49475) | `Import proactively` |
| 2026-09-30 | [`d1c4e3c3e695`](./2026-09-30.md#d1c4e3c3e695f8f1f430f459e20c57a511f770a5) | Keep compressed rollout snippet searches on one blocking worker (#49692) | `Import proactively` |
| 2026-09-30 | [`d7b0d4aa6631`](./2026-09-30.md#d7b0d4aa663172876871b605d53283bc0120882b) | Return authoritative replay history when resuming a thread (#49599) | `Import proactively` |
| 2026-09-30 | [`d8f69ea8bc99`](./2026-09-30.md#d8f69ea8bc998a33c19a08176cfc22fd4352c592) | Preserve bootstrap discovery across authentication changes (#49432) | `Import proactively` |
| 2026-09-30 | [`de020167989f`](./2026-09-30.md#de020167989fde4937204a064ea2fedd54f3c931) | Persist explicit user goal edits in model history (#49598) | `User review required` |
| 2026-09-30 | [`ecc205edacf5`](./2026-09-30.md#ecc205edacf57be9a9e1556cc4fd1f5244a94d42) | Move thread history projection into one blocking task (#49693) | `Import proactively` |
| 2026-09-30 | [`eefe0ce1a8e3`](./2026-09-30.md#eefe0ce1a8e39788a4de7ef5bafd47e51339610b) | Bind the app-server time provider to a local variable (#49411) | `Import proactively` |
| 2026-10-01 | [`08a7031b6acb`](./2026-10-01.md#08a7031b6acb4a8e00372be0e83e21cbab0db49d) | Return world-state snapshots and context updates together (#49894) | `Import proactively` |
| 2026-10-01 | [`08b07eef1261`](./2026-10-01.md#08b07eef1261b49b1071a8a4f60ed4d529d86a0d) | Preserve daemon diagnostics and include updater logs in reports (#49843) | `Import proactively` |
| 2026-10-01 | [`0f8df3d2140a`](./2026-10-01.md#0f8df3d2140a0e120361eb4f0f806ce930b8a312) | Move shadow skill ranking off the turn preparation path (#49812) | `Import proactively` |
| 2026-10-01 | [`106772e3c68b`](./2026-10-01.md#106772e3c68b54820a02dbbe952a02458c0cae27) | Preserve background thread requests when forking in the TUI (#49783) | `Import proactively` |
| 2026-10-01 | [`1f77c0cfa634`](./2026-10-01.md#1f77c0cfa63484e1feee46de8dd5669581cd3adb) | Persist empty paginated threads when naming them (#49785) | `Import proactively` |
| 2026-10-01 | [`2635431edbd4`](./2026-10-01.md#2635431edbd43279a4da9c1199456758fbbc8ad3) | Add attachment owner lookup to the app-server (#50094) | `User review required` |
| 2026-10-01 | [`26a49b38824a`](./2026-10-01.md#26a49b38824a9c9d951e44b2e7a54e48d5cb2abb) | Add paginated reverse lookup for thread attachments (#50083) | `Deferred` |
| 2026-10-01 | [`799324821d36`](./2026-10-01.md#799324821d36a822923cee7814d3b80f7ec3cf99) | Box the resume future in the agents overview permissions test (#49822) | `Import proactively` |
| 2026-10-01 | [`7d4c7a0767cd`](./2026-10-01.md#7d4c7a0767cdf81ff76c8fa5a05c22929568e3c6) | Enable API-key model discovery by default (#49807) | `Deferred` |
| 2026-10-01 | [`a5d56d8120ab`](./2026-10-01.md#a5d56d8120abcadb41a5f76354ec522356bfe882) | Accept unknown Codex error variants in the app-server protocol (#49806) | `Import proactively` |
| 2026-10-01 | [`bc197b77bc57`](./2026-10-01.md#bc197b77bc57ff659c5d5e9aa708e1e49ed61757) | Use embedded mode for elevated Windows TUI sessions (#49855) | `Deferred` |
| 2026-10-01 | [`cc31e374fc15`](./2026-10-01.md#cc31e374fc15cd616fca19c07d55f9fa7e6d7e31) | Follow MCP tool pagination in legacy protocol mode (#50035) | `Import proactively` |
| 2026-10-01 | [`cda82a2c6853`](./2026-10-01.md#cda82a2c6853b484e0ba56d38f13902adfb2a6a1) | Recover daemon startup and updater re-exec after cwd deletion (#49819) | `Import proactively` |
| 2026-10-01 | [`d6c3b448a413`](./2026-10-01.md#d6c3b448a41311ece3255c52ec3dbfd9ff36f154) | Test session index thread-name append and removal (#49959) | `Import proactively` |
| 2026-10-01 | [`e2a7d53c9cd4`](./2026-10-01.md#e2a7d53c9cd4beeb98e3b502c2beb3a373e1324d) | Launch Windows daemon children in a dedicated working directory (#49850) | `Import proactively` |
| 2026-10-02 | [`09bced5ad950`](./2026-10-02.md#09bced5ad95069494aa5a2da49c77a50a317a9b1) | Populate thread previews from delegated task inputs (#50462) | `Import proactively` |
| 2026-10-02 | [`0df76892b132`](./2026-10-02.md#0df76892b13277eca943a6507c7964e4b0f99a5b) | Support ordered response items in world-state context updates (#50441) | `Deferred` |
| 2026-10-02 | [`1ab8c6ef28a8`](./2026-10-02.md#1ab8c6ef28a8261aaddb528dd3875124acd1c91e) | Fix thread unloading after disconnect during MCP startup (#50380) | `Import proactively` |
| 2026-10-02 | [`3629508849fe`](./2026-10-02.md#3629508849fe7679ae6362ffa1541fb0ab9fb0cb) | Preserve native USD amounts in thread usage responses (#50442) | `Import proactively` |
| 2026-10-02 | [`3c3a990da04e`](./2026-10-02.md#3c3a990da04e04a4db37f4c1fe3fc00cd934d6f5) | Measure rollout persistence size reductions (#50454) | `Import proactively` |
| 2026-10-02 | [`602d2add6e6d`](./2026-10-02.md#602d2add6e6df7e2c301c0507434b02a7463337d) | Persist additional tool definitions in rollout history (#50435) | `User review required` |
| 2026-10-02 | [`820f85cf597b`](./2026-10-02.md#820f85cf597b492adc6149759a22e71c72b01afa) | Truncate oversized MCP results in paginated thread history (#50458) | `Import proactively` |
| 2026-10-02 | [`84d5437b6e55`](./2026-10-02.md#84d5437b6e558f5159551949a35aedb84ddb89be) | Keep the subagent picker consistent with thread archive state (#50345) | `Import proactively` |
| 2026-10-02 | [`9b0a676d5dca`](./2026-10-02.md#9b0a676d5dcac661f8aeaa02edb73a1ccf730295) | Bundle rollout attachments into a gzip tar archive (#50446) | `Import proactively` |
| 2026-10-02 | [`c73775f19ef0`](./2026-10-02.md#c73775f19ef05b40dcb17da4fc5506960c52fe7c) | Remove initial messages from session configuration events (#50360) | `Import proactively` |
| 2026-10-03 | [`1a169eda1105`](./2026-10-03.md#1a169eda1105f0f0f5ff589e0e9704c7e618730c) | Distinguish daemon release identity from executable contents (#50559) | `Import proactively` |
| 2026-10-03 | [`7d5f55bdad2d`](./2026-10-03.md#7d5f55bdad2d184019831f48a7c77d35af0fa1d5) | Persist realtime transcript tails before closure without inference (#50531) | `Deferred` |
| 2026-10-04 | [`4ad985e2caaf`](./2026-10-04.md#4ad985e2caaf877b96dafd8138dae2def467e01e) | Track inference tool changes in turn analytics (#50964) | `Import proactively` |
| 2026-10-04 | [`8f82b8a31cfd`](./2026-10-04.md#8f82b8a31cfdf690479067a179eba2c761102643) | Use the managed daemon for eligible remote-control launches (#50803) | `Deferred` |
| 2026-10-04 | [`b989f795b11a`](./2026-10-04.md#b989f795b11ad4897b6acb7c1b9b5cc202782fd6) | Fall back to mklink when Windows daemon junction updates are denied (#50802) | `Import proactively` |
| 2026-10-04 | [`dde5f8c5ae9b`](./2026-10-04.md#dde5f8c5ae9b644c6882b3a48581ac317255d597) | Allow `/archive` while a turn is running (#50764) | `Deferred` |
| 2026-10-04 | [`f365d5754bb6`](./2026-10-04.md#f365d5754bb643983079676c29e8f7562399019e) | Restrict TUI MCP startup notifications to owned threads (#50781) | `Import proactively` |
| 2026-10-05 | [`36df9544a3a7`](./2026-10-05.md#36df9544a3a77508da9d3f89528854883b826187) | Clean up Unix app-server control-socket startup lock files (#51191) | `Import proactively` |
| 2026-10-05 | [`685270a56a96`](./2026-10-05.md#685270a56a96c76ae5b0853373a19d6ed5bc6fd4) | Make apply_patch preserve line endings unconditionally (#51203) | `Import proactively` |
| 2026-10-05 | [`a6c5f3a9fa8e`](./2026-10-05.md#a6c5f3a9fa8e7ccfba7d53921d2f06d06d3b9f79) | Add browser extension request headers to config requirements (#51194) | `Deferred` |
| 2026-10-05 | [`b4e3726d73a3`](./2026-10-05.md#b4e3726d73a34e7906715464cad4d801326f2483) | Test thread archiving before the first turn (#51193) | `Import proactively` |
| 2026-10-06 | [`0ef50c9f663f`](./2026-10-06.md#0ef50c9f663f65faa75339e0805d20524237f6eb) | Prevent invalidated wakeups from starting a turn (#51427) | `Import proactively` |
| 2026-10-06 | [`28cd3e25014c`](./2026-10-06.md#28cd3e25014cef77ef42a624dbb90ade3e897560) | Remove obsolete fields from persisted turn context (#51492) | `Import proactively` |
| 2026-10-06 | [`2c3565ebd80a`](./2026-10-06.md#2c3565ebd80a328e81bdb25319211fadc1ab83a3) | Include root turn IDs in host-owned Apps tool calls (#51421) | `Deferred` |
| 2026-10-06 | [`4171362e4447`](./2026-10-06.md#4171362e44473be3c4010e42abd0be727e1f230e) | Preserve review targets and scope misalignment continuation metadata (#51217) | `Import proactively` |
| 2026-10-06 | [`551bd409ebf0`](./2026-10-06.md#551bd409ebf03fc6ea0dcad0915368d8a493f012) | Preserve turn attribution across recovery and compaction (#51402) | `Import proactively` |
| 2026-10-06 | [`5679e675f49e`](./2026-10-06.md#5679e675f49e4316f6fda592314d83f9ab3c90c6) | Keep submission logs useful without exposing payloads (#51467) | `Import proactively` |
| 2026-10-06 | [`5ddd19e8a905`](./2026-10-06.md#5ddd19e8a9059bdc570526a866176964dfab96a1) | Record the multi-agent version in turn analytics (#51333) | `Import proactively` |
| 2026-10-06 | [`7ac954ea2411`](./2026-10-06.md#7ac954ea24116700632906d89715f7755cd3523e) | Enforce Fast and Ultra Fast policies independently (#51253) | `Import proactively` |
| 2026-10-06 | [`80e0b51c9e44`](./2026-10-06.md#80e0b51c9e44853471fae105032fa000c77d3e4a) | Make session lookup pagination stable and report listing failures (#51230) | `Deferred` |
| 2026-10-06 | [`822e58cc3d66`](./2026-10-06.md#822e58cc3d666166c7446c5b1ea2e52f5d09594c) | Handle partial answers in realtime routing and thread search (#51260) | `Import proactively` |
| 2026-10-06 | [`8519cde1badf`](./2026-10-06.md#8519cde1badf7cf9d6788cfe42f67ac71acd1c95) | Raise the managed app-server file descriptor limit on Unix (#51470) | `Import proactively` |
| 2026-10-06 | [`8b6bb1c77de0`](./2026-10-06.md#8b6bb1c77de0be7395c5c0431ad695a70be829bf) | Add a partial answer message phase (#51241) | `User review required` |
| 2026-10-06 | [`946bb6c58398`](./2026-10-06.md#946bb6c58398bdb4d688725dcdedf3a318457418) | Finish idle thread unloads after slow shutdown (#51420) | `Import proactively` |
| 2026-10-06 | [`a9abdeaff177`](./2026-10-06.md#a9abdeaff1773ec6e867253ae59b19128077676f) | Fix core integration tests for updated turn APIs (#51441) | `Import proactively` |
| 2026-10-06 | [`a9bc7bebfa43`](./2026-10-06.md#a9bc7bebfa439e608543f45e9fc3415895c39153) | Load rollout history on a single blocking worker (#51499) | `Import proactively` |
| 2026-10-06 | [`b0a6b8d86f45`](./2026-10-06.md#b0a6b8d86f455d3db9dd869fc1178a5fbd865e7f) | Record resolved model and reasoning effort in sub-agent activity (#51463) | `Import proactively` |
| 2026-10-06 | [`c9870d01577d`](./2026-10-06.md#c9870d01577d2ddb39b55672e66f8fe6844a6a0d) | Use PathUri for skill identity and path matching (#51482) | `Import proactively` |
| 2026-10-06 | [`dab2cd60568d`](./2026-10-06.md#dab2cd60568dedac5b226e9fafdf1523474d5cb8) | Update collaboration snapshots for full-history fork instructions (#51335) | `Import proactively` |
| 2026-10-06 | [`f1dd04959cd6`](./2026-10-06.md#f1dd04959cd61a0ea4b065f48a4925b6c0d5bd77) | Preserve turn attribution when queued mail wakes durable sleep (#51419) | `Import proactively` |
| 2026-10-07 | [`0e1520605f67`](./2026-10-07.md#0e1520605f67969b58e53762be4d675c036a981d) | Fix a discovery race in the external auth refresh test (#51710) | `Import proactively` |
| 2026-10-07 | [`1fbe15c962cc`](./2026-10-07.md#1fbe15c962cc3d8eabec36d67987c83cfc4eeec9) | Add thread ID exclusions to `thread/list` (#51595) | `User review required` |
| 2026-10-07 | [`406eb53c074a`](./2026-10-07.md#406eb53c074a29bd65aa7a8bd21d574234faf315) | Distinguish thread listing failures from exhausted history (#51602) | `Import proactively` |
| 2026-10-07 | [`a6baf8867cb4`](./2026-10-07.md#a6baf8867cb4c9726213c0884a5c8b11f0cfd8bf) | Signal abandonment of unanswered MCP elicitations (#51611) | `Import proactively` |
| 2026-10-07 | [`b38942ce7ce7`](./2026-10-07.md#b38942ce7ce73a5cee7981f1f1b6f1a3d2c5a5c0) | Preserve diagnostic reasons for MCP user verification failures (#51808) | `Import proactively` |
| 2026-10-07 | [`b7a76bce20fe`](./2026-10-07.md#b7a76bce20feac9e54d5a9b0c01ed463aa2fbf36) | Add an app-server prompt prefix compatibility test (#51857) | `Import proactively` |
| 2026-10-07 | [`f73a478c87a7`](./2026-10-07.md#f73a478c87a78a553f8009e6b26b44cc700c7f35) | Keep global app-server config independent of the launch directory (#51872) | `Import proactively` |
| 2026-10-08 | [`14c8b7771ab2`](./2026-10-08.md#14c8b7771ab2b617a131f5d8e55e98d18e56ed09) | Attribute compaction analytics to the correct voice session (#51963) | `Import proactively` |
| 2026-10-08 | [`2c3156adf4c9`](./2026-10-08.md#2c3156adf4c9454f0e01c839669255c6a32b5416) | Honor custom OTLP metrics exporters when analytics is disabled (#52278) | `Import proactively` |
| 2026-10-08 | [`2fdf047c9631`](./2026-10-08.md#2fdf047c9631c9ed01a31b62efb7891718a931a8) | Fix idle cleanup for disconnected multi-agent v2 children (#52081) | `Import proactively` |
| 2026-10-08 | [`845345b1dd4f`](./2026-10-08.md#845345b1dd4f8e2a5d5ce91363fff666e41b07fc) | Preserve subagent capabilities independently of forked history (#52241) | `Import proactively` |
| 2026-10-08 | [`89d68bc8d2cd`](./2026-10-08.md#89d68bc8d2cdcaa1695cf55e9273208093153cb2) | Add configurable default plugin activation policy (#52220) | `Deferred` |
| 2026-10-08 | [`99aa05341d15`](./2026-10-08.md#99aa05341d1564cf4ca2b463c83b607ea5d1bb3e) | Persist remote-control RPC preferences in managed daemon settings (#52304) | `Deferred` |
| 2026-10-08 | [`bc1f22455d1f`](./2026-10-08.md#bc1f22455d1f5f3fe3d4dc3ca964ec75966a6c1c) | Set a User-Agent for package archive downloads (#52177) | `Import proactively` |
| 2026-10-08 | [`d2a6dac20b3b`](./2026-10-08.md#d2a6dac20b3b76d4b95d8a699ef7bc6ee3a852d6) | Preserve agent state when unloading idle multi-agent v2 children (#52034) | `Import proactively` |
| 2026-10-08 | [`ec8251c545a1`](./2026-10-08.md#ec8251c545a1aeca985b6a0db5517cd8e1b34b35) | Add experimental prediction forks that inherit parent context (#51884) | `Deferred` |
| 2026-10-09 | [`04ad786998e9`](./2026-10-09.md#04ad786998e9ec2683ba8c387aaf311adc6f1233) | Add retention annotations to submitted tool outputs (#52671) | `Deferred` |
| 2026-10-09 | [`07757f448f2d`](./2026-10-09.md#07757f448f2d3cf475d0f328732c1c277bce8aa1) | Explain session creation failures during server shutdown (#52721) | `Import proactively` |
| 2026-10-09 | [`0ada5d8806cd`](./2026-10-09.md#0ada5d8806cdad498230d5b1b2924091e04c8feb) | Expose experimental durable thread read state in the app server (#52350) | `Deferred` |
| 2026-10-09 | [`45311351c218`](./2026-10-09.md#45311351c218f62ea81b72f16e9b6667b572059d) | Add opt-in retention for turn tool outputs (#52686) | `Deferred` |
| 2026-10-09 | [`47c063de5188`](./2026-10-09.md#47c063de518810144f6b518ad78d643ccde37e22) | Add experimental thread item lookup by ID (#52637) | `Deferred` |
| 2026-10-09 | [`751e7edd0b20`](./2026-10-09.md#751e7edd0b2012ca831c7fdcdd3400631e4cfe14) | Skip background workspace routing reads without ChatGPT auth (#52635) | `Import proactively` |
| 2026-10-09 | [`82883da25e3b`](./2026-10-09.md#82883da25e3be5883e905afa02c3a639d02a97aa) | Notify subscribers when thread read state changes (#52384) | `Import proactively` |
| 2026-10-09 | [`8ec80de0ef55`](./2026-10-09.md#8ec80de0ef55d2fb393332d75c32df9751496a58) | Count interrupted goal continuations toward the no-activity limit (#52640) | `Import proactively` |
| 2026-10-09 | [`b6e8a2e207e5`](./2026-10-09.md#b6e8a2e207e5591780061c62c50df417670f8d75) | Add durable thread read state with revision-checked updates (#52337) | `User review required` |
| 2026-10-09 | [`e45069d770c6`](./2026-10-09.md#e45069d770c6d0b8984c58a6e7cbaf221ad37599) | Add experimental app-server thread read-state updates (#52395) | `Import proactively` |
| 2026-10-10 | [`b871a1b9f6eb`](./2026-10-10.md#b871a1b9f6eb58aa21ec6c307b5b08b1ba5b2613) | Verify reset notification during code-mode host recovery (#52952) | `Import proactively` |
| 2026-10-10 | [`be9060ff9620`](./2026-10-10.md#be9060ff9620e170a05a6353ab293efb7c704248) | Stabilize the realtime steering timeline test (#52923) | `Import proactively` |
| 2026-10-10 | [`de8fab6d7adf`](./2026-10-10.md#de8fab6d7adfcef8b4ce6f02f3b5c8be4092015a) | Upgrade `rmcp` to `3.5.1` and harden lifecycle tests (#52972) | `Import proactively` |
| 2026-10-11 | [`41fd60f5618d`](./2026-10-11.md#41fd60f5618dc4e1d7d2730e956fe791006e66be) | Allow selecting and copying TUI turn tips (#53152) | `Deferred` |
| 2026-10-11 | [`8c0f70116741`](./2026-10-11.md#8c0f7011674145da7e10627769fb221c74fbf7a0) | Add connection-bound reply handles for app-server requests (#53049) | `User review required` |
| 2026-10-11 | [`95db2cd5d5f3`](./2026-10-11.md#95db2cd5d5f37f80affb9b50ce1de70ebb4513ff) | Keep remote app-server events deliverable during shutdown (#53155) | `Import proactively` |
| 2026-10-11 | [`9f3d25588b3f`](./2026-10-11.md#9f3d25588b3f0bdf452d4eda919f2c2365a89bba) | Keep agents navigation available after a thread is archived (#53201) | `Import proactively` |
| 2026-10-12 | [`6f47fcb0bea4`](./2026-10-12.md#6f47fcb0bea40155ba00a352fd833c2df2738076) | Route TUI request replies to their originating connection and thread (#53436) | `Import proactively` |
| 2026-10-12 | [`7a0a8ac9c4ab`](./2026-10-12.md#7a0a8ac9c4aba9b687e5a11b5daa6404dab09728) | Close input admission while a turn finishes recording (#53452) | `Import proactively` |
| 2026-10-12 | [`c308e127e0e8`](./2026-10-12.md#c308e127e0e8cd040f670ca0e0496ae1c7519d3f) | Suppress repeated turn diffs in `codex exec` human output (#53444) | `Import proactively` |
| 2026-10-12 | [`ddb33e34b278`](./2026-10-12.md#ddb33e34b278ca1b7d16d1e68f8ad363a2042f2e) | Remove selected protocol, apply-patch, MCP, and TUI tests (#53358) | `Import proactively` |
| 2026-10-12 | [`fc4921f487a1`](./2026-10-12.md#fc4921f487a1f07275394a0476acb3d1edf1cd65) | Add regression coverage for network grants on cold thread resume (#53424) | `Deferred` |
| 2026-10-13 | [`044e74fef3d1`](./2026-10-13.md#044e74fef3d1af2b01f4375158ac1e7ad4ae17ea) | Preserve inherited context in prediction forks (#53678) | `Import proactively` |
| 2026-10-13 | [`237b7d4323f8`](./2026-10-13.md#237b7d4323f876237c10c14592525c8bec25b74e) | Preserve the remote cwd filter in the in-app resume picker (#53728) | `Import proactively` |
| 2026-10-13 | [`6786dccbe726`](./2026-10-13.md#6786dccbe7261b75103260475bc0a04cb36c2c0a) | Forward client voice session IDs in realtime requests (#53784) | `Import proactively` |
| 2026-10-13 | [`73bbb654b3b9`](./2026-10-13.md#73bbb654b3b9796b11624914b3be908ab8a0551f) | Extract shared multi-agent v2 thread spawning (#53660) | `Import proactively` |
| 2026-10-13 | [`86966c323005`](./2026-10-13.md#86966c3230053548bf3005cd59f4d80f6afdec3b) | Materialize the thread before testing a discovered daemon restart (#53631) | `Import proactively` |
| 2026-10-13 | [`9e03960cf880`](./2026-10-13.md#9e03960cf8807c37fc987bf17b4e745b8b3339d2) | Wire background compaction into the turn lifecycle (#53800) | `Import proactively` |
| 2026-10-13 | [`a373281e9ef0`](./2026-10-13.md#a373281e9ef021ffb1b340123ce96c75bac869fb) | Retain rollout migration locks across worker cancellation (#53794) | `Import proactively` |
| 2026-10-13 | [`ca2b076fffbc`](./2026-10-13.md#ca2b076fffbc24ad5fa57258ece8a8277b326138) | Add safe diagnostics to workspace routing configuration errors (#53805) | `Import proactively` |
| 2026-10-13 | [`cb7091b18037`](./2026-10-13.md#cb7091b1803761ea882bf4017f519b9b83b83fd4) | Preserve URLs and email addresses during external agent migration (#53706) | `Import proactively` |
| 2026-10-13 | [`e762c05bd686`](./2026-10-13.md#e762c05bd686dd1eb3595aef2e14cc37c739f401) | Apply feature overrides when restarting the app-server daemon (#53751) | `Import proactively` |
| 2026-10-13 | [`efa159f7aa3f`](./2026-10-13.md#efa159f7aa3fcdb23a2f3aaf1f6f1249376bffc7) | Reuse compiled search patterns across compressed rollouts (#53732) | `Import proactively` |
| 2026-10-14 | [`0960e379a555`](./2026-10-14.md#0960e379a5555f1b6d7b97d7980039902213c26c) | Prune unused app-server daemon releases safely (#53827) | `Import proactively` |
| 2026-10-14 | [`0c02887a0c4d`](./2026-10-14.md#0c02887a0c4d09a99eb8e078aa12ff3edc9eb331) | Include compressed rollouts in doctor storage statistics (#54003) | `Deferred` |
| 2026-10-14 | [`2b21129c6b81`](./2026-10-14.md#2b21129c6b8120e63f9ac510fe99e8b02ef43292) | Add Responses API client metadata patches for turn start and steer (#54053) | `Deferred` |
| 2026-10-14 | [`2f05c0a71f59`](./2026-10-14.md#2f05c0a71f59fb2f05800846351f3e9926c3e52b) | Fix daemon updater socket binding and bootstrap readiness on Unix (#53845) | `Import proactively` |
| 2026-10-14 | [`400f2356d60c`](./2026-10-14.md#400f2356d60c4663d46817a40b4d09c5c86f35ad) | Preserve conversation history when compaction fails (#53932) | `Import proactively` |
| 2026-10-14 | [`4906ddf5b80d`](./2026-10-14.md#4906ddf5b80d4cf4e54ac1a22dc000c389835e9c) | Resume unloaded subagent threads when selected in the TUI (#53895) | `Import proactively` |
| 2026-10-14 | [`5670b1c18e86`](./2026-10-14.md#5670b1c18e861e5f89bafaff768b86ed579edd81) | Bound rollout input for memory extraction (#53940) | `Import proactively` |
| 2026-10-14 | [`57b5e89be8ef`](./2026-10-14.md#57b5e89be8efbf72bdf9f4502d1d046e369cd85d) | Extract TUI user-turn submission into a dedicated module (#54080) | `Import proactively` |
| 2026-10-14 | [`76ed3b484d62`](./2026-10-14.md#76ed3b484d62ccceb9fa512ff4dd139f421814c3) | Fix session lookup and resume after rollout compression (#54008) | `Import proactively` |
| 2026-10-14 | [`78dc2503840d`](./2026-10-14.md#78dc2503840d05f4411ae184bb5560ef327fce3c) | Stop rollout reads when the calling task is cancelled (#53947) | `Import proactively` |
| 2026-10-14 | [`7eed30ad1bea`](./2026-10-14.md#7eed30ad1beaebc20b7d848ce9ee887525a379b0) | Preserve running turn recovery across thread settings updates (#53879) | `Import proactively` |
| 2026-10-14 | [`9d3ae38e13ae`](./2026-10-14.md#9d3ae38e13aecaf018c03d5c1331c91f8c985e0f) | Limit unarchive scans to the requested thread's archived rollouts (#54060) | `Import proactively` |
| 2026-10-14 | [`9e7b2a577f4a`](./2026-10-14.md#9e7b2a577f4ab1a098194126555e82869e9bb473) | Forward turn metadata to image generation and edit requests (#54058) | `Import proactively` |
| 2026-10-14 | [`c6327eba6a5a`](./2026-10-14.md#c6327eba6a5a31304c289f2121cc74f56f9a441b) | Filter rollout filenames before checking compressed siblings (#53997) | `Import proactively` |
| 2026-10-14 | [`cec8c6fb876e`](./2026-10-14.md#cec8c6fb876e90b20250ec740e970e3f33b7396f) | Keep fork lineage ahead of paginated transcript history (#54044) | `Import proactively` |
| 2026-10-14 | [`e342df7b668a`](./2026-10-14.md#e342df7b668acea2a0417d56cf872dab837276ef) | Reuse staged file digests when preparing daemon installs (#53853) | `Import proactively` |
| 2026-10-15 | [`2a6ecbb93c42`](./2026-10-15.md#2a6ecbb93c42c3eea835126c0c332517968ab2f4) | Preserve recent speech when trimming realtime transcripts (#54257) | `Import proactively` |
| 2026-10-15 | [`341de32c33fb`](./2026-10-15.md#341de32c33fb00b88b5dd50be70610dc59dad045) | Allow root turn suspension with live descendants (#54414) | `Deferred` |
| 2026-10-15 | [`592964c0cb4d`](./2026-10-15.md#592964c0cb4d8f1b0c32d2a81f2bc602e7ae88cc) | Record local image inputs on completed image-generation items (#54420) | `Import proactively` |
| 2026-10-15 | [`7943713993c2`](./2026-10-15.md#7943713993c2220b7616b5fc71f2de0e503282e8) | Reload the model catalog after account changes (#54169) | `Import proactively` |
| 2026-10-15 | [`922510f4d757`](./2026-10-15.md#922510f4d7570b42838e015787ed53abfa8761a1) | Use the turn index for turn-scoped item pagination (#54351) | `User review required` |
| 2026-10-15 | [`97ebf2d94d62`](./2026-10-15.md#97ebf2d94d62e2ff2045f3ea17b6e11eae9a21fb) | Run managed daemon processes at background priority on macOS (#54401) | `Import proactively` |
| 2026-10-15 | [`aabf13307a04`](./2026-10-15.md#aabf13307a044706d4da0b73dfdffcfe136528a5) | Store generated image items as attachment references (#54398) | `Import proactively` |
| 2026-10-15 | [`e90f49a2692e`](./2026-10-15.md#e90f49a2692e94e8b11be0263093547e1a74abd6) | Treat explicit connections to the verified default daemon as local (#54421) | `Import proactively` |
| 2026-10-16 | [`0df4aa67781a`](./2026-10-16.md#0df4aa67781ae45a3c97befa4db48b734afe46d9) | Honor MCP tool-list changes within a turn (#54656) | `Import proactively` |
| 2026-10-16 | [`0e67502123b4`](./2026-10-16.md#0e67502123b4c6952b9fa5d672d2907fe4059f56) | Preserve MCP tool-list metadata in server status (#54686) | `Import proactively` |
| 2026-10-16 | [`2bf2bdf82c33`](./2026-10-16.md#2bf2bdf82c33f4c66ba3f0767b8e4b6a933aac40) | Bind plugin hook approval to plugin contents (#54619) | `Import proactively` |
| 2026-10-16 | [`4491caf1ad06`](./2026-10-16.md#4491caf1ad063f753381500454b599d750e50848) | Stream rollout records during thread history materialization (#54639) | `Import proactively` |
| 2026-10-16 | [`96eb400d1231`](./2026-10-16.md#96eb400d1231f0199e96f640c740096087cf3e9b) | Distinguish retryable thread-store errors from unknown write outcomes (#54664) | `Import proactively` |
| 2026-10-16 | [`a083a1e49ea4`](./2026-10-16.md#a083a1e49ea499b0dd37afa36f4d8c70d5536056) | Distinguish MCP approval cancellation reasons (#54679) | `Import proactively` |
| 2026-10-16 | [`fa46ef3af641`](./2026-10-16.md#fa46ef3af641f21d0facc69bd11e562fce6a3f9a) | Make remote-control conflict retry tests deterministic (#54647) | `Import proactively` |
| 2026-10-17 | [`7b44ac166143`](./2026-10-17.md#7b44ac1661433ae3a6f375fde873fa7123c935fe) | Gate app-server WebSocket compression behind a feature flag (#54949) | `Import proactively` |
| 2026-10-17 | [`82d958597f67`](./2026-10-17.md#82d958597f672b0f05c8f1fe3a43a9b93dc28ba5) | Gate the file-copy extension behind `multi_env_copy_file` (#54741) | `Import proactively` |
| 2026-10-17 | [`8d9f6466ff02`](./2026-10-17.md#8d9f6466ff0294785d56f55b1fe363d132fe4133) | Add negotiated zstd compression to app-server WebSockets (#54930) | `Deferred` |
| 2026-10-17 | [`d13db678cb53`](./2026-10-17.md#d13db678cb5390f77d7fa6ba762f85b787fedcec) | Stop applying macOS background priority to managed daemons (#54763) | `Import proactively` |
| 2026-10-17 | [`df8990e2614c`](./2026-10-17.md#df8990e2614c8834ae9dc0327eb4834c821e34cc) | Upgrade `zip` to 9.0.0 and update plugin archive extraction (#54761) | `Import proactively` |
| 2026-10-18 | [`70b3bb6d8814`](./2026-10-18.md#70b3bb6d8814ab3f2045238b4ae14c43da99783b) | Add observation-only model request lifecycle hooks (#55144) | `Deferred` |
| 2026-10-19 | [`1277bcfddc69`](./2026-10-19.md#1277bcfddc69800c5e3538dccf3f26bedbd80290) | Add one-shot app-server startup configuration discovery (#55300) | `Deferred` |
| 2026-10-19 | [`1569abc5dcff`](./2026-10-19.md#1569abc5dcff34a161a37c0697617b5025e1a851) | Enable enterprise-managed authorization for MCP connections (#55334) | `Deferred` |
| 2026-10-19 | [`1e4ba692b841`](./2026-10-19.md#1e4ba692b841edc8e51295a631f07d83d9f69b0d) | Preserve thread metadata before rollout files are materialized (#55380) | `Import proactively` |
| 2026-10-19 | [`47db41930433`](./2026-10-19.md#47db41930433487eb1419b3d5887bce7bd53fbe5) | Batch thread history materialization into SQLite (#55244) | `Import proactively` |
| 2026-10-19 | [`49df716ba92c`](./2026-10-19.md#49df716ba92cf49a44140241b366d914c27cfc80) | Remove the long-thread accuracy warning after compaction (#55355) | `Import proactively` |
| 2026-10-19 | [`5fe32c00015a`](./2026-10-19.md#5fe32c00015a84e6b704f6f37460bc925cd93ec0) | Preserve rollout access metadata across compression and restoration (#55252) | `Import proactively` |
| 2026-10-19 | [`61fa059d49f3`](./2026-10-19.md#61fa059d49f3386836e18fb337e71d48da8e6642) | Use thread connections for enterprise MCP status listings (#55394) | `Deferred` |
| 2026-10-19 | [`896604a12d0c`](./2026-10-19.md#896604a12d0c7edea37b59ed340dd4e282172fa5) | Record OAuth provenance and login identity for account tokens (#55397) | `Import proactively` |
| 2026-10-19 | [`8dd6099e265f`](./2026-10-19.md#8dd6099e265f14b80cb1b3fcf10c29a12bb6b23d) | Remove thread title generation spinners from the TUI (#55324) | `Import proactively` |
| 2026-10-19 | [`8f8e7a6e095d`](./2026-10-19.md#8f8e7a6e095d36b9375b5202e05fc9d1bdb95584) | Add experimental `thread/sample` requests to the app server (#55339) | `Deferred` |
| 2026-10-19 | [`9032fc76839e`](./2026-10-19.md#9032fc76839e23a0e9defa71920089187fad1b51) | Unload idle ephemeral composer prediction threads without delay (#55255) | `Import proactively` |
| 2026-10-19 | [`b1fd48c48e2c`](./2026-10-19.md#b1fd48c48e2cc227f4ea818a6030be9b86cde055) | Avoid cloning events in the app-server thread listener (#55283) | `Import proactively` |
| 2026-10-19 | [`c0ce901923c8`](./2026-10-19.md#c0ce901923c81843d100659fe4cfa5cb0cb1068e) | Share dynamic tool specifications across turn snapshots (#55261) | `Import proactively` |
| 2026-10-19 | [`cca40e57a468`](./2026-10-19.md#cca40e57a468353bfa69a56bef93389475ed0faa) | Track MCP enterprise auth cancellation across config reloads (#55287) | `Import proactively` |
| 2026-10-19 | [`d28be12577cf`](./2026-10-19.md#d28be12577cf5e267c97d3018a490d76da4d28e7) | Honor per-thread unload delays in app-server (#55270) | `Import proactively` |
| 2026-10-19 | [`d70c2c2cdf02`](./2026-10-19.md#d70c2c2cdf0226b4b5d393d30c6785850d0ce85b) | Add configured thread environment inheritance for subagents (#55327) | `Deferred` |
| 2026-10-19 | [`e05028e7b0fd`](./2026-10-19.md#e05028e7b0fdd6a483a720920594b65f0b85a872) | Prefetch sibling rollout paths when reading history pages (#55396) | `Import proactively` |
| 2026-10-19 | [`e389b2444c75`](./2026-10-19.md#e389b2444c75cbc92d40b2135ca5db300d076979) | Add configured thread environment inheritance for subagents (#55375) | `Deferred` |
| 2026-10-19 | [`ee8b4ea181dc`](./2026-10-19.md#ee8b4ea181dc5baa735c0cd9ec5009a85d2e46fb) | Import only the continuation of backgrounded Claude sessions (#55365) | `Import proactively` |
| 2026-10-19 | [`fea1d7f7d163`](./2026-10-19.md#fea1d7f7d163521a868c344527da79ba0efc146d) | Add support for Academic Researchers plans (#55387) | `Deferred` |
| 2026-10-20 | [`014489466a17`](./2026-10-20.md#014489466a1739c2c563cdfe57880c2c7c8680e3) | Add optional MCP disablement for prediction forks and samples (#55463) | `Deferred` |
| 2026-10-20 | [`2623f206bd40`](./2026-10-20.md#2623f206bd40e0a9a9b16a49b4da415585c0e45f) | Recheck account login ownership for enterprise MCP (#55680) | `Import proactively` |
| 2026-10-20 | [`48b6a9d8e0b5`](./2026-10-20.md#48b6a9d8e0b5e0d809a60eac1cb48486d6ba5107) | Reserve a sibling rollout candidate slot for the first ancestor (#55627) | `Import proactively` |
| 2026-10-20 | [`4be9cfdf30bc`](./2026-10-20.md#4be9cfdf30bcb1526099aaee4512b31c89df7500) | Improve diagnostics for compressed rollout read failures (#55545) | `Import proactively` |
| 2026-10-20 | [`69727ae17029`](./2026-10-20.md#69727ae170297c1057108ea170e6f0cf41ba993f) | Clean up failed and abandoned marketplace staging directories (#55689) | `Import proactively` |
| 2026-10-20 | [`7827cd886db8`](./2026-10-20.md#7827cd886db8d129e06bc843185d3d2b0d5cef52) | Track credential sources separately from auth owners (#55410) | `Import proactively` |
| 2026-10-20 | [`801bd4d773ab`](./2026-10-20.md#801bd4d773ab374959344c36640ddbdec270ddb7) | Search compressed rollouts concurrently (#55652) | `Import proactively` |
| 2026-10-20 | [`98f92bd0b557`](./2026-10-20.md#98f92bd0b5574bb9debed669cd48443ee5167434) | Fix in-process app-server shutdown with full queues (#55645) | `Import proactively` |
| 2026-10-20 | [`a4c9194247c6`](./2026-10-20.md#a4c9194247c6a1e79aa3e72cc6cd77238f2ed354) | Add cancellation for experimental thread samples (#55441) | `Import proactively` |
| 2026-10-20 | [`b2aa07db8ac8`](./2026-10-20.md#b2aa07db8ac892a865de888dd08726680fcf873c) | Add scoped leases for persisted account OAuth grants (#55546) | `Deferred` |
| 2026-10-20 | [`c465563aa962`](./2026-10-20.md#c465563aa9626d99420062135b187d299f904f81) | Enable app routing for remote curated Agent Plugins (#55667) | `Deferred` |
| 2026-10-20 | [`feadd9441e67`](./2026-10-20.md#feadd9441e67f2d8970ffc390212c32f3dfbba43) | Preserve enterprise MCP authorization across runtime refreshes (#55696) | `Import proactively` |
| 2026-10-21 | [`21b1f5f5344d`](./2026-10-21.md#21b1f5f5344db8adb6eb1be6a2e61f6085fdc8f8) | Bound retained turn analytics and clean up closed threads (#55847) | `Import proactively` |
| 2026-10-21 | [`3f31c0a040ca`](./2026-10-21.md#3f31c0a040ca58b86cea70a8d40ad79d481e45d2) | Resolve MCP backend identity from runtime events in rollout traces (#55710) | `Import proactively` |
| 2026-10-21 | [`430a16799d90`](./2026-10-21.md#430a16799d90e296fb27b1e94f206023902b7e56) | Preserve thread model settings across resume and revert (#55923) | `Import proactively` |
| 2026-10-21 | [`57232f111c50`](./2026-10-21.md#57232f111c50e59d748d010048c9e10832ed79d7) | Add managed Desktop workspace storage requirements (#55965) | `Deferred` |
| 2026-10-21 | [`57d4fac64adb`](./2026-10-21.md#57d4fac64adbcaed0523ca6bf8d58c4ef6875157) | Avoid redundant clones when sending app-server notifications (#55723) | `Import proactively` |
| 2026-10-21 | [`6203846fb3ef`](./2026-10-21.md#6203846fb3ef040f62eed9b35bc6629c8e88980a) | Support confidential OAuth clients for enterprise-managed MCP auth (#56001) | `Deferred` |
| 2026-10-21 | [`6bae551093d2`](./2026-10-21.md#6bae551093d24ad6539a7daba2d9a13b80b28d15) | Bound managed app-server and updater stderr logging (#55856) | `Import proactively` |
| 2026-10-21 | [`9b39316997fd`](./2026-10-21.md#9b39316997fd8bcecdb2007722f52ef858f0dbd8) | Bound remote installed-plugin inventory pagination (#55946) | `Import proactively` |
| 2026-10-21 | [`9e12b148918b`](./2026-10-21.md#9e12b148918b6189b90ccd33be68f7811adfe81f) | Refresh cloud plugin and skill catalogs after mid-turn auth changes (#55957) | `Import proactively` |
| 2026-10-21 | [`a3b685331fc7`](./2026-10-21.md#a3b685331fc700bec554b2fe2a929a65f4fe1548) | Add local diagnostic ZIP exports to the app server (#55940) | `User review required` |
| 2026-10-21 | [`c419fd01a70d`](./2026-10-21.md#c419fd01a70dd1d05356da443ac44328bda2e6f4) | Fix enterprise MCP credential selection and reauthentication (#55859) | `Import proactively` |
| 2026-10-21 | [`d138398cef7d`](./2026-10-21.md#d138398cef7d49e46fe33f1d0ddbd2ca0ed16aa4) | Expand enterprise MCP authorization lifecycle test coverage (#55883) | `Import proactively` |
| 2026-10-21 | [`d571eaf1f61d`](./2026-10-21.md#d571eaf1f61d0d93a797506335a99a9950ffe37b) | Preserve code cell response metadata in rollout traces (#55725) | `Import proactively` |
| 2026-10-21 | [`deede7201da2`](./2026-10-21.md#deede7201da27a178c39befa6b4959bb7d451645) | Keep rollout trace terminal sessions distinct when process IDs are reused (#56011) | `Import proactively` |
| 2026-10-22 | [`2326a3f74b4b`](./2026-10-22.md#2326a3f74b4b7dd79de063ef20685b6ccad7da12) | Scope model catalogs and caches by thread originator (#56284) | `Import proactively` |
| 2026-10-22 | [`342ed69a174c`](./2026-10-22.md#342ed69a174c206c1de52a59491b131e0753cfa5) | Require managed configuration from the configured server (#56055) | `Import proactively` |
| 2026-10-22 | [`40db507eeca2`](./2026-10-22.md#40db507eeca2ff778c040c325449eb72e90c0590) | Avoid redundant settings checkpoints when resuming threads (#56250) | `Import proactively` |
| 2026-10-22 | [`67fb9da4b644`](./2026-10-22.md#67fb9da4b6442c16c4032b1e074f9b1c80a43bbc) | Preserve safe enterprise MCP authentication error details (#56153) | `Import proactively` |
| 2026-10-22 | [`6f34d3d40816`](./2026-10-22.md#6f34d3d40816dac9d7a6ca94c3e3974b3f40c532) | Allow thread sampling after failed or interrupted turns (#56020) | `User review required` |
| 2026-10-22 | [`943c70d8d203`](./2026-10-22.md#943c70d8d203c61404744939eb64a1261acd5d97) | Support client Responses API metadata in `thread/sample` (#56031) | `User review required` |
| 2026-10-22 | [`a390571745da`](./2026-10-22.md#a390571745da105cfdc85443dd1d38bf78e5ea6e) | Default fresh memory profiles to v2 (#56211) | `Import proactively` |
| 2026-10-22 | [`a3d93e1e692f`](./2026-10-22.md#a3d93e1e692f1144cf8685175a5d1f6bf7294f57) | Make the code mode yield/resume test yield explicitly between phases (#56236) | `Import proactively` |
| 2026-10-22 | [`c22765480030`](./2026-10-22.md#c227654800307e23f440efaaa86131fabbd9388c) | Consolidate Rust unit tests under dedicated test modules (#56247) | `Import proactively` |
| 2026-10-22 | [`d124fe9de1c1`](./2026-10-22.md#d124fe9de1c1253d5ddc0a013d33fb32b0ff46e6) | Add negotiated network policy metadata support for MCP calls (#56278) | `User review required` |
| 2026-10-22 | [`ffda54f5b8d5`](./2026-10-22.md#ffda54f5b8d5595eb0963594de32e206f98cc77f) | Reuse current model context across paginated resume config loading (#56224) | `Import proactively` |
| 2026-10-23 | [`2130b56710d4`](./2026-10-23.md#2130b56710d45edfdc91744e53e2440d27321a16) | Make the Realtime V2 background-agent steering test deterministic (#56453) | `Import proactively` |
| 2026-10-23 | [`5b792948f941`](./2026-10-23.md#5b792948f9414288b9196ae0559f218439affe20) | Track accepted conversation history in rollout traces (#56312) | `Import proactively` |
| 2026-10-23 | [`810501d60d24`](./2026-10-23.md#810501d60d242b6195a9b92b01f37f5e95f5c97d) | Add trace instrumentation for turn startup and response streaming (#56515) | `Deferred` |
| 2026-10-23 | [`90894a780d59`](./2026-10-23.md#90894a780d59b30f0c72d35fa08c964233697de7) | Prevent memory imports from racing with consolidation (#56462) | `Import proactively` |
| 2026-10-23 | [`970111f6cb62`](./2026-10-23.md#970111f6cb62cc02fe36a82ea7d71b37a2889c97) | Allow configuring GCP project and location in provider settings (#56560) | `Deferred` |
| 2026-10-23 | [`a96fa4f22070`](./2026-10-23.md#a96fa4f220700544a1202ec7ae411e432715151f) | Allow callers to set the thread source for sampling (#56364) | `User review required` |
| 2026-10-23 | [`b559e6a39884`](./2026-10-23.md#b559e6a3988489b636e1e79672eff330bf0679dc) | Add experimental thread memory summary reads (#56465) | `Deferred` |
| 2026-10-23 | [`c483edf1775f`](./2026-10-23.md#c483edf1775f1fcc360190d0709fbabfc6a55cdd) | Include environment hooks in thread hook listings (#56553) | `Deferred` |
| 2026-10-23 | [`e103bbc06f2f`](./2026-10-23.md#e103bbc06f2f516afe47eb9b1d8d172128912023) | Apply model catalog overrides to history, notes, and context tools (#56313) | `Import proactively` |
| 2026-10-23 | [`e5ea2ed50b3a`](./2026-10-23.md#e5ea2ed50b3a28092b13a4dd64463748b060b7af) | Match rollout trace image content across request snapshots (#56342) | `Import proactively` |
| 2026-10-23 | [`e6ebdfe00b52`](./2026-10-23.md#e6ebdfe00b520e21ed50ac0362656f32dd64d2fc) | Support memory v2 in external-agent memory imports (#56414) | `Deferred` |
| 2026-10-23 | [`f970f4be6783`](./2026-10-23.md#f970f4be6783428af0d39527102917a5305c71d7) | Track resume picker pagination and selection metadata per backend (#56543) | `Import proactively` |

### B03 — Agents, forks, and message-board runtime

- Status: `Waiting for user decision`
- Members: 85
- Proactive: 44; Deferred: 39; Review: 2
- Implementation order: adapt or add tests first, manually adapt production code to Ruxex, run affected unit tests and relevant E2E tests, then record the Ruxex commit.

| Date | Upstream commit | Subject | Decision |
| --- | --- | --- | --- |
| 2026-09-19 | [`7a15548c6a5a`](./2026-09-19.md#7a15548c6a5a54c5af9dcb4b5e9cd89387cd89f0) | Move child completion routing into the agent controller (#46552) | `Import proactively` |
| 2026-09-19 | [`c026e7a622ec`](./2026-09-19.md#c026e7a622ecae45f0af1939729e7baab9d1637b) | Expose a backend-independent agent control contract (#46547) | `Import proactively` |
| 2026-09-20 | [`e29eceb75131`](./2026-09-20.md#e29eceb7513163ba1f600d0b87f6751ec9323d24) | Preserve streamed answers when subagents finish (#46867) | `Import proactively` |
| 2026-09-21 | [`3938a27c1dd6`](./2026-09-21.md#3938a27c1dd635eff02a624f6d64799639aeb714) | Route agent delivery through captured send requests (#46994) | `Import proactively` |
| 2026-09-21 | [`48f897e4ce94`](./2026-09-21.md#48f897e4ce94152818ce0563b3a8aac3a70ce9b8) | Expand agent message board tool integration coverage (#46989) | `Import proactively` |
| 2026-09-21 | [`6149914a0e59`](./2026-09-21.md#6149914a0e59363b6777080b3e953b05d592dbac) | Wire agent message boards into persistent multi-agent runtimes (#47017) | `Deferred` |
| 2026-09-21 | [`695ab0a0b427`](./2026-09-21.md#695ab0a0b427350bdef49a7e24555893807567f6) | Add collaboration tools for the agent message board (#46985) | `Deferred` |
| 2026-09-21 | [`774d64258260`](./2026-09-21.md#774d64258260f94628cc9fc55cfabd59a668571c) | Populate the agents overview from background tool responses (#46946) | `Deferred` |
| 2026-09-21 | [`7d99ee82d743`](./2026-09-21.md#7d99ee82d74325cabf485ea1e2adbd0c2625ab19) | Add agent inspection without restoring unloaded runtimes (#47010) | `Deferred` |
| 2026-09-21 | [`87bc50f9d423`](./2026-09-21.md#87bc50f9d423a62617addd5721f8bd7c54f26bc5) | Cache OS discovery for user agents and telemetry (#47038) | `Import proactively` |
| 2026-09-21 | [`8f2c15c39871`](./2026-09-21.md#8f2c15c39871c0698cd76a22ae239d36f7e8c9b8) | Implement paginated queries for the local agent message board (#46978) | `Deferred` |
| 2026-09-21 | [`abbdde95b593`](./2026-09-21.md#abbdde95b593594c4daa2677392dbfd1dd4ccb8b) | Add shared agent message-board extension interfaces (#46959) | `Deferred` |
| 2026-09-21 | [`f484f7228d0f`](./2026-09-21.md#f484f7228d0f41df061ce246f379ad0c82dd0472) | Share SQLite pools across local agent message-board handles (#47049) | `Import proactively` |
| 2026-09-21 | [`fe6455a4384d`](./2026-09-21.md#fe6455a4384d928f009e4e40459701604ea20b7e) | Add a partial index for agent message board root posts (#47063) | `Deferred` |
| 2026-09-22 | [`16bb1b9d7e21`](./2026-09-22.md#16bb1b9d7e21f73d67f20aacc3a7c37d80cde352) | Implement `AgentControl` for `LocalAgentControl` (#47271) | `Import proactively` |
| 2026-09-22 | [`2c087a0cc2ef`](./2026-09-22.md#2c087a0cc2ef1d731086a1d0cb0081bdd792cc4d) | Fix Escape navigation in read-only agent sessions (#47320) | `Import proactively` |
| 2026-09-22 | [`5121aeff13aa`](./2026-09-22.md#5121aeff13aa6615081f9467c8051289201893fc) | Overlap subagent spawn persistence and clean up cancelled children (#47348) | `Import proactively` |
| 2026-09-22 | [`5c07856e25b5`](./2026-09-22.md#5c07856e25b565c86c6266ff51edff58299fd4e6) | Separate local agent runtime state from controller handles (#47301) | `Import proactively` |
| 2026-09-22 | [`78fdcd2311bf`](./2026-09-22.md#78fdcd2311bf979d4938cd8ba671fd8f04eb87c3) | Overlap agent metadata reads when resuming a V2 root (#47369) | `Import proactively` |
| 2026-09-22 | [`89a7298aff55`](./2026-09-22.md#89a7298aff55db29f9cb2ad2430e3efe02b7a454) | Add regression coverage for message board unsubscribe persistence (#47281) | `Import proactively` |
| 2026-09-22 | [`a3275210bd25`](./2026-09-22.md#a3275210bd253f1902bf688b25bc84adaf5d144e) | Match agent status symbols to the selected row style (#47319) | `Deferred` |
| 2026-09-22 | [`ac7634b9f73e`](./2026-09-22.md#ac7634b9f73ec1bf96466be7a5869f0949d20b30) | Stream agent snapshots from local status subscriptions (#47267) | `Deferred` |
| 2026-09-22 | [`acc20df49f8c`](./2026-09-22.md#acc20df49f8c730afd50943e55b5b95cc262cd96) | Prevent V2 agent eviction from racing with queued messages (#47240) | `Import proactively` |
| 2026-09-22 | [`c6c09fe29fb0`](./2026-09-22.md#c6c09fe29fb0b926ced5146e6ed47b6ff756c1a4) | Consolidate agent operations in the `AgentControl` implementation (#47292) | `Import proactively` |
| 2026-09-22 | [`d1092865f8ec`](./2026-09-22.md#d1092865f8ec63735006211f65ee109ce91c30b9) | Allow forking conversations locked by another app in the TUI (#47185) | `Import proactively` |
| 2026-09-22 | [`d93909a939e4`](./2026-09-22.md#d93909a939e40c3c1031f5a713bacad8c0d9afe6) | Skip stored title lookups for ephemeral forks (#47342) | `Import proactively` |
| 2026-09-22 | [`f908e5a968ce`](./2026-09-22.md#f908e5a968ce03c91ce8d7008703cc27108a412a) | Route multi-agent spawning through `SpawnRequest` (#47235) | `Import proactively` |
| 2026-09-23 | [`0b23f365f8dc`](./2026-09-23.md#0b23f365f8dc24eeb6c1cf6fc2da7292621b4ba1) | Route agent lookups and V2 context through `AgentControl` (#47536) | `Deferred` |
| 2026-09-23 | [`27969c0ae9c1`](./2026-09-23.md#27969c0ae9c1ec23e359ff5133b4c36a8dd5b1ac) | Route session agent operations through `AgentControl` (#47520) | `Deferred` |
| 2026-09-23 | [`30fc6864cc13`](./2026-09-23.md#30fc6864cc1318121eca1843c217fe00ce1212f1) | Add an option to disable multi-agent v2 direct messaging (#47540) | `Deferred` |
| 2026-09-23 | [`b086ad5493f0`](./2026-09-23.md#b086ad5493f06decd4eadeedb89929466c72f179) | Make Linux descriptor cleanup fork-safe (#47654) | `Import proactively` |
| 2026-09-23 | [`dce62a50a23c`](./2026-09-23.md#dce62a50a23c4006b7d0e81e331889f05c30fc36) | Route message board agent resolution through the selected controller (#47633) | `Import proactively` |
| 2026-09-23 | [`e1f3fafad8cc`](./2026-09-23.md#e1f3fafad8ccec5983a509d0c771102d490d6ffe) | Route V2 child loading through `AgentControl` (#47629) | `Import proactively` |
| 2026-09-23 | [`e6da6fd7c7fd`](./2026-09-23.md#e6da6fd7c7fd557a7b23ddea2916a075ca47bf82) | Add opt-in OTLP logging for final agent responses (#47649) | `Deferred` |
| 2026-09-23 | [`f2654ea6c9a3`](./2026-09-23.md#f2654ea6c9a33d5e224d35e8503bca209c963608) | Support model-specific descriptions for agent message board tools (#47670) | `Deferred` |
| 2026-09-24 | [`9442a175ba52`](./2026-09-24.md#9442a175ba5224366406330117d38e46543d5224) | Prune expired in-memory message board registry entries (#47952) | `Deferred` |
| 2026-09-24 | [`b19cebecc016`](./2026-09-24.md#b19cebecc0169097bda7539af03c886e03bdeafe) | Add integration coverage for host agent controllers (#47820) | `Deferred` |
| 2026-09-24 | [`dda227891d27`](./2026-09-24.md#dda227891d27b6e0f3f244eda149a98d098969ad) | Add an in-memory agent message board for ephemeral sessions (#47946) | `Deferred` |
| 2026-09-24 | [`fdbe23da4556`](./2026-09-24.md#fdbe23da4556dcce488e5392a9f187734ba850da) | Allow hosts to provide agent controllers through ThreadManager (#47808) | `Deferred` |
| 2026-09-25 | [`466d5f583e04`](./2026-09-25.md#466d5f583e041869c97e2b37e617e282fb473277) | Add Serde support to agent message board request types (#48077) | `Import proactively` |
| 2026-09-25 | [`6d083e3bc4ca`](./2026-09-25.md#6d083e3bc4caabf547e0f492f0faa628f3210697) | Add an HTTP client for remote agent message boards (#48100) | `Deferred` |
| 2026-09-25 | [`782826663df3`](./2026-09-25.md#782826663df3e898d0c594a13f6f75cc2a498644) | Bound agent message board SSE frames before parsing (#48190) | `Deferred` |
| 2026-09-25 | [`9ef08dcf2bda`](./2026-09-25.md#9ef08dcf2bda8ba57c569bc69dd079b4643805f2) | Add multi-agent spawn latency and failure metrics (#47984) | `Import proactively` |
| 2026-09-28 | [`3749d1eff7df`](./2026-09-28.md#3749d1eff7df4ac9a0e2d32099813ba56500b930) | Preserve pending environments when spawning subagents (#49075) | `Import proactively` |
| 2026-09-29 | [`68e1a421f55e`](./2026-09-29.md#68e1a421f55ec8a0e900c5a250aa83c13727b671) | Support remote agent message boards in multi-agent sessions (#49267) | `Deferred` |
| 2026-09-30 | [`2e5fea64eefc`](./2026-09-30.md#2e5fea64eefcaa19f48458b2386011b619f69c70) | Add an opt-in model catalog to multi-agent context (#49560) | `Deferred` |
| 2026-10-01 | [`8e44ad94b051`](./2026-10-01.md#8e44ad94b0519fbca39ed452bb2e1a6b43ad76a4) | Add coordinated shutdown for local agent trees (#49814) | `Import proactively` |
| 2026-10-01 | [`960e878df4bf`](./2026-10-01.md#960e878df4bf87b9d7fa0849f14561d7cdcca942) | Preserve queued agent mail across session eviction (#50087) | `Import proactively` |
| 2026-10-01 | [`e4e33a56b073`](./2026-10-01.md#e4e33a56b073b5d9a550aaf513798480f88e4a38) | Enable dynamic tool inheritance for fresh V2 subagents (#50082) | `Deferred` |
| 2026-10-02 | [`b6903c0669df`](./2026-10-02.md#b6903c0669df044e4a089dab949fd44d54aa9355) | Preserve terminal hyperlinks in agents overview previews (#50431) | `Import proactively` |
| 2026-10-02 | [`dff5270b298b`](./2026-10-02.md#dff5270b298b7ee5fb1c47460a3d6e0d89985cad) | Clarify Git worktree choices for new and forked conversations (#50416) | `Import proactively` |
| 2026-10-05 | [`c8949e55c28a`](./2026-10-05.md#c8949e55c28a97eb2770971eb3c6a918bdefd9af) | Ignore stale refresh responses in agents overview tests (#51064) | `Import proactively` |
| 2026-10-05 | [`cacdc4661934`](./2026-10-05.md#cacdc4661934c77142738722ffde215e0db3f46e) | Honor prior cancellation before starting Codex delegates (#51063) | `Import proactively` |
| 2026-10-06 | [`24edd7b89026`](./2026-10-06.md#24edd7b89026865149d58d0a090694a2146b6d3c) | Expose detailed agent tree shutdown failure reports (#51515) | `Import proactively` |
| 2026-10-06 | [`41acdad24666`](./2026-10-06.md#41acdad2466628487911f985ece0ca2880ac6c1e) | Track sub-agent result delivery outcomes (#51331) | `Import proactively` |
| 2026-10-06 | [`c0c230e6730b`](./2026-10-06.md#c0c230e6730b3b3c9101b8aff4b9aea4027cea5b) | Add bounded diagnostics for agent spawn failures (#51355) | `Import proactively` |
| 2026-10-06 | [`c2ae67d76931`](./2026-10-06.md#c2ae67d76931fcfac287d0bacf94f2b055b57da3) | Record multi-agent wait duration by outcome (#51332) | `Import proactively` |
| 2026-10-07 | [`b17c74cfd5eb`](./2026-10-07.md#b17c74cfd5ebb39fe70ffaff78de198120278636) | Record telemetry for AGENTS.md changes made by apply_patch (#51652) | `Import proactively` |
| 2026-10-08 | [`b12ac84be9b5`](./2026-10-08.md#b12ac84be9b5835259581c68dfc7cbe63d49e5f2) | Add regression coverage for cancelled agent reloads (#52040) | `Import proactively` |
| 2026-10-08 | [`d650bd7c05a9`](./2026-10-08.md#d650bd7c05a9b67b1d397c501acfa5caa8183655) | Add optional descriptions to agent message board channels (#52183) | `Deferred` |
| 2026-10-09 | [`36ae1561b932`](./2026-10-09.md#36ae1561b9324c93d5638b45eb19fe2cc070a581) | Expose message board permissions and structured denials (#52568) | `Deferred` |
| 2026-10-09 | [`47b80571f0d6`](./2026-10-09.md#47b80571f0d6587060a580c47f6af0cfaefb0d5c) | Add template setup for local agent message boards (#52621) | `Deferred` |
| 2026-10-09 | [`7f2f4fd46e0f`](./2026-10-09.md#7f2f4fd46e0f50798fd8d3436a8e14a5386d253b) | Consolidate local message board storage helpers (#52629) | `Import proactively` |
| 2026-10-09 | [`8a0437ff34c9`](./2026-10-09.md#8a0437ff34c920af41dd83af71bc8474c452fb10) | Add an opt-in feature for subagent model context defaults (#52659) | `Deferred` |
| 2026-10-11 | [`8553558345de`](./2026-10-11.md#8553558345de65dddf41ca4f6200e5b7b84d9cf9) | Show subagent navigation shortcuts in TUI help (#53153) | `Deferred` |
| 2026-10-12 | [`6c372fbef45f`](./2026-10-12.md#6c372fbef45f730b6d8aa727b0ec8332365588db) | Allow turns when the agent message board is unavailable (#53369) | `Deferred` |
| 2026-10-13 | [`308ddf5e7c2a`](./2026-10-13.md#308ddf5e7c2aa7b8102eda043dc7656e691a96c8) | Allow hiding native tools while retaining the multi-agent V2 runtime (#53617) | `Deferred` |
| 2026-10-13 | [`6042ec37b0e6`](./2026-10-13.md#6042ec37b0e6468ce550939dc96cd17e12a381d1) | Stabilize write ordering in the shared message board scenario (#53632) | `Import proactively` |
| 2026-10-14 | [`534c70863c10`](./2026-10-14.md#534c70863c107e2bdedfa25b72bc1b035cb3208f) | Defer folder trust checks until starting a task from agents overview (#54048) | `Import proactively` |
| 2026-10-14 | [`95273b84fbff`](./2026-10-14.md#95273b84fbff40f9df7d71d50f1ca52c6489b25d) | Keep the agent dashboard browsable during task removal (#53908) | `Import proactively` |
| 2026-10-18 | [`8c076942cb70`](./2026-10-18.md#8c076942cb70fba8e5c4732c23f1faabc4aed484) | Allow agent messages to wake idle root sessions behind a feature flag (#54961) | `Deferred` |
| 2026-10-19 | [`79a19cbb3008`](./2026-10-19.md#79a19cbb3008afe2882ccc2c5d93f02cde9a0823) | Add ephemeral runtime registration and signed assertions (#55402) | `User review required` |
| 2026-10-19 | [`c3bb20602d77`](./2026-10-19.md#c3bb20602d77b0c8e62f84e9956e8086f439dfeb) | Add wire types for runtime agent registration (#55398) | `Deferred` |
| 2026-10-19 | [`fbb2f9dbec30`](./2026-10-19.md#fbb2f9dbec300d59699e52a47ecd9fa49f28b948) | Use step environment snapshots for subagent inheritance (#55331) | `Deferred` |
| 2026-10-20 | [`091008729d55`](./2026-10-20.md#091008729d5572712ac08964a9602eb71cf31428) | Share runtime authentication across requests and credential refreshes (#55413) | `Import proactively` |
| 2026-10-20 | [`305db3808cf8`](./2026-10-20.md#305db3808cf860ced585808ee4a191bb018a3b18) | Make pending injections interrupt agent waits and sleep (#55644) | `Deferred` |
| 2026-10-20 | [`66d0de9983e8`](./2026-10-20.md#66d0de9983e82b266e89ee29523324c9d08a5472) | Add lifecycle hooks for inter-agent messages (#55657) | `Deferred` |
| 2026-10-20 | [`cb00a6756945`](./2026-10-20.md#cb00a67569450aea2fdb1b1f31fc042fc0b5416f) | Add runtime assertion authentication modes and an auth provider (#55636) | `User review required` |
| 2026-10-21 | [`a47d1338c1be`](./2026-10-21.md#a47d1338c1be494948e9694a91561f66921acb17) | Extract shared message-board code into `codex-message-board` (#55970) | `Import proactively` |
| 2026-10-21 | [`e41e8821749c`](./2026-10-21.md#e41e8821749ca6904b00161c31f097628d3756a9) | Add storage namespaces for local message boards (#55971) | `Deferred` |
| 2026-10-22 | [`b337fe06b7d8`](./2026-10-22.md#b337fe06b7d8d6e5b42295e04c5e92c2b9c46567) | Defer agent mail after empty final answers (#56133) | `Import proactively` |
| 2026-10-23 | [`2839ed763bd3`](./2026-10-23.md#2839ed763bd3f404398e1f0c95bf4e2d172213c0) | Add paginated channel subscription queries to the local message board (#56495) | `Deferred` |
| 2026-10-23 | [`2f1cc5979020`](./2026-10-23.md#2f1cc5979020a1a2b08a752b1ede68d981e63009) | Add channel upserts to message boards (#56494) | `Deferred` |
| 2026-10-23 | [`8531184b49bc`](./2026-10-23.md#8531184b49bcb160175a84e140b659a83a426df7) | Make agent-message wakes respect interrupts and Plan mode (#56330) | `Import proactively` |
| 2026-10-23 | [`b719a7fb7fe0`](./2026-10-23.md#b719a7fb7fe0c43a505f30a933b63d13e2a13e80) | Add silent append support to message boards (#56461) | `Deferred` |

### B04 — MCP, plugins, skills, and tool metadata

- Status: `Waiting for user decision`
- Members: 130
- Proactive: 103; Deferred: 26; Review: 1
- Implementation order: adapt or add tests first, manually adapt production code to Ruxex, run affected unit tests and relevant E2E tests, then record the Ruxex commit.

| Date | Upstream commit | Subject | Decision |
| --- | --- | --- | --- |
| 2026-09-19 | [`184ea8452638`](./2026-09-19.md#184ea845263887846e0ead9309e20e26401878f7) | Add authenticated remote plugin measurement references (#46542) | `Deferred` |
| 2026-09-19 | [`3801fc8deaaf`](./2026-09-19.md#3801fc8deaaffab2a7a90c378a5db9b03b377f86) | Make local child process launch settings explicit (#46660) | `Import proactively` |
| 2026-09-19 | [`46d87f3b1c34`](./2026-09-19.md#46d87f3b1c34c8355d17b04d07eff54dfc40cc62) | Add shared plugin catalog discovery APIs (#46558) | `Deferred` |
| 2026-09-19 | [`4abcb8d1dacc`](./2026-09-19.md#4abcb8d1dacc1343c7695acf815b096089e5fe9e) | Restore rich tool details in persisted TUI transcripts (#46710) | `Deferred` |
| 2026-09-19 | [`6cf2ff11b391`](./2026-09-19.md#6cf2ff11b391fdd83d688b1740623ce55acf1a85) | Bind executor plugin measurements to the trusted plugin version (#46528) | `Import proactively` |
| 2026-09-19 | [`7e0463b568f8`](./2026-09-19.md#7e0463b568f8506f61b58ca46dcb1846e06ba31c) | Skip skill discovery when injecting items into initialized threads (#46546) | `Import proactively` |
| 2026-09-19 | [`a7e069dc3a88`](./2026-09-19.md#a7e069dc3a88b5bfaba6e6cc57fa0100ef5135b2) | Separate plugin catalog listing from package resolution (#46567) | `Deferred` |
| 2026-09-19 | [`ed0ee704b440`](./2026-09-19.md#ed0ee704b44059229e74a030742804916451dadf) | Recover executed tool call metadata under recorder capacity pressure (#46712) | `Import proactively` |
| 2026-09-21 | [`3483294596ea`](./2026-09-21.md#3483294596ea86edd1fdc30bf360b8951b9d7022) | Preserve delivered messages through post-tool hook failures (#47039) | `Import proactively` |
| 2026-09-21 | [`5aa4939be4ed`](./2026-09-21.md#5aa4939be4ed6ae20f843a8f1b273bf6fb8ac23f) | Support model-catalog overrides for Code Mode tool messages (#47118) | `Deferred` |
| 2026-09-21 | [`69759baefd3b`](./2026-09-21.md#69759baefd3ba349d430316a6ad91d2d0f629add) | Preserve foreign working directories in extension tool environments (#47129) | `Import proactively` |
| 2026-09-21 | [`afe4249cfe6b`](./2026-09-21.md#afe4249cfe6bdf218ce8807a670fad122d2b60ed) | Restrict Unix local MCP servers to stdio descriptors (#47094) | `Import proactively` |
| 2026-09-21 | [`dd6eff395494`](./2026-09-21.md#dd6eff3954949d074ca45ee80da89499b955bf8e) | Share stored JSON payloads across code mode cells with `Arc` (#47036) | `Import proactively` |
| 2026-09-21 | [`ebc05da3bdb7`](./2026-09-21.md#ebc05da3bdb76f25861e7cb418bd06d28cadc609) | Decouple tool restrictions from session identity with `ToolPolicy` (#46999) | `Import proactively` |
| 2026-09-22 | [`8edfca892d46`](./2026-09-22.md#8edfca892d46624a114a1e9a7095a8ed4e334232) | Reuse cloud skill catalogs across turns until invalidated (#47350) | `Deferred` |
| 2026-09-22 | [`94174e44cbc5`](./2026-09-22.md#94174e44cbc54cece45f6052328ca0c2cd7a8a2a) | Preserve MCP request trace context across transport workers (#47212) | `Import proactively` |
| 2026-09-22 | [`c11ed24c2a14`](./2026-09-22.md#c11ed24c2a1416fcae816af9127f350817371b2f) | Update code mode catalog snapshot hashes (#47149) | `Import proactively` |
| 2026-09-23 | [`0a2eb4696c26`](./2026-09-23.md#0a2eb4696c26ac33204bcd255721ab30220a4774) | Parallelize instruction refresh and tool preparation (#47458) | `Import proactively` |
| 2026-09-23 | [`8c66a5a243d1`](./2026-09-23.md#8c66a5a243d11339b56c6683b60aa5273c7ecb22) | Add model-specific prefixes to indirect tool descriptions (#47642) | `Deferred` |
| 2026-09-23 | [`a79a18cd0eb8`](./2026-09-23.md#a79a18cd0eb8c9264f0488168407aee821e2f7ef) | Expose tool dispatch and timing observations to extensions (#47662) | `Deferred` |
| 2026-09-23 | [`b2aa42e6f103`](./2026-09-23.md#b2aa42e6f10374515040a993df0d88bcb716318c) | Support model catalog overrides for MCP resource tool specs (#47677) | `Deferred` |
| 2026-09-24 | [`339e981ba71b`](./2026-09-24.md#339e981ba71b131eae3e2086a22b143f455a8cb8) | Make MCP and Code Mode input schema budgets configurable (#47936) | `Deferred` |
| 2026-09-24 | [`4021746a1568`](./2026-09-24.md#4021746a1568c764619e868c310a079d6c36079b) | Measure deferred tool namespace fragments before and after truncation (#47873) | `Import proactively` |
| 2026-09-24 | [`53446f90a566`](./2026-09-24.md#53446f90a56692dede3c8f413e8d486a6adb77b5) | Preserve more tool metadata within outgoing message budgets (#47758) | `Import proactively` |
| 2026-09-24 | [`58838dfa5ba1`](./2026-09-24.md#58838dfa5ba1d4b5383898cc2d08f270323d62d0) | Validate loaded plugins outside the cache lock (#47858) | `Import proactively` |
| 2026-09-24 | [`b62fcd3c4b41`](./2026-09-24.md#b62fcd3c4b4126e8dc619ab5334f5658c4d84657) | Allow cached catalogs to satisfy MCP startup readiness (#47935) | `Import proactively` |
| 2026-09-24 | [`e7165c2dbb14`](./2026-09-24.md#e7165c2dbb14c912d682cef254d7430a5137885a) | Preserve tool result metadata more selectively under size limits (#47714) | `Import proactively` |
| 2026-09-24 | [`e8098eb40617`](./2026-09-24.md#e8098eb406170239aeef3ce63a1ce427f2b999ae) | Separate selected plugin identities from MCP contributions (#47939) | `Import proactively` |
| 2026-09-25 | [`0f0efab8d3cb`](./2026-09-25.md#0f0efab8d3cb257d815f858841d3305df45ffd70) | Reuse MCP handlers across equivalent bindings (#47988) | `Import proactively` |
| 2026-09-25 | [`4b9e0cc77f26`](./2026-09-25.md#4b9e0cc77f264399385903b2a169b4bfd8f3d262) | Suppress console windows for local Windows MCP servers (#48238) | `Import proactively` |
| 2026-09-25 | [`75e0e0aad97a`](./2026-09-25.md#75e0e0aad97a86138b8b1ec87d9b544b4a35ecbf) | Prepare MCP calls directly from advertised tool identities (#47981) | `Import proactively` |
| 2026-09-25 | [`d0a64e91ae6b`](./2026-09-25.md#d0a64e91ae6b64550ca8bed14f1e7248f3e3d972) | Validate tool suggestion install URLs before showing the app link (#48015) | `Import proactively` |
| 2026-09-26 | [`0f9a731adeb4`](./2026-09-26.md#0f9a731adeb4ccc75a7a417945ec01ee98e997d7) | Preserve deferred tool namespace names before descriptions (#48574) | `Import proactively` |
| 2026-09-27 | [`274d41a39850`](./2026-09-27.md#274d41a39850eaaf4b60d0cfa66ea52dc917b274) | Prevent Linux ETXTBSY races in MCP stdio tests (#48724) | `Import proactively` |
| 2026-09-27 | [`41f9084b3081`](./2026-09-27.md#41f9084b30812db321a0b592def4f500d1e79cf4) | Remove WebSocket headers and tool payloads from info logs (#48686) | `Import proactively` |
| 2026-09-28 | [`011f803f3cd4`](./2026-09-28.md#011f803f3cd44802354873de259778913c56ef94) | Avoid collecting unused Git metadata in skill analytics (#49076) | `Import proactively` |
| 2026-09-28 | [`456212ca2155`](./2026-09-28.md#456212ca2155747d7e9ee752ad3ef010c39be9a5) | Use explicit histogram buckets for tool and skill context metrics (#48819) | `Import proactively` |
| 2026-09-28 | [`bfdb1571780b`](./2026-09-28.md#bfdb1571780b326ce978696de12b1d81b5e978f2) | Reuse the HTTP connection pool for remote plugin requests (#49100) | `Import proactively` |
| 2026-09-29 | [`13f580ef0908`](./2026-09-29.md#13f580ef090887bf0dfcdd366fa783ac553e273e) | Deduplicate cloud and executor skill listings before budgeting (#49127) | `Import proactively` |
| 2026-09-29 | [`4773a132c351`](./2026-09-29.md#4773a132c3512cc9dcfff1e03bcf57bbbd490cf8) | Add `/mcp login <name>` to the TUI (#49290) | `Deferred` |
| 2026-09-30 | [`bd5185820df8`](./2026-09-30.md#bd5185820df893016f51e34a7e7e19ebc8e7240b) | Compare tool call metadata in the recorder refresh test (#49408) | `No production adaptation required` |
| 2026-10-01 | [`8a400e7812b0`](./2026-10-01.md#8a400e7812b0c7488a2bf7963741dceb1aa2e693) | Cache the placeholder regex for MCP hook argument expansion (#49956) | `Import proactively` |
| 2026-10-01 | [`9561a345310f`](./2026-10-01.md#9561a345310f74d4798d678109ccb4eb9d8cd984) | Keep plugin and skill snapshots scoped to each step (#50050) | `Import proactively` |
| 2026-10-02 | [`7d3e696c4c12`](./2026-10-02.md#7d3e696c4c120d15ee51c5cfa10d74aa356c9e0f) | Preserve Windows environment variables for remote MCP servers (#50129) | `Import proactively` |
| 2026-10-02 | [`a20fe6335f96`](./2026-10-02.md#a20fe6335f960a350483d0079db2ec281c68202c) | Replace the Figma OAuth exception with Mercado Pago (#50189) | `Import proactively` |
| 2026-10-02 | [`d4eed6dca57f`](./2026-10-02.md#d4eed6dca57fc72298808aed5a4bfe43285bf98f) | Assert that only direct tool calls emit timing events (#50445) | `Import proactively` |
| 2026-10-03 | [`58ae3ba61186`](./2026-10-03.md#58ae3ba61186c39b849a6ebe60e60f4b11690373) | Keep third-party tools deferred in strict Code Mode Only (#50687) | `Import proactively` |
| 2026-10-03 | [`6326163b9abd`](./2026-10-03.md#6326163b9abd7802c0e57be4e326e5f898bbba75) | Send incremental tool catalog updates in Responses Lite (#50540) | `Deferred` |
| 2026-10-03 | [`be48ae396e57`](./2026-10-03.md#be48ae396e57621cee374423a19c2f427e1b4373) | Account for JSON overhead when truncating MCP tool results (#50470) | `Import proactively` |
| 2026-10-04 | [`335c7f8ecab2`](./2026-10-04.md#335c7f8ecab29c4462a553bbf174276d2c67a766) | Gate stable environment tool exposure behind a feature flag (#50962) | `Import proactively` |
| 2026-10-04 | [`7f892275e310`](./2026-10-04.md#7f892275e31002f0422477c6219189284560e689) | Isolate tracing in the strict third-party tool deferral test (#50977) | `Import proactively` |
| 2026-10-05 | [`28a264fbc766`](./2026-10-05.md#28a264fbc766a59f2b550b8318f88e4a4b8dffe1) | Add promise settlement streaming helpers to code mode (#51126) | `Deferred` |
| 2026-10-05 | [`402f5b6fdf28`](./2026-10-05.md#402f5b6fdf28a7a72c153dbe6fe01e252205d7d6) | Clarify incremental tool namespace updates in Responses Lite (#51119) | `Import proactively` |
| 2026-10-05 | [`93f8e79fd21e`](./2026-10-05.md#93f8e79fd21e3b96f19e1cbcae04d35eae11c788) | Distinguish namespace removals in incremental tool updates (#51202) | `Import proactively` |
| 2026-10-06 | [`044da98b4a7f`](./2026-10-06.md#044da98b4a7ff2dd0079f706280e0a6934048222) | Expose selected environments to MCP contributors (#51503) | `Deferred` |
| 2026-10-06 | [`4d1579433666`](./2026-10-06.md#4d15794336668d6098c0e36eb8e96cbbbfdb1d2d) | Add ranked tool discovery to JavaScript code mode (#51209) | `Deferred` |
| 2026-10-06 | [`a811c72969cc`](./2026-10-06.md#a811c72969ccf3edf6be205f19bc535e4fb191a7) | Measure raw MCP tool catalog sizes in telemetry (#51215) | `Import proactively` |
| 2026-10-06 | [`b3b83ace1705`](./2026-10-06.md#b3b83ace1705ec093738ed8fb9cb7e468b2c9346) | Classify executor capability root ownership independently of parsing (#51491) | `User review required` |
| 2026-10-06 | [`eb2a33aa18a0`](./2026-10-06.md#eb2a33aa18a0445325366eba31d07f62e940f77b) | Make URLs clickable in MCP elicitation prompts (#51450) | `Deferred` |
| 2026-10-07 | [`0a2a64e94200`](./2026-10-07.md#0a2a64e94200b03af3901bad4cd918030739c697) | Honor tool-description-first ordering in Code Mode (#51704) | `Import proactively` |
| 2026-10-07 | [`1199b3976277`](./2026-10-07.md#1199b3976277424fbc5035b55ca11ce69c860a69) | Wait for helper completion in HTTP header cancellation tests (#51689) | `Import proactively` |
| 2026-10-07 | [`18e28fe1b96d`](./2026-10-07.md#18e28fe1b96db7e1d6b13584bec37c41f71b8b0e) | Complete dynamic tool lifecycles on cancellation (#51556) | `Import proactively` |
| 2026-10-07 | [`3e544e843713`](./2026-10-07.md#3e544e843713d254dfc22ced37b902235a7573f8) | Reduce tool metadata cloning in the code-mode runtime (#51761) | `Import proactively` |
| 2026-10-07 | [`4d53e6ba0e71`](./2026-10-07.md#4d53e6ba0e71ed45510b5846424666a7687ffb33) | Reuse MCP handler metadata for cache comparisons (#51755) | `Import proactively` |
| 2026-10-07 | [`72c959528ee4`](./2026-10-07.md#72c959528ee4ca74abdb8575fbbc4e36960a8c47) | Enable code mode interruption by default (#51835) | `Deferred` |
| 2026-10-07 | [`746103b592a3`](./2026-10-07.md#746103b592a381704b665d2431e8997b3a7d738e) | Use readable plugin mentions without suppressing same-name skills (#51865) | `Deferred` |
| 2026-10-07 | [`95ec46861938`](./2026-10-07.md#95ec468619386ebb93506ac2091a48e5a558d25c) | Add a feature flag for Code Mode tool description ordering (#51690) | `Deferred` |
| 2026-10-07 | [`9dd798805fdf`](./2026-10-07.md#9dd798805fdf9dd0f836f35ccc9480cb0af17784) | Prevent recursive Tokio tracing in telemetry and SQLite logs (#51775) | `Import proactively` |
| 2026-10-07 | [`bd5564fec447`](./2026-10-07.md#bd5564fec447bd42b4e60d051967670d2eaa2aa8) | Expand OpenAI Docs migration guidance to the GPT-6 model family (#51768) | `Import proactively` |
| 2026-10-07 | [`cef2a6453143`](./2026-10-07.md#cef2a64531430e96efd69ceed18b8c34dc2cb98f) | Share MCP binding tool metadata with prepared calls (#51823) | `Import proactively` |
| 2026-10-07 | [`fbe97c792908`](./2026-10-07.md#fbe97c79290821e75a817adb7059adfca4404fef) | Record tool registration metrics for each sampling request (#51868) | `Import proactively` |
| 2026-10-08 | [`35a9b704447b`](./2026-10-08.md#35a9b704447b7876997ddd8541ccf0cb64c864c7) | Preserve tool call completeness when recorded arguments are truncated (#51892) | `Import proactively` |
| 2026-10-08 | [`515c291d875e`](./2026-10-08.md#515c291d875e07faa64e3fa43dc9bbffed8db31f) | Preserve large arguments in executed tool call metadata (#52268) | `Import proactively` |
| 2026-10-08 | [`70efa82b09ed`](./2026-10-08.md#70efa82b09ed8999ea038e5b7977f2ab821158e0) | Record metrics for incremental tool updates (#51893) | `Import proactively` |
| 2026-10-08 | [`c079352d8380`](./2026-10-08.md#c079352d8380316f748a9c88522589758d858c4f) | Allow plugin toggles while searching and preserve filters on refresh (#52200) | `Import proactively` |
| 2026-10-09 | [`2f761ae82100`](./2026-10-09.md#2f761ae8210082c21cdd471131fa2118680a6059) | Fix marketplace path matching for Windows junctions (#52696) | `Import proactively` |
| 2026-10-09 | [`5b3c1522f853`](./2026-10-09.md#5b3c1522f85316063e43b97fa7e13751d55bccdb) | Preserve per-session routing for gRPC code mode (#52381) | `Import proactively` |
| 2026-10-09 | [`96be88cb5393`](./2026-10-09.md#96be88cb53931c0e77c8686602b8ad673628bee8) | Reject reserved Serde JSON keys in code mode (#52681) | `Import proactively` |
| 2026-10-09 | [`c2617273706f`](./2026-10-09.md#c2617273706f142ea1cc7196a2d423893b0767e4) | Preserve code mode cancellation during output serialization (#52685) | `Import proactively` |
| 2026-10-10 | [`6c0c6759d061`](./2026-10-10.md#6c0c6759d0617d7670826b61aca0a931bbb6db1f) | Retain client-marked tool outputs across compaction (#52937) | `Import proactively` |
| 2026-10-10 | [`e4490eff65d9`](./2026-10-10.md#e4490eff65d9f4dcd986f140e81927058bfa296d) | Allow model catalogs to override incremental tool notices (#52736) | `Deferred` |
| 2026-10-11 | [`a07c8ebd57fe`](./2026-10-11.md#a07c8ebd57feedc4cf5dfe854c8e9101de54186b) | Handle broken pipes gracefully in plugin list output (#53109) | `Import proactively` |
| 2026-10-13 | [`4c1cd8bbccb3`](./2026-10-13.md#4c1cd8bbccb337c7043817c8e894058b103c9cdd) | Keep discovered Apps tools available during startup recovery (#53672) | `Import proactively` |
| 2026-10-13 | [`5d4922595ae0`](./2026-10-13.md#5d4922595ae0eb81dde3c0f0f412ad771628d205) | Prefer remote Visualize installs over the bundled plugin (#53747) | `Import proactively` |
| 2026-10-13 | [`8af93416ddf2`](./2026-10-13.md#8af93416ddf2c56575a7e9c890d5f446ec909943) | Use the bundled model catalog in the MCP startup invalidation test (#53795) | `Import proactively` |
| 2026-10-13 | [`8b507a8e4566`](./2026-10-13.md#8b507a8e4566288b4b35e81905673900198c479b) | Remove redundant Codex manual cache directory check (#53780) | `Import proactively` |
| 2026-10-13 | [`b56a579fc6f5`](./2026-10-13.md#b56a579fc6f5ca54a690c0e463e0a33f5c276d63) | Remove the product-gating TODO from `SkillPolicy` (#53781) | `Import proactively` |
| 2026-10-13 | [`c12cdc924bdd`](./2026-10-13.md#c12cdc924bdd3ce8a2e57d16b882c865d363fceb) | Remove `modelSlug` from the latest-model resolver output (#53745) | `Import proactively` |
| 2026-10-14 | [`0b73618146d5`](./2026-10-14.md#0b73618146d55a2565589bf4e2d5e40e8f298d9e) | Keep host MCP and plugin configuration local with remote environments (#53990) | `Import proactively` |
| 2026-10-14 | [`0fd85b075288`](./2026-10-14.md#0fd85b075288c4fee50f0df3e4e4f98f80c693fe) | Fix Windows environment handling for remote MCP servers (#54069) | `Import proactively` |
| 2026-10-14 | [`12fd621f6278`](./2026-10-14.md#12fd621f6278f3ca97451718578c29385a884fe4) | Preserve specific MCP failure codes in metrics and spans (#54094) | `Import proactively` |
| 2026-10-14 | [`348ef773eec1`](./2026-10-14.md#348ef773eec1a42d6b3cede64a50aeb3e0448e71) | Record per-tool incremental update and description size metrics (#54133) | `Import proactively` |
| 2026-10-14 | [`4ae6016a9388`](./2026-10-14.md#4ae6016a93880877f8f5f79a64e462c466d6b3f9) | Retire rejected MCP OAuth refresh tokens (#54079) | `Import proactively` |
| 2026-10-14 | [`cb03f28a4034`](./2026-10-14.md#cb03f28a4034b6ee901885808d27182ac586830a) | Refresh model instructions in incremental-tool context windows (#53872) | `Import proactively` |
| 2026-10-15 | [`160b2b1ef621`](./2026-10-15.md#160b2b1ef62114eeb41e41e5e3be0000d944b713) | Preserve ChatGPT authentication for Gov MCP servers (#54170) | `Import proactively` |
| 2026-10-15 | [`aff92080f2b8`](./2026-10-15.md#aff92080f2b8d5f63a63b4ada7b92054287f237c) | Add remote plugin metadata to plugin configuration (#54424) | `Deferred` |
| 2026-10-16 | [`b26b0a7a777b`](./2026-10-16.md#b26b0a7a777bd9c153da2e1e86f00ee1600476bd) | Prefer resource-specific scopes during MCP OAuth discovery (#54653) | `Import proactively` |
| 2026-10-16 | [`beae473be856`](./2026-10-16.md#beae473be8564d884a8cfe9704194a5c803d2ac1) | Allow dynamic tool calls to execute in parallel (#54654) | `Deferred` |
| 2026-10-16 | [`e7de1f444089`](./2026-10-16.md#e7de1f444089c29ac002e3df08d940f6de8a2613) | Add an opt-in tool for copying files between environments (#54674) | `Deferred` |
| 2026-10-17 | [`e16b631d08cd`](./2026-10-17.md#e16b631d08cdf82773831f9bbfb793da687d5299) | Reduce memory overhead in tool JSON schemas (#54754) | `Import proactively` |
| 2026-10-19 | [`03478b4b9217`](./2026-10-19.md#03478b4b9217a6ff66eb343a5242b644ea0ce61c) | Expose skill installations and stabilize package aliases (#55330) | `Import proactively` |
| 2026-10-19 | [`37594ab7cc6f`](./2026-10-19.md#37594ab7cc6f9ee8f5e900878cd7211190c451f0) | Handle response body failures in OpenAI docs downloads (#55379) | `Import proactively` |
| 2026-10-19 | [`4e7b9e285ba4`](./2026-10-19.md#4e7b9e285ba4ac6ce18ca452cf9e88d3d75ec857) | Recheck EMA request authority at HTTP boundaries (#55288) | `Import proactively` |
| 2026-10-19 | [`982695cdbcce`](./2026-10-19.md#982695cdbcce52eb398b7cc8ba3fcec208ac00e3) | Consolidate cached and fetched Codex manual handling (#55342) | `Import proactively` |
| 2026-10-19 | [`9b865ab31e2c`](./2026-10-19.md#9b865ab31e2c225cb82d99d1e62d7a6f670729ca) | Prepare skill catalogs before building step tools (#55357) | `Import proactively` |
| 2026-10-19 | [`cc44ffb3e200`](./2026-10-19.md#cc44ffb3e2008e25dd216d59fa7871a0b9434346) | Start dormant MCP servers for hook calls (#55263) | `Import proactively` |
| 2026-10-19 | [`f6e03d4eacf8`](./2026-10-19.md#f6e03d4eacf8907fc7a5079cb7cf4bb51e759818) | Route skill reads by alias source (#55303) | `Import proactively` |
| 2026-10-19 | [`f7a135ec6173`](./2026-10-19.md#f7a135ec6173d19c881b7b7b0ef75f2e1e6d1985) | Reconcile V2 executor plugins with cloud catalogs (#55301) | `Import proactively` |
| 2026-10-20 | [`35757adb728b`](./2026-10-20.md#35757adb728b268a1e75eabeb3e4cf9a6def5c18) | Enable MCP OAuth refresh coordination by default (#55701) | `Import proactively` |
| 2026-10-20 | [`36512a34ffe7`](./2026-10-20.md#36512a34ffe71a48bd9aab66fc81c3b909ef2dfc) | Add account credential support to EMA token exchange helpers (#55685) | `Deferred` |
| 2026-10-20 | [`6b13dffc5e4c`](./2026-10-20.md#6b13dffc5e4c334fe87f823b214ed15a4bf37a11) | Reuse Codex account grants for enterprise MCP authentication (#55698) | `Import proactively` |
| 2026-10-20 | [`75c78768d2c0`](./2026-10-20.md#75c78768d2c04da11326adb2354ff32083fe60e8) | Reclaim aged legacy marketplace staging directories (#55690) | `Import proactively` |
| 2026-10-20 | [`d4e03acb6147`](./2026-10-20.md#d4e03acb6147dc83f5e013fd91f729da72712a03) | Tag tool registration metrics with effective incremental tools mode (#55692) | `Import proactively` |
| 2026-10-21 | [`082900149cc7`](./2026-10-21.md#082900149cc7c0985e15a1d1ce0330a9e81e53b1) | Add a critical-path timing recorder for direct tool calls (#56008) | `Import proactively` |
| 2026-10-21 | [`2744e8686c3d`](./2026-10-21.md#2744e8686c3defaa96f7aff6c51a9df961c51224) | Share prepared MCP tool definitions across sessions (#55724) | `Import proactively` |
| 2026-10-21 | [`3fd08b540570`](./2026-10-21.md#3fd08b540570a5dd9c61d98560a0231f7b43891f) | Back off failed cloud skill and plugin catalog refreshes (#55968) | `Import proactively` |
| 2026-10-21 | [`aafd81a7ad3b`](./2026-10-21.md#aafd81a7ad3be2d8ddc3439056841af306cbddcc) | Avoid nested CLI startup in the MCP OAuth persistence test (#56018) | `Import proactively` |
| 2026-10-21 | [`b48277327d42`](./2026-10-21.md#b48277327d427b92ace30c1ee9d4817f8fa836b7) | Centralize cloud skill cache state under the catalog lock (#55705) | `Import proactively` |
| 2026-10-22 | [`0769c6aeeb1a`](./2026-10-22.md#0769c6aeeb1a81cec715e496d2486ece6fcef21b) | Enable executor cleanup hooks with V2 capability discovery (#56044) | `Deferred` |
| 2026-10-22 | [`2dbe6bfa4419`](./2026-10-22.md#2dbe6bfa44195e0c63b9a2ee59f46482bfd64fae) | Cancel MCP tool requests when response futures are dropped (#56132) | `Import proactively` |
| 2026-10-22 | [`61a991990fa4`](./2026-10-22.md#61a991990fa4fd0c511ccaaab3875371796be956) | Reject backslashes in HTTP(S) marketplace URL userinfo (#56263) | `Import proactively` |
| 2026-10-22 | [`61f34c35af77`](./2026-10-22.md#61f34c35af77777db4d77d0ea2aeb8547a6b4523) | Fix the MCP auth status test's IdP OAuth configuration (#56139) | `Import proactively` |
| 2026-10-22 | [`8ba7aed55f85`](./2026-10-22.md#8ba7aed55f85cdea4876c906a26275fac291d82c) | Stabilize the marketplace staging cleanup test (#56210) | `Import proactively` |
| 2026-10-22 | [`c260087efe8c`](./2026-10-22.md#c260087efe8cc6dcc0526e7018ae9e0e3fc134e3) | Avoid blocking EMA auth status checks on account refresh (#56127) | `Import proactively` |
| 2026-10-22 | [`cb2581dae8de`](./2026-10-22.md#cb2581dae8de14ca5282332ec329c335bf04195f) | Preserve MCP disable reasons across runtime overlays (#56266) | `Import proactively` |
| 2026-10-22 | [`ef33e43a159b`](./2026-10-22.md#ef33e43a159beeeb95dddf7cde05926cc9871929) | Fix MCP cancellation tests to use the metadata callback (#56292) | `Import proactively` |
| 2026-10-23 | [`b82d542aefbb`](./2026-10-23.md#b82d542aefbbcc1f46e508cdeaa4b454877d180f) | Add turn, tool, and skill attribution to traces (#56524) | `Deferred` |
| 2026-10-23 | [`bca6f238cb93`](./2026-10-23.md#bca6f238cb9340cf02b990cbd7c0692674d7b3e8) | Preserve tracing context throughout tool dispatch (#56514) | `Import proactively` |
| 2026-10-23 | [`de06fd0be4f4`](./2026-10-23.md#de06fd0be4f465bc28c3111bdadaabda5dfdebcd) | Allow installed plugin metadata to skip display enrichment (#56483) | `Import proactively` |
| 2026-10-23 | [`ea819e1f4d07`](./2026-10-23.md#ea819e1f4d0759777106e5a6fb9984752b20f972) | Record skill and plugin catalog sizes on publication (#56530) | `Import proactively` |

### B05 — Provider, Responses, and model behavior

- Status: `Waiting for user decision`
- Members: 61
- Proactive: 44; Deferred: 16; Review: 1
- Implementation order: adapt or add tests first, manually adapt production code to Ruxex, run affected unit tests and relevant E2E tests, then record the Ruxex commit.

| Date | Upstream commit | Subject | Decision |
| --- | --- | --- | --- |
| 2026-09-19 | [`49e248d4c3ad`](./2026-09-19.md#49e248d4c3ad76fc29519bd0aa9e532f448fa90f) | Add opt-in compaction after final responses (#46541) | `Deferred` |
| 2026-09-19 | [`61b08f101142`](./2026-09-19.md#61b08f101142f02e0f79e62be601a3b96a3415d0) | Disable reasoning summaries by default for new TUI threads (#46533) | `Deferred` |
| 2026-09-19 | [`de33f2d22a4a`](./2026-09-19.md#de33f2d22a4a2272c5d9fb6928c7e6e2bce7f142) | Preserve reasoning order in TUI activity groups (#46565) | `Import proactively` |
| 2026-09-19 | [`f43faf132dd2`](./2026-09-19.md#f43faf132dd279a4dd8259c49c0d833fd7754141) | Align persisted TUI activity groups and reasoning with live output (#46711) | `Import proactively` |
| 2026-09-19 | [`f5b7ca724950`](./2026-09-19.md#f5b7ca724950707fb4ba45d63ed18574e0cffb25) | Preserve request-level reasoning effort for memory and title workers (#46531) | `Import proactively` |
| 2026-09-21 | [`171dd0d39c4b`](./2026-09-21.md#171dd0d39c4b5fffc3e27c870f77c47670951f09) | Increase OpenAI file blob upload timeout from 60 seconds to 5 minutes (#47122) | `Import proactively` |
| 2026-09-21 | [`8e7f50cba9de`](./2026-09-21.md#8e7f50cba9de847b0df487af5ec86371f21ea5f5) | Preserve assistant answers in bounded TUI task responses (#47100) | `Import proactively` |
| 2026-09-21 | [`cf6754e6828d`](./2026-09-21.md#cf6754e6828db447c288d72ca188d3dbc46feb56) | Remove the `ultrafast` service tier from `gpt-5.6-sol` (#47130) | `Import proactively` |
| 2026-09-21 | [`deb0a08f240f`](./2026-09-21.md#deb0a08f240fb9b630e514417ad2256cf8e4afba) | Update model catalog descriptions and GPT-5.6-Sol priority (#47085) | `Deferred` |
| 2026-09-22 | [`2c3295306f9f`](./2026-09-22.md#2c3295306f9f70531841cf9bb674bfd386a52735) | Retry transient OpenAI file blob upload failures (#47393) | `Import proactively` |
| 2026-09-22 | [`49e95cc73f4e`](./2026-09-22.md#49e95cc73f4eb2999b1d14f863c009168df6122b) | Add GPT-6 Sol and Luna to the model catalog (#47332) | `Deferred` |
| 2026-09-23 | [`24462234b2ae`](./2026-09-23.md#24462234b2aeeb27373e17bbe226baf9c0e97d3b) | Refresh bundled model metadata and instructions (#47397) | `Import proactively` |
| 2026-09-23 | [`8de2d336b880`](./2026-09-23.md#8de2d336b880bed7e6135fedfc2e65e722a744b3) | Add extension hooks for model requests and response streams (#47679) | `Deferred` |
| 2026-09-23 | [`ae132dc50ab5`](./2026-09-23.md#ae132dc50ab5d8214a8fe599c3c1cfbb5f2ef186) | Serialize numeric custom reasoning effort as JSON numbers (#47590) | `Import proactively` |
| 2026-09-24 | [`7dae8c53d97e`](./2026-09-24.md#7dae8c53d97e61cd774e4d6bcca5243c29ca615c) | Avoid recursive TUI event dispatch for model picker selections (#47717) | `Import proactively` |
| 2026-09-24 | [`af09e926d693`](./2026-09-24.md#af09e926d69353fcaa17f17f966a11062c0e7297) | Apply the unchanged-model compaction shortcut to all session sources (#47934) | `Import proactively` |
| 2026-09-24 | [`dbb875d23cfd`](./2026-09-24.md#dbb875d23cfd1a327cf6b51d7b50dad4b4601268) | Bound tool-call observations to the outgoing Responses message budget (#47957) | `Import proactively` |
| 2026-09-24 | [`f5960fcc22b9`](./2026-09-24.md#f5960fcc22b918e658bc486e53a80031d64fd41e) | Reduce dependency coupling in shared configuration crates (#47713) | `Import proactively` |
| 2026-09-25 | [`10fd75bba78c`](./2026-09-25.md#10fd75bba78c7bbc59826ef1fa69df9c67615bd0) | Extract Responses failure parsing into a dedicated module (#48229) | `Import proactively` |
| 2026-09-25 | [`dd9a7101d9dd`](./2026-09-25.md#dd9a7101d9ddbaab68030e04d288d71bead68552) | Extract Responses header conversion into a shared module (#48200) | `Import proactively` |
| 2026-09-25 | [`f92655d07f40`](./2026-09-25.md#f92655d07f40c9999915cd9244291ad3acfc2dca) | Preempt model responses when new user input arrives (#48141) | `Deferred` |
| 2026-09-26 | [`75a714843b7f`](./2026-09-26.md#75a714843b7fc3c4e76235f31e2fd32aaf596ebf) | Preserve Markdown tables and whitespace when copying TUI responses (#48549) | `Import proactively` |
| 2026-09-28 | [`222e24b737e3`](./2026-09-28.md#222e24b737e36ec8bbb56be2a27638c428b266bf) | Render follow-up directive labels in the TUI and copied responses (#49089) | `Deferred` |
| 2026-09-29 | [`677b07eca037`](./2026-09-29.md#677b07eca037cce69dc9b82323792b22a5101c52) | Isolate realtime conversation tests from Responses prewarm connections (#49275) | `Import proactively` |
| 2026-09-29 | [`6c4924056590`](./2026-09-29.md#6c49240565907ea587a4f64ac8e408333c0f743e) | Correct provider authentication storage documentation (#49118) | `Import proactively` |
| 2026-09-29 | [`8d48f71922dc`](./2026-09-29.md#8d48f71922dcb8b8cb17a9aef63b4a610538a617) | Hide reasoning summary settings in `/status` for server connections (#49145) | `Import proactively` |
| 2026-09-29 | [`a9118edae8b7`](./2026-09-29.md#a9118edae8b77bf23b7182fd071a7b6251898bed) | Move content-filter guidance into the shared Responses retry handler (#49130) | `Import proactively` |
| 2026-09-29 | [`ff3c82c8a9d0`](./2026-09-29.md#ff3c82c8a9d0195f2cc92f6ac0492fd972d7acdd) | Preserve server reasoning summary and verbosity settings in the TUI (#49144) | `Import proactively` |
| 2026-09-30 | [`6ba4bf9e647c`](./2026-09-30.md#6ba4bf9e647cf8bc17eb9005af5ca9498bd7e654) | Honor server retry advice across Responses retries and fallback (#49441) | `Import proactively` |
| 2026-09-30 | [`ed0cc1a4ab30`](./2026-09-30.md#ed0cc1a4ab30e1e83f1214e7a368c55b83fd089d) | Serialize Responses routing fields before large inputs (#49675) | `Import proactively` |
| 2026-10-01 | [`08e2b58b07b8`](./2026-10-01.md#08e2b58b07b8a423d6577b66fb7756e980b53dbf) | Clarify service tier default save errors in the TUI (#49835) | `Import proactively` |
| 2026-10-01 | [`9d53dd91b5b5`](./2026-10-01.md#9d53dd91b5b5411dbfa8366fe7fd352ab3b1ddec) | Prevent shared instruction providers from delegating to themselves (#50093) | `Import proactively` |
| 2026-10-01 | [`b527ce47348e`](./2026-10-01.md#b527ce47348e7937730d06bc3504f17a8ac723f2) | Honor server model defaults when starting fresh TUI threads (#50013) | `Import proactively` |
| 2026-10-01 | [`f542ba68e502`](./2026-10-01.md#f542ba68e502bb6e5079e9b616c4b4d4f2fea2e0) | Clarify V2 spawn model override guidance for context catalogs (#49786) | `Import proactively` |
| 2026-10-02 | [`44dd77b71e88`](./2026-10-02.md#44dd77b71e88c78295736bffd3dc3b684c13be6d) | Honor Retry-After headers in failed Responses events (#50418) | `Import proactively` |
| 2026-10-02 | [`4f99920b8cbb`](./2026-10-02.md#4f99920b8cbb9dc694140719a0ea0f175c250a5b) | Expose the model selected for a running turn's next step (#50128) | `Deferred` |
| 2026-10-03 | [`3e238776e857`](./2026-10-03.md#3e238776e857eccd3bde6bff3026e2e9798f6524) | Show model and reasoning effort near the top of task details (#50727) | `Deferred` |
| 2026-10-04 | [`afb436df8b70`](./2026-10-04.md#afb436df8b70bb5bc57b86d9a3e829968988cd21) | Honor server reasoning summary defaults in new TUI threads (#50811) | `Import proactively` |
| 2026-10-04 | [`c2f7fe89d87c`](./2026-10-04.md#c2f7fe89d87ce853900d0b5cb1f5dc4863e44d73) | Use server model defaults for connected TUI fresh starts (#50913) | `Deferred` |
| 2026-10-06 | [`2f412e60d735`](./2026-10-06.md#2f412e60d735b47caeec6b0e79070f673a191c4f) | Remove default model labels from TUI model pickers (#51235) | `Import proactively` |
| 2026-10-08 | [`e974aad3b1a8`](./2026-10-08.md#e974aad3b1a8f144273e882c614aefe69eaef615) | Support model-specific function description prefixes (#51930) | `Import proactively` |
| 2026-10-13 | [`816fa702ec66`](./2026-10-13.md#816fa702ec66a4c81b204701a03371a1ac3511e6) | Account for token usage from incomplete responses (#53670) | `Import proactively` |
| 2026-10-13 | [`c03bbff4831a`](./2026-10-13.md#c03bbff4831a23a6dfc6f178838b73a50a9f554a) | Constrain API-key model access programs to server support (#53689) | `Import proactively` |
| 2026-10-15 | [`0884c8047af8`](./2026-10-15.md#0884c8047af8b02df31da8c622110a724c6b3cbc) | Support inline model, reasoning, and speed selection in `/model` (#54378) | `Deferred` |
| 2026-10-17 | [`ffd3429d7549`](./2026-10-17.md#ffd3429d754991183433b4d41b67551e5656ba27) | Preserve model catalog caches across header credential rotation (#54946) | `Import proactively` |
| 2026-10-19 | [`1b89beea82ae`](./2026-10-19.md#1b89beea82ae60e0dbeacb38f1ad2eae9dfed377) | Avoid cloning the full catalog for model metadata lookups (#55265) | `Import proactively` |
| 2026-10-19 | [`54f99017f23a`](./2026-10-19.md#54f99017f23aefff4a1ed3912c47f789849f0257) | Refresh rejected proxy credentials and back off failed renewals (#55299) | `Import proactively` |
| 2026-10-19 | [`57560e9d53f8`](./2026-10-19.md#57560e9d53f89c7737c7b2069ba2eb8fddbf0687) | Enable GitHub Actions OIDC authentication in the Responses API proxy (#55320) | `Deferred` |
| 2026-10-19 | [`7c073196a167`](./2026-10-19.md#7c073196a1673a0b248b2e5edfb7548d2c2290ac) | Keep only the latest adjacent reasoning effort update in requests (#55293) | `Import proactively` |
| 2026-10-19 | [`8cac68328a9d`](./2026-10-19.md#8cac68328a9d74679c19923f2aa784915ef0e20b) | Add renewable federated auth primitives to the Responses API proxy (#55298) | `Deferred` |
| 2026-10-19 | [`a91b0bf1bff4`](./2026-10-19.md#a91b0bf1bff472d5638e42fc0fedfffbb3569813) | Add GitHub Actions OIDC token acquisition to the responses API proxy (#55297) | `Deferred` |
| 2026-10-19 | [`f6e8ca3fc12e`](./2026-10-19.md#f6e8ca3fc12ea26a0b45395ca880164de9d4db32) | Encapsulate Responses API proxy authentication (#55296) | `Import proactively` |
| 2026-10-21 | [`02c6a2d3f1a0`](./2026-10-21.md#02c6a2d3f1a0b906f53770ad45ef61327509d551) | Drain WebSocket responses on mailbox preemption (#55969) | `Import proactively` |
| 2026-10-21 | [`067249a28329`](./2026-10-21.md#067249a28329feec4a097f55413894acb4a6378a) | Preserve provider model catalogs when refreshes fail (#55958) | `Import proactively` |
| 2026-10-21 | [`07cc649c658b`](./2026-10-21.md#07cc649c658be7018b6913299f12f843b24679f8) | Avoid double-counting retained reasoning in token usage (#55991) | `Import proactively` |
| 2026-10-22 | [`09ec2451bf9a`](./2026-10-22.md#09ec2451bf9a6750a19f334e9bd0fd9ebe6494b5) | Add request-scoped OAuth credentials and session lifecycle APIs (#56036) | `User review required` |
| 2026-10-22 | [`0eb98b46176f`](./2026-10-22.md#0eb98b46176fd41fc285e5e2f82218bf0ccd70f5) | Upgrade Python SDK model generation to datamodel-code-generator 0.64.0 (#56272) | `Import proactively` |
| 2026-10-22 | [`2f3fb111be71`](./2026-10-22.md#2f3fb111be71d277c80bd5b96d9019748b921f5a) | Enable reasoning effort overrides by default (#56163) | `Deferred` |
| 2026-10-22 | [`e0fdd608e8d2`](./2026-10-22.md#e0fdd608e8d255f8b4b2c8fa5eecc855efb89fb9) | Bump `@modelcontextprotocol/sdk` to `1.32.0` (#56286) | `Import proactively` |
| 2026-10-23 | [`878070b5a662`](./2026-10-23.md#878070b5a6623d52c34151b79ce618a5485f9cea) | Support model-specific writable-root permission wording (#56309) | `Deferred` |
| 2026-10-23 | [`adca78ddae79`](./2026-10-23.md#adca78ddae79568279d0731dfdebef1df16f3cde) | Remove the 1 MiB response limit for explicit model catalogs (#56525) | `Import proactively` |

### B06 — TUI and CLI behavior

- Status: `Waiting for upstream series`
- Members: 244
- Proactive: 149; Deferred: 95; Review: 0
- Implementation order: adapt or add tests first, manually adapt production code to Ruxex, run affected unit tests and relevant E2E tests, then record the Ruxex commit.

| Date | Upstream commit | Subject | Decision |
| --- | --- | --- | --- |
| 2026-09-19 | [`018687bd8cf8`](./2026-09-19.md#018687bd8cf859241ab08cbf9ccc405adbd1d74f) | Unify TUI picker styling and improve compact layouts (#46692) | `Deferred` |
| 2026-09-19 | [`132c2be239ec`](./2026-09-19.md#132c2be239ecbc1f2a9bb22d9210fefe887986a5) | Improve TUI contrast, keyboard hints, and picker layouts (#46680) | `Deferred` |
| 2026-09-19 | [`1ac4b6973bca`](./2026-09-19.md#1ac4b6973bca17804de30690cbe15f3e6324d472) | Extract the TUI transcript overlay into its own module (#46719) | `Import proactively` |
| 2026-09-19 | [`30ab0ee65b5d`](./2026-09-19.md#30ab0ee65b5dc871df14498087132c59aad3d8bb) | Cache transcript layouts across measurement and rendering (#46720) | `Import proactively` |
| 2026-09-19 | [`321c50fc2c53`](./2026-09-19.md#321c50fc2c53da542714e9b1393b1e4e457f1167) | Add compact activity rendering and preserve transcript source text (#46709) | `Deferred` |
| 2026-09-19 | [`392f56a611c4`](./2026-09-19.md#392f56a611c412b9b2eb1d9d4e59a3b42bee483a) | Cancel pending transcript Home jumps on subsequent navigation (#46722) | `Import proactively` |
| 2026-09-19 | [`5ee2bdf1e0e0`](./2026-09-19.md#5ee2bdf1e0e04064bc427ae31a49a01e8f7064df) | Unify TUI picker styling and improve compact session layouts (#46697) | `Deferred` |
| 2026-09-19 | [`79d2144ec698`](./2026-09-19.md#79d2144ec698497f710be77e5288b162c72bb11f) | Standardize TUI picker presentation and fix wrapping boundaries (#46691) | `Deferred` |
| 2026-09-19 | [`a63ae005d464`](./2026-09-19.md#a63ae005d4648b6f91d14c1bbbd8f28b4cbc7e7c) | Anchor transcript scrolling to entries and bound viewport rendering (#46721) | `Import proactively` |
| 2026-09-19 | [`c9d13e8c757c`](./2026-09-19.md#c9d13e8c757cd330a572e9a39e365866069aef07) | Unify TUI prompts with shared picker styling and layouts (#46695) | `Deferred` |
| 2026-09-19 | [`d5afb62bb3d8`](./2026-09-19.md#d5afb62bb3d8f900530b49a1268cb110380bedfd) | Notify users when asynchronous questions arrive in the TUI (#46574) | `Deferred` |
| 2026-09-19 | [`e28fa3e7a448`](./2026-09-19.md#e28fa3e7a4481e1b196dd43c57bff96809f22b60) | Unify TUI completion popup styling and preserve result identity (#46694) | `Deferred` |
| 2026-09-19 | [`fcaa035438e4`](./2026-09-19.md#fcaa035438e45eacea079a400dd9b7ad55106e81) | Preserve logical source text and styles across TUI wrapping (#46708) | `Import proactively` |
| 2026-09-20 | [`15312c326c9b`](./2026-09-20.md#15312c326c9b8d76fb74f8189f50d035c148bca4) | Select read-only permissions for temporary structured threads (#46844) | `Import proactively` |
| 2026-09-20 | [`2426ed7684c8`](./2026-09-20.md#2426ed7684c87f9a627c60b54271cfed77c979df) | Support stadium nodes in Mermaid terminal diagrams (#46856) | `Deferred` |
| 2026-09-20 | [`2db0ab7af1d9`](./2026-09-20.md#2db0ab7af1d9d7e6f0545b2778b9ac8fd789008a) | Add right-click copying for transcript and composer selections (#46895) | `Deferred` |
| 2026-09-20 | [`2f34d236f533`](./2026-09-20.md#2f34d236f533f5248263628e44d474616e545494) | Add `/tui` to choose the terminal UI mode for the next launch (#46883) | `Deferred` |
| 2026-09-20 | [`46f7d99d0e99`](./2026-09-20.md#46f7d99d0e99365323e67eb7f1f39e72d24bd1a0) | Stabilize usage dashboard navigation and add keyboard help (#46863) | `Deferred` |
| 2026-09-20 | [`4ca8e7afff48`](./2026-09-20.md#4ca8e7afff4803ca98fa4d0f94bcd7dd4ccf2646) | Make transcript list spacing uniform after streaming (#46899) | `Import proactively` |
| 2026-09-20 | [`551844b3efc4`](./2026-09-20.md#551844b3efc426c563128b0e011e36dad95a865b) | Add compact transcript browsing and prompt navigation to the TUI (#46739) | `Deferred` |
| 2026-09-20 | [`5c5308fc9a9e`](./2026-09-20.md#5c5308fc9a9ee789049d646ef11e5400384b9c6f) | Add an animated Codex logo to fresh conversations and onboarding (#46752) | `Deferred` |
| 2026-09-20 | [`5e5cadad6b90`](./2026-09-20.md#5e5cadad6b90f90a999cbc49745a91c61bc0cdbf) | Refresh analytics on identity changes and compact usage menus (#46862) | `Deferred` |
| 2026-09-20 | [`6b0a1a832564`](./2026-09-20.md#6b0a1a8325640767b46f41525561f4f16b59abfd) | Add a warning footer and dedicated warnings viewer to the TUI (#46751) | `Deferred` |
| 2026-09-20 | [`79ef00ded170`](./2026-09-20.md#79ef00ded170bb2f096430f45ba4cadba8e69611) | Honor the system clock preference in TUI completion timestamps (#46845) | `Import proactively` |
| 2026-09-20 | [`7c1e485c871d`](./2026-09-20.md#7c1e485c871dd028a359e9e12d81cb38150b68c5) | Increase the default terminal probe timeout from 100 ms to 250 ms (#46855) | `Deferred` |
| 2026-09-20 | [`8b79d8a5f537`](./2026-09-20.md#8b79d8a5f537068a9128687af2dd5c1a5fac01ac) | Count TUI launches by fullscreen transcript configuration (#46879) | `Deferred` |
| 2026-09-20 | [`9ed1b7469921`](./2026-09-20.md#9ed1b7469921e056b7cdbe2b98d1b3251d08e9f9) | Preserve startup drafts and submit them when the session is ready (#46750) | `Import proactively` |
| 2026-09-20 | [`9fdff739ea0e`](./2026-09-20.md#9fdff739ea0eca7881f180858dfd71b7bd1fa483) | Enable plain clicks on transcript links and style bare URLs (#46884) | `Deferred` |
| 2026-09-20 | [`a2de8fedcc3a`](./2026-09-20.md#a2de8fedcc3abe3cdde09b43515db820fb6b95b5) | Move fullscreen transcript control to TUI configuration (#46849) | `Deferred` |
| 2026-09-20 | [`ad538e1c2dc5`](./2026-09-20.md#ad538e1c2dc54056bff54087505762ea89584b32) | Limit the welcome logo animation to onboarding (#46859) | `Import proactively` |
| 2026-09-20 | [`b48232f975dd`](./2026-09-20.md#b48232f975dde694e34c77b3d616376124aaf160) | Enable mouse navigation in the TUI usage view (#46866) | `Deferred` |
| 2026-09-20 | [`bba48e5ce8bb`](./2026-09-20.md#bba48e5ce8bbcaee4af0bb2c71ad2936e4e24ea9) | Add independent controls for TUI visual effects (#46832) | `Deferred` |
| 2026-09-20 | [`d2d196bd6ab9`](./2026-09-20.md#d2d196bd6ab9b28f61234773b2f4464aac65f23f) | Remove analytics TUI snapshots (#46861) | `Import proactively` |
| 2026-09-20 | [`d6d43270bd79`](./2026-09-20.md#d6d43270bd791c41624fd3ee25a37e80a38d374e) | Improve usage report layouts and preserve reading positions (#46864) | `Deferred` |
| 2026-09-20 | [`d7d9f2b9b53b`](./2026-09-20.md#d7d9f2b9b53b53568b30c4dea7e095ec276ebe9c) | Add selection and copying to the transcript viewer (#46732) | `Deferred` |
| 2026-09-20 | [`d99213289f9a`](./2026-09-20.md#d99213289f9abe6daeefa50cde67634d9ea6eeed) | Honor the effective terminal color level in activity charts (#46897) | `Deferred` |
| 2026-09-20 | [`de6ae71ad431`](./2026-09-20.md#de6ae71ad4314f9445f3afd3e0cad8e13342f400) | Stabilize transcript and composer interactions in the TUI (#46749) | `Import proactively` |
| 2026-09-20 | [`df7f717c856e`](./2026-09-20.md#df7f717c856e0634b04a12f7d4fc9e8ecb3e65be) | Add transcript search and per-activity detail controls (#46734) | `Deferred` |
| 2026-09-20 | [`e39839509a2e`](./2026-09-20.md#e39839509a2e1a14ede71d92d8c565c1979b8363) | Add mouse selection and editing to the fullscreen composer (#46858) | `Deferred` |
| 2026-09-20 | [`eaff9a5f1461`](./2026-09-20.md#eaff9a5f1461207616f6e68ec35abd35c61b033b) | Hide Back to bottom when the current transcript tail is visible (#46902) | `Import proactively` |
| 2026-09-20 | [`f1feda229999`](./2026-09-20.md#f1feda2299990ab3678e715fe9d39d75a02f0491) | Integrate the interactive transcript into the alternate-screen TUI (#46733) | `Deferred` |
| 2026-09-20 | [`f3f07753ca3d`](./2026-09-20.md#f3f07753ca3db9321a8074d4448afeeb6e293245) | Extract shared text selection helpers for the TUI (#46857) | `Import proactively` |
| 2026-09-21 | [`05f39d7346e6`](./2026-09-21.md#05f39d7346e6b6e1f9c7908ebf96a4353dea87a6) | Remove the `Error` prefix from TUI `config.toml` load errors (#46929) | `Import proactively` |
| 2026-09-21 | [`57567b8d9e5a`](./2026-09-21.md#57567b8d9e5a158ddce2a8bb5f75ac455b1e9b83) | Use shared, keymap-aware tips in the fullscreen composer (#46931) | `Deferred` |
| 2026-09-21 | [`a4c8fc7a6b01`](./2026-09-21.md#a4c8fc7a6b01ffc8965761d5da32fd8b33b1fb21) | Deduplicate deprecation notices in the TUI transcript (#47065) | `Import proactively` |
| 2026-09-21 | [`a86631502d49`](./2026-09-21.md#a86631502d49274cb47208925c7d3dcece032029) | Add independent TUI rendering toggles for Mermaid, math, and tables (#46938) | `Deferred` |
| 2026-09-21 | [`bb054a2b03f1`](./2026-09-21.md#bb054a2b03f15787dcc2f9bd8ebfde713bc5b695) | Keep quota warnings visible in the TUI (#46912) | `Import proactively` |
| 2026-09-21 | [`bf91058b81c5`](./2026-09-21.md#bf91058b81c50cc8488a26b67199493b5a6d3389) | Lowercase the retry hint for conversations open in another app (#47112) | `Import proactively` |
| 2026-09-21 | [`c37b7307db57`](./2026-09-21.md#c37b7307db57ab8510af14426f739a23e8186679) | Format analytics credit usage with consistent decimal precision (#47086) | `Import proactively` |
| 2026-09-21 | [`e8cd975a55a1`](./2026-09-21.md#e8cd975a55a1e28955195f1f69a9cb81a5e4898d) | Keep TUI hints immediately above the composer (#47096) | `Deferred` |
| 2026-09-21 | [`ee2149a9b42f`](./2026-09-21.md#ee2149a9b42ff487910514d1236349efcd00a641) | Preserve transcript position when opening settings pickers (#46910) | `Import proactively` |
| 2026-09-21 | [`f07aaf920b14`](./2026-09-21.md#f07aaf920b14d7a746e435add34b8bbd37da5da6) | Highlight selected newlines in the transcript view (#47068) | `Deferred` |
| 2026-09-22 | [`00d92ad3ee7f`](./2026-09-22.md#00d92ad3ee7f29142bca187a3391b8f38064baf0) | Standardize TUI menu description wording and punctuation (#47329) | `Deferred` |
| 2026-09-22 | [`0a73d55b80af`](./2026-09-22.md#0a73d55b80afd2aa88051848bd28524d132fd01e) | Avoid redundant spacing above the transcript composer (#47331) | `Import proactively` |
| 2026-09-22 | [`13a966fc652d`](./2026-09-22.md#13a966fc652d1037c7ee93a15b2bdfa0610748ae) | Honor the system clock preference across more TUI timestamps (#47321) | `Import proactively` |
| 2026-09-22 | [`1b1b58f5a532`](./2026-09-22.md#1b1b58f5a532286959592208216376f1123f8ff0) | Move the activity inspection hint into keyboard shortcut help (#47338) | `Deferred` |
| 2026-09-22 | [`3ebfe7b30098`](./2026-09-22.md#3ebfe7b3009839dd4fee195ffe69753fe61b85a1) | Reserve spacing after recaps at the transcript tail (#47355) | `Import proactively` |
| 2026-09-22 | [`506a328dab11`](./2026-09-22.md#506a328dab110591d3c1449a15217596e7e9cd61) | Restore composer hints after clearing selection at the transcript bottom (#47384) | `Import proactively` |
| 2026-09-22 | [`5106a5234ff1`](./2026-09-22.md#5106a5234ff10f49f2403e84b1f668ac2b4a61b5) | Enable the fullscreen transcript by default (#47178) | `Deferred` |
| 2026-09-22 | [`5a11c456060c`](./2026-09-22.md#5a11c456060c2ce131de1ba195fc7729f7d7db66) | Render TUI Markdown lists with Unicode bullets and checkboxes (#47322) | `Deferred` |
| 2026-09-22 | [`639d2478cc2e`](./2026-09-22.md#639d2478cc2e16d6ca715952d2e726a3aecc024e) | Render aligned equations and optimization notation in TUI math (#47191) | `Import proactively` |
| 2026-09-22 | [`720a21ae6697`](./2026-09-22.md#720a21ae6697b0d260ab14eff859ba4a7fb42f35) | Prevent horizontal transcript selection from triggering autoscroll (#47137) | `Import proactively` |
| 2026-09-22 | [`7db578fca663`](./2026-09-22.md#7db578fca663b3e0026996e9c50104784d420744) | Preserve user prompt URL destinations across terminal widths (#47360) | `Import proactively` |
| 2026-09-22 | [`7e6c4c20fded`](./2026-09-22.md#7e6c4c20fded27103cee6516968c89e2861e3589) | Enable `/import` in remote and local daemon sessions (#47317) | `Deferred` |
| 2026-09-23 | [`39598ed17885`](./2026-09-23.md#39598ed17885970828acd42a6370131ed0190a98) | Support Shift-click to extend transcript selections (#47414) | `Deferred` |
| 2026-09-23 | [`4981037e99a3`](./2026-09-23.md#4981037e99a322ee9cf29bc8730cb64d263fa00b) | Use secondary text styling for the transcript footer shortcut hint (#47441) | `Deferred` |
| 2026-09-23 | [`4b664e0ef039`](./2026-09-23.md#4b664e0ef0397f82e68c60088a90fcd035deb796) | Respect tmux mouse settings in fullscreen and overlays (#47399) | `Deferred` |
| 2026-09-23 | [`591ffb1aaef4`](./2026-09-23.md#591ffb1aaef4fb83afac02a2dd97c0f1b05c6647) | Prefer Shift-arrow hints for queued messages and questions (#47618) | `Deferred` |
| 2026-09-23 | [`811fe5fa4311`](./2026-09-23.md#811fe5fa4311d33a4d4626509025b6c61100245e) | Recover unsent question drafts when TUI turns end (#47422) | `Import proactively` |
| 2026-09-23 | [`ab021e9f137c`](./2026-09-23.md#ab021e9f137c9daacf6ae06a495f3c7d6199fab2) | Add configurable copy-on-select for transcript selections (#47639) | `Deferred` |
| 2026-09-23 | [`cb1eea3e98eb`](./2026-09-23.md#cb1eea3e98ebc433ab5f9c12ce043e979d1902df) | Explain Mermaid rendering fallbacks in the TUI (#47572) | `Deferred` |
| 2026-09-23 | [`e0a64cf2bc45`](./2026-09-23.md#e0a64cf2bc4535eb330c22857260a7856c1e8749) | Keep unfinished link destinations out of rich streaming previews (#47589) | `Import proactively` |
| 2026-09-24 | [`12fd929f7243`](./2026-09-24.md#12fd929f724371104a1dea7c14930b078d6b4a18) | Keep the TUI responsive during clipboard copies (#47847) | `Import proactively` |
| 2026-09-24 | [`3e9d1d29370e`](./2026-09-24.md#3e9d1d29370ee7239585b9d1d576bea8263768ec) | Avoid nested read locking when rendering browser sign-in (#47861) | `Import proactively` |
| 2026-09-24 | [`40e13e6281e4`](./2026-09-24.md#40e13e6281e4725b0e81c8488ada69747be89cb7) | Move fullscreen startup tips into the transcript (#47954) | `Deferred` |
| 2026-09-24 | [`5c8fc15cc992`](./2026-09-24.md#5c8fc15cc99241d3a1a415a629841540cdff4b28) | Alias `tui.whimsy` to `tui.effects.starfield` (#47908) | `Deferred` |
| 2026-09-24 | [`6743672ad7be`](./2026-09-24.md#6743672ad7be1956e7875b22111eeeb8c05f2802) | Add TUI composer support for prompt suggestions (#47911) | `Deferred` |
| 2026-09-24 | [`90b67c96ca0e`](./2026-09-24.md#90b67c96ca0e70e0beead6085db46d8beef934fe) | Use Tokio's clock for TUI paste timing (#47881) | `Import proactively` |
| 2026-09-24 | [`a33fb9751c1e`](./2026-09-24.md#a33fb9751c1e468b0062aae63e8f06e9d75e2376) | Preserve Markdown formatting when copying transcript selections (#47896) | `Import proactively` |
| 2026-09-24 | [`b3ea9fa63279`](./2026-09-24.md#b3ea9fa632793eb3fcc33d7a9210afe400a44cff) | Include TUI client logs in diagnostic uploads (#47889) | `Import proactively` |
| 2026-09-24 | [`c098f97e5305`](./2026-09-24.md#c098f97e5305394c7f30a876c1393cf34a3eedca) | Enable terminal input approval by default (#47799) | `Deferred` |
| 2026-09-24 | [`e154f7b80789`](./2026-09-24.md#e154f7b80789b742b162cf0e44de0bc021068a20) | Generate TUI prompt suggestions after successful turns (#47929) | `Deferred` |
| 2026-09-24 | [`f6fd7f17ed2e`](./2026-09-24.md#f6fd7f17ed2ef4bf28e5b320789d764350ce4529) | Clean up temporary Codex homes after TUI tests (#47894) | `Import proactively` |
| 2026-09-25 | [`4193f1acfb1b`](./2026-09-25.md#4193f1acfb1bc3567c02bdefb5a975cbf0717c26) | Add a keep-and-next action to the warnings viewer (#48206) | `Deferred` |
| 2026-09-25 | [`58670eeac4b0`](./2026-09-25.md#58670eeac4b0bdb9fcb86929d8631c14aee0d9f6) | Isolate executable fixture copies in CLI tests on Linux (#48213) | `Import proactively` |
| 2026-09-25 | [`7703f8b35460`](./2026-09-25.md#7703f8b35460f54df49918349fdcce956fe12ac5) | Dismiss viewed TUI warnings when closing the viewer (#48205) | `Import proactively` |
| 2026-09-25 | [`b8a1fe4afc97`](./2026-09-25.md#b8a1fe4afc973a2e9cdc9d71e696a2f7a80d32e1) | Add configurable right-click paste to the fullscreen TUI (#48118) | `Deferred` |
| 2026-09-25 | [`d5355e95ef24`](./2026-09-25.md#d5355e95ef24de93ef4b3c070c7fc221a71a4e93) | Keep Codex visible during external editor handoff (#48211) | `Import proactively` |
| 2026-09-26 | [`16c21e3f36e3`](./2026-09-26.md#16c21e3f36e3ece9d505e552905c74f640af0942) | Preserve table cell source metadata through TUI rendering (#48548) | `Import proactively` |
| 2026-09-26 | [`25270df2615e`](./2026-09-26.md#25270df2615eb4da5b9d4a9a392226933fb096c5) | Keep TUI reconnect attempts running until the shared deadline (#48318) | `Import proactively` |
| 2026-09-26 | [`6a39914e379e`](./2026-09-26.md#6a39914e379e48f0bbab9806fe3cf61fdc29e07f) | Keep working tips stable during transcript interaction (#48560) | `Import proactively` |
| 2026-09-26 | [`6e1ab4d294cd`](./2026-09-26.md#6e1ab4d294cdf1d2606a91bf1acf6de0f0959c7a) | Fix Mermaid shape, relationship, and state description parsing (#48489) | `Import proactively` |
| 2026-09-26 | [`7b7d934408e6`](./2026-09-26.md#7b7d934408e6df20950a7a2501a669650279ab06) | Fix TUI math rendering for zero and big wedge expressions (#48551) | `Import proactively` |
| 2026-09-26 | [`7ed14a27db9e`](./2026-09-26.md#7ed14a27db9ea557a95ab093b449b8a9f9fe2554) | Make onboarding login links easier to copy (#48544) | `Import proactively` |
| 2026-09-26 | [`b334d5b3f2d9`](./2026-09-26.md#b334d5b3f2d9441b95286a8c2af8c2152737d977) | Default to copying transcript selections in more terminals (#48469) | `Deferred` |
| 2026-09-26 | [`d6f25a54bab1`](./2026-09-26.md#d6f25a54bab19bc95105cfe5415c41a6a9d007f2) | Fade blossom replays back to the idle state (#48547) | `Deferred` |
| 2026-09-26 | [`e8fdbf1f7c87`](./2026-09-26.md#e8fdbf1f7c8710d6c03409230c78269dbb7a9746) | Use a consistent borderless session header in the TUI (#48562) | `Deferred` |
| 2026-09-27 | [`334b6e7321d5`](./2026-09-27.md#334b6e7321d55697a2415a42e1725c814d3c0df4) | Remove follow-up prompt suggestions from the TUI (#48621) | `Deferred` |
| 2026-09-27 | [`449d42ced99c`](./2026-09-27.md#449d42ced99c23c774b79843d4a121e019fbfe65) | Stop showing previous-session summaries when switching TUI sessions (#48626) | `Import proactively` |
| 2026-09-27 | [`4c8cf3964dc0`](./2026-09-27.md#4c8cf3964dc0ae324222b77cc907940f0aedbb8a) | Fix SGR mouse reporting for Windows terminal capture (#48799) | `Import proactively` |
| 2026-09-27 | [`596f8c5c3c85`](./2026-09-27.md#596f8c5c3c855b78eae3222c5ee0aad3a91f4b43) | Match pinned transcript headers to the original prompt style (#48775) | `Import proactively` |
| 2026-09-27 | [`819cdb726dab`](./2026-09-27.md#819cdb726dab1c01f49099ce7d960a5f1383c2fb) | Render `/status` without borders and wrap long values (#48754) | `Import proactively` |
| 2026-09-27 | [`89bf86d0bd8f`](./2026-09-27.md#89bf86d0bd8f575758dc7d75657886e0ba6a9e62) | Remove the `current` badge from TUI task rows (#48776) | `Import proactively` |
| 2026-09-27 | [`8f195c93d7e7`](./2026-09-27.md#8f195c93d7e7acfef95acf273f0e49cce917e291) | Preserve blank TUI sessions when switching tasks (#48628) | `Import proactively` |
| 2026-09-27 | [`98072cf5f68a`](./2026-09-27.md#98072cf5f68a2959961d5aa0ef0c9a78d6db80d1) | Preserve empty Markdown list markers in the TUI (#48623) | `Import proactively` |
| 2026-09-27 | [`abc8f0c9a1ba`](./2026-09-27.md#abc8f0c9a1ba5754f4bf71343f50283a7a72dcff) | Use the terminal palette for ordered Markdown list markers (#48800) | `Import proactively` |
| 2026-09-27 | [`d8ec479c3489`](./2026-09-27.md#d8ec479c34895214b44c062f060d97224c191a50) | Set the provisioned macOS CLI bundle name to ChatGPT (#48643) | `Import proactively` |
| 2026-09-27 | [`d9487a29308c`](./2026-09-27.md#d9487a29308c378ff65ee049a68cb2c0d06a98e1) | Allow transcript wheel scrolling while a modal is open (#48805) | `Import proactively` |
| 2026-09-27 | [`e75b6b1e0c87`](./2026-09-27.md#e75b6b1e0c87b13b36ff552d6201755107553cce) | Show hidden output line counts in compact terminal activity (#48761) | `Deferred` |
| 2026-09-28 | [`1cc7e2361237`](./2026-09-28.md#1cc7e2361237ce7244430ee1d581c77f95c57ac8) | Show a short, neutral TUI interruption notice (#48830) | `Deferred` |
| 2026-09-28 | [`3074be908ac7`](./2026-09-28.md#3074be908ac7f270b1b4781f2f8c55662067b96a) | Copy selections within inline code as plain text (#49041) | `Import proactively` |
| 2026-09-28 | [`44fe510ce3ee`](./2026-09-28.md#44fe510ce3ee61c8ef623adcbf89b901c73ddd61) | Expand native Mermaid flowchart syntax support (#48895) | `Deferred` |
| 2026-09-28 | [`64bf4e7e629b`](./2026-09-28.md#64bf4e7e629b0243f483db3e5cecd8e451707007) | Show the Plan mode cycling hint in the fullscreen status line (#49037) | `Deferred` |
| 2026-09-28 | [`6af89155d040`](./2026-09-28.md#6af89155d040dde270374671ac5a366ac0a0c807) | Show a hand pointer over transcript links in Ghostty and Kitty (#48827) | `Deferred` |
| 2026-09-28 | [`fe50d010e203`](./2026-09-28.md#fe50d010e203a9b8dda2c7737d7d8e4a80e6ab44) | Update and centralize TUI subscription labels (#49079) | `Import proactively` |
| 2026-09-29 | [`01964952881a`](./2026-09-29.md#01964952881af1c1e68c8d02d16d373e1b33a6e2) | Add X11 primary selection and middle-click paste support (#49112) | `Deferred` |
| 2026-09-29 | [`0d7b8117d3cc`](./2026-09-29.md#0d7b8117d3cc6d9ffe21d22375d70307133e9f16) | Support projectless TUI sessions with workspace defaults (#49160) | `Deferred` |
| 2026-09-29 | [`1983c48fd186`](./2026-09-29.md#1983c48fd186614529e4fad418ac38a1ae00c6b6) | Continue Markdown blockquotes when pasting multiline text (#49357) | `Import proactively` |
| 2026-09-29 | [`94d642d8b40e`](./2026-09-29.md#94d642d8b40e45e2e544770f0d1f28df9a717f06) | Clarify credential storage wording across authentication UI and docs (#49361) | `Import proactively` |
| 2026-09-29 | [`c6c7c8d27069`](./2026-09-29.md#c6c7c8d2706981bfc2368bc896e4d07876300c3b) | Omit blockquote markers when copying quoted selections in the TUI (#49153) | `Import proactively` |
| 2026-09-29 | [`ceea67163f93`](./2026-09-29.md#ceea67163f93b4317c9352d1cd2af9fee32537b9) | Remove randomized greetings from TUI session headers (#49395) | `Import proactively` |
| 2026-09-29 | [`d6571322d270`](./2026-09-29.md#d6571322d270b77c7d044fc714272929a6b467c7) | Fix Windows path inference for opaque URIs with slash prefixes (#49388) | `Import proactively` |
| 2026-09-29 | [`d7f7fa410698`](./2026-09-29.md#d7f7fa410698fbbb75548cde553a4fbf43fc268e) | Remove the plus separator after Option symbols in TUI key hints (#49136) | `Import proactively` |
| 2026-09-30 | [`0b43721d8d1f`](./2026-09-30.md#0b43721d8d1f734658e41bffe12a6ba6c9240abd) | Copy selected file paths as plain text in the TUI (#49564) | `Deferred` |
| 2026-09-30 | [`596ae94fb00f`](./2026-09-30.md#596ae94fb00f8325d6bf835a9ebdc90baab4ad33) | Classify SQLite corruption using typed error codes (#49710) | `Import proactively` |
| 2026-09-30 | [`60947e234156`](./2026-09-30.md#60947e234156ac12bdb7fba2477d3965f166bd34) | Add account security setup reminders to the TUI (#49715) | `Import proactively` |
| 2026-09-30 | [`b588812e8cc8`](./2026-09-30.md#b588812e8cc82f88a3141ab6bcdf654cbb18361d) | Use server-authoritative permissions in the TUI (#49472) | `Import proactively` |
| 2026-09-30 | [`d1d0e8955857`](./2026-09-30.md#d1d0e89558578f622b93702686963e67948fd84c) | Infer Windows UNC paths with forward and mixed slashes (#49424) | `Do not import` |
| 2026-10-01 | [`1182834d1325`](./2026-10-01.md#1182834d1325c14878349ff8ca85d4d696a70ef7) | Remove browser-open success messages from the TUI (#49816) | `Import proactively` |
| 2026-10-01 | [`1cc9917508ab`](./2026-10-01.md#1cc9917508ab791e0bf996e6ada8d91eb3edfe33) | Keep transcript Find expansion scoped to the current match (#50045) | `Import proactively` |
| 2026-10-01 | [`2fde968de8bc`](./2026-10-01.md#2fde968de8bc9949f79d2f5565422f865b8f0fcc) | Preserve local launch permissions across TUI sessions and reconnects (#49809) | `Import proactively` |
| 2026-10-01 | [`444da310e108`](./2026-10-01.md#444da310e108da16aaeb18fd790b0ac464f08aca) | Respect approval policies in temporary structured threads (#49912) | `Import proactively` |
| 2026-10-01 | [`57a38c1bebc7`](./2026-10-01.md#57a38c1bebc77a04565b5a8ff08110f62f9f6e5d) | Preserve question context in recovered TUI answer drafts (#50052) | `Import proactively` |
| 2026-10-01 | [`606b1395654e`](./2026-10-01.md#606b1395654ec4eb95cdc59049bcfa684843ba7c) | Preserve server web-search settings in the TUI (#49799) | `Import proactively` |
| 2026-10-01 | [`6ece7bfc21bc`](./2026-10-01.md#6ece7bfc21bc28c1ae4e7d7aec8ca828b2b6e3c0) | Keep fullscreen prompts bounded and scrollable (#50109) | `Import proactively` |
| 2026-10-01 | [`7b88d09d61b8`](./2026-10-01.md#7b88d09d61b862e229832e446c87863ade890b48) | Decouple TUI startup presentation from execution configuration (#49875) | `Import proactively` |
| 2026-10-01 | [`819efd72732a`](./2026-10-01.md#819efd72732adf600c833c6b026fcfbd1e8968b7) | Preserve validation errors for invalid TUI keybindings (#49910) | `Import proactively` |
| 2026-10-01 | [`90d7f2715a44`](./2026-10-01.md#90d7f2715a4434f68b56a8ab80a6c3943f692d89) | Update elevated-launch warning snapshot to use `⌃o` for copy (#49867) | `Import proactively` |
| 2026-10-01 | [`a97fb78085f1`](./2026-10-01.md#a97fb78085f16ff5c4ab58b44584297c90006350) | Keep transcript Find results readable after closing the query (#50039) | `Import proactively` |
| 2026-10-01 | [`cca440697f2e`](./2026-10-01.md#cca440697f2e6aebf671f71fbc43c0d6d0dc2628) | Flush expired paste bursts before handling Enter (#49810) | `Import proactively` |
| 2026-10-01 | [`e58b49332915`](./2026-10-01.md#e58b49332915a9712835589619d428d20be4d37b) | Consolidate chat composer footer logic in `footer_state` (#50105) | `Import proactively` |
| 2026-10-01 | [`ec0cfa5da8e9`](./2026-10-01.md#ec0cfa5da8e9fac464b94cb36396904546eee4e8) | Allow cleanup of replay-only side conversations with missing threads (#49800) | `Deferred` |
| 2026-10-02 | [`14a477ea8971`](./2026-10-02.md#14a477ea89712071944244022e8a10142845456e) | Bound tmux option probes to one second (#50219) | `Import proactively` |
| 2026-10-02 | [`3f97f2b3bd89`](./2026-10-02.md#3f97f2b3bd890744240374a832137b4b7d0fe1f4) | Restore the account email in `/status` after account updates (#50199) | `Import proactively` |
| 2026-10-02 | [`4dd51f4a5f20`](./2026-10-02.md#4dd51f4a5f2037f8aa322fe7807315e6530a4ec8) | Make transcript mouse scroll speed configurable (#50209) | `Deferred` |
| 2026-10-02 | [`50b4c5e58c1b`](./2026-10-02.md#50b4c5e58c1b89f3d1b74155cc1cc1716c2be208) | Use printable ASCII terminal titles under GNU Screen (#50375) | `Deferred` |
| 2026-10-02 | [`91168365a579`](./2026-10-02.md#91168365a579420f7932f48bfb1ad15d810da3af) | Render ANSI styles in TUI hook system messages (#50359) | `Import proactively` |
| 2026-10-02 | [`a75987455a28`](./2026-10-02.md#a75987455a2879ca151cea5e118fa307be868583) | Report the configured TUI mode in `codex doctor` (#50200) | `Import proactively` |
| 2026-10-02 | [`c536ffcb1827`](./2026-10-02.md#c536ffcb18271cd5ad8e6a0675b3e1f49b9dc555) | Add keyboard copy selection to the owned transcript (#50434) | `Deferred` |
| 2026-10-02 | [`cb6da58876af`](./2026-10-02.md#cb6da58876afed3ede0ab11084f67dd5394ecb48) | Use the server permission catalog for TUI permission shortcuts (#50140) | `Deferred` |
| 2026-10-02 | [`d61c7a824f95`](./2026-10-02.md#d61c7a824f951abfb2133ed8aebefaed651156d4) | Honor pager bindings for transcript page keys (#50396) | `Import proactively` |
| 2026-10-02 | [`d64360f6281d`](./2026-10-02.md#d64360f6281df5c062b86715c5c49a9dce03edfb) | Support Ctrl+Insert for copying TUI selections (#50215) | `Deferred` |
| 2026-10-02 | [`e3c2a8393725`](./2026-10-02.md#e3c2a8393725c0fd8b0d1084301bf15ef35b6092) | Release stable Markdown tables into scrollback during streaming (#50207) | `Deferred` |
| 2026-10-02 | [`e7ea5f4a8658`](./2026-10-02.md#e7ea5f4a8658ebe49e879be933effed2340fa276) | Add managed worktree tools to the TUI (#50148) | `Deferred` |
| 2026-10-02 | [`f88a6efe437e`](./2026-10-02.md#f88a6efe437eec1329d8e8810c788c90ac547888) | Honor configured keybindings before transcript navigation (#50389) | `Import proactively` |
| 2026-10-03 | [`3a69ec3ef8fb`](./2026-10-03.md#3a69ec3ef8fb87b41e99b8942af061ad58ae6732) | Preserve local Markdown link labels in the TUI (#50695) | `Import proactively` |
| 2026-10-03 | [`447eac3b8118`](./2026-10-03.md#447eac3b81183b32c1a09f5fba9617abfacbebe3) | Decode Windows Terminal's mapped Shift+Enter sequence (#50720) | `Import proactively` |
| 2026-10-03 | [`6b43e6fe1fca`](./2026-10-03.md#6b43e6fe1fca7c03f0b0689074cbf51100a7f9d2) | Skip daemon auto-start for Windows-mounted WSL homes (#50555) | `Import proactively` |
| 2026-10-03 | [`9ce35d337aac`](./2026-10-03.md#9ce35d337aac3a3b1d610e0e2e18295bd76bbe0f) | Use Enter to accept transcript Find results and Escape to cancel (#50503) | `Import proactively` |
| 2026-10-03 | [`b65ab465cefa`](./2026-10-03.md#b65ab465cefa7b89c68317366b67e44431b90af4) | Reject unknown TUI keys in strict config validation (#50525) | `Import proactively` |
| 2026-10-03 | [`b741e480e203`](./2026-10-03.md#b741e480e203f037ca726bc2a76d99a8e8668e66) | Allow transcript selection and copying while bottom modals are open (#50564) | `Deferred` |
| 2026-10-03 | [`d42aecc56be3`](./2026-10-03.md#d42aecc56be30bda8df6137c1d74d6ed9365a23f) | Center TUI confirmations over their retained backdrop (#50504) | `Import proactively` |
| 2026-10-03 | [`f5a2272c6cb6`](./2026-10-03.md#f5a2272c6cb6d22635b7bd58b9a2f9ab3c1ca683) | Copy transcript selections as literal text while preserving rich HTML (#50467) | `Import proactively` |
| 2026-10-04 | [`ab45264919aa`](./2026-10-04.md#ab45264919aaeb8a421cc156f1a5459ef9d60b72) | Preserve review lifecycle ordering on failure (#50804) | `Import proactively` |
| 2026-10-05 | [`0d3868a30c37`](./2026-10-05.md#0d3868a30c37d3fd90bdc64730fa93de01907739) | Wait for SIGCONT when resuming the TUI (#51192) | `Import proactively` |
| 2026-10-06 | [`0b863c69f503`](./2026-10-06.md#0b863c69f50335acd92164aab971cb58d298c2fe) | Preserve URL destinations in wrapped hook details (#51473) | `Import proactively` |
| 2026-10-06 | [`44fe0289c6ef`](./2026-10-06.md#44fe0289c6ef35a0153c291e81cf9a232b49499c) | Make URLs clickable in TUI user input questions (#51449) | `Deferred` |
| 2026-10-06 | [`4854cfa64343`](./2026-10-06.md#4854cfa64343bc507c9bc28faa6612e68ead5a10) | Preserve clickable URLs in pending input previews (#51471) | `Import proactively` |
| 2026-10-06 | [`4b5c11134974`](./2026-10-06.md#4b5c11134974dd6ec733b89d0e5a5f4c7d74856d) | Make banner URLs clickable across wrapped lines (#51452) | `Deferred` |
| 2026-10-06 | [`594283af5c0a`](./2026-10-06.md#594283af5c0a4c99cdede91b0d7951524b2535fb) | Make URLs in asynchronous question titles clickable (#51391) | `Deferred` |
| 2026-10-06 | [`5da229e54d9d`](./2026-10-06.md#5da229e54d9d8f5779116c66decb18a21a9c8e2f) | Make URLs in the TUI warnings viewer clickable (#51451) | `Deferred` |
| 2026-10-06 | [`858aea34491f`](./2026-10-06.md#858aea34491fec77cb5069548fc842d4a177a84a) | Make URLs clickable in user verification prompts (#51458) | `Deferred` |
| 2026-10-06 | [`8e23d1836fd2`](./2026-10-06.md#8e23d1836fd22d1c920533f39851db1c7d795744) | Show hook status messages as titles in the hooks browser (#51433) | `Deferred` |
| 2026-10-06 | [`b0a5191bd6ac`](./2026-10-06.md#b0a5191bd6ac383313353d86b464859596c9ef39) | Preserve clickable URLs in TUI selection rows (#51472) | `Import proactively` |
| 2026-10-06 | [`bc3ebcb77d18`](./2026-10-06.md#bc3ebcb77d189f92572a6ed2f358dfbad81edf7a) | Preserve clickable URLs in TUI approval headers (#51439) | `Import proactively` |
| 2026-10-06 | [`e44b9bb3da03`](./2026-10-06.md#e44b9bb3da036320853abd032e2e2f065342b964) | Preserve status usage hyperlinks when the URL wraps (#51457) | `Import proactively` |
| 2026-10-06 | [`eb2d3c6416e0`](./2026-10-06.md#eb2d3c6416e0db91b462f1853445145fceb43d89) | Preserve live TUI settings when configuration reloads fail (#51510) | `Import proactively` |
| 2026-10-06 | [`ff9ab4aed96a`](./2026-10-06.md#ff9ab4aed96aa2e105f78b9521dbbd3a6b329dc8) | Suppress repeated image paste presses in legacy terminals (#51411) | `Import proactively` |
| 2026-10-07 | [`406b0c44460c`](./2026-10-07.md#406b0c44460cf71e90c21c7abe6e581add3a6421) | Enable instant interrupts by default (#51812) | `Deferred` |
| 2026-10-07 | [`eef47199272f`](./2026-10-07.md#eef47199272f4eb456f45d7b795109f8c3029c76) | Preserve line breaks and links in multiline async questions (#51866) | `Import proactively` |
| 2026-10-08 | [`39f427708738`](./2026-10-08.md#39f427708738953ac01c40c3d9351236e8ea21c0) | Enable text selection in the fullscreen TUI footer (#52270) | `Deferred` |
| 2026-10-08 | [`96d238d0b950`](./2026-10-08.md#96d238d0b95096c3dce32b31ebaef4650bcae1fc) | Include foreground activity in iTerm2 session status (#52226) | `Deferred` |
| 2026-10-08 | [`ade12342e21c`](./2026-10-08.md#ade12342e21c34a859d14cd5be47e7531fabb734) | Expose Codex lifecycle status in iTerm2 sessions (#52203) | `Deferred` |
| 2026-10-09 | [`22ebb0fb178f`](./2026-10-09.md#22ebb0fb178fe369190123c256ba200585765725) | Clamp wrapped source ranges before remapping terminal hyperlinks (#52330) | `Import proactively` |
| 2026-10-09 | [`2351d9e1b608`](./2026-10-09.md#2351d9e1b608e6f9d9a3699b71d7eb39ee41cfa4) | Avoid an extra blank quote line when pasting text with a trailing newline (#52418) | `Import proactively` |
| 2026-10-09 | [`4bad6d78e9b5`](./2026-10-09.md#4bad6d78e9b50f9fa8bd941f1db012ed491ad2da) | Report terminal program status with OSC 7501 (#52725) | `Import proactively` |
| 2026-10-09 | [`5fe4fc8f7cd1`](./2026-10-09.md#5fe4fc8f7cd16688b5c661d8cdb76e2a340b046d) | Forward clipboard copies to the terminal in Herdr sessions (#52557) | `Import proactively` |
| 2026-10-09 | [`9a59289fbea0`](./2026-10-09.md#9a59289fbea0e7e829c06aecfdd627651854da0f) | Fix races in remote environment and session replacement tests (#52591) | `Import proactively` |
| 2026-10-09 | [`d3f9b22b7456`](./2026-10-09.md#d3f9b22b745602cb6b46e436e0ed8302a40c1fdf) | Limit daemon feature compatibility checks to explicit CLI overrides (#52648) | `Import proactively` |
| 2026-10-10 | [`322bbf4d8486`](./2026-10-10.md#322bbf4d8486efd7dbbcf49598711a9e3fefc282) | Classify voice session failures and record terminal outcomes (#52756) | `Import proactively` |
| 2026-10-10 | [`34f33e4ada8f`](./2026-10-10.md#34f33e4ada8f417203d87f3b5858c97add8dad6b) | Keep startup modals protected when terminal input draining times out (#52968) | `Import proactively` |
| 2026-10-10 | [`47bcb3a3f9e0`](./2026-10-10.md#47bcb3a3f9e0904aed485389aaf410282b6b23fa) | Reduce stack usage in async TUI tests (#52959) | `Import proactively` |
| 2026-10-10 | [`7a8a9456e9a1`](./2026-10-10.md#7a8a9456e9a17a63f3980e7b321de9dcbe1e823c) | Reuse and prewarm the WSL clipboard text reader (#52967) | `Import proactively` |
| 2026-10-10 | [`c3d3b142d10f`](./2026-10-10.md#c3d3b142d10f4316b46e35aad7e5317e7e506cb7) | Make pinned transcript prompts clickable (#52778) | `Deferred` |
| 2026-10-10 | [`cc7ba3360128`](./2026-10-10.md#cc7ba33601286a751477832b01de7b6f8de0dd9c) | Add a searchable `/config` preferences panel to the TUI (#52990) | `Deferred` |
| 2026-10-10 | [`ecebd679c389`](./2026-10-10.md#ecebd679c389a74cc60334dcf1e11fe28779a3eb) | Defer owned transcript redraws during raw paste bursts (#52964) | `Import proactively` |
| 2026-10-11 | [`27065a91ccbf`](./2026-10-11.md#27065a91ccbf5ba8532844be3777fcf83b77820d) | Allow restarting managed background servers from the TUI (#53173) | `Deferred` |
| 2026-10-11 | [`72f999edc784`](./2026-10-11.md#72f999edc7848961b99c4b98bf1e62d12fa35798) | Simplify the custom terminal drawing API (#53156) | `Import proactively` |
| 2026-10-11 | [`97c1f628983a`](./2026-10-11.md#97c1f628983a401b14fefc2b2b2573806c4e9931) | Fix startup draft recovery for pending Vim Replace keys (#53148) | `Import proactively` |
| 2026-10-11 | [`a9b0815167c2`](./2026-10-11.md#a9b0815167c27ab13719dd12d13b63e02b70d221) | Support Windows clipboard text paste shortcuts in the TUI (#53224) | `Deferred` |
| 2026-10-12 | [`0c329ad2c9d5`](./2026-10-12.md#0c329ad2c9d5a2592128a1d536c66e16f9397e46) | Prefer the Windows clipboard and bound copy attempts on WSL (#53453) | `Import proactively` |
| 2026-10-12 | [`6c7d409a6d31`](./2026-10-12.md#6c7d409a6d31ce745c95dda9a09a1050deb82649) | Preserve status line accents on 16-color terminals (#53440) | `Import proactively` |
| 2026-10-12 | [`7c1b64c5df07`](./2026-10-12.md#7c1b64c5df07ae124c70a9a0094811cd3e23fcb5) | Stop identifying Apple Terminal from `TERM_SESSION_ID` (#53438) | `Import proactively` |
| 2026-10-12 | [`92caad782887`](./2026-10-12.md#92caad782887cde063572fc0a0efbf5b1317520d) | Preserve pending Effects toggles across save refreshes (#53496) | `Import proactively` |
| 2026-10-12 | [`b3c3bad9760f`](./2026-10-12.md#b3c3bad9760fc2214fab2e93659c240d147e6168) | Apply configuration toggles live and improve settings navigation (#53429) | `Import proactively` |
| 2026-10-13 | [`31434401fa3c`](./2026-10-13.md#31434401fa3cd111866acdc1bba2a73b561f53d5) | Add keyboard selection support to the transcript view (#53693) | `Deferred` |
| 2026-10-13 | [`3517671c6061`](./2026-10-13.md#3517671c6061e1704cf1c0df4a5039de23514df8) | Extend copy-on-select to composer text (#53753) | `Import proactively` |
| 2026-10-13 | [`72ad8747e926`](./2026-10-13.md#72ad8747e926daa9a761ec858a99c061e2734a18) | Render Markdown web links as labels in Rex (#53725) | `Import proactively` |
| 2026-10-13 | [`7b06e4af6640`](./2026-10-13.md#7b06e4af664049a3c1ed3113c4ee52ec3d5993f2) | Publish Rust releases with the GitHub CLI (#53673) | `Import proactively` |
| 2026-10-13 | [`f82e36592278`](./2026-10-13.md#f82e36592278a0a782fb0d0fc97e5fa10e832ac2) | Deduplicate TUI modifier handling and terminal color caching (#53556) | `Import proactively` |
| 2026-10-14 | [`01ff831ce9c1`](./2026-10-14.md#01ff831ce9c1c9d3ae98f271014b22401308ec06) | Centralize vertical navigation in TUI list views (#54081) | `Import proactively` |
| 2026-10-14 | [`764a70fc113d`](./2026-10-14.md#764a70fc113d494c38e7874c01efba15a5e4e6d8) | Restore elapsed time for replayed in-progress turns (#54047) | `Import proactively` |
| 2026-10-14 | [`c22a06c18070`](./2026-10-14.md#c22a06c18070c3e33bd2648c7210a00a2a9f6f80) | Record bucketed terminal sizes in TUI telemetry (#53859) | `Import proactively` |
| 2026-10-14 | [`dec54912f9db`](./2026-10-14.md#dec54912f9dbd307ebbe8beeb3509a6da30dbf96) | Report terminal completion states and blocking reasons (#54114) | `Import proactively` |
| 2026-10-15 | [`111362a6a0a9`](./2026-10-15.md#111362a6a0a92d08885b1ae62d3760714907ad94) | Add a keyboard shortcut menu for presets (#54381) | `Deferred` |
| 2026-10-15 | [`141bf7680d77`](./2026-10-15.md#141bf7680d7721ba8271b91473323fb6b58ebdd0) | Add session-only arguments to `/title` and `/statusline` (#54387) | `Deferred` |
| 2026-10-15 | [`1544ffc79d60`](./2026-10-15.md#1544ffc79d60d29d492ca432fc0a5b2f3745ddce) | Add configuration support for TUI input presets (#54379) | `Deferred` |
| 2026-10-15 | [`36803b180a50`](./2026-10-15.md#36803b180a50e611331b75ff2b4c475c02846b53) | Allow saving current permission settings as presets (#54385) | `Deferred` |
| 2026-10-15 | [`86ee40c3801a`](./2026-10-15.md#86ee40c3801a8c4fbb35b1159c32da1447cf09d9) | Keep TUI Markdown colors readable against their backgrounds (#54315) | `Import proactively` |
| 2026-10-15 | [`98fd26206d96`](./2026-10-15.md#98fd26206d96a4a2f7aee7258fc60c4ec3f1a566) | Use neutral labels for nonzero exits in compact TUI output (#54433) | `Deferred` |
| 2026-10-15 | [`9fcb1f5ed49c`](./2026-10-15.md#9fcb1f5ed49c938b6fa5a9b4c8f897f3ddfd734c) | Support line breaks in Mermaid flowchart node labels (#54419) | `Deferred` |
| 2026-10-15 | [`ac9db7d8b556`](./2026-10-15.md#ac9db7d8b556c7c34a73793aed34b1f980a4fe39) | Generate keymap descriptors and binding accessors from one catalog (#54151) | `Import proactively` |
| 2026-10-15 | [`b095b2ef236d`](./2026-10-15.md#b095b2ef236d8f45f85a9d5526bfbf2618ab2d63) | Support inline permission profile selection with `/permissions` (#54383) | `Deferred` |
| 2026-10-15 | [`dffa0deb6926`](./2026-10-15.md#dffa0deb69268ed4ac31578794c9ea897b336ae3) | Show image previews in the fullscreen transcript (#54216) | `Deferred` |
| 2026-10-15 | [`e89b7c442c04`](./2026-10-15.md#e89b7c442c0433ad5167e72a5a25bb6d0712766a) | Apply syntax theme colors to Markdown prose in the TUI (#54217) | `Import proactively` |
| 2026-10-15 | [`f847a4bd3c6c`](./2026-10-15.md#f847a4bd3c6ca3d8474a6e0dace1ed39f7ec0067) | Add quick dial presets for terminal title and status line layouts (#54388) | `Deferred` |
| 2026-10-16 | [`440312f11396`](./2026-10-16.md#440312f11396a58b312dd31798ec3393810d8ff8) | Keep TUI session selections bound to their backend (#54681) | `Import proactively` |
| 2026-10-17 | [`3b8f43a06bb3`](./2026-10-17.md#3b8f43a06bb3105e81406b35ff69f94977d18ae4) | Defer unknown TUI config fields to the shared strict-config policy (#54766) | `Import proactively` |
| 2026-10-19 | [`36e5861fd9cf`](./2026-10-19.md#36e5861fd9cfa795b1144d3b37113847db11916b) | Mark `transcript_v2` as removed (#55388) | `Import proactively` |
| 2026-10-19 | [`63ba55817a13`](./2026-10-19.md#63ba55817a13e2b2cb546a4f1794e50054a60d56) | Manage TUI backend sessions independently (#55381) | `Import proactively` |
| 2026-10-19 | [`e4568a3538c9`](./2026-10-19.md#e4568a3538c9cab6627d64ff8cdfc7d98b79b7db) | Reject TUI events from a different backend (#55304) | `Import proactively` |
| 2026-10-20 | [`1a35bc4ccbc3`](./2026-10-20.md#1a35bc4ccbc38417bde1c3a0fc96624fca5b7dfe) | Render async question prompts as Markdown in the TUI (#55411) | `Import proactively` |
| 2026-10-20 | [`7389eebf45df`](./2026-10-20.md#7389eebf45df47895b386334bc1c5b9809eb9605) | Fix TUI permission selection and session state handling (#55682) | `Import proactively` |
| 2026-10-20 | [`78d4c7c7fae2`](./2026-10-20.md#78d4c7c7fae278d2b0a904316c274681c62461a8) | Extract shared TUI user prompt helpers from IDE context (#55597) | `Import proactively` |
| 2026-10-20 | [`8196bcdc189a`](./2026-10-20.md#8196bcdc189ad2c189d631b321fa54b8e09b9bb0) | Check daemon feature overrides and prompt for mismatch recovery (#55442) | `Import proactively` |
| 2026-10-20 | [`a3c643bf8da2`](./2026-10-20.md#a3c643bf8da200a4b7b22b901f03e55108e59563) | Retry TUI startup after a server shutdown refusal (#55409) | `Import proactively` |
| 2026-10-20 | [`fa71afc8a701`](./2026-10-20.md#fa71afc8a701a88cc8c92b94240f7a23e31b3568) | Rank `$` mentions by recent selection frequency (#55596) | `Import proactively` |
| 2026-10-22 | [`35e21028080c`](./2026-10-22.md#35e21028080ccbda9de04d2239b0a0b2336d868f) | Group consecutive transcript prompts into one background block (#56280) | `Import proactively` |
| 2026-10-22 | [`47a1f6bb77ba`](./2026-10-22.md#47a1f6bb77baa8ba237757196a62cadf97dc854b) | Honor server approval reviewer defaults on fresh TUI startup (#56090) | `Import proactively` |
| 2026-10-22 | [`86e17e98c7f9`](./2026-10-22.md#86e17e98c7f901277aeeb4649cf1b01462b082f2) | Prepare TUI startup to accept multiple backend sessions (#56097) | `Import proactively` |
| 2026-10-22 | [`be537fb60c96`](./2026-10-22.md#be537fb60c96d2f89c2ee4be161edb7206bf791e) | Remove the `u64` cast from WSL DrvFS detection (#56214) | `Import proactively` |
| 2026-10-22 | [`c2d5ea52e1b7`](./2026-10-22.md#c2d5ea52e1b7ab11ee2d71dc44ea1ef00f9c8eb3) | Keep TUI preferences editable when execution settings are invalid (#56091) | `Import proactively` |
| 2026-10-23 | [`334527705c4a`](./2026-10-23.md#334527705c4a2d077efce577925247d87f10d2b1) | Separate directory and repository scopes in the TUI (#56308) | `Import proactively` |
| 2026-10-23 | [`398500106707`](./2026-10-23.md#398500106707f0a3217c7cf83edc2bf226cc342f) | Respect server approval policy defaults during TUI startup (#56516) | `Import proactively` |

### B07 — Execution, sandbox, and permission boundaries

- Status: `Waiting for user decision`
- Members: 194
- Proactive: 142; Deferred: 42; Review: 10
- Implementation order: adapt or add tests first, manually adapt production code to Ruxex, run affected unit tests and relevant E2E tests, then record the Ruxex commit.

| Date | Upstream commit | Subject | Decision |
| --- | --- | --- | --- |
| 2026-09-19 | [`01f92c57802a`](./2026-09-19.md#01f92c57802a743b5d3893abc9df7670eef8cfe9) | Remove `com.apple.runningboard` from Seatbelt platform defaults (#46532) | `Import proactively` |
| 2026-09-19 | [`22dea110204c`](./2026-09-19.md#22dea110204c8a07abcbd6495d7eeae65faa0d9f) | Allow unrelated namespace mounts in Linux sandbox socket checks (#46535) | `Import proactively` |
| 2026-09-19 | [`29a57861e31a`](./2026-09-19.md#29a57861e31aecb97c7d2d8420bc3ed234d58e8c) | Fix standalone network proxy policy initialization (#46578) | `Import proactively` |
| 2026-09-19 | [`595cc91e8cbb`](./2026-09-19.md#595cc91e8cbb1c2ca822d0311dcf12709410c582) | Avoid fork when spawning macOS filesystem helpers (#46661) | `Import proactively` |
| 2026-09-19 | [`6b78e825aeac`](./2026-09-19.md#6b78e825aeac93fa361c976a718f5cf0595840ad) | Limit the agent command center to 10 recent sessions on startup (#46579) | `Deferred` |
| 2026-09-19 | [`78245b47af2a`](./2026-09-19.md#78245b47af2a7aafcabe025828ceecca69db4df1) | Deny XPC service lookups in macOS Seatbelt profiles (#46583) | `Import proactively` |
| 2026-09-19 | [`b244e9bafeca`](./2026-09-19.md#b244e9bafeca0872218d8e93001d85f15c4c230e) | Add a command-start callback for tool lifecycle extensions (#46539) | `Deferred` |
| 2026-09-19 | [`b33199b1fbcf`](./2026-09-19.md#b33199b1fbcf96ea1e4035729158f6ecd1760192) | Add bounded filesystem path diagnostics to `codex doctor` (#46543) | `Import proactively` |
| 2026-09-19 | [`d84ebff59295`](./2026-09-19.md#d84ebff59295a5fc897bd7b703ade933c5312198) | Preserve macOS Seatbelt exclusions in scratch directories (#46571) | `Import proactively` |
| 2026-09-19 | [`daed4b9755a4`](./2026-09-19.md#daed4b9755a4215a149ae7e5538d8cd34709cb40) | Split shell snapshot credential tests into focused cases (#46715) | `Import proactively` |
| 2026-09-19 | [`ef9f3d022aff`](./2026-09-19.md#ef9f3d022aff6bd087c640392dc8608743aa411d) | Allow recovery commands when the current thread is unavailable (#46566) | `Import proactively` |
| 2026-09-19 | [`fd59b42a09df`](./2026-09-19.md#fd59b42a09df58d301839cac889c2db69c40f6cb) | Preserve Windows package identity for sandboxed descendants (#46575) | `Import proactively` |
| 2026-09-20 | [`569888ac65e1`](./2026-09-20.md#569888ac65e124ca752f6a0937b8722d245651eb) | Refine the agent command center layout and metadata editing (#46838) | `Deferred` |
| 2026-09-20 | [`724cdfbd32d3`](./2026-09-20.md#724cdfbd32d35fde9e0da1f1e141b836836f9a65) | Simplify agent command center hints and back navigation (#46840) | `Deferred` |
| 2026-09-20 | [`93b44c3a73ad`](./2026-09-20.md#93b44c3a73ad9dfae709751c231a8b512d9eabf3) | Streamline agent command-center shortcuts and layout (#46882) | `Deferred` |
| 2026-09-20 | [`d4cb75357fd7`](./2026-09-20.md#d4cb75357fd7c547500cb64658bea28a9979e284) | Improve agent command center rows and page navigation (#46837) | `Deferred` |
| 2026-09-20 | [`eef77aa21144`](./2026-09-20.md#eef77aa21144edcff79942c9f0ff98a8a527103b) | Add status filter tabs to the agent command center (#46839) | `Deferred` |
| 2026-09-21 | [`023d81b057de`](./2026-09-21.md#023d81b057dec77fc5c6ffa0ec646d29bef10782) | Preserve required Windows runtime variables for filesystem helpers (#47108) | `Import proactively` |
| 2026-09-21 | [`7cec3e110ce6`](./2026-09-21.md#7cec3e110ce6dc5c14704dfa06f6db9c23a530fc) | Mask daemon socket paths exposed through ancestor bind mounts (#47079) | `Import proactively` |
| 2026-09-22 | [`517f51a6f710`](./2026-09-22.md#517f51a6f7101e2bda4c96cbd0605f8c55aba20b) | Extract exec-server CLI startup into a dedicated module (#47143) | `Import proactively` |
| 2026-09-22 | [`54487a5b614b`](./2026-09-22.md#54487a5b614b798bb454122a777623336c6a9845) | Bound inbound exec-server requests across client transports (#47362) | `Import proactively` |
| 2026-09-22 | [`c6ad6f33eb1f`](./2026-09-22.md#c6ad6f33eb1fba74f8aab266465b91249fe4ef3f) | Restrict Windows sandbox default object access to the logon session (#47361) | `Import proactively` |
| 2026-09-23 | [`17cd2834e3d3`](./2026-09-23.md#17cd2834e3d38706cead62cb97b8b403d3b8a4ec) | Repair rejected Windows sandbox credentials during provisioning (#47695) | `Import proactively` |
| 2026-09-23 | [`5f371ba30ae4`](./2026-09-23.md#5f371ba30ae4a4bf4527eb0729ab6b36e3a0c123) | Fix no-reparse directory opens on Windows 10 (#47672) | `Import proactively` |
| 2026-09-23 | [`60f0ab6cbfe4`](./2026-09-23.md#60f0ab6cbfe464dd6fc2603e53604995423714db) | Use the shared process launcher for Unix shell snapshots (#47611) | `Import proactively` |
| 2026-09-23 | [`6f6af0fce438`](./2026-09-23.md#6f6af0fce4381233fa2e83dc24641167e68dfcb5) | Record sandbox backends in command execution analytics (#47568) | `Deferred` |
| 2026-09-23 | [`6fa3bcaae8aa`](./2026-09-23.md#6fa3bcaae8aaa7c90735816191c3bda0ae5654e4) | Add exec-server RPC timing and process startup tracing (#47680) | `Deferred` |
| 2026-09-23 | [`7aa65199098d`](./2026-09-23.md#7aa65199098d20f7e1977ff537523e720d939f36) | Use native POSIX spawning for command hooks (#47610) | `Import proactively` |
| 2026-09-23 | [`8bbe8f870247`](./2026-09-23.md#8bbe8f8702471b90c75df20104bf91d20001b0d7) | Fix read-only metadata mount ordering for nested writable roots (#47623) | `Import proactively` |
| 2026-09-23 | [`a69d757cd8ef`](./2026-09-23.md#a69d757cd8ef8310001186865911b69e4b4175e5) | Emit command lifecycle events for unified exec launch failures (#47529) | `Deferred` |
| 2026-09-23 | [`b8d365b18ec2`](./2026-09-23.md#b8d365b18ec27188a46d3305dd002180921bcc2b) | Extend child commands with session and descriptor controls (#47604) | `Deferred` |
| 2026-09-23 | [`c530382cf38a`](./2026-09-23.md#c530382cf38a5133dd1f7561f9c972e312dd0caa) | Clarify registered Windows sandbox setup errors (#47673) | `Import proactively` |
| 2026-09-23 | [`c847294c60c7`](./2026-09-23.md#c847294c60c77fe0215c6eeabd725684539ebb37) | Classify retryable exec-server preparation errors by type (#47638) | `Import proactively` |
| 2026-09-23 | [`ec4d27ae8088`](./2026-09-23.md#ec4d27ae8088e67785fb6d913462c8433f99dd47) | Test interrupted one-shot command launch failure persistence (#47539) | `Import proactively` |
| 2026-09-24 | [`51d45620702c`](./2026-09-24.md#51d45620702cd3c825d3520ddc7d7e8052a16b87) | Support close-on-exec attachments without changing PTY semantics (#47797) | `Import proactively` |
| 2026-09-24 | [`6f51c65958eb`](./2026-09-24.md#6f51c65958ebdd5a57708e012d045f172fc9aab4) | Fix macOS system-alias matching in patch permission checks (#47879) | `Import proactively` |
| 2026-09-24 | [`a708fc683934`](./2026-09-24.md#a708fc683934d7d7ea7560354968fedd2be28581) | Handle Btrfs device mismatches when masking daemon sockets (#47968) | `Import proactively` |
| 2026-09-24 | [`c19dcd975d8c`](./2026-09-24.md#c19dcd975d8cf428e3c04cc9bd3a2b9b3bb37f64) | Allow directory moves under global Seatbelt basename denies (#47920) | `Import proactively` |
| 2026-09-24 | [`c8c1ee5da8af`](./2026-09-24.md#c8c1ee5da8af5c79ec8433ee88d3d4ebdcfd80fd) | Transport large Windows sandbox launch payloads through the environment (#47919) | `Import proactively` |
| 2026-09-24 | [`e0ef5a1a0f64`](./2026-09-24.md#e0ef5a1a0f6421601baaa679fb37eddaa4e9c8c1) | Reduce generic code duplication in RPC and Markdown rendering (#47751) | `Import proactively` |
| 2026-09-25 | [`1d804e91b754`](./2026-09-25.md#1d804e91b75454927fb4f51615b067ba6158c37c) | Protect `.aws` directories under sandbox writable roots (#48176) | `Import proactively` |
| 2026-09-25 | [`645b683a9e71`](./2026-09-25.md#645b683a9e712ae6d90a364e3cf597466e4146d0) | Preserve filesystem denials when preparing approved commands (#48155) | `Import proactively` |
| 2026-09-25 | [`66c4e5a20d21`](./2026-09-25.md#66c4e5a20d211f329de32b0452b35a9aac66b3c5) | Keep startup drafts visible during command center session handoff (#48121) | `Import proactively` |
| 2026-09-25 | [`8f0b05910efb`](./2026-09-25.md#8f0b05910efb6f290dc8a31afc1c3f250f0baf99) | Fix zsh alias quoting in sourced shell snapshots (#48187) | `Import proactively` |
| 2026-09-25 | [`a92ccbde5328`](./2026-09-25.md#a92ccbde5328697894d09935a656f7c5f07a7d3f) | Preserve Git directory protections across writable roots (#47974) | `Import proactively` |
| 2026-09-25 | [`aa380897f67b`](./2026-09-25.md#aa380897f67b91e1a47d530d7286d497b6726d3f) | Add startup-only PID namespace inheritance to exec-server (#47989) | `Deferred` |
| 2026-09-25 | [`b63a296775ec`](./2026-09-25.md#b63a296775ec51368352c251c182edf0ab71c0fd) | Honor shell environment policy in legacy snapshots (#48099) | `Import proactively` |
| 2026-09-25 | [`c98e263fb536`](./2026-09-25.md#c98e263fb5365a512bb997a103d6ee8aa14c23e6) | Replay exec-server shell snapshots through unnamed files (#48078) | `Import proactively` |
| 2026-09-25 | [`d9275a63ed1c`](./2026-09-25.md#d9275a63ed1c4ff6ed6b56db8669453ad678b6cf) | Allow Left to open the command center from read-only conversations (#48132) | `Deferred` |
| 2026-09-25 | [`e75b36efdec6`](./2026-09-25.md#e75b36efdec6416f82306d307bc001461efe295a) | Show multiline command previews in `/ps` (#48101) | `Import proactively` |
| 2026-09-25 | [`f6a4bd81e310`](./2026-09-25.md#f6a4bd81e3109d79842230dd344b6fe895d5bd1a) | Generate unique exec-server process IDs for every request (#48168) | `Import proactively` |
| 2026-09-26 | [`06f97622f852`](./2026-09-26.md#06f97622f8529feef0d230fc37519c36c6bb2eeb) | Add context to Windows sandbox runtime registration errors (#48531) | `Import proactively` |
| 2026-09-26 | [`09a3d2108f23`](./2026-09-26.md#09a3d2108f23a7b99984c408f1954083c7ec7e97) | Display reconnect commands on a separate line (#48350) | `Import proactively` |
| 2026-09-26 | [`228ae3da8dda`](./2026-09-26.md#228ae3da8dda6edcff6a2078992e239729723a7d) | Allow macOS TLS trust evaluation in network-enabled Seatbelt profiles (#48565) | `Import proactively` |
| 2026-09-26 | [`b8d5e3f12e58`](./2026-09-26.md#b8d5e3f12e58fff942093457bf1350f9ceb695e9) | Allow exec-server to proxy permitted private IPs upstream (#48568) | `User review required` |
| 2026-09-27 | [`18344a972ddf`](./2026-09-27.md#18344a972ddf06aa0bb80773bfb7564576e8991b) | Centralize executable fixture creation to avoid Linux ETXTBSY races (#48727) | `Import proactively` |
| 2026-09-27 | [`67a709665ac7`](./2026-09-27.md#67a709665ac7b50311b93e32612c9a8281684787) | Fix the session-start helper call in the command center test (#48646) | `Import proactively` |
| 2026-09-27 | [`985cf47a4eb6`](./2026-09-27.md#985cf47a4eb6084b2ff6b30ebdb1216acda85bb4) | Allow provisioned executors more time to come online (#48575) | `Import proactively` |
| 2026-09-28 | [`5a5a4aa79696`](./2026-09-28.md#5a5a4aa79696a4c8a46dea1c9c04066b22559332) | Keep configuration values out of Windows sandbox policy events (#49067) | `Import proactively` |
| 2026-09-28 | [`df3e439c02ef`](./2026-09-28.md#df3e439c02efc90f29a26d2d602a54eea2e8a91e) | Fix Windows sandbox ACL repair for long runtime paths (#49058) | `Import proactively` |
| 2026-09-28 | [`e6f4af1d92bb`](./2026-09-28.md#e6f4af1d92bbfadcc49217006c930f2a760f1cc8) | Wait briefly for the Windows sandbox provisioning service to start (#48829) | `Import proactively` |
| 2026-09-28 | [`f5430515a8ac`](./2026-09-28.md#f5430515a8acf9251f60cacae6c86ae43686c960) | Resolve Windows sandbox PowerShell fallbacks on the exec server (#49098) | `Import proactively` |
| 2026-09-28 | [`f817e1690570`](./2026-09-28.md#f817e1690570aac60480babbd2243cc991ba2118) | Use the numeric ioctl value in the macOS sandbox policy (#49028) | `Import proactively` |
| 2026-09-29 | [`0462dcc062b8`](./2026-09-29.md#0462dcc062b822bb8fff16cc31ce6eeab69823b9) | Use executable fixture copying in the bundled bwrap test (#49246) | `Import proactively` |
| 2026-09-29 | [`17a9df60e420`](./2026-09-29.md#17a9df60e420b58e3edc55efb1bb052e39492bbc) | Add an experimental flag for bundled tools in login shells (#49403) | `Deferred` |
| 2026-09-29 | [`26dd19ef478c`](./2026-09-29.md#26dd19ef478cfa294406413bafedb84b49f93ea4) | Retry Windows sandbox runner logon once on error 1056 (#49325) | `Import proactively` |
| 2026-09-29 | [`50d9c5deac4a`](./2026-09-29.md#50d9c5deac4aa8cafc95db886d1435726ff0f77c) | Run piped legacy Windows sandbox processes without a console (#49308) | `Import proactively` |
| 2026-09-29 | [`65c3f40befaa`](./2026-09-29.md#65c3f40befaa213788ca7f97cc955ba996ffa3f5) | Model exec-server session attachment state as an enum (#49286) | `Import proactively` |
| 2026-09-29 | [`804d6306e84f`](./2026-09-29.md#804d6306e84f570393cdd1eec94c41464f503b1a) | Allow approved filesystem escalation while preserving denied reads (#49353) | `Import proactively` |
| 2026-09-29 | [`8f6517772bc2`](./2026-09-29.md#8f6517772bc20dab715e97f9d7aac15df00bbea9) | Add history pagination to the agent command center (#49106) | `Deferred` |
| 2026-09-29 | [`9212b3eca86e`](./2026-09-29.md#9212b3eca86e6eca061b15fe9f8971ca57ebe3c9) | Serialize tests that share Windows sandbox accounts (#49389) | `Import proactively` |
| 2026-09-29 | [`995138d71ac0`](./2026-09-29.md#995138d71ac06b9df5996f049e69dea414ad9764) | Carry shell invocation metadata and report executor PATH directories (#49360) | `Import proactively` |
| 2026-09-29 | [`ed9e5a26a88b`](./2026-09-29.md#ed9e5a26a88b8f3e36e70ccab0d8b9c5e14f015b) | Clean up canceled exec-server RPC requests immediately (#49332) | `Import proactively` |
| 2026-09-29 | [`f35a0fdc5d5f`](./2026-09-29.md#f35a0fdc5d5fe591fbb2cad7ca6c491c884c3b4b) | Preserve Windows sandbox runner launch errors (#49261) | `Import proactively` |
| 2026-09-29 | [`fbc169827e97`](./2026-09-29.md#fbc169827e9789f9e7ba2ed97c15e2021c3312bb) | Suppress Windows console windows for background subprocesses (#49164) | `Import proactively` |
| 2026-09-30 | [`3b16b5a5b03a`](./2026-09-30.md#3b16b5a5b03aba9a960b5dedde48fb687863c6e9) | Rename exec-server file handle management identifiers (#49702) | `Import proactively` |
| 2026-09-30 | [`67352b5b4adc`](./2026-09-30.md#67352b5b4adc3d3e8865ac8bf343b952b8c040e7) | Recover exec-server sessions after environment info timeouts (#49407) | `Import proactively` |
| 2026-09-30 | [`76a6e55d5ac6`](./2026-09-30.md#76a6e55d5ac69e3ba6b0481f8f3b94256f46dfa7) | Restore executor tool paths after login shell startup (#49467) | `Import proactively` |
| 2026-09-30 | [`7e8878f60560`](./2026-09-30.md#7e8878f60560f8ff964fe38712e53def06402db7) | Preserve PowerShell relative paths in the elevated Windows sandbox (#49690) | `Import proactively` |
| 2026-09-30 | [`d42056091ade`](./2026-09-30.md#d42056091aded7feb1d88ac7e83972108b2aa478) | Add a fork shortcut to the TUI command center (#49517) | `Deferred` |
| 2026-09-30 | [`f2b2e5b2a96d`](./2026-09-30.md#f2b2e5b2a96d845a16d649f0984c0bd06c15ef37) | Make exec-server file reads cancellable between chunks (#49696) | `Import proactively` |
| 2026-09-30 | [`fcbed044c8c6`](./2026-09-30.md#fcbed044c8c6994c97c7387d21ff3077cf3a5812) | Escape command drafts when recovering question answers (#49678) | `Import proactively` |
| 2026-10-01 | [`1a913493dcc4`](./2026-10-01.md#1a913493dcc48018637f7efa7066a78cbea40511) | Clean up process groups for failed shell snapshot captures (#49782) | `Import proactively` |
| 2026-10-01 | [`875bf9209bd9`](./2026-10-01.md#875bf9209bd9aeb0a5f102888ccd90b2b59cc3c9) | Define exec-server protocol types for streamed file writes (#49778) | `User review required` |
| 2026-10-01 | [`a73898c249b3`](./2026-10-01.md#a73898c249b3cc95849ae1ea91231a0c9a9e29e2) | Use dedicated parameters for sandboxed file opens (#49818) | `Deferred` |
| 2026-10-01 | [`a933dd77dbe1`](./2026-10-01.md#a933dd77dbe101d7bd746ea3c7d1f8174eca4a05) | Share byte buffers across exec-server output chunks (#49972) | `Import proactively` |
| 2026-10-01 | [`ae7aad586ea0`](./2026-10-01.md#ae7aad586ea0769f329e214e9827c541823fd191) | Fix Linux sandbox startup with multiple denied files (#50059) | `Import proactively` |
| 2026-10-01 | [`c51f5bfb821f`](./2026-10-01.md#c51f5bfb821f813fa3591d4a4d03a764930cb982) | Handle unsupported `fs/writeBlock` requests in exec-server (#49811) | `Import proactively` |
| 2026-10-01 | [`c538fbabe5cb`](./2026-10-01.md#c538fbabe5cba2b4e7f03ab6469bc8e1b8bfeddf) | Share cached exec-server environment info with Arc (#49798) | `Import proactively` |
| 2026-10-01 | [`da2e174a66a6`](./2026-10-01.md#da2e174a66a6572204a91373321b7d22d87fb12e) | Scope extension filesystem access to callback permissions (#49898) | `Import proactively` |
| 2026-10-01 | [`e7798c99444c`](./2026-10-01.md#e7798c99444cfde9dbf2230f7e7699b920d757d5) | Add capability-gated writable file streams to the exec-server client (#49805) | `User review required` |
| 2026-10-01 | [`ecc78e4cf560`](./2026-10-01.md#ecc78e4cf5607ecf5f080d682eae0ecb650868ae) | Add renewable EMA HTTP authentication and credential versioning (#49987) | `Deferred` |
| 2026-10-02 | [`12a30d4e6dbd`](./2026-10-02.md#12a30d4e6dbd7e527898142b6f4f65fd8070a3b5) | Add a CLI command to uninstall the legacy Windows sandbox (#50437) | `Deferred` |
| 2026-10-02 | [`37f53199a6b2`](./2026-10-02.md#37f53199a6b2450b94bf4c7ea41c261bbdb39742) | Use the shared text editor for command-center task renaming (#50216) | `Deferred` |
| 2026-10-02 | [`4cedd0caac89`](./2026-10-02.md#4cedd0caac89f9fdcb5ad385e8093da5363d91c2) | Add end-to-end coverage for MCP sandbox state enforcement (#50339) | `Deferred` |
| 2026-10-02 | [`bee28e8a061c`](./2026-10-02.md#bee28e8a061c38c781c74299f2c1d90e3732da18) | Cap persisted command output in paginated history at 64 KiB (#50427) | `Import proactively` |
| 2026-10-02 | [`c39bfa4c8ff9`](./2026-10-02.md#c39bfa4c8ff9a57c46a9d3766676d6435e262691) | Enable writable file streaming in exec-server (#50177) | `Deferred` |
| 2026-10-02 | [`c5d242fa7907`](./2026-10-02.md#c5d242fa7907bff1b7a7e26e95febc548c0a6963) | Consolidate command execution output into `aggregated_output` (#50402) | `Import proactively` |
| 2026-10-02 | [`d25c114d494d`](./2026-10-02.md#d25c114d494ddb693290b76bf5e5f64ecbdb38fc) | Bound in-flight file opens in exec-server (#50162) | `Import proactively` |
| 2026-10-02 | [`ef8cfe5e96f7`](./2026-10-02.md#ef8cfe5e96f7782b38745f3f86da61af3ba35e10) | Retry registry authentication outages and jitter executor reconnects (#50465) | `Import proactively` |
| 2026-10-03 | [`47379efd5289`](./2026-10-03.md#47379efd5289cba801c5a66273064fbe3bf92f60) | Keep Command Center selection adjacent after task removal (#50505) | `Import proactively` |
| 2026-10-03 | [`6c15cc4aaf42`](./2026-10-03.md#6c15cc4aaf4295fcc88ef4afca410320d3002614) | Use the app-server default output cap for TUI workspace commands (#50477) | `Import proactively` |
| 2026-10-03 | [`8f7a0f7a8781`](./2026-10-03.md#8f7a0f7a878199c6886600370e5be6bd37ca38a3) | Skip managed config loading for registered Windows sandbox refreshes (#50480) | `Import proactively` |
| 2026-10-03 | [`c542fb93ef4b`](./2026-10-03.md#c542fb93ef4b49c06e854e1cde9f892e8d3e8094) | Record Windows sandbox service stop diagnostics (#50507) | `Import proactively` |
| 2026-10-04 | [`acf9818faef9`](./2026-10-04.md#acf9818faef932ad0b26049c6f7f8b1acd94149b) | Remember Command Center grouping across launches (#50786) | `Import proactively` |
| 2026-10-04 | [`b8dceb0d4f29`](./2026-10-04.md#b8dceb0d4f29e49e73daa08f57fcf5181186f354) | Open slash commands from empty drafts in Vim Normal mode (#50788) | `Import proactively` |
| 2026-10-04 | [`cd7d9e128cc5`](./2026-10-04.md#cd7d9e128cc53a8cc3d510772721ccd897b2c366) | Show unavailable slash commands when searched in side conversations (#50756) | `Deferred` |
| 2026-10-04 | [`de3721a7be07`](./2026-10-04.md#de3721a7be07054c8c2a41102b5a501f34155361) | Recover malformed Windows deny-read ACL state safely (#50940) | `Import proactively` |
| 2026-10-06 | [`0b47040dc9eb`](./2026-10-06.md#0b47040dc9eb08d9c37d51e0824870ce5b27101b) | Add correlated, credential-free rendezvous connection diagnostics (#51483) | `Import proactively` |
| 2026-10-06 | [`414d165b47a9`](./2026-10-06.md#414d165b47a90d44a231d62a6c2cf2f876306c1a) | Preserve wrapped help links in Windows sandbox prompts (#51459) | `Import proactively` |
| 2026-10-06 | [`580b18cb7488`](./2026-10-06.md#580b18cb7488a76096238820f779b417510441d0) | Start the Windows sandbox service during registered Core setup (#51256) | `Import proactively` |
| 2026-10-06 | [`588f616e8b6a`](./2026-10-06.md#588f616e8b6aee417d4b2f480e48b7e6d8741993) | Measure shell snapshot use and wait time per command (#51347) | `Import proactively` |
| 2026-10-06 | [`7298a6e02003`](./2026-10-06.md#7298a6e02003ec6975843dcf8068197a315793ee) | Add shared task pinning to the agent command center (#51500) | `Deferred` |
| 2026-10-06 | [`7aa8f5104963`](./2026-10-06.md#7aa8f510496316b86a91a75ed0a71598ef03c640) | Reject sandbox-writable bubblewrap executables from PATH (#51211) | `Import proactively` |
| 2026-10-06 | [`7efc49b25858`](./2026-10-06.md#7efc49b258587fbe872c7897e90ffa6144bea2a0) | Align Windows sandbox temp permissions with the child environment (#51512) | `Import proactively` |
| 2026-10-06 | [`9545947c6da3`](./2026-10-06.md#9545947c6da3f8ecc9a94eefd3d81e8799f185ec) | Fix Windows 10 drive-letter opens for no-follow filesystem operations (#51511) | `Import proactively` |
| 2026-10-06 | [`ccde2fc8b7fd`](./2026-10-06.md#ccde2fc8b7fd08216757d3d3f67362847ab8341b) | Protect ripgrep lookup during Linux sandbox construction (#51407) | `Import proactively` |
| 2026-10-06 | [`ddabe594e6f0`](./2026-10-06.md#ddabe594e6f053f2d6d193b5768ece5f4c8c771d) | Bound relay connection attempts and handle pongs during blocked writes (#51502) | `Import proactively` |
| 2026-10-06 | [`e32365a2c61a`](./2026-10-06.md#e32365a2c61a78c8b4f00b15e3559e0391f4eb60) | Allow larger shell snapshots when replaying from a file (#51350) | `Deferred` |
| 2026-10-07 | [`11da6b9edcbb`](./2026-10-07.md#11da6b9edcbbf1cffdb9b7686970b5b4d5af9c83) | Add policy-based file contents integrity checks (#51828) | `Import proactively` |
| 2026-10-07 | [`37eaae6eeb71`](./2026-10-07.md#37eaae6eeb71c3bd3d165f52515086db8f5e7dee) | Require hostname authorization before proxy DNS lookups (#51650) | `Import proactively` |
| 2026-10-07 | [`622e9e369621`](./2026-10-07.md#622e9e36962105af50b9c250902e8b8c66a950d8) | Preserve sandbox overrides across execution requests (#51786) | `Import proactively` |
| 2026-10-07 | [`ac9b5b838051`](./2026-10-07.md#ac9b5b8380517ded445b09dd3196d8d9e2ba3c59) | Ignore ripgrep configuration when expanding sandbox deny globs (#51527) | `Import proactively` |
| 2026-10-07 | [`b5c37c35fd51`](./2026-10-07.md#b5c37c35fd517bf6d1d1515a5f7552c221d2ffe3) | Add a macOS Seatbelt backend for sandbox integrity (#51841) | `User review required` |
| 2026-10-07 | [`c2eb1f42a091`](./2026-10-07.md#c2eb1f42a0914bf7a204fccb54797c17badacd5a) | Add a sandbox integrity runner with outcome and timing metrics (#51842) | `User review required` |
| 2026-10-07 | [`cba716c0f1ca`](./2026-10-07.md#cba716c0f1caa4f2528dedbb1075b50126709d7e) | Run sandbox integrity checks before commands and filesystem operations (#51843) | `User review required` |
| 2026-10-07 | [`d83bb540ec64`](./2026-10-07.md#d83bb540ec64bf6b009bca0283b0be91ea33f26a) | Move Windows sandbox tests into a dedicated integration binary (#51678) | `Import proactively` |
| 2026-10-07 | [`d9960e12bb17`](./2026-10-07.md#d9960e12bb178bae15d25bde5916642a94675466) | Add a bubblewrap backend for sandbox integrity checks (#51840) | `Import proactively` |
| 2026-10-07 | [`dd12f892f1b8`](./2026-10-07.md#dd12f892f1b857e5161abd33a2967125b265d550) | Avoid sharing violations when repairing Windows runtime ACLs (#51822) | `Import proactively` |
| 2026-10-08 | [`3b6ab3471a2d`](./2026-10-08.md#3b6ab3471a2d4d994c9fad322fca749564bc05e0) | Normalize formatting in docs, web assets, and sample scripts (#52256) | `Import proactively` |
| 2026-10-08 | [`54685110a730`](./2026-10-08.md#54685110a730ff2186f27c9f36d40ed83bae3071) | Support request IDs in exec-server WebSocket handshakes (#52234) | `Deferred` |
| 2026-10-08 | [`78a19f5ab563`](./2026-10-08.md#78a19f5ab563038142292857553b41be0f2348f2) | Expose a verified sandbox launcher to local MCP servers (#52182) | `Deferred` |
| 2026-10-08 | [`79773e8826ae`](./2026-10-08.md#79773e8826aeb6bca09305728297908b0d5bc6c3) | Enforce application network policy in standalone MCP commands (#52215) | `User review required` |
| 2026-10-08 | [`9b738582b13c`](./2026-10-08.md#9b738582b13c2cdbeff54af0afd04c50c3e7ba09) | Expose originating tool call metadata in command environments (#52119) | `Deferred` |
| 2026-10-08 | [`bd274564e1b1`](./2026-10-08.md#bd274564e1b1b3febb3886d274e6d97c2a3d275e) | Add opt-in credential masking for proxied sandboxed sessions (#52302) | `Deferred` |
| 2026-10-08 | [`dd178bb7cabb`](./2026-10-08.md#dd178bb7cabb828b253992e082301fa7bf18c735) | Preserve native errors in Windows sandbox ACL diagnostics (#51896) | `Import proactively` |
| 2026-10-08 | [`ea27864f99f0`](./2026-10-08.md#ea27864f99f0b086cec2f9f0251b7190fb9844f1) | Log Windows sandbox availability and backend selection (#51974) | `Import proactively` |
| 2026-10-09 | [`0c22ce8f2eb5`](./2026-10-09.md#0c22ce8f2eb533c6e570ef173b5a83b1d3e3ad95) | Update the exec-server stable compatibility baseline to Codex 0.162.1 (#52700) | `Import proactively` |
| 2026-10-09 | [`52950c592404`](./2026-10-09.md#52950c592404f11cc245a83a27fb02f2c23bc0c1) | Validate Windows sandbox accounts before password repair (#52682) | `Import proactively` |
| 2026-10-09 | [`6e093c608122`](./2026-10-09.md#6e093c6081223bf264890656ae33886225d09280) | Add timeout-aware exec-server configuration reads (#52679) | `Deferred` |
| 2026-10-09 | [`7efd184c2410`](./2026-10-09.md#7efd184c2410c02a9d0feb4896855f0f18a8f47e) | Add observers for initial exec-server connection attempts (#52724) | `Deferred` |
| 2026-10-09 | [`89c8759d8c49`](./2026-10-09.md#89c8759d8c497b30629a2d67a51c163aa9ed3c44) | Bound local exec-server output draining after process exit (#52652) | `Import proactively` |
| 2026-10-09 | [`b49e2211a579`](./2026-10-09.md#b49e2211a579553d235c88f9c19eb988331cd159) | Handle completion events in either order in shell snapshot tests (#52660) | `Import proactively` |
| 2026-10-09 | [`c63929af5051`](./2026-10-09.md#c63929af50518ca638ccad12f2805298cb7f25a1) | Align Linux sandbox names and docs with bubblewrap and seccomp (#52610) | `Import proactively` |
| 2026-10-09 | [`cb6f6141f86f`](./2026-10-09.md#cb6f6141f86f1db4f45b4932bdc81007719575cc) | Move the exec-server test fixture into shared test support (#52674) | `Import proactively` |
| 2026-10-10 | [`1badea29abf4`](./2026-10-10.md#1badea29abf49e56530a778ac3bb2d3da3bc4d5e) | Keep agent command center rows stable across progress updates (#52946) | `Import proactively` |
| 2026-10-12 | [`12cca49c81c8`](./2026-10-12.md#12cca49c81c8aa6a37a5cc4e39964ee3b70f9458) | Reject duplicate Host headers in the network proxy (#53506) | `Import proactively` |
| 2026-10-12 | [`f60516235442`](./2026-10-12.md#f60516235442c99cb14490b2758e5f7a882b029d) | Preserve Windows runtime registration failure diagnostics (#53426) | `Import proactively` |
| 2026-10-13 | [`20b4ff2f35f2`](./2026-10-13.md#20b4ff2f35f2193608385e4caa8da8ceee45a3cf) | Enrich V2 capability discovery with resolved plugin and skill metadata (#53763) | `Import proactively` |
| 2026-10-13 | [`21c16161191f`](./2026-10-13.md#21c16161191ff965031d1ac41fe9046011610743) | Support Command/Super shortcuts in the TUI keymap (#53754) | `Deferred` |
| 2026-10-13 | [`703d253ed676`](./2026-10-13.md#703d253ed676c26e4b9708a374b7344b5c1bcc53) | Wire V2 capability discovery through exec-server RPC (#53723) | `Import proactively` |
| 2026-10-13 | [`7d0b91d5d686`](./2026-10-13.md#7d0b91d5d686688893fecb356e27fdebaad66ee1) | Separate MCP sandbox policy from launcher metadata (#53709) | `Import proactively` |
| 2026-10-13 | [`923f276f579c`](./2026-10-13.md#923f276f579c64251825ba767ff5bc36a28e36a0) | Keep the Agent command center visible during archive and delete (#53804) | `Import proactively` |
| 2026-10-13 | [`a68a7a6ab052`](./2026-10-13.md#a68a7a6ab05242735b816c71dfa4c3f794c6efd7) | Improve remote exec-server test startup diagnostics (#53668) | `Import proactively` |
| 2026-10-13 | [`cf0ed4c60016`](./2026-10-13.md#cf0ed4c600161fcaa18eeac02b5fc715e1076955) | Acquire and cache opt-in executor capability inventories via V2 (#53736) | `Import proactively` |
| 2026-10-14 | [`80b88a700e43`](./2026-10-14.md#80b88a700e4300022fe1893ecca9b8376d0c85d9) | Avoid false URL-launch detections in literal PowerShell commands (#54076) | `Import proactively` |
| 2026-10-14 | [`9671d20fc82e`](./2026-10-14.md#9671d20fc82ea269d49ecc70b29a8630b29c1a32) | Scope TUI skill refreshes and workspace commands to explicit directories (#54050) | `Import proactively` |
| 2026-10-14 | [`d292f721242e`](./2026-10-14.md#d292f721242ef355ce5a38da90e0e33edc4790e6) | Allow stdin review under active grants with sandbox prompts disabled (#54119) | `Deferred` |
| 2026-10-15 | [`0003732489a5`](./2026-10-15.md#0003732489a514748e61a0f76a652ac992a5f4f5) | Add scoped executor credential auth for remote exec-server (#54346) | `User review required` |
| 2026-10-15 | [`16e578dd1dda`](./2026-10-15.md#16e578dd1ddaf4f5db9cd1f3dbfea0958fd74339) | Add rotating stdin credentials for remote exec-server (#54434) | `Deferred` |
| 2026-10-15 | [`1c5ca2a9791d`](./2026-10-15.md#1c5ca2a9791ded685e94233ee07832e619ec2af2) | Support remote credential masking with Zsh shell snapshots (#54432) | `Deferred` |
| 2026-10-15 | [`4ca84a106966`](./2026-10-15.md#4ca84a10696623fcd7c200db3df9c41671d3bad0) | Allow large file writes over direct exec-server WebSockets (#54135) | `Deferred` |
| 2026-10-15 | [`6b991972c5b5`](./2026-10-15.md#6b991972c5b539198dcea9bac4fc14e57559c3af) | Remove obsolete protected-path monitoring from the Linux sandbox (#54445) | `Import proactively` |
| 2026-10-15 | [`7c152d468447`](./2026-10-15.md#7c152d468447244583f5013887a4bc1371cf1f4c) | Add regression tests for case-variant metadata protection (#54250) | `Import proactively` |
| 2026-10-15 | [`7d33d0d93f9c`](./2026-10-15.md#7d33d0d93f9c751281e0f5e8e9e76353eee87ea8) | Enable credential masking for remote exec-server commands (#54309) | `User review required` |
| 2026-10-15 | [`c313eb75fd93`](./2026-10-15.md#c313eb75fd932f9a3b05461a2a227d7a7f7f06f0) | Prevent duplicate assistant messages around interleaved commands (#54365) | `Import proactively` |
| 2026-10-15 | [`dcd9e954598c`](./2026-10-15.md#dcd9e954598cd5e9f80570204896d779c60c1ecd) | Add `/presets` for reusable prompts and slash commands (#54380) | `Deferred` |
| 2026-10-16 | [`34420d42648f`](./2026-10-16.md#34420d42648f627f766bd347532dcaad5475a68b) | Preserve disconnect reasons in Noise handshake errors (#54637) | `Import proactively` |
| 2026-10-16 | [`a45b8ccb016f`](./2026-10-16.md#a45b8ccb016fb74f63a6bc29e701568e387cbe38) | Add streaming file replacement to filesystem abstractions (#54505) | `Deferred` |
| 2026-10-16 | [`b5115cdd669d`](./2026-10-16.md#b5115cdd669d37a8635a9742851e97ba4fb1d42c) | Resolve stable releases dynamically for exec-server compatibility tests (#54623) | `Import proactively` |
| 2026-10-16 | [`b9775a0d57b0`](./2026-10-16.md#b9775a0d57b09b43e5bab17a245ec053cc6b4426) | Honor home deletion preferences during Windows uninstall (#54708) | `Import proactively` |
| 2026-10-16 | [`e790ce271749`](./2026-10-16.md#e790ce271749ce431e446a9e2cc66485fe60f931) | Use a synchronous mutex for remote metadata request tracking (#54475) | `Deferred` |
| 2026-10-19 | [`0d337e5fe788`](./2026-10-19.md#0d337e5fe78860b25053bb2eb37c3876e566c428) | Fix recall of command-line initial prompts in the TUI (#55372) | `Import proactively` |
| 2026-10-19 | [`431185b78b3d`](./2026-10-19.md#431185b78b3d283cc58b3434988b80e653a0d9bf) | Defer environment disconnect notifications until recovery fails (#55401) | `Import proactively` |
| 2026-10-19 | [`a309f9206aac`](./2026-10-19.md#a309f9206aacc2feffde83ab7d8c50527fddcae3) | Integrate discovery V2 with step-scoped skill catalogs (#55322) | `Import proactively` |
| 2026-10-19 | [`b8f880bc38b8`](./2026-10-19.md#b8f880bc38b885bcca81cdfb50dae0ed18d7c3b4) | Reject replaceable symlinks in pre-sandbox executable lookup (#55384) | `Import proactively` |
| 2026-10-19 | [`c937a6eb7ea1`](./2026-10-19.md#c937a6eb7ea1ea6459da0c971e46c42621273255) | Trace harness-side Noise handshake failures (#55406) | `Import proactively` |
| 2026-10-19 | [`d9a36f6ec704`](./2026-10-19.md#d9a36f6ec7044b6f1e5d2ce7fd364660dc26efee) | Support catalog templates for approved command prefixes (#55366) | `Import proactively` |
| 2026-10-20 | [`5ac988843600`](./2026-10-20.md#5ac9888436003b24f748bb5938629a7bf0bfbc50) | Move Linux sandbox unit tests into separate files (#55620) | `Import proactively` |
| 2026-10-20 | [`d34f104c22ae`](./2026-10-20.md#d34f104c22ae815fc213641b8a637c5947f14ac1) | Reject unpackaged Windows sandbox provisioning clients (#55593) | `Import proactively` |
| 2026-10-20 | [`e18a20d6eb6f`](./2026-10-20.md#e18a20d6eb6f5ede32a69fb56e3fe276d5e9d914) | Add correlated diagnostics for Noise relay handshakes (#55498) | `Import proactively` |
| 2026-10-20 | [`ec8edd576cb4`](./2026-10-20.md#ec8edd576cb4f938e44d3e837ae4e369b44083d5) | Refine enterprise MCP auth errors and request authority checks (#55691) | `Import proactively` |
| 2026-10-21 | [`3206e532d751`](./2026-10-21.md#3206e532d7515c50f22af6dad3a20b9f3c38b639) | Assert shell tool overlap directly in the parallelism test (#56006) | `Import proactively` |
| 2026-10-21 | [`3c4c04b224b5`](./2026-10-21.md#3c4c04b224b55ce0eefca7fb8503b6f128d32b7d) | Add an experimental command to migrate NFS databases to DELETE mode (#55975) | `Deferred` |
| 2026-10-21 | [`4faf60f1265a`](./2026-10-21.md#4faf60f1265a495a68fd792701c60fa334714683) | Pause relay test time while inspecting handshake diagnostics (#55848) | `Import proactively` |
| 2026-10-21 | [`bd5a836567bf`](./2026-10-21.md#bd5a836567bf4182f327f08ac4af626525705806) | Refresh remote exec-server credentials after unauthorized responses (#55987) | `Import proactively` |
| 2026-10-22 | [`2a77e82f0921`](./2026-10-22.md#2a77e82f09212c9e1bbf02656afca5d8172c5f9e) | Support configured credential providers in remote execution (#56049) | `Deferred` |
| 2026-10-22 | [`38da85f49e0a`](./2026-10-22.md#38da85f49e0ab8247d1840f7437ce06fd12d7856) | Preserve config overrides across CLI subcommand boundaries (#56022) | `Import proactively` |
| 2026-10-22 | [`94350a910143`](./2026-10-22.md#94350a910143c406013683ae634708320f93f1f6) | Add managed network review domains before allow rules (#56154) | `User review required` |
| 2026-10-22 | [`96a0d8d551a1`](./2026-10-22.md#96a0d8d551a1419e78c5d83f8dc3bdc066826324) | Use cargo_bin in the Windows sandbox manifest test (#56220) | `Import proactively` |
| 2026-10-22 | [`9a8a85bc5393`](./2026-10-22.md#9a8a85bc539325f2819dd0717f88b6ba6286495e) | Add AWS SigV4 signing to network proxy MITM hooks (#56182) | `Deferred` |
| 2026-10-22 | [`e4deed398182`](./2026-10-22.md#e4deed398182c5275c52bcb77c6733680e2715f5) | Mark built-in dummy credentials and strengthen collision checks (#56297) | `Import proactively` |
| 2026-10-23 | [`d4bdafd378a8`](./2026-10-23.md#d4bdafd378a8e328a67a4cd3557b5fe634af378c) | Preserve `SystemRoot` for Windows filesystem sandbox helpers (#56468) | `Import proactively` |
| 2026-10-23 | [`d8acdf151577`](./2026-10-23.md#d8acdf151577132d4ee1de47a4d1822ae5acb762) | Make sandbox mount cleanup tests tolerate file descriptor reuse (#56454) | `Import proactively` |

### B08 — Dependencies, diagnostics, tests, and maintenance

- Status: `Waiting for user decision`
- Members: 221
- Proactive: 179; Deferred: 40; Review: 2
- Implementation order: adapt or add tests first, manually adapt production code to Ruxex, run affected unit tests and relevant E2E tests, then record the Ruxex commit.

| Date | Upstream commit | Subject | Decision |
| --- | --- | --- | --- |
| 2026-09-19 | [`328feb0c298f`](./2026-09-19.md#328feb0c298ffa093f4b37a6eb7761930caed7d7) | Track executor registrations across connection refresh and recovery (#46555) | `Import proactively` |
| 2026-09-19 | [`50a90f340887`](./2026-09-19.md#50a90f340887e806459aa5536eed7a1bdb99374d) | Skip polling delays in the external queue lifecycle test (#46714) | `Import proactively` |
| 2026-09-19 | [`568518357023`](./2026-09-19.md#568518357023708ecbeddb764ef8956ecaa076d7) | Use captured environment state for permissions and daemon recovery (#46568) | `Import proactively` |
| 2026-09-19 | [`5701a576ccc3`](./2026-09-19.md#5701a576ccc3eb93420334103a9ae66f4c096450) | Retry busy executable launches in packaged daemon tests (#46524) | `Import proactively` |
| 2026-09-19 | [`a5290028a293`](./2026-09-19.md#a5290028a2936b91ec9305f6de7780463620ca70) | Extend server version notices to prerelease and local clients (#46673) | `Deferred` |
| 2026-09-19 | [`c2a924fc4a39`](./2026-09-19.md#c2a924fc4a3970198679d24c9fa5df5643af5041) | Fix stale environment config in the network approval test (#46560) | `Import proactively` |
| 2026-09-19 | [`c7828dd0109c`](./2026-09-19.md#c7828dd0109c75bcb5b07dc2d95c68947e56239a) | Keep step settings and approval environments consistent (#46556) | `Import proactively` |
| 2026-09-19 | [`d086e2752dad`](./2026-09-19.md#d086e2752dadd044213dc617f4e4fe99f3d9b2fa) | Move local child-process spawning into `codex-utils-pty` (#46659) | `Import proactively` |
| 2026-09-19 | [`d5b29951aca5`](./2026-09-19.md#d5b29951aca5205bc8415120b56dd4bc295f5c83) | Centralize retry decisions and delays in `CodexErr` (#46540) | `Import proactively` |
| 2026-09-20 | [`88ef37c64974`](./2026-09-20.md#88ef37c64974e1209411f6d449b0db942b3eb731) | Identify local background servers in `/status` (#46905) | `Deferred` |
| 2026-09-21 | [`0ad18769ad8d`](./2026-09-21.md#0ad18769ad8dfb7a6cf16b1bdd26b6e5f8773f64) | Avoid exposing config values in `codex doctor` errors (#46962) | `Import proactively` |
| 2026-09-21 | [`0e69d3bd9b12`](./2026-09-21.md#0e69d3bd9b1263dbe7a0c564e67fd4a7656dc1a5) | Batch message-board subscriber reads into one SQLite result (#47050) | `Import proactively` |
| 2026-09-21 | [`3627bb91b2f1`](./2026-09-21.md#3627bb91b2f14f47c1a415412ec2f1e132e50647) | Avoid building a host SQLite driver for SQLx macros (#47084) | `Import proactively` |
| 2026-09-21 | [`4d160ea71ddd`](./2026-09-21.md#4d160ea71dddc8e3b7e780b1aa159e79baaa535f) | Index channel posts by timestamp for latest-message lookups (#46979) | `Import proactively` |
| 2026-09-21 | [`51d221a9b77a`](./2026-09-21.md#51d221a9b77ad4810608f148e90231f0e48b5c82) | Stop redundant message-board reads when limits reach one (#47064) | `Import proactively` |
| 2026-09-21 | [`62ea6d41e85d`](./2026-09-21.md#62ea6d41e85d0c75181f2794d0f2dc40e55fce3c) | Serialize environment updates and cancel removed pending attachments (#47077) | `Import proactively` |
| 2026-09-21 | [`ffbd30aeb71a`](./2026-09-21.md#ffbd30aeb71a669045a70e5ff74ed4aa116f79af) | Canonicalize the workspace trust key in the doctor config test (#46981) | `Import proactively` |
| 2026-09-22 | [`0160eb3497bc`](./2026-09-22.md#0160eb3497bc2640c461c7cc5af65836491ded8e) | Preserve explicit message-board unsubscribes when posting (#47257) | `Import proactively` |
| 2026-09-22 | [`3b7a8520efde`](./2026-09-22.md#3b7a8520efdee155234919fcf1f44f05a303df68) | Identify failed SQLite databases in `codex doctor` output (#47330) | `Import proactively` |
| 2026-09-22 | [`3d5cdc51d00c`](./2026-09-22.md#3d5cdc51d00c3ac39db881dc1b3bbebb946191f5) | Test spawn settings reporting after child removal (#47258) | `Import proactively` |
| 2026-09-22 | [`6ea62c4396a1`](./2026-09-22.md#6ea62c4396a1c0942a3ea271e062bdd9e0d2737f) | Resolve proxy routes for standalone web search redirects (#47204) | `Import proactively` |
| 2026-09-22 | [`722dae7afd75`](./2026-09-22.md#722dae7afd75c21b60cd88e5edc8d0726b596603) | Color paths and URLs in `codex doctor` by check status (#47358) | `Deferred` |
| 2026-09-22 | [`75ec81c8623c`](./2026-09-22.md#75ec81c8623c0ca50b129de5a82521a23d37b726) | Enable automatic daemon startup by default (#47179) | `Deferred` |
| 2026-09-22 | [`8a577399c774`](./2026-09-22.md#8a577399c774d1881ef030971d8f271694f55536) | Keep report reason tags within Sentry limits (#47325) | `Import proactively` |
| 2026-09-22 | [`a37f260dc653`](./2026-09-22.md#a37f260dc6534813827f7abc8634788b08d29458) | Preserve empty cell metadata under pending tool-call pressure (#47174) | `Import proactively` |
| 2026-09-22 | [`b04575705206`](./2026-09-22.md#b045757052064f36d64805fbe56ddd426ef91e9b) | Honor system proxy settings for standalone web search (#47142) | `Import proactively` |
| 2026-09-22 | [`d6093d32287b`](./2026-09-22.md#d6093d32287bdcfed2b7f2bbc866ad4b544f8dff) | Offer explicit recovery for incompatible background servers (#47318) | `Deferred` |
| 2026-09-23 | [`0ed77c274b4f`](./2026-09-23.md#0ed77c274b4f8dfecc62af4d3dfd5021546c8e4a) | Add bounded buffering for global operation metrics (#47619) | `Import proactively` |
| 2026-09-23 | [`15922a50aa5f`](./2026-09-23.md#15922a50aa5fae5fb8998ce6079a8d0099b539d0) | Expand Linux spawn-helper lifecycle test coverage (#47613) | `Import proactively` |
| 2026-09-23 | [`4b3b569f3975`](./2026-09-23.md#4b3b569f3975ba4e3bf64589e4d15039066077f8) | Allow WebSocket test server shutdown while waiting for requests (#47698) | `Import proactively` |
| 2026-09-23 | [`50fc133d05bc`](./2026-09-23.md#50fc133d05bc17b9cbf93a71e8d54ae9c8c439f8) | Route pipe processes through the shared child launcher (#47605) | `Import proactively` |
| 2026-09-23 | [`876ce590e179`](./2026-09-23.md#876ce590e179ba8b4adbf5e64b9954a67f6f2fac) | Preserve streamed assistant message order in retained context (#47584) | `Import proactively` |
| 2026-09-23 | [`88b498ded45c`](./2026-09-23.md#88b498ded45c7a7ce6c57b4d8cd48ce9639c0a3e) | Route Linux PTY launches through the process setup helper (#47617) | `Import proactively` |
| 2026-09-23 | [`897b7ae285b2`](./2026-09-23.md#897b7ae285b22e27e64ca27f6295058dd897d13a) | Add a reap-only drop policy for child processes (#47603) | `Import proactively` |
| 2026-09-23 | [`8e17909b2787`](./2026-09-23.md#8e17909b27875b76b1e9a883a604ed24e969d609) | Preserve managed network policy in route-aware transports (#47663) | `Import proactively` |
| 2026-09-23 | [`9477081ccd33`](./2026-09-23.md#9477081ccd334537072d7f3dffe5e05be0f73146) | Support quoted labels and ampersands in Mermaid flowcharts (#47678) | `Deferred` |
| 2026-09-23 | [`9d8de196748b`](./2026-09-23.md#9d8de196748b57d7f463a7757eaa447b6483a331) | Honor Retry-After and preserve server retry deadlines (#47641) | `Import proactively` |
| 2026-09-23 | [`ab7439231c46`](./2026-09-23.md#ab7439231c46f7809721d93081de70c76ca5c70c) | Add executor capability discovery V2 infrastructure (#47683) | `Deferred` |
| 2026-09-23 | [`b7add4df3d95`](./2026-09-23.md#b7add4df3d95e2d41249e54d3c3a1bd14680848a) | Stabilize retry timing and WebSocket tests (#47437) | `Import proactively` |
| 2026-09-23 | [`c44deff7b108`](./2026-09-23.md#c44deff7b1083e9660ac55d02122481f1cdf139b) | Extract WebSocket authentication into `codex-websocket-auth` (#47447) | `Import proactively` |
| 2026-09-23 | [`d47b9a8c002f`](./2026-09-23.md#d47b9a8c002f41433784fef2536892d49c9facfb) | Preserve early unified exec output in completion events (#47665) | `Import proactively` |
| 2026-09-23 | [`db3cb33ecafa`](./2026-09-23.md#db3cb33ecafaae10420995b0ef172503e954c3bc) | Honor network policy in remote control and recover remote execution (#47410) | `Import proactively` |
| 2026-09-23 | [`df3ecee6f0a7`](./2026-09-23.md#df3ecee6f0a7dee6d5aa7ab5c9ba0c1e3c856531) | Launch Linux pipe processes through a fresh setup helper (#47612) | `Import proactively` |
| 2026-09-24 | [`0037b39cc1d5`](./2026-09-24.md#0037b39cc1d5525bd3b92fb3c3341f7a18270e87) | Retry file blob uploads on HTTP 502 and 504 (#47926) | `Import proactively` |
| 2026-09-24 | [`0b78ddb03588`](./2026-09-24.md#0b78ddb03588ca6718ca9b8da34aa4e95dfb9e8d) | Move config key alias normalization ahead of merging (#47903) | `Import proactively` |
| 2026-09-24 | [`1866e5167790`](./2026-09-24.md#1866e516779012d3ff20541d21074f00be010b32) | Make project trust lookup paths explicit and defer root resolution (#47924) | `Import proactively` |
| 2026-09-24 | [`2347a61d0d0e`](./2026-09-24.md#2347a61d0d0ee627f3d2f196443b181c487b55f6) | Add bounded credential-storage telemetry helpers (#47901) | `Import proactively` |
| 2026-09-24 | [`2457571e4cd1`](./2026-09-24.md#2457571e4cd109e602d3ac278d93ec24f1f2e483) | Preserve local-binding inheritance in environment network policies (#47898) | `Import proactively` |
| 2026-09-24 | [`282cd7b01937`](./2026-09-24.md#282cd7b019378746cb87bd91a95d8b4bcae12aa3) | Stabilize Rosetta test timing and retry delay telemetry (#47832) | `Import proactively` |
| 2026-09-24 | [`29f056c26c09`](./2026-09-24.md#29f056c26c09b51db123069ed3ec2095b227d6db) | Honor catalog schemas for asynchronous user input (#47773) | `Import proactively` |
| 2026-09-24 | [`2deb542491ab`](./2026-09-24.md#2deb542491ab25d5dede8c1b288e2818a127985b) | Render remote permission paths using executor context (#47867) | `Import proactively` |
| 2026-09-24 | [`421082e79d31`](./2026-09-24.md#421082e79d3186b0077bccbbbe2c77c0b1a26520) | Avoid blocking async proxy resolution on the system proxy cache (#47852) | `Import proactively` |
| 2026-09-24 | [`4891c4e35fe8`](./2026-09-24.md#4891c4e35fe8b306433f6c0f04dd74bd9e4257e9) | Update unified exec output buffers atomically (#47712) | `Import proactively` |
| 2026-09-24 | [`51d66fdd3b75`](./2026-09-24.md#51d66fdd3b75e1d34fdb82616e7e9c4089684b56) | Support file references in image edit requests (#47956) | `Deferred` |
| 2026-09-24 | [`6989c6548b37`](./2026-09-24.md#6989c6548b3737f108e2bb5ae1171b1d2032e30c) | Fix spawn flag typing and isolate project configuration tests (#47704) | `Import proactively` |
| 2026-09-24 | [`766d2377a8c0`](./2026-09-24.md#766d2377a8c0d31b080f06b680700a1342fd0fc0) | Add an opt-in flag to defer mailbox preemption (#47913) | `Deferred` |
| 2026-09-24 | [`869b5527cd8c`](./2026-09-24.md#869b5527cd8c4a49cffde232f7e47a1c16b0596a) | Fix a lost wakeup in the unified exec termination test (#47814) | `Import proactively` |
| 2026-09-24 | [`8ac495b79988`](./2026-09-24.md#8ac495b79988995cae163d5a67ecd72b78f73629) | Fix sleep interruption test event handling and fixture lifetime (#47813) | `Import proactively` |
| 2026-09-24 | [`975d30c043cd`](./2026-09-24.md#975d30c043cdd03607859503473e045f6a5b476b) | Avoid repeated table clones during config merging (#47902) | `Import proactively` |
| 2026-09-24 | [`a16381c4457e`](./2026-09-24.md#a16381c4457e23191d4786968011434c37a04041) | Align Cargo and Bazel Rust debug information defaults (#47748) | `Import proactively` |
| 2026-09-24 | [`a98a07759a3f`](./2026-09-24.md#a98a07759a3f4eefa9fe375b9551f29f043507f8) | Skip startup prewarm preparation when the WebSocket is ready (#47745) | `Deferred` |
| 2026-09-24 | [`d838c2346d05`](./2026-09-24.md#d838c2346d05f4feb7928ea92e13e83dd479ba10) | Allow idle threads to prewarm and repair WebSocket connections (#47701) | `Deferred` |
| 2026-09-24 | [`df06bb054cc1`](./2026-09-24.md#df06bb054cc1259c64790433a8da2f2cdd8aeed8) | Initialize media estimates outside the global cache lock (#47856) | `Import proactively` |
| 2026-09-24 | [`f0f38b68f728`](./2026-09-24.md#f0f38b68f7287dc7c1b966a459711b3b9ea08172) | Support nested canonical paths in config key aliases (#47904) | `Import proactively` |
| 2026-09-25 | [`17f37f56be7e`](./2026-09-25.md#17f37f56be7e23602c758c4b6039ca32755d3e4d) | Suppress console windows when spawning the code-mode host on Windows (#48138) | `Import proactively` |
| 2026-09-25 | [`1bf73324cadc`](./2026-09-25.md#1bf73324cadc72a53ed467edc7d3fd2b145a6166) | Add opt-in code-mode yielding on new user input (#48135) | `Deferred` |
| 2026-09-25 | [`55543d87724b`](./2026-09-25.md#55543d87724bb66bdd51bde65254feb9b4c9ed10) | Add early yielding for code-mode observations (#48123) | `Deferred` |
| 2026-09-25 | [`60713126ee0d`](./2026-09-25.md#60713126ee0dbc483fba83fc032dfaa5998521ec) | Avoid stdin approval for runtime-only permission grants (#48073) | `Import proactively` |
| 2026-09-25 | [`63eb71c9da95`](./2026-09-25.md#63eb71c9da95a6e4deaf34ee9340c88a7cac9a65) | Preserve late result metadata for truncated code-mode calls (#48222) | `Import proactively` |
| 2026-09-25 | [`888948e5229d`](./2026-09-25.md#888948e5229d094021b2329347098bf376141921) | Optimize `blake3` in Bazel fastbuilds (#48197) | `Import proactively` |
| 2026-09-25 | [`8f5387d10453`](./2026-09-25.md#8f5387d104534d280a3c2b15ef9f5dbb3aa3a758) | Honor execution environment proxy requirements (#48198) | `Import proactively` |
| 2026-09-25 | [`bd3d4d1436bb`](./2026-09-25.md#bd3d4d1436bb41b94fd38ba9bdd34d74524e7a9f) | Preserve user text parts during local compaction (#48115) | `Import proactively` |
| 2026-09-25 | [`f5ffa4695995`](./2026-09-25.md#f5ffa4695995bb6664fe8754ecb5ed2973f2801e) | Preserve queued output for observers during code-mode termination (#48207) | `Import proactively` |
| 2026-09-26 | [`1a89aec960cd`](./2026-09-26.md#1a89aec960cd92e2c59ce49b7f3c3347a915e4a9) | Prevent console windows for piped Windows child processes (#48483) | `Import proactively` |
| 2026-09-27 | [`659b35f1316e`](./2026-09-27.md#659b35f1316eda27ef61850dd0832c4a4e95c120) | Preserve punctuation and semicolons in Mermaid labels (#48814) | `Import proactively` |
| 2026-09-27 | [`814de47b69dd`](./2026-09-27.md#814de47b69dd63a2660fd14f9af66690888d183e) | Centralize persistent mode enablement checks (#48611) | `Import proactively` |
| 2026-09-28 | [`1b1835f751eb`](./2026-09-28.md#1b1835f751ebdc0cfc50b3fe55d4571dbb294563) | Isolate realtime auth fallback test from startup prewarm (#48973) | `Import proactively` |
| 2026-09-28 | [`22d1d9336f75`](./2026-09-28.md#22d1d9336f75d4b6191327d6d32ad2ac2021aba5) | Propagate Cargo package versions to Bazel Rust targets (#49074) | `Import proactively` |
| 2026-09-28 | [`2ce64843be07`](./2026-09-28.md#2ce64843be07eb0bc99c188e8e751776fd86b310) | Update `h2` from 0.4.16 to 0.4.19 in Cargo and Bazel lockfiles (#49096) | `Import proactively` |
| 2026-09-28 | [`33a0f766a647`](./2026-09-28.md#33a0f766a647208b471cfbcad889c67fd324ee04) | Reclaim unused SQLite log database pages in the background (#49069) | `Import proactively` |
| 2026-09-28 | [`69f714055918`](./2026-09-28.md#69f7140559180269e2eb8f5be6e0c20eb37b0c85) | Isolate the memory startup metadata test from Git enrichment (#49000) | `Import proactively` |
| 2026-09-28 | [`7e049b3eaa17`](./2026-09-28.md#7e049b3eaa17cf01f726ee8ff70da4dcf7557f87) | Balance Windows Bazel test shards using duration estimates (#49103) | `Import proactively` |
| 2026-09-28 | [`9563713df2f7`](./2026-09-28.md#9563713df2f748e7bd0792e96bbb2a49b916e66b) | Notify lifecycle extensions of compaction usage limits (#49097) | `Import proactively` |
| 2026-09-28 | [`c2d2f422e63e`](./2026-09-28.md#c2d2f422e63e5d4db83e0da93858b5a3e2ad12e1) | Preserve SQLite vacuum modes and surface pool initialization errors (#49102) | `Import proactively` |
| 2026-09-29 | [`69043f05c4c5`](./2026-09-29.md#69043f05c4c5e2afd0cec7aaf0dbc8166f7d63b5) | Point remote compaction tests at the mock ChatGPT server (#49114) | `Import proactively` |
| 2026-09-29 | [`87d3e068478c`](./2026-09-29.md#87d3e068478cb299cb25937e9793b5e5d5a7c668) | Preserve live tool-call metadata across request windows (#49401) | `Import proactively` |
| 2026-09-29 | [`a6f09397aa59`](./2026-09-29.md#a6f09397aa591f6ea9f02037c55c10a8cbf6f61f) | Compact the inline hidden tag buffer once per chunk (#49300) | `Import proactively` |
| 2026-09-29 | [`bd4204efc24a`](./2026-09-29.md#bd4204efc24a888c6671681f5cd41fbf9377c163) | Compile hook matchers during discovery (#49379) | `Import proactively` |
| 2026-09-29 | [`cf12c86dc569`](./2026-09-29.md#cf12c86dc569e383c419e821046a8e59b4c8614a) | Simplify configuration fingerprint canonicalization (#49295) | `Import proactively` |
| 2026-09-30 | [`16a7c0f0ebc6`](./2026-09-30.md#16a7c0f0ebc6e366b3ecee183d21e02b5a100b7b) | Filter graceful shutdown guard and trigger traces from SQLite logs (#49414) | `Import proactively` |
| 2026-09-30 | [`17e5a4dbcb41`](./2026-09-30.md#17e5a4dbcb4106955dad3e6a661459c77193ea1d) | Upgrade the argument comment lint toolchain and Dylint (#49706) | `Do not import` |
| 2026-09-30 | [`1fc8d548077f`](./2026-09-30.md#1fc8d548077fc72c4e3d048a78173af07385217f) | Make the strict network approval test independent of request order (#49595) | `Import proactively` |
| 2026-09-30 | [`5e96aabd68dc`](./2026-09-30.md#5e96aabd68dc6194f5ad928cb7f5d0bc71b72df9) | Avoid full-string scans in token-budget truncation (#49712) | `Import proactively` |
| 2026-09-30 | [`8ea2c0e0d4bc`](./2026-09-30.md#8ea2c0e0d4bc1261258eb532b5a9d616147b6571) | Prune diagnostic logs periodically by age and database size (#49425) | `Import proactively` |
| 2026-09-30 | [`ab84d71f5767`](./2026-09-30.md#ab84d71f5767e4a565ce81c2c426287cb48c7918) | Omit payloads from multiline ANSI warnings (#49416) | `Import proactively` |
| 2026-09-30 | [`d940fb21f35e`](./2026-09-30.md#d940fb21f35ef32a736016a4c196150c6380bdb3) | Truncate input text in protocol debug output (#49415) | `Import proactively` |
| 2026-10-01 | [`236be1ad9f61`](./2026-10-01.md#236be1ad9f6169269bbad436a0eca41d21948680) | Improve diagnostics for report attachment failures (#49852) | `Import proactively` |
| 2026-10-01 | [`33aea33b39a5`](./2026-10-01.md#33aea33b39a5e35c78de75c332106479b16c6994) | Update the Rust toolchain action for argument-comment linting (#49801) | `Import proactively` |
| 2026-10-01 | [`3bbf8ec3a1e1`](./2026-10-01.md#3bbf8ec3a1e103c30d575f14f1827b193a8d415f) | Add a requirements feature gate for the browser annotation API (#49784) | `Deferred` |
| 2026-10-01 | [`b44ca872b105`](./2026-10-01.md#b44ca872b1059c65f0eb6a7534e9e8ae55df54a8) | Persist world-state snapshots alongside rendered context (#49847) | `Deferred` |
| 2026-10-01 | [`d9cf20389066`](./2026-10-01.md#d9cf203890663ee27969b8e0735e08cf35010bbf) | Check token estimate filtering directly on the tracing subscriber (#50054) | `Import proactively` |
| 2026-10-01 | [`dd90f160ed9b`](./2026-10-01.md#dd90f160ed9bf91320c417335ddff848b6a23dd0) | Prevent stale file search results from being labeled with a new query (#49946) | `Import proactively` |
| 2026-10-02 | [`2739e828581c`](./2026-10-02.md#2739e828581c3deee59a270cd740675dc278ed14) | Allow opting into 16 KiB ARM64 code pages for macOS signing (#50384) | `Deferred` |
| 2026-10-02 | [`7135b303d918`](./2026-10-02.md#7135b303d918fc80fecb8053a92173bd211d2c0a) | Upgrade `age` to 0.12.1 and remove the obsolete advisory exception (#50166) | `Import proactively` |
| 2026-10-02 | [`8d44977aa2fb`](./2026-10-02.md#8d44977aa2fb9ae1b128660668dc5b36966613fa) | Add opt-in JSON diagnostics for TCP tunnels (#50131) | `Import proactively` |
| 2026-10-02 | [`a0bfaac70ccd`](./2026-10-02.md#a0bfaac70ccdd14291763d832f6f7f3f4558e662) | Add the `incremental_tools` feature flag (#50464) | `Deferred` |
| 2026-10-02 | [`a4bfd07d51fa`](./2026-10-02.md#a4bfd07d51fa941d70e41cc0345b631ab5a41224) | Stabilize paused-time code-mode service tests (#50443) | `Import proactively` |
| 2026-10-02 | [`b997c72a99cd`](./2026-10-02.md#b997c72a99cd889feafff4a607c630c734d7192b) | Add `dots` to issue labeler guidance (#50183) | `Deferred` |
| 2026-10-02 | [`f4e18a95bb04`](./2026-10-02.md#f4e18a95bb0496aeee91ffee27092d2d215c3898) | Skip unrelated subtrees during config alias normalization (#50354) | `Import proactively` |
| 2026-10-03 | [`19e554bb7010`](./2026-10-03.md#19e554bb70103b17aa7dca6bbd0dbf1c8b2eddb6) | Avoid reading the current directory when resolving absolute paths (#50558) | `Import proactively` |
| 2026-10-03 | [`550eb50545a7`](./2026-10-03.md#550eb50545a78468a09ce86b82426338640ef22d) | Keep environment-backed tools exposed across readiness changes (#50741) | `Import proactively` |
| 2026-10-03 | [`86a54b051c08`](./2026-10-03.md#86a54b051c08f34f373c507ae16a91915ab08700) | Add scenario coverage for remote `/compact` context preservation (#50516) | `Deferred` |
| 2026-10-03 | [`b172810921f8`](./2026-10-03.md#b172810921f89847cd310ecc496f9c901760e933) | Let the transport create the Windows remote-control socket directory (#50700) | `Import proactively` |
| 2026-10-05 | [`54dd21741d52`](./2026-10-05.md#54dd21741d52cc8a25f44a518a2699c60f25928d) | Allow concurrent release builds while serializing publication (#51198) | `Deferred` |
| 2026-10-05 | [`5cbcf810e5f4`](./2026-10-05.md#5cbcf810e5f4e77a34989075f99721d1028ce492) | Prevent stable release pointers from moving backward (#51186) | `Import proactively` |
| 2026-10-05 | [`aa6635ece5ac`](./2026-10-05.md#aa6635ece5acecb7dac021b55132d37708f9a9f3) | Retry transient gRPC code-mode session admission failures (#51185) | `Import proactively` |
| 2026-10-06 | [`28b91c7c3116`](./2026-10-06.md#28b91c7c311600edcc5f4e2831ca3ec35ca9c83d) | Honor the OTLP metrics temporality preference (#51220) | `Import proactively` |
| 2026-10-06 | [`44984d20817f`](./2026-10-06.md#44984d20817fc58026a8bbccf053e288c6897c8f) | Retry realtime sideband attachment while an existing call activates (#51460) | `Import proactively` |
| 2026-10-06 | [`9479e1fdb739`](./2026-10-06.md#9479e1fdb7396c3e3254f5e123489af829186893) | Bind capability roots to environment selections (#51493) | `User review required` |
| 2026-10-06 | [`f6cf05af1dc7`](./2026-10-06.md#f6cf05af1dc701322664bb3f22ee4dafbc733ffc) | Honor Retry-After in WebSocket error events (#51440) | `Import proactively` |
| 2026-10-07 | [`4aaee872e31a`](./2026-10-07.md#4aaee872e31abefe0d32e91faab23b09b6968824) | Add completion-aware realtime attachment and session-scoped detach (#51539) | `Deferred` |
| 2026-10-07 | [`8f21b7fffbef`](./2026-10-07.md#8f21b7fffbefe77529325fc86c95289371841c6c) | Remove duplicate field initializers in extension registry tests (#51800) | `Import proactively` |
| 2026-10-08 | [`0b755b1945bf`](./2026-10-08.md#0b755b1945bf4a31560df2ce1469aeb044dccbc9) | Emit one developer notice per incremental namespace update batch (#52295) | `Import proactively` |
| 2026-10-08 | [`137f25f6439c`](./2026-10-08.md#137f25f6439ce8be758f15dbd1e1b297c6cd0c3b) | Recover gRPC code-mode sessions after missing-session errors (#52235) | `Import proactively` |
| 2026-10-08 | [`14ff9c3a43eb`](./2026-10-08.md#14ff9c3a43ebf9bf543cf5f7c45cff2bfea2e512) | Preserve UTF-8 byte semantics in network domain matching (#52277) | `Import proactively` |
| 2026-10-08 | [`19b7bffd7bd5`](./2026-10-08.md#19b7bffd7bd5c325a45b91111ce64c85610b90ba) | Cover instant interruption in steered-input persistence tests (#52048) | `Import proactively` |
| 2026-10-08 | [`48bd7667ca27`](./2026-10-08.md#48bd7667ca270a8ce7e3e1aa57f88a0f54aee15b) | Add scenario coverage for worker context across idle eviction (#52062) | `Deferred` |
| 2026-10-08 | [`529cd6b8602f`](./2026-10-08.md#529cd6b8602f77ce946502d2e67df6b41bc795af) | Use a dedicated matcher for network domain policies (#51897) | `Deferred` |
| 2026-10-08 | [`77c01c1d3708`](./2026-10-08.md#77c01c1d37088f97cc9621c50c584f27214beb21) | Remove duplicate field initializers from extension registry tests (#52288) | `Import proactively` |
| 2026-10-08 | [`82e70121f86b`](./2026-10-08.md#82e70121f86bc1f6fea7f2bb7bbc169d259b3c6b) | Honor the user input setting for asynchronous questions (#51908) | `Import proactively` |
| 2026-10-08 | [`8c818cbed4bc`](./2026-10-08.md#8c818cbed4bc83d30f5d515674b809646fe2676d) | Make environment context date and timezone configurable (#52164) | `Deferred` |
| 2026-10-08 | [`a645742bb561`](./2026-10-08.md#a645742bb5615026ae673c9527f146ee551d4949) | Stop bundling patched zsh in Codex packages (#52213) | `Import proactively` |
| 2026-10-08 | [`c9e0fffbbc05`](./2026-10-08.md#c9e0fffbbc05c36478686444f6ab1edbadfe2768) | Enable parallel execution for read-only tools (#52245) | `Deferred` |
| 2026-10-08 | [`d570b3632b10`](./2026-10-08.md#d570b3632b100e36f931797cdbdb8906d0fcafa8) | Enforce application network policy for daemon updates (#52223) | `User review required` |
| 2026-10-08 | [`e9e6cf63492d`](./2026-10-08.md#e9e6cf63492da108a3cabaf3cc7b23d0640ef4cf) | Strip symbols from source-built release package binaries (#52178) | `Import proactively` |
| 2026-10-08 | [`fac5d0ba91b9`](./2026-10-08.md#fac5d0ba91b991ddb5a0f049289acd84135927b5) | Report specific reasons for WebSocket continuation failures (#51895) | `Import proactively` |
| 2026-10-09 | [`2dc0791c7d2c`](./2026-10-09.md#2dc0791c7d2cb4decda6b8ebd8b54d38d7411f64) | Add opt-in gRPC over stdio for the code-mode host (#52723) | `Deferred` |
| 2026-10-09 | [`4bc83aa5f7eb`](./2026-10-09.md#4bc83aa5f7ebfec8492bb5747affe0fc8433fcad) | Retry bootstrap GETs through the system proxy after request failures (#52702) | `Import proactively` |
| 2026-10-09 | [`64335d1afd83`](./2026-10-09.md#64335d1afd83f3aaabd6a3a58ebbdf0bf7b994ff) | Upgrade `hyper` to 1.11.1 and test connection closure (#52608) | `Import proactively` |
| 2026-10-09 | [`65b82cdf96f9`](./2026-10-09.md#65b82cdf96f9d305494bc8c6c8d2404d37709030) | Prevent brokered credential aliases from bypassing MITM hooks (#52661) | `Import proactively` |
| 2026-10-09 | [`aab4f9614877`](./2026-10-09.md#aab4f9614877c45e651229446b52358e1de6368c) | Refresh persisted capability roots from owner-provided configuration (#52676) | `Import proactively` |
| 2026-10-10 | [`165299214855`](./2026-10-10.md#165299214855c2e48facf9bb3bc3c0fc277ee639) | Make code-mode `exit()` stop the entire cell (#52748) | `Deferred` |
| 2026-10-10 | [`806d9732c974`](./2026-10-10.md#806d9732c974bc8a51b8317c1bd8985544fe627c) | Report exec runtime resets before accepting further requests (#52825) | `Import proactively` |
| 2026-10-12 | [`0c7362d121d1`](./2026-10-12.md#0c7362d121d1c7f69540262425fd4e2209fd8e8d) | Ignore the unsafe-libyaml advisory in dependency audits (#53382) | `Import proactively` |
| 2026-10-12 | [`16845bb8e85d`](./2026-10-12.md#16845bb8e85d781f6e745bdec011384450929ff9) | Isolate arg0 helper directories by operating system (#53412) | `Import proactively` |
| 2026-10-12 | [`4a93bd249a4e`](./2026-10-12.md#4a93bd249a4e73e24c3cc889053c947c6986362a) | Update `quinn-proto` to 0.11.18 (#53485) | `Import proactively` |
| 2026-10-12 | [`6e5f665feb9b`](./2026-10-12.md#6e5f665feb9b71e1500a5721036fbdb8e871f4b0) | Test final polling of remote processes with open output streams (#53339) | `Import proactively` |
| 2026-10-12 | [`7e9c061ceb38`](./2026-10-12.md#7e9c061ceb388c67097acac069d6b7f8a60e5011) | Finish remote exec cleanup when output streams remain open (#53331) | `Import proactively` |
| 2026-10-12 | [`97239fd052db`](./2026-10-12.md#97239fd052db455bd08c41afcdab738e0fe83b87) | Add explicit local board connections with fixed runtime access (#53400) | `Import proactively` |
| 2026-10-13 | [`1a69ad3237fc`](./2026-10-13.md#1a69ad3237fc485273ccfa9731239fdf789b43e4) | Add infrastructure for speculative remote V2 compaction (#53614) | `Deferred` |
| 2026-10-13 | [`4e7ef7d19d59`](./2026-10-13.md#4e7ef7d19d59c51ac04e62d062664bfd012d76da) | Keep spawned children provisional until the spawn result returns (#53654) | `Import proactively` |
| 2026-10-13 | [`669449b02c4b`](./2026-10-13.md#669449b02c4b58de4bcd22485eb60c98a6e38a54) | Add async compaction feature configuration (#53595) | `Deferred` |
| 2026-10-13 | [`815ef9bbc47b`](./2026-10-13.md#815ef9bbc47bef3bde9cfceaaaf3ef74bb43d052) | Fix cancellation of macOS user-verification prompts (#53790) | `Import proactively` |
| 2026-10-13 | [`9167a0b2d4a7`](./2026-10-13.md#9167a0b2d4a74d44a4d388057d304682e47313c6) | Reuse background results for remote V2 compaction (#53620) | `Import proactively` |
| 2026-10-13 | [`ab82af0b2cfc`](./2026-10-13.md#ab82af0b2cfc1e894de72f5971b79d451dc2b337) | Share hook configuration across cloned hook views (#53657) | `Import proactively` |
| 2026-10-13 | [`b043762f9e3e`](./2026-10-13.md#b043762f9e3eb65dd5164ff5880f139bc6d084c7) | Preserve remote exec output until it is collected (#53622) | `Import proactively` |
| 2026-10-13 | [`be5296d4672f`](./2026-10-13.md#be5296d4672f59205c1b8a76f46e74eae4f10d51) | Prepare compaction helpers for background requests (#53606) | `Import proactively` |
| 2026-10-14 | [`0c5720ea0fd2`](./2026-10-14.md#0c5720ea0fd2af84d7b6743a6686750fb8570ab7) | Keep environment tools stable across readiness changes (#54098) | `Import proactively` |
| 2026-10-14 | [`0d58012ec20d`](./2026-10-14.md#0d58012ec20d0f42ea1c11056815e4a0c5122e99) | Test memory extraction budgets with excluded items and ciphertext (#53963) | `Import proactively` |
| 2026-10-14 | [`1ffa95ee8a02`](./2026-10-14.md#1ffa95ee8a02ec68f6ccd758998bcb7d4b32bcdb) | Wait for readiness in the SIGTERM cancellation test (#54099) | `Import proactively` |
| 2026-10-14 | [`71923ba3051d`](./2026-10-14.md#71923ba3051dc99d3a617c256f686d7415fc1cfc) | Update `gix` to 0.89.0 and `gix-url` to 0.40.0 (#54078) | `Import proactively` |
| 2026-10-14 | [`7a08c5f6271e`](./2026-10-14.md#7a08c5f6271e818d832eb9061dac935936e2b539) | Avoid reconnect races in the daemon restart test (#53969) | `Import proactively` |
| 2026-10-14 | [`80ad49e3b7ca`](./2026-10-14.md#80ad49e3b7ca466cd35fc012bf1e23bed836de72) | Coalesce overlapping native TLS root certificate loads (#54056) | `Import proactively` |
| 2026-10-14 | [`8cbce633876e`](./2026-10-14.md#8cbce633876eb71be1e897c1599c713cec7a262c) | Bound concurrent Git diffs for untracked files (#54032) | `Import proactively` |
| 2026-10-14 | [`954750be9c40`](./2026-10-14.md#954750be9c4044e7ef2fea18e22648e8c6c4cb0f) | Bump `rustls` to `0.23.46` (#53996) | `Import proactively` |
| 2026-10-14 | [`97771b1d78cf`](./2026-10-14.md#97771b1d78cf6366bc3946fe9e4bbca4ebee31d1) | Isolate daemon package tests to reduce disk contention (#53897) | `Import proactively` |
| 2026-10-14 | [`ae110493973d`](./2026-10-14.md#ae110493973d5b72f60d42efe2fae91d51ca9c0e) | Expose gateway authentication as a reusable OAuth session API (#54070) | `Deferred` |
| 2026-10-14 | [`c0cfbfbb989f`](./2026-10-14.md#c0cfbfbb989ff80c4027c8ea86a15d411d1d5214) | Extend memory extension resource retention to 14 days (#54033) | `Import proactively` |
| 2026-10-14 | [`f87ba04ec3cd`](./2026-10-14.md#f87ba04ec3cd386a92e775405b252a310b40bb0d) | Report the reason for ChatGPT token refresh requests (#53991) | `Import proactively` |
| 2026-10-15 | [`1ef30bcbe784`](./2026-10-15.md#1ef30bcbe784f1006548adc1542985be32b05109) | Preserve existing SQLite journal modes on startup (#54389) | `Import proactively` |
| 2026-10-15 | [`3bae578af33e`](./2026-10-15.md#3bae578af33e43268e7bc2d6d5d0266f704bb554) | Protect metadata writes through macOS case aliases (#54273) | `Import proactively` |
| 2026-10-15 | [`7ae831991792`](./2026-10-15.md#7ae831991792744434027c431dfbc7e81dd5c12c) | Retry SQLite operations on transient writer contention (#54369) | `Import proactively` |
| 2026-10-15 | [`9c84e89ce089`](./2026-10-15.md#9c84e89ce0893a552058d26dc6746ff6a40ea6e6) | Skip SQLite page reclamation on Linux NFS mounts (#54417) | `Import proactively` |
| 2026-10-16 | [`221e4e7c0538`](./2026-10-16.md#221e4e7c05386d06aa7b91da2a9b04f7f2bd921f) | Preserve prereleased events when creating GitHub prereleases (#54724) | `Import proactively` |
| 2026-10-16 | [`527c5deb74e5`](./2026-10-16.md#527c5deb74e5cc390f8838ac361e340cb23c2234) | Close arg0 lock files before removing temporary directories (#54739) | `Import proactively` |
| 2026-10-16 | [`6ae8b8aa7c60`](./2026-10-16.md#6ae8b8aa7c608730d4315d59c5c081897ae02780) | Upgrade V8 to 152.2.0 and backport Array.sort safety guards (#54519) | `Import proactively` |
| 2026-10-16 | [`835d3049f03e`](./2026-10-16.md#835d3049f03ea71491299b2548c5ab8fa932f897) | Enable BuildBuddy for voice release builds (#54737) | `Deferred` |
| 2026-10-16 | [`859bbc87fc80`](./2026-10-16.md#859bbc87fc80c18a72e08b181e301ddb4f8dabea) | Keep prereleases in draft until release assets finish uploading (#54716) | `Import proactively` |
| 2026-10-16 | [`b3cc59afcf28`](./2026-10-16.md#b3cc59afcf28bf8f8cc35023be67b9e2f62fc066) | Retry npm release publishing and match macOS verification runners (#54641) | `Import proactively` |
| 2026-10-19 | [`18d09e1eda68`](./2026-10-19.md#18d09e1eda6811effcb4acc110795eec7e970135) | Add a shared cleanup lock guard for local log directories (#55358) | `Deferred` |
| 2026-10-19 | [`1ccc0d64e80f`](./2026-10-19.md#1ccc0d64e80ff5a189d072257a0198ec63857b1d) | Clarify API key login and status messages (#55407) | `Import proactively` |
| 2026-10-19 | [`4635d489254b`](./2026-10-19.md#4635d489254ba6d80eb6d2ef501929b0c617b6ec) | Stabilize daemon reconnect and Rosetta package startup tests (#55274) | `Import proactively` |
| 2026-10-19 | [`71ab2ea77bc7`](./2026-10-19.md#71ab2ea77bc7fca41bb641e8a131bcba7b304a21) | Stop memory dual-write when v2 is selected (#55262) | `Import proactively` |
| 2026-10-19 | [`770ed6dcea40`](./2026-10-19.md#770ed6dcea40fedb83bb831f6d5e3760a933cc9a) | Allow plus signs in metric tags (#55227) | `Import proactively` |
| 2026-10-19 | [`89461ea68338`](./2026-10-19.md#89461ea6833832b0551ce24ee32205824d5c108d) | Prune unused standalone releases after successful installation (#55149) | `Import proactively` |
| 2026-10-19 | [`aeec726de8c0`](./2026-10-19.md#aeec726de8c08c7a13ecaf7e4e46d592e25a8222) | Reduce path storage in the file-search index (#55351) | `Import proactively` |
| 2026-10-19 | [`bb8f5304fcd9`](./2026-10-19.md#bb8f5304fcd9f901d8e0747fccb8c8904cbc504e) | Fall back when macOS rejects high stdio descriptors (#55278) | `Import proactively` |
| 2026-10-19 | [`cc1a4ec76373`](./2026-10-19.md#cc1a4ec7637310af92313cf3ae0da729adddb23a) | Stop the closest remote SHA search at zero distance (#55302) | `Import proactively` |
| 2026-10-20 | [`18795fc12571`](./2026-10-20.md#18795fc12571a3530d3e4ac18366e41cd2f57448) | Support Windows registry overrides for `requirements.toml` (#55456) | `Deferred` |
| 2026-10-20 | [`46d252f8bf8b`](./2026-10-20.md#46d252f8bf8b09f4ef765bc72e40f02c229da3e6) | Make the startup migration contention test deterministic (#55524) | `Import proactively` |
| 2026-10-20 | [`4dbd9d30ea04`](./2026-10-20.md#4dbd9d30ea0466ec5bb1524f24c56cfc8ee4c5b1) | Ignore the basic-toml advisory in dependency audits (#55547) | `Import proactively` |
| 2026-10-20 | [`61469096fe08`](./2026-10-20.md#61469096fe086ccdbe87c06f9efc4ee59a4b51fe) | Update `rustls-webpki` and `webpki-roots` lockfile versions (#55616) | `Import proactively` |
| 2026-10-20 | [`7a152fbcf9ec`](./2026-10-20.md#7a152fbcf9ecd32d9fb4b45edf80127ee4c8499e) | Treat code-mode collaboration messages as plaintext (#55633) | `Import proactively` |
| 2026-10-20 | [`87f3c3284e16`](./2026-10-20.md#87f3c3284e1669b90ad9f4925039e9e3bc769b61) | Track code-mode cell lifecycle and result buffering time (#55653) | `Import proactively` |
| 2026-10-20 | [`c6e4438a5d5b`](./2026-10-20.md#c6e4438a5d5befe6c8e59b01b6534eb8f0792230) | Guard auth persistence and reloads against concurrent login changes (#55646) | `Import proactively` |
| 2026-10-20 | [`e8aa7c1678ca`](./2026-10-20.md#e8aa7c1678caa9a368243d72bf0e9b7859d97f15) | Avoid panics after background compaction fails before a checkpoint (#55581) | `Import proactively` |
| 2026-10-21 | [`82ef0672e343`](./2026-10-21.md#82ef0672e34391ffe1bd61e1185ed6e9d664a64e) | Scope MITM credential substitution to trusted origins and placeholders (#56003) | `Import proactively` |
| 2026-10-21 | [`8d06d62e15eb`](./2026-10-21.md#8d06d62e15ebbe1ad42324232b6e8a7e2e432793) | Allow consumers to select message-board tools (#55973) | `Deferred` |
| 2026-10-21 | [`af91f6ef2cd6`](./2026-10-21.md#af91f6ef2cd66ead278ebe7d5751973eef430aee) | Add a critical-path utility for exclusive duration attribution (#55804) | `Deferred` |
| 2026-10-21 | [`d51e9c935fab`](./2026-10-21.md#d51e9c935fab31e13f7b576a026ae56fb18603bf) | Preserve per-execution trace context in code-mode stdio callbacks (#55993) | `Import proactively` |
| 2026-10-21 | [`d5bc4161b658`](./2026-10-21.md#d5bc4161b658cff3f2176aa422a52b3a06928214) | Add a Windows proxy authentication tunnel helper (#55994) | `Deferred` |
| 2026-10-21 | [`db58a987b8a0`](./2026-10-21.md#db58a987b8a0b94b81ab321f0e0192757758df67) | Allow unified exec to fall back when credential masking is unavailable (#55719) | `Deferred` |
| 2026-10-21 | [`f496b09bd141`](./2026-10-21.md#f496b09bd141af0a6ec1a7ebf3e5fc557ac795dc) | Retry proxy CONNECT challenges with Windows authentication (#56000) | `Import proactively` |
| 2026-10-22 | [`004c13292f3e`](./2026-10-22.md#004c13292f3ef05e51e2e90e5a2dc6b13b651916) | Fix entrypoint lookup in package debug-symbol smoke tests (#56032) | `Import proactively` |
| 2026-10-22 | [`129220bf057d`](./2026-10-22.md#129220bf057daf978e097d5395295d0a2f2f07c8) | Support bounded request bodies in AWS SigV4 signing (#56277) | `Import proactively` |
| 2026-10-22 | [`5a1c247dd5fd`](./2026-10-22.md#5a1c247dd5fd682c22a34af93df697dd06300a32) | Decouple managed configuration loading from ChatGPT authentication (#56295) | `Import proactively` |
| 2026-10-22 | [`66f6da603648`](./2026-10-22.md#66f6da60364847b5244d550497fb742cbbae4119) | Lock Python release build tools with uv (#56290) | `Import proactively` |
| 2026-10-22 | [`85de07048c21`](./2026-10-22.md#85de07048c21c349f491e24b4eb4e5b674fd5a28) | Isolate V8 source dependencies and their C++ runtime settings (#56261) | `Import proactively` |
| 2026-10-22 | [`8eaf5fd115f0`](./2026-10-22.md#8eaf5fd115f036d207be236009938261758e30f2) | Bump uv to 0.13.5 in CI and release workflows (#56248) | `Import proactively` |
| 2026-10-22 | [`9100b9c3d38c`](./2026-10-22.md#9100b9c3d38c0a9c33653f1760be307f195ee7a6) | Pin Linux artifact signing to Cosign v3.1.4 (#56288) | `Import proactively` |
| 2026-10-22 | [`b4ad33d1a3c1`](./2026-10-22.md#b4ad33d1a3c10c9652dd57f7afd7a949f8f26895) | Cache decrypted Codex auth and managed secrets (#56081) | `Import proactively` |
| 2026-10-22 | [`ca87ef86e101`](./2026-10-22.md#ca87ef86e101c4e0b18847116fadcff078abaeaa) | Wait for cloud config refresh cancellation in loader lifetime tests (#56274) | `Import proactively` |
| 2026-10-22 | [`e8bb1b664170`](./2026-10-22.md#e8bb1b664170a123af4b844a47e6c6e992b52fe2) | Limit decoded HTTP/3 response headers in the TCP tunnel (#56024) | `Import proactively` |
| 2026-10-22 | [`eca79b000c48`](./2026-10-22.md#eca79b000c48bace74eaef474ec4c5d1ae5ba34a) | Use fixed-token auth fixtures in Vertex tests (#56293) | `Import proactively` |
| 2026-10-23 | [`0d7f3ea03b9e`](./2026-10-23.md#0d7f3ea03b9ef73a42136fb071eb16266966ea49) | Initialize SQLite databases with DELETE journaling on NFS (#56552) | `Import proactively` |
| 2026-10-23 | [`15fbcac045c2`](./2026-10-23.md#15fbcac045c29f77783bb176c20f54102f97fe32) | Enable masked AWS credentials for local proxy signing (#56554) | `Deferred` |
| 2026-10-23 | [`75a4146411d2`](./2026-10-23.md#75a4146411d22e4c487ba86f026201594cdbaa85) | Add support for environment-scoped managed hooks (#56304) | `Deferred` |
| 2026-10-23 | [`7c568b331b9e`](./2026-10-23.md#7c568b331b9e0810ae7164f4bd680cec5f69f2fc) | Extract the head-tail buffer into a utility crate (#56535) | `Import proactively` |
| 2026-10-23 | [`ca9d103f39fd`](./2026-10-23.md#ca9d103f39fde8a65f84b5e65eb9a30268cf70fb) | Update `ctutils` from 0.4.2 to 0.4.5 (#56439) | `Import proactively` |
| 2026-10-23 | [`dc4844c907f8`](./2026-10-23.md#dc4844c907f8ce118c52619ed139f296dcb37705) | Add TLS forwarding tests for MITM SigV4 requests (#56301) | `Import proactively` |
| 2026-10-23 | [`f3a90545aed3`](./2026-10-23.md#f3a90545aed3188ee1e5c119ec89a742de399cdc) | Add managed configuration loaders for custom config servers (#56314) | `Deferred` |

## Excluded scope

Entries classified `Do not import` remain in their date ledgers and are intentionally absent from implementation bundles. They target removed functionality, unsupported Codex-only surfaces, or infrastructure with no retained Ruxex equivalent.

## Next action

Review this reconstruction by bundle, resolve the explicit review items, then implement one small bundle at a time. Do not cherry-pick upstream commits; use the linked diffs to adapt tests and implementation manually.
