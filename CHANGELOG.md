# Changelog

## [63.3.0](https://github.com/MagiCrazy/neumann-cockpit/compare/neumann-cockpit-v63.2.0...neumann-cockpit-v63.3.0) (2026-07-03)


### Features

* **boot:** auto-continue after the self-check + boot config key ([#141](https://github.com/MagiCrazy/neumann-cockpit/issues/141)) ([ec45226](https://github.com/MagiCrazy/neumann-cockpit/commit/ec45226f4f618af6882de369585c79a28d2a78ba))
* **cockpit:** 1-9 accelerators in the contextual menu ([#137](https://github.com/MagiCrazy/neumann-cockpit/issues/137)) ([bd1c81c](https://github.com/MagiCrazy/neumann-cockpit/commit/bd1c81c1a5267162aea7e6166b5031de92c563ea))
* **cockpit:** paging + jump on pane lists, and l-open hints ([#142](https://github.com/MagiCrazy/neumann-cockpit/issues/142)) ([5f73b18](https://github.com/MagiCrazy/neumann-cockpit/commit/5f73b188638c47fb75aee187999fb101c519fe43))
* **help:** bigger scrollable help + command-mode section ([#138](https://github.com/MagiCrazy/neumann-cockpit/issues/138)) ([16dac18](https://github.com/MagiCrazy/neumann-cockpit/commit/16dac186987c3158302e7601a85fbb7cd7cc3cd9))


### Bug Fixes

* **cockpit:** render the command-line caret at the cursor ([#136](https://github.com/MagiCrazy/neumann-cockpit/issues/136)) ([786faf4](https://github.com/MagiCrazy/neumann-cockpit/commit/786faf4587bfa11744d42c4a7f3a5dabdc9c6247))
* **theme:** make urgency signals stand out in the mono palettes ([#140](https://github.com/MagiCrazy/neumann-cockpit/issues/140)) ([68e2931](https://github.com/MagiCrazy/neumann-cockpit/commit/68e2931634c646fe65c86ad5f4d49d4e6ab2830e))

## [63.2.0](https://github.com/MagiCrazy/neumann-cockpit/compare/neumann-cockpit-v63.1.4...neumann-cockpit-v63.2.0) (2026-07-03)


### Features

* **store:** persist scan history in local SQLite ([#133](https://github.com/MagiCrazy/neumann-cockpit/issues/133)) ([91f76be](https://github.com/MagiCrazy/neumann-cockpit/commit/91f76be462a53233b8a6164501844b1801fc6da1))

## [63.1.4](https://github.com/MagiCrazy/neumann-cockpit/compare/neumann-cockpit-v63.1.3...neumann-cockpit-v63.1.4) (2026-07-03)


### Bug Fixes

* **input:** make cockpit keys work with CapsLock ([#131](https://github.com/MagiCrazy/neumann-cockpit/issues/131)) ([89df53c](https://github.com/MagiCrazy/neumann-cockpit/commit/89df53c695d181580f5927d7c65a5a616aa6a4da))
* **overlays:** only render an overlay when its wizard is active ([#132](https://github.com/MagiCrazy/neumann-cockpit/issues/132)) ([50f964d](https://github.com/MagiCrazy/neumann-cockpit/commit/50f964d8585df9a3b4b3113d069625d08764d3da))
* **tui:** stop capturing the mouse ([#129](https://github.com/MagiCrazy/neumann-cockpit/issues/129)) ([dd71083](https://github.com/MagiCrazy/neumann-cockpit/commit/dd71083fd60d7cfa2f94573eac0f2962ee0b1dea))

## [63.1.3](https://github.com/MagiCrazy/neumann-cockpit/compare/neumann-cockpit-v63.1.2...neumann-cockpit-v63.1.3) (2026-07-03)


### Bug Fixes

* **cockpit:** scroll compact panes to keep the cursor visible ([#124](https://github.com/MagiCrazy/neumann-cockpit/issues/124)) ([80e566e](https://github.com/MagiCrazy/neumann-cockpit/commit/80e566e94aec9626c2c2fb9b91fee42748154555))

## [63.1.2](https://github.com/MagiCrazy/neumann-cockpit/compare/neumann-cockpit-v63.1.1...neumann-cockpit-v63.1.2) (2026-07-03)


### Bug Fixes

* **api:** harden HTTP client — timeouts + richer user-agent ([#121](https://github.com/MagiCrazy/neumann-cockpit/issues/121)) ([ee2292f](https://github.com/MagiCrazy/neumann-cockpit/commit/ee2292f2f1a0aeed80608750fcccbba711c58199))
* **errors:** surface swallowed fetch errors instead of silent no-ops ([#123](https://github.com/MagiCrazy/neumann-cockpit/issues/123)) ([e0698ea](https://github.com/MagiCrazy/neumann-cockpit/commit/e0698eadb56ab938133e9c42c1ba677f101317c9))
* **refresh:** back off periodic auto-refresh on consecutive failures ([#122](https://github.com/MagiCrazy/neumann-cockpit/issues/122)) ([2a20276](https://github.com/MagiCrazy/neumann-cockpit/commit/2a20276087480b3df6dbe8ae982f2b178a3ee28f))
* **tui:** restore terminal on panic via panic hook ([#119](https://github.com/MagiCrazy/neumann-cockpit/issues/119)) ([3f0fcca](https://github.com/MagiCrazy/neumann-cockpit/commit/3f0fcca4a3d0263c14890a3e6b58d921c065decf))

## [63.1.1](https://github.com/MagiCrazy/neumann-cockpit/compare/neumann-cockpit-v63.1.0...neumann-cockpit-v63.1.1) (2026-07-03)


### Bug Fixes

* **inventory:** reorder context menu to atomic craft, move stock, jettison ([#116](https://github.com/MagiCrazy/neumann-cockpit/issues/116)) ([3343148](https://github.com/MagiCrazy/neumann-cockpit/commit/334314887bb7ffa771b6fa7514c9aa4c97b793fa))

## [63.1.0](https://github.com/MagiCrazy/neumann-cockpit/compare/neumann-cockpit-v63.0.1...neumann-cockpit-v63.1.0) (2026-07-02)


### Features

* **cockpit:** unified tiling interface — 3×3 dashboard, keyboard-first, command mode ([#115](https://github.com/MagiCrazy/neumann-cockpit/issues/115)) ([e15e28e](https://github.com/MagiCrazy/neumann-cockpit/commit/e15e28e5adaa56d46b35231dda22edfb2bea6d68))


### Documentation

* **readme:** refresh for the cockpit — command mode, Map, live updates ([#113](https://github.com/MagiCrazy/neumann-cockpit/issues/113)) ([6b0e29f](https://github.com/MagiCrazy/neumann-cockpit/commit/6b0e29f122e4a3d95041ff312ca508bc5ffbd506))

## [63.0.1](https://github.com/MagiCrazy/neumann-cockpit/compare/neumann-cockpit-v63.0.0...neumann-cockpit-v63.0.1) (2026-07-01)


### Documentation

* add CONTRIBUTING guide and link it from the README ([#82](https://github.com/MagiCrazy/neumann-cockpit/issues/82)) ([cf23039](https://github.com/MagiCrazy/neumann-cockpit/commit/cf2303960c735c03589bbf804b997717c73ea5d9))

## [63.0.0](https://github.com/MagiCrazy/neumann-cockpit/compare/neumann-cockpit-v23.5.0...neumann-cockpit-v63.0.0) (2026-06-30)


* align client major with API version (v63) ([#81](https://github.com/MagiCrazy/neumann-cockpit/issues/81)) ([7cf59be](https://github.com/MagiCrazy/neumann-cockpit/commit/7cf59be8b871694b2567fda8f641700212cb5737))


### Features

* **api:** v48-v62 telemetry catch-up (bloc A) ([#69](https://github.com/MagiCrazy/neumann-cockpit/issues/69)) ([0210d33](https://github.com/MagiCrazy/neumann-cockpit/commit/0210d33af03bb58498caf47b167bcdd3c3062a9e))
* **mannies:** refill deuterium tank at a refuel station ([#71](https://github.com/MagiCrazy/neumann-cockpit/issues/71)) ([f99670a](https://github.com/MagiCrazy/neumann-cockpit/commit/f99670a11a1782ccb91673a8f1e8b59df4c29d97))
* **messaging:** inter-probe inbox, sent & compose (bloc F) ([#79](https://github.com/MagiCrazy/neumann-cockpit/issues/79)) ([be8bc98](https://github.com/MagiCrazy/neumann-cockpit/commit/be8bc982104fe5b48362c0880dd2a1c2bfc8b0e6))
* **mining:** optional target container in the local mine wizard ([#78](https://github.com/MagiCrazy/neumann-cockpit/issues/78)) ([6598d9b](https://github.com/MagiCrazy/neumann-cockpit/commit/6598d9b28bf07262cca413e4f4c3256fbaa13141))
* **missions:** list and abandon probe missions ([#73](https://github.com/MagiCrazy/neumann-cockpit/issues/73)) ([772e8c4](https://github.com/MagiCrazy/neumann-cockpit/commit/772e8c4821b98b0b0368c960665442d258281b37))
* **probe:** mind-snapshot reassign for a dead or trapped probe ([#72](https://github.com/MagiCrazy/neumann-cockpit/issues/72)) ([ff0ab67](https://github.com/MagiCrazy/neumann-cockpit/commit/ff0ab67f577687c62503e86134c4850302fc87b2))
* **scut:** inspect networks and show coverage (bloc E2) ([#75](https://github.com/MagiCrazy/neumann-cockpit/issues/75)) ([e3e977d](https://github.com/MagiCrazy/neumann-cockpit/commit/e3e977d6611cd82209a814a0fe75b7a6e37a5a20))
* **scut:** remote manny visibility and abandon (bloc E3) ([#76](https://github.com/MagiCrazy/neumann-cockpit/issues/76)) ([1de7412](https://github.com/MagiCrazy/neumann-cockpit/commit/1de741284a929e7dc39ab2680818162569e68117))
* **scut:** remote-mine a forgotten manny via SCUT (bloc E3b) ([#77](https://github.com/MagiCrazy/neumann-cockpit/issues/77)) ([b34d388](https://github.com/MagiCrazy/neumann-cockpit/commit/b34d388e19efb7d5efc2e4a70e342c592dd1ce1e))
* **scut:** turn on and deploy SCUT relays (bloc E1) ([#74](https://github.com/MagiCrazy/neumann-cockpit/issues/74)) ([0c667a1](https://github.com/MagiCrazy/neumann-cockpit/commit/0c667a16b13a423a13c693e5904ef116b868f3c2))

## [23.5.0](https://github.com/MagiCrazy/neumann-cockpit/compare/neumann-cockpit-v23.4.1...neumann-cockpit-v23.5.0) (2026-06-24)


### Features

* **alerts:** persistent alerts & damage warnings (v47 bloc 2) ([#61](https://github.com/MagiCrazy/neumann-cockpit/issues/61)) ([d1d1054](https://github.com/MagiCrazy/neumann-cockpit/commit/d1d10540ce5041fa0d8d4c0e4f3f770e8900be4d))
* **api:** v47 prerequisite type patches and mine targetContainerId ([#59](https://github.com/MagiCrazy/neumann-cockpit/issues/59)) ([74c43a5](https://github.com/MagiCrazy/neumann-cockpit/commit/74c43a54dc8b62efab9414b2dcd91f40dcb5a5d5))
* **containers:** storage containers CRUD and routing rules ([#62](https://github.com/MagiCrazy/neumann-cockpit/issues/62)) ([3c92b20](https://github.com/MagiCrazy/neumann-cockpit/commit/3c92b200e8879f5493b210bfa92192a4ecf591cc))
* **mannies:** drop storage container on a planet ([#65](https://github.com/MagiCrazy/neumann-cockpit/issues/65)) ([399902f](https://github.com/MagiCrazy/neumann-cockpit/commit/399902fba4a6ed717de49e2c0631fcd9f64c52bb))
* **mannies:** drop-manny-cargo action ([#64](https://github.com/MagiCrazy/neumann-cockpit/issues/64)) ([e3dafc9](https://github.com/MagiCrazy/neumann-cockpit/commit/e3dafc94c09be04df15913152e951705e81e0b89))
* **storage-move:** inter-container stock transfers ([#63](https://github.com/MagiCrazy/neumann-cockpit/issues/63)) ([7fc953e](https://github.com/MagiCrazy/neumann-cockpit/commit/7fc953eeaf79b5b4fd723d5317c84dfeee1e6653))


### Documentation

* **claude:** sync architecture & endpoints with v47 blocs 1-5a ([#66](https://github.com/MagiCrazy/neumann-cockpit/issues/66)) ([01b1067](https://github.com/MagiCrazy/neumann-cockpit/commit/01b10676807ea840c91718c17c2519399476adf5))

## [23.4.1](https://github.com/MagiCrazy/neumann-cockpit/compare/neumann-cockpit-v23.4.0...neumann-cockpit-v23.4.1) (2026-06-24)


### Bug Fixes

* **input:** ignore non-press key events to prevent double-trigger on Windows ([#57](https://github.com/MagiCrazy/neumann-cockpit/issues/57)) ([f7c3822](https://github.com/MagiCrazy/neumann-cockpit/commit/f7c3822280a416e7a533b6ae36e4926cc3dc2fbd))

## [23.4.0](https://github.com/MagiCrazy/neumann-cockpit/compare/neumann-cockpit-v23.3.1...neumann-cockpit-v23.4.0) (2026-06-11)


### Features

* **ui:** retro phosphor-CRT theme with idle animations ([#54](https://github.com/MagiCrazy/neumann-cockpit/issues/54)) ([0fcaced](https://github.com/MagiCrazy/neumann-cockpit/commit/0fcaced7fc3fadec9836fc05080833dd8ce00bbe))

## [23.3.1](https://github.com/MagiCrazy/neumann-cockpit/compare/neumann-cockpit-v23.3.0...neumann-cockpit-v23.3.1) (2026-06-11)


### Documentation

* align CLAUDE.md architecture paths with the module split ([#52](https://github.com/MagiCrazy/neumann-cockpit/issues/52)) ([34521cb](https://github.com/MagiCrazy/neumann-cockpit/commit/34521cb52eca80a1400cab5bfa01a3819d9e9b95))

## [23.3.0](https://github.com/MagiCrazy/neumann-cockpit/compare/neumann-cockpit-v23.2.0...neumann-cockpit-v23.3.0) (2026-06-11)


### Features

* **api:** implement /api/probe/visited-sectors and show visited cells on the map ([#46](https://github.com/MagiCrazy/neumann-cockpit/issues/46)) ([1df25b9](https://github.com/MagiCrazy/neumann-cockpit/commit/1df25b98df1d0f4de881cb4c29b70c883945dec4))
* **inventory:** [M] fills max amount in jettison input ([#43](https://github.com/MagiCrazy/neumann-cockpit/issues/43)) ([a89460f](https://github.com/MagiCrazy/neumann-cockpit/commit/a89460faea66cabf7f6ffa68b3ee73d7a432582c))
* **inventory:** read-only detail popup for the selected row ([#42](https://github.com/MagiCrazy/neumann-cockpit/issues/42)) ([07110c1](https://github.com/MagiCrazy/neumann-cockpit/commit/07110c1b83c6f19ae3178dfdad076117f1714247))
* **map:** center cell info line, legend, recenter and jump to coords ([#36](https://github.com/MagiCrazy/neumann-cockpit/issues/36)) ([025c661](https://github.com/MagiCrazy/neumann-cockpit/commit/025c661e041fa78a70fead0fafd62c96d24f327c))
* **nav:** waypoints overlay listing known destinations ([#34](https://github.com/MagiCrazy/neumann-cockpit/issues/34)) ([49930e7](https://github.com/MagiCrazy/neumann-cockpit/commit/49930e7935a323abba50c6d9c43db159ea637cea))
* **scanner:** batch scan progress gauge ([#40](https://github.com/MagiCrazy/neumann-cockpit/issues/40)) ([e54ca88](https://github.com/MagiCrazy/neumann-cockpit/commit/e54ca8837c89b9ae3767668388278ae09bd57f8a))
* **scanner:** object-first actions on sector objects ([#33](https://github.com/MagiCrazy/neumann-cockpit/issues/33)) ([c68e980](https://github.com/MagiCrazy/neumann-cockpit/commit/c68e980e10a0dcb3929b9cfde9654828245ab6c5))
* **scanner:** richer history list with icons, distance and cyclic filter ([#38](https://github.com/MagiCrazy/neumann-cockpit/issues/38)) ([7731aa4](https://github.com/MagiCrazy/neumann-cockpit/commit/7731aa4aabfcce7f28dbf7b13c611e424d39db19))
* **scanner:** stamp and display scan age ([#41](https://github.com/MagiCrazy/neumann-cockpit/issues/41)) ([e2cf7c7](https://github.com/MagiCrazy/neumann-cockpit/commit/e2cf7c732ff0d08a51708d874f2a1c33d8c3ca63))
* **travel:** live validation, relative input and current position ([#39](https://github.com/MagiCrazy/neumann-cockpit/issues/39)) ([a025409](https://github.com/MagiCrazy/neumann-cockpit/commit/a02540979e6acc423c008d79fd5ac8e7269e72d6))
* **ui:** help overlay listing all keybindings ([#37](https://github.com/MagiCrazy/neumann-cockpit/issues/37)) ([737c927](https://github.com/MagiCrazy/neumann-cockpit/commit/737c9279c99e90370bd688ceee146c9fa48b2ff9))
* **ui:** Tab cycles panel focus ([#44](https://github.com/MagiCrazy/neumann-cockpit/issues/44)) ([bde6929](https://github.com/MagiCrazy/neumann-cockpit/commit/bde692911a0e13a97bf965e68923f791ed2cb3d4))
* **ui:** transient success toasts in the status bar ([#45](https://github.com/MagiCrazy/neumann-cockpit/issues/45)) ([8ee0d1d](https://github.com/MagiCrazy/neumann-cockpit/commit/8ee0d1d96cb83c5ac6a796243b2462b9fdeea94f))


### Documentation

* refresh CLAUDE.md after the TUI improvements series ([#47](https://github.com/MagiCrazy/neumann-cockpit/issues/47)) ([5174884](https://github.com/MagiCrazy/neumann-cockpit/commit/51748842485fa916008922394392260570b4a9f8))

## [23.2.0](https://github.com/MagiCrazy/neumann-cockpit/compare/neumann-cockpit-v23.1.2...neumann-cockpit-v23.2.0) (2026-06-11)


### Features

* **inventory:** selectable rows, direct jettison, containers and tanks display ([#32](https://github.com/MagiCrazy/neumann-cockpit/issues/32)) ([a28172d](https://github.com/MagiCrazy/neumann-cockpit/commit/a28172db8b7de97f96d941abec62f2e987afb855))

## [23.1.2](https://github.com/MagiCrazy/neumann-cockpit/compare/neumann-cockpit-v23.1.1...neumann-cockpit-v23.1.2) (2026-06-11)


### Bug Fixes

* **ui:** scroll scan history list with selection ([#30](https://github.com/MagiCrazy/neumann-cockpit/issues/30)) ([7047b71](https://github.com/MagiCrazy/neumann-cockpit/commit/7047b710ad623fd8c1028a5ce079cf766d039e18))

## [23.1.1](https://github.com/MagiCrazy/neumann-cockpit/compare/neumann-cockpit-v23.1.0...neumann-cockpit-v23.1.1) (2026-06-09)


### Bug Fixes

* **app:** propagate inspect/recover errors to overlay + update CLAUDE.md ([#25](https://github.com/MagiCrazy/neumann-cockpit/issues/25)) ([3d6c56d](https://github.com/MagiCrazy/neumann-cockpit/commit/3d6c56dd518d47801a9ba430b63f88c7b3702650))

## [23.1.0](https://github.com/MagiCrazy/neumann-cockpit/compare/neumann-cockpit-v23.0.0...neumann-cockpit-v23.1.0) (2026-06-08)


### Features

* **cockpit:** add craft action for mannies + fix MannyTask variants and task_estimated_end_time ([31e5a12](https://github.com/MagiCrazy/neumann-cockpit/commit/31e5a1247c57713ae1dfb9e6acde2ac0492a8275))
* **cockpit:** add deploy waypoint-bookmark action ([9f7675b](https://github.com/MagiCrazy/neumann-cockpit/commit/9f7675bb06680f22e5e8c4d8a99be706ceab3867))
* **cockpit:** add jettison action for inventory stocks and mannies ([9c0f8e4](https://github.com/MagiCrazy/neumann-cockpit/commit/9c0f8e4333a76afa9ee88b39af0fcefe62aec653))
* **cockpit:** add rename manny action (PATCH /mannies/{id}) ([5c4f9fb](https://github.com/MagiCrazy/neumann-cockpit/commit/5c4f9fbd3887850ade30e6c08acea4b605908cfb))
* **cockpit:** add repair/mine actions for mannies and sector map view ([4039627](https://github.com/MagiCrazy/neumann-cockpit/commit/40396270f7b489b0047f6c54d3cd9e82ba8b0cdf))
* **cockpit:** add salvage and recall actions for mannies ([6557de7](https://github.com/MagiCrazy/neumann-cockpit/commit/6557de746645e07a07f10944218c2426455609bf))
* **cockpit:** display app version in status bar ([#5](https://github.com/MagiCrazy/neumann-cockpit/issues/5)) ([4e75104](https://github.com/MagiCrazy/neumann-cockpit/commit/4e751042b24b0578048f29e2088ae375514c1328))
* **cockpit:** fetch and display API version in status bar ([ebaf45b](https://github.com/MagiCrazy/neumann-cockpit/commit/ebaf45b7e0282102325a957e6fe80ba617fb206d))
* **cockpit:** initial TUI — probe, inventory, scanner, mannies, travel ([6bffea5](https://github.com/MagiCrazy/neumann-cockpit/commit/6bffea5c2b9b232eb6b67d9929b197d2a45294c4))
* **scanner:** show minable asteroid resource types in sector detail ([#22](https://github.com/MagiCrazy/neumann-cockpit/issues/22)) ([ab4d7d3](https://github.com/MagiCrazy/neumann-cockpit/commit/ab4d7d31cf9ab8a8bd68544a1edaf8a35aeeef86))
* v23 API, dynamic recipes, scanner enhancements, mannies actions ([#20](https://github.com/MagiCrazy/neumann-cockpit/issues/20)) ([d0dc94d](https://github.com/MagiCrazy/neumann-cockpit/commit/d0dc94d76de1ce6660055e5cce7fa4d81817f821))


### Bug Fixes

* **ci:** use client-id and install Bob app on repo ([#9](https://github.com/MagiCrazy/neumann-cockpit/issues/9)) ([6b7f42b](https://github.com/MagiCrazy/neumann-cockpit/commit/6b7f42be23d14f6409f1d13589a5a9aedd379172))
* **ci:** wrap release-please config in packages block ([#6](https://github.com/MagiCrazy/neumann-cockpit/issues/6)) ([b42fdea](https://github.com/MagiCrazy/neumann-cockpit/commit/b42fdea1382f2ff189640b28856544867491e4b0))
* **lint:** fix all clippy warnings (derivable_impls, unnecessary_map_or, single_match) ([61fc655](https://github.com/MagiCrazy/neumann-cockpit/commit/61fc6553fb70dbc1e0bbfab4b028b6843c8a930a))


### Documentation

* add GPL-3.0 license ([#13](https://github.com/MagiCrazy/neumann-cockpit/issues/13)) ([487d81b](https://github.com/MagiCrazy/neumann-cockpit/commit/487d81b6f1355c809e71461f88eb4d6d427b2726))
* **readme:** add badges and prebuilt binaries install ([#17](https://github.com/MagiCrazy/neumann-cockpit/issues/17)) ([106eae9](https://github.com/MagiCrazy/neumann-cockpit/commit/106eae9528ba22cf6e89c73655ec9fd7b3b4ddf4))
* **readme:** rewrite with quickstart, features, and server links ([d35709a](https://github.com/MagiCrazy/neumann-cockpit/commit/d35709a47f3e546af57578f7e65090510b1b3e6c))

## [11.1.3](https://github.com/MagiCrazy/neumann-cockpit/compare/neumann-cockpit-v11.1.2...neumann-cockpit-v11.1.3) (2026-06-03)


### Documentation

* **readme:** add badges and prebuilt binaries install ([#17](https://github.com/MagiCrazy/neumann-cockpit/issues/17)) ([106eae9](https://github.com/MagiCrazy/neumann-cockpit/commit/106eae9528ba22cf6e89c73655ec9fd7b3b4ddf4))

## [11.1.2](https://github.com/MagiCrazy/neumann-cockpit/compare/neumann-cockpit-v11.1.1...neumann-cockpit-v11.1.2) (2026-06-03)


### Documentation

* add GPL-3.0 license ([#13](https://github.com/MagiCrazy/neumann-cockpit/issues/13)) ([487d81b](https://github.com/MagiCrazy/neumann-cockpit/commit/487d81b6f1355c809e71461f88eb4d6d427b2726))

## [11.1.1](https://github.com/MagiCrazy/neumann-cockpit/compare/neumann-cockpit-v11.1.0...neumann-cockpit-v11.1.1) (2026-06-03)


### Bug Fixes

* **ci:** use client-id and install Bob app on repo ([#9](https://github.com/MagiCrazy/neumann-cockpit/issues/9)) ([6b7f42b](https://github.com/MagiCrazy/neumann-cockpit/commit/6b7f42be23d14f6409f1d13589a5a9aedd379172))

## [11.1.0](https://github.com/MagiCrazy/neumann-cockpit/compare/neumann-cockpit-v11.0.0...neumann-cockpit-v11.1.0) (2026-06-03)


### Features

* **cockpit:** add craft action for mannies + fix MannyTask variants and task_estimated_end_time ([31e5a12](https://github.com/MagiCrazy/neumann-cockpit/commit/31e5a1247c57713ae1dfb9e6acde2ac0492a8275))
* **cockpit:** add deploy waypoint-bookmark action ([9f7675b](https://github.com/MagiCrazy/neumann-cockpit/commit/9f7675bb06680f22e5e8c4d8a99be706ceab3867))
* **cockpit:** add jettison action for inventory stocks and mannies ([9c0f8e4](https://github.com/MagiCrazy/neumann-cockpit/commit/9c0f8e4333a76afa9ee88b39af0fcefe62aec653))
* **cockpit:** add rename manny action (PATCH /mannies/{id}) ([5c4f9fb](https://github.com/MagiCrazy/neumann-cockpit/commit/5c4f9fbd3887850ade30e6c08acea4b605908cfb))
* **cockpit:** add repair/mine actions for mannies and sector map view ([4039627](https://github.com/MagiCrazy/neumann-cockpit/commit/40396270f7b489b0047f6c54d3cd9e82ba8b0cdf))
* **cockpit:** add salvage and recall actions for mannies ([6557de7](https://github.com/MagiCrazy/neumann-cockpit/commit/6557de746645e07a07f10944218c2426455609bf))
* **cockpit:** display app version in status bar ([#5](https://github.com/MagiCrazy/neumann-cockpit/issues/5)) ([4e75104](https://github.com/MagiCrazy/neumann-cockpit/commit/4e751042b24b0578048f29e2088ae375514c1328))
* **cockpit:** fetch and display API version in status bar ([ebaf45b](https://github.com/MagiCrazy/neumann-cockpit/commit/ebaf45b7e0282102325a957e6fe80ba617fb206d))
* **cockpit:** initial TUI — probe, inventory, scanner, mannies, travel ([6bffea5](https://github.com/MagiCrazy/neumann-cockpit/commit/6bffea5c2b9b232eb6b67d9929b197d2a45294c4))


### Bug Fixes

* **ci:** wrap release-please config in packages block ([#6](https://github.com/MagiCrazy/neumann-cockpit/issues/6)) ([b42fdea](https://github.com/MagiCrazy/neumann-cockpit/commit/b42fdea1382f2ff189640b28856544867491e4b0))
* **lint:** fix all clippy warnings (derivable_impls, unnecessary_map_or, single_match) ([61fc655](https://github.com/MagiCrazy/neumann-cockpit/commit/61fc6553fb70dbc1e0bbfab4b028b6843c8a930a))


### Documentation

* **readme:** rewrite with quickstart, features, and server links ([d35709a](https://github.com/MagiCrazy/neumann-cockpit/commit/d35709a47f3e546af57578f7e65090510b1b3e6c))
