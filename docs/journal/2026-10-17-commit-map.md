# 2026-10-17: Commit map for the history of PR #88

## Current State

- The 318 commits of pull request [lmmx/chloro#88](https://github.com/lmmx/chloro/pull/88) between `884f9c9` (master, release 0.8.0) and the handoff commit were re-recorded with the maintainer as author and committer, and with the agent attribution trailers (`Co-Authored-By: Claude …`, `Claude-Session: …`) removed from their messages — the rewrite changed every one of those commits' hashes, so the hashes the journal cites from before 2026-10-17 name the commits in the left column of the table below.
- The rewrite changed no tree: every rewritten commit has the same tree, the same parents (mapped), the same author and committer dates and the same message apart from the removed trailers as the commit it replaces (checked commit by commit), and the branch head's tree is `0bbeb4e356ee25bc6e3276d58b566b6e3e31491d` before and after.
- Commits authored by `pre-commit-ci[bot]` keep their author and committer; the four commits authored by the maintainer keep their author and committer and lose only the trailers.
- The commits the journal names most often map as follows: `884f9c9` (the start) is unchanged; `96c9abc` (the last code commit, the E30 merge) is `2e1bc8f`; `1051c1e` (the handoff) is `c660779`.
- The experiment branches on origin (`20261014-*`, `20261015-*`) were not rewritten — each still holds the original commits of its own experiment under their original hashes; commits made only on the integration branch (merges, journal updates) exist under their original hashes only in the map below.

## Map

Ordered as `git rev-list --topo-order --reverse 884f9c9..` lists the branch; the subject is the rewritten commit's.

| before | after | subject |
|---|---|---|
| `72af2ac36` | `0b82b81bb` | fix: nested blocks of a macro definition get trailing semicolons |
| `e0ff87938` | `34d66b7ca` | fix: a repeat array `[x; n]` is not block-like |
| `56a373b58` | `2c43d1895` | fix: doc comments in macro arguments are tokens, as in rustc |
| `7978e1c31` | `00aac218d` | fix: reject expressions rustc's parser rejects (`<-`, `..a = b`, `a..=`) |
| `f43d45f32` | `b278abad5` | fix: reject ambiguous `+` in types and unbraced const parameter defaults |
| `5f043e8d0` | `bbc2e887b` | fix: reject `builtin #` constructs rustc's parser does not have |
| `ccf6f21f0` | `c1ef5d517` | fix: reject edition 2015 trait method parameters that are patterns |
| `f519a29a1` | `8e054fab7` | fix: reject misplaced doc comments and pre-2021 C strings |
| `83e9fdec3` | `1da9b5b90` | fix: parse a parenthesized first bound of a bare trait object, as rustc |
| `0479fd3aa` | `169ed0c09` | fix: a float literal without `.` is a field name, as in rustc |
| `1be8a3fae` | `c6bdaf1f4` | feat: `#[rustfmt::skip::macros(..)]` and `#[rustfmt::skip::attributes(..)]` |
| `085621533` | `1073a87c0` | feat: const block items (`const { .. }` in a file or module) |
| `7c778e073` | `652bae9da` | fix: parse `impl impl Trait {}` and `impl Tr for impl Trait {}`, as rustc |
| `d79729287` | `8f6b2b0a9` | docs(journal): open the autoresearch programme |
| `76e31d67f` | `3b336a534` | docs(journal): record E1, E2 and E6 hypotheses in the ledger |
| `4cd6336cb` | `8b93cbaa0` | docs(journal): profile baseline at 884f9c9 and the E3/E4/E7 hypotheses |
| `b8c8531c4` | `565d4d223` | wip: benchmark harness and rustfmt version matrix (agents still writing) |
| `ee99c8701` | `d006bdd40` | wip: benchmark harness edits (agent still writing) |
| `6e893f46e` | `15ed8ffa2` | feat(cli): cache content digests of files found already formatted |
| `2bd6b3a44` | `b30b96823` | docs(journal): record EXP-006, the CLI fixed-point cache |
| `d9f5a86a1` | `7db33e8f9` | Merge branch '20261014-cli-cache' (E6: CLI fixed-point cache) |
| `a5f1b0687` | `988fdef9e` | wip: harness and report edits in progress (agents still writing) |
| `fbe868562` | `c4ed735d6` | docs(journal): baseline harness results; correct the first-order rustfmt comparison |
| `d60c1fae2` | `2f794d744` | feat(core): format a file's top-level items in parallel |
| `f847a8739` | `53a4f4b4a` | docs(journal): EXP-001, items formatted in parallel; CLI splits files of 128 KiB and more |
| `ecb7832a1` | `070f4fd9c` | docs(journal): name the EXP-001 commits |
| `7faa8c91a` | `bd03dc7a4` | Merge branch '20261014-parallel-items' (E1: top-level items formatted in parallel) |
| `7861cd505` | `377133857` | docs(journal): rustfmt version matrix (1.7.0, 1.8.0, 1.9.0) and the version-knob design |
| `c88d19e44` | `361cb26dc` | docs(journal): results so far (E6, E1 merged; E2 measured; R3, R4 findings) |
| `0bb8e0659` | `75a64a603` | build: fat LTO and one codegen unit for release builds |
| `d10c1e37a` | `668026f54` | benchmarks: reproducible profile-guided build of the CLI |
| `8d170a2f5` | `a0c1a70eb` | docs(journal): record EXP-002 (CLI build profile, PGO, allocator, scheduler) |
| `a2016a9ec` | `55fb8b277` | docs(journal): EXP-002 after the scheduler hand-off to E1 |
| `7960826f7` | `5fcba8b2f` | Merge branch '20261014-cli-build' (E2: release profile and PGO script) |
| `54389d32c` | `b4fc65a27` | chore(pre-commit): autofix run |
| `e959c08ae` | `1ff5d6708` | docs(journal): best-known-good moves to 7960826 (554,185,388 Ir on the sample) |
| `dc20cd7c6` | `5f24cc928` | docs(journal): EXP-007 flat substrate feasibility record; E7b opened |
| `4841d9b98` | `af1d64ed9` | perf(nodes): classify statements on the green tree |
| `5e7e36cd8` | `8a92a8436` | perf(nodes): collect attributes on the green tree |
| `362e1eb25` | `46bfe7f66` | perf: answer one-shot node queries from the green tree |
| `fb07072b0` | `62480dc83` | perf: locate child nodes on the green tree where siblings were rejected |
| `62c434880` | `c5befa8bf` | perf(macros): read macro names, delimiters and token trees from the green tree |
| `5c02ef255` | `e13ae5780` | perf(context): build each block once per run |
| `92516fe98` | `f1d2a2ae9` | docs(journal): record EXP-004, green-tree reads on the red-node paths |
| `6a12d8661` | `c02db0705` | Merge branch '20261014-green-reads' (E4: green-tree reads on the red-node creation paths) |
| `aa1e0581b` | `966a7d59d` | docs(journal): best-known-good moves to the E4 merge (518,951,890 Ir on the sample) |
| `0d3631175` | `1f0cd1cc3` | docs(journal): M1 quiet-host measurements of base, head, head-pgo and rustfmt |
| `b37725157` | `17745d1db` | bench: disable chloro's cache in the harness by default; absolute PGO root; M1 in the j… |
| `875425c41` | `0eac98ff1` | feat(config): `rustfmt_version` option selecting rustfmt 1.8.0 or 1.9.0 |
| `3259c1de4` | `1e891ed3d` | feat(rustc_compat): toolchain acceptance of rustfmt 1.8.0 (P1, P2, P8) |
| `1f23fd546` | `9215f61d1` | feat(expr): raw labels of break and continue as rustfmt 1.8.0 writes them (F1) |
| `98f05593f` | `cd9576531` | feat(closures): labelled blocks as closure bodies as rustfmt 1.8.0 formats them (F2) |
| `9298a6aba` | `64d5fa8bf` | feat(imports): `use self::self` and the pre-2024 `_` import order of rustfmt 1.8.0 (F3,… |
| `9dfef1a1a` | `e59ad4c15` | feat(items): `super let` formatted as `let` under rustfmt 1.8.0 (F4) |
| `2279dada4` | `28aa21ebf` | feat(missed_spans): rustfmt 1.8.0's blank-line offset and its panic (F6) |
| `0e712fc09` | `c49088997` | test(conformance): rustfmt 1.8.0 snapshots, reproducer suite, version-aware conform and… |
| `99035f4a7` | `d1705c52a` | Merge remote-tracking branch 'origin/claude/chloro-perf-research-swarm-nrscj8' into 202… |
| `b28e09b01` | `4a0e311e1` | docs: EXP-008 rustfmt_version record, README conformance section, ledger row E8 |
| `79d69c9f8` | `882443979` | Merge branch '20261014-rustfmt-version' (E8: rustfmt_version option, rustfmt 1.8.0 supp… |
| `205c9ddc1` | `169f183c5` | docs(journal): E8 merged (rustfmt_version, 1.8.0 support) |
| `bf3e8fde0` | `0b9e242e9` | docs(journal): best-known-good moves to the E8 merge |
| `80e474082` | `73d75cef4` | docs(journal): E7b, the flat substrate port, as reported on its branch |
| `8e7ec6749` | `0219b8c86` | Merge branch 'claude/chloro-perf-research-swarm-nrscj8' into 20261014-conformance-fixes |
| `b6e858e64` | `b64675dae` | fix: an unknown literal prefix is no lexer error before edition 2021 |
| `b327b027b` | `6708ebfa0` | feat(cst): flat syntax array substrate and generated typed layer (E7b, step 2) |
| `9031948dd` | `2492f797c` | feat(formatter): read syntax from the flat tree (E7b, step 3: the sweep) |
| `ac5c15a2f` | `1e1fa000d` | feat(cst): tests, examples and lints on the flat port (E7b, step 3b) |
| `0fc5056a1` | `09d6406ce` | chore(deps): rowan and ra_ap_syntax become dev-dependencies of chloro-core |
| `e259666c3` | `33324092f` | Merge branch 'claude/chloro-perf-research-swarm-nrscj8' into 20261014-flat-port (E4) |
| `353a18e94` | `c7288fafc` | test(context): a type alias for the block cache test's key (clippy) |
| `2f1354717` | `ea2339c7f` | docs(journal): EXP-007b record (draft: timing pairs, config_matrix and build time pendi… |
| `d709ac180` | `1ad9394c6` | docs(journal): EXP-007b record complete; ledger row E7b |
| `bc6c76e05` | `80f42e3c1` | Merge branch 'claude/chloro-perf-research-swarm-nrscj8' into 20261014-flat-port (E8) |
| `b4fdd1a90` | `3a1af91ba` | docs(journal): EXP-007b gates on the E8 merge |
| `8d418eee2` | `fe34bd2c2` | Merge branch '20261014-flat-port' (E7b: the flat syntax array under the formatter) |
| `729fd36f6` | `78fcb588d` | Merge branch 'claude/chloro-perf-research-swarm-nrscj8' (E7b flat syntax array) into 20… |
| `b46424395` | `716c60953` | docs(formatter): module map entry of rustc_syntax |
| `60eb8e0fe` | `385815c32` | docs(journal): profile of the flat build at 8d418ee |
| `573a785e0` | `70f4ccbcd` | docs(journal): M2, the E7b integration head against base and rustfmt on a quiet host |
| `b31f36aee` | `8cf900572` | Merge branch '20261014-measure-m2' (M2: quiet-host measurements of the flat-substrate h… |
| `21bca2745` | `d03a883d0` | docs: milestone 2 in the journal and README; harness formats stdin copies through stdin |
| `4a1700a20` | `775f1d778` | docs(journal): current state after the E7b and M2 merges |
| `9e0e59e17` | `f9739eaa2` | docs(journal): conformance ledger (24 cases; fixes on 20261014-conformance-fixes) |
| `0f5f23528` | `19db4fe60` | perf(utils): test eight bytes at a time for ASCII single-line widths |
| `b6a249db0` | `d74d6c6a1` | perf(comment): borrow comment-free snippets in filter_normal_code |
| `16e2e3df4` | `259abb03e` | perf(comment): skip lone r, / and ' and string contents in skip_plain |
| `d46aa59e4` | `a58d88b89` | perf(expr): scan only the last two code characters in span_ends_with_comma |
| `9b77c2371` | `6406b73a3` | perf(comment): skip comment interiors in comment slicing and find_uncommented |
| `350e6e40f` | `e0235a68c` | perf(comment): find the next state-changing byte eight bytes at a time |
| `6f7c38660` | `a3afd181d` | perf(comment): skip plain code and comment interiors in CommentCodeSlices |
| `abdc10c1a` | `a96dfd5b2` | perf(context): extend the line-start table only up to the asked position |
| `c2c0934b8` | `4e72e1092` | Merge branch 'claude/chloro-perf-research-swarm-nrscj8' into 20261014-hot-paths |
| `3c2cd306c` | `02ca1ddf0` | perf(nodes): borrow node_text from the token when a node has one token |
| `83dd37c6e` | `5838c14ca` | perf(types, chains): no GreenNode clones in single_ident_path, one buffer for a callee |
| `887674bfd` | `860514d42` | perf(expr): return an expression without attributes as rewritten |
| `4b5bb3ab8` | `35e1475e3` | perf(utils): measure an ASCII first or last line without splitting |
| `a383516c2` | `bfe5af329` | perf(utils): count the newlines of a multi-line ASCII width eight bytes at a time |
| `e2128cf7d` | `db2cea1ad` | perf(chains): size the joined chain once in join_rewrites |
| `0178efa60` | `9ad28d43b` | perf(lists): start write_list's buffer at the size of its parts |
| `c28d4036f` | `a02ae1ba8` | perf: join strings without format! at the 35 hottest string-only sites |
| `adcacf250` | `49edb1797` | Merge branch 'claude/chloro-perf-research-swarm-nrscj8' into 20261014-hot-paths (E7b, E8) |
| `ed4303ae3` | `593298635` | perf(comment): compare comment texts before their payloads in changed_comment_content |
| `a8e9cdfc0` | `bc89f41a5` | perf(lists): look for a `:` before searching for `::` in write_list |
| `ff250c6b4` | `216ac0869` | perf(comment): return from find_uncommented when the pattern has matched |
| `f99d772cd` | `dfb540e4f` | perf(comment): skip within lines in LineClasses |
| `0946a4f79` | `249548ca0` | Merge branch 'claude/chloro-perf-research-swarm-nrscj8' into 20261014-hot-paths (docs: … |
| `e477dfd1a` | `39279bab7` | docs(journal): EXP-003 hot paths record and ledger row E3 |
| `effe42cf7` | `afe0941b2` | Merge branch 'claude/chloro-perf-research-swarm-nrscj8' into 20261014-hot-paths (docs: M2) |
| `7cf3c9eaf` | `6c0af7208` | Merge branch '20261014-hot-paths' (E3: hot-path levers on the flat build) |
| `b4917ad13` | `ca64d1003` | docs(journal): E3 merged (hot-path levers on the flat build) |
| `2ad6240fe` | `0486e62b9` | Merge branch 'claude/chloro-perf-research-swarm-nrscj8' (E3 hot paths) into 20261014-co… |
| `91e8e0275` | `3307555ca` | perf(skip): no skip context work outside items with rustfmt::skip lists |
| `11e594594` | `8488b228d` | perf(visitor): const block items take their block from RunState::block_of |
| `6b7120fc2` | `46f65c986` | perf(rustc_compat): one pass over the flat tree; ancestors found on demand |
| `b0aa3d27e` | `8a8f09d33` | perf(rustc_syntax): reparse reuses the first lex and parse, and skips edits that cannot… |
| `59074a830` | `36397f2cd` | docs(journal): conformance ledger after the E3 merge and the cost commits |
| `22d48e99b` | `bb9065ea0` | docs(journal): R2 conformance results as gated on its branch |
| `e8d87c287` | `a18987d1d` | Merge branch 'claude/chloro-perf-research-swarm-nrscj8' (journal: R2 results) into 2026… |
| `113367a89` | `af52d668a` | Merge branch '20261014-conformance-fixes' (R2: conformance ledger and fixes) |
| `4867896e1` | `069f5a7f6` | docs: conformance 1220/1220 in the README, ledger and journal; best-known-good moves to… |
| `8b67257d8` | `88c1ce29a` | docs(journal): what the programme established and what is left |
| `c752f20f6` | `81933ea7f` | docs(journal): handoff for the next session |
| `c1c13c112` | `c4f4f6f21` | docs(journal): EXP-010, fused tree build feasibility (not run: about 2% for a parser-ru… |
| `0166f0b73` | `8bb6878d4` | docs(journal): EXP-009, glibc malloc tunables (rejected: inside the spread) |
| `1eae9f64d` | `f4afb2e12` | docs(journal): review of the cache key, deferred-gap replay and arena handles; handoff … |
| `a64083404` | `a9b99a714` | fix(cli): process a file reachable several ways once |
| `09339ea03` | `ac1eec821` | fix(cache): name the temporary file after its target and create it exclusively |
| `cd8ceea64` | `30dcb835a` | fix(core): refuse a source longer than u32::MAX bytes instead of wrapping offsets |
| `0edf43420` | `38fe2263d` | fix(core): fall back to the sequential walk when a unit's record cannot be stitched |
| `9ee980458` | `4f185aa38` | test(core): the parallel item formatter matches the sequential one on the fixtures |
| `72bd4b1b8` | `8c7ae414f` | fix(cache): record the file key in the header, check the executable at save, sweep stal… |
| `eb2d8599a` | `86cee7007` | docs(journal): EXP-011, the review fixes and their gates |
| `bdfb9d1d2` | `bd9c76826` | Merge branch '20261014-review-fixes' (review findings 1-10: fixes and tests) |
| `9f690d9d4` | `9edc1226f` | docs(journal): R5 review fixes in the ledger and journal; best-known-good moves to bdfb9d1 |
| `81034ba70` | `37dc0618c` | docs(readme): conformance and performance as the lead sections, with the tested matrix … |
| `8e21a7d3c` | `ea288b7b3` | docs(benchmarks): point at the measurement records the README quotes |
| `2fc1640e6` | `a15bebf44` | docs: M3 quiet-host measurement of the E3 + R2 integration head against base and rustfmt |
| `6802902af` | `0353bad83` | Merge branch '20261014-measure-m3' (M3: quiet-host measurement of the E3 + R2 head) |
| `ba75e89ac` | `af790fd51` | docs: M3 numbers in the README, the best-known-good row and the journal |
| `31ce20efc` | `1b89e4c14` | docs(journal): lineage of the flat syntax array, crediting the 2026-10-10 and 2026-10-1… |
| `383fa281d` | `4dd27c13b` | docs(journal): theory record, incremental formatting and process-level levers (2026-10-15) |
| `0043dabc4` | `69aa8f368` | docs(journal): extensions-theory entry skeleton with the final-head component and cache… |
| `b69e00f77` | `2b3c76135` | docs(journal): overnight run handoff in the extensions-theory entry |
| `8bc52467e` | `d750b51c1` | docs(journal): handoff: scheduled wake tasks verified, rustd attempt 5, E15 launched |
| `9f3a262a9` | `186b66ef4` | perf(cli): canonicalise once per root and per symlink, sort the file list once |
| `de23ab4a1` | `c50bc9682` | perf(cli): record the executable's digest under its identity instead of hashing it ever… |
| `afd60e678` | `16b27a79a` | docs(journal): EXP-012 the CLI's work outside formatting, with the E12 ledger row |
| `92fdef099` | `397159b7f` | Merge branch '20261015-cli-waste' (E12: the CLI's work outside formatting) |
| `400ed535f` | `a168b72fb` | docs(journal): E12 merged; best-known-good commit moves to the merge (chloro-core uncha… |
| `1e558696d` | `5756f8c96` | docs(journal): handoff: E13 launched, rustd basic result |
| `b27bcc8e5` | `3dd142f2d` | docs(journal): rustd feasibility trial on chloro-core: 198 tests match, 1.4 s edit-to-b… |
| `c2a27393f` | `8e5de886a` | ci(release): profile-guided build for the targets the runner executes natively |
| `91eefce47` | `955b4ea7f` | docs: EXP-013 profile-guided release binaries, the E13 ledger row, the READMEs |
| `45e847056` | `3479ddbd7` | Merge branch '20261015-pgo-ci' (E13: profile-guided release builds for native targets) |
| `a36b5e808` | `ff5a36907` | docs(journal): E13 merged; gates reproduced on the profile-guided build |
| `7b539176d` | `21b91c52c` | docs(journal): handoff: E15 first result and follow-up |
| `cf79ced7a` | `7dddd9c43` | docs(journal): theory record, scanning, widths and string assembly (2026-10-15) |
| `f2a25dd14` | `25b6e17b0` | docs(journal): handoff: E14 and E16 running, records landed so far |
| `27b622168` | `a6009261d` | docs(journal): theory records, the parse floor and the already-formatted fast path (202… |
| `d179b5034` | `7001c6a79` | docs(journal): handoff: four records landed, E17 launched |
| `2e6860709` | `26665fa31` | docs(journal): extensions-theory entry: bounds by record for the four landed records |
| `07cab15c0` | `fa71ec5ab` | feat(core): a cache of formatted top-level units (format_source_with_unit_cache) |
| `a4db417a0` | `a167552c1` | feat(cli): a persistent unit cache next to the fixed-point cache |
| `8114d8bb2` | `02ad66d90` | test: unit cache equivalence in the library and the CLI, and the E15 harness |
| `c000c6640` | `5fb36c537` | test: unit cache harness options for sampling and the edit timing |
| `2f14ffa0c` | `7e6423883` | test: count unit records with a tail or a moved gap in the E15 harness |
| `b8c388247` | `f11880046` | docs(journal): EXP-015 item-level cache record and ledger row |
| `6f4405e49` | `ee8674377` | Merge the integration head (E12, E13) into 20261015-item-cache |
| `cafc1e813` | `d67245693` | feat(cli): make the unit cache opt-in (--unit-cache) |
| `8ae8a92a9` | `aa45a8e97` | docs(journal): EXP-015 follow-up: merged branch, opt-in unit cache, re-measured |
| `4e48bdbc1` | `20cb3f7de` | Merge branch '20261015-item-cache' (E15: opt-in item-level unit cache) |
| `768c2885b` | `1796c9097` | docs(journal): E15 merged; gates reproduced |
| `af68024f0` | `136ce0aee` | docs(journal): already-formatted fast path record, final version |
| `5309fc33e` | `26c847b37` | docs(journal): theory record, decide-once formatting, exact reuse and a shared sink (20… |
| `aa44d4c46` | `992955100` | docs(journal): extensions-theory entry: decide-once bounds, the ranked compound picture… |
| `b1ff8bd38` | `f2dc019eb` | docs(journal): handoff: records complete, E18 running, morning order |
| `fe5da9a10` | `dcdb902c9` | docs(journal): handoff: launch times corrected to the commit clock |
| `d8b066e13` | `c9a8863c0` | perf(visitor): pass in_trait to visit_assoc_item instead of searching for the parent |
| `0479cbcf8` | `f9c779971` | perf(macro_args): build Expr and Ty fragments from the prefix parse when it consumed it… |
| `d14d25942` | `a908a7b4f` | perf(cst): build the flat tree from the parser output in one pass |
| `7f6e98e4c` | `510515e5f` | docs(journal): EXP-017, the parse floor: P8, P5 and P1 kept (-7.11% Ir) |
| `a3a60bdea` | `eb505dcbd` | Merge branch '20261015-parse-floor' (E17: in_trait passed down, macro fragments parsed … |
| `8b87ebdbd` | `51279aef2` | docs(journal): the E17 merge hash in the ledger and handoff |
| `70cbc9316` | `25dd4a43f` | perf(chains): skip the last child's vertical candidate when it cannot win |
| `a02cec52d` | `5a1a64b10` | docs(journal): EXP-018 dead candidates record and E18 ledger row |
| `387d7230a` | `eebc9cbcb` | Merge branch '20261015-dead-candidates' (E18: the chain's dead vertical candidate skipped) |
| `18d5eb5a7` | `d0a9e0aea` | docs(journal): E18 merged; best-known-good 387d723 at 280,278,276 Ir (-52.3% from the s… |
| `0d78c924b` | `bdf8d702e` | perf(bench): alloc-timing wrapper and optional global allocators (EXP-014) |
| `8e6e36c5b` | `d54783bbe` | docs(journal): EXP-014 allocators: in-call time share, harness rows, RSS |
| `3fd39b4cd` | `9e22b3d78` | Merge branch '20261015-allocators' (E14: allocator time share and peak RSS measured; gl… |
| `ee723c97b` | `2da2ce5df` | docs(journal): E14 merged; allocator verdict in the handoff |
| `ca71f90fc` | `92004c51e` | perf(lists): answer list gaps without `/` by byte scans (S3) |
| `4358bb9bb` | `52a56fd4d` | perf(utils): `filtered_str_fits` in one pass over the lines (S1) |
| `6e6deca71` | `a4b99f955` | perf(visitor): count `line_number` on demand (S2, first part) |
| `fb4865108` | `802ccadcd` | perf(utils): count newlines by word; stop at the second for `== 1` (S2, second part) |
| `4499fbcd1` | `2af3547ac` | perf(comment): `find_uncommented` of one byte by a two-byte search (S4) |
| `d80cdfdd5` | `c52fd82a4` | perf: hot `format!` calls with string arguments to `concat` (S7) |
| `70f75e533` | `ef2532e5d` | perf(utils): ASCII-first `trim` at the hot `str::trim` sites (S8) |
| `883223331` | `1a5d2d196` | perf(comment): `find_any_byte` sixteen bytes per step with SSE2 (S5) |
| `5950f4c5f` | `af9ae1bef` | perf(types): one exact allocation for a single-identifier path (S12) |
| `b092f470c` | `919bf3c0c` | perf(shape): `Indent::to_string` inline to its slice case (S15) |
| `5cb66ffd7` | `d26b9efea` | perf(context): extend the line table by byte search (S13) |
| `4e8080c9f` | `db05efb20` | docs(journal): E16 record and ledger row (scanning and string levers) |
| `e640fbfa3` | `f4be63e07` | Merge branch '20261015-scanner-paths' (E16: scanner, list-gap and string-assembly fast … |
| `61afa3264` | `0a50a78bc` | docs(journal): E16 merged; best-known-good e640fbf at 248,202,094 Ir (-57.8% from the s… |
| `6b78728ab` | `e39a54a8a` | docs(journal): the 2026-10-15 instruction-count rows in what the programme established |
| `aeb32802b` | `ae9be7d7d` | docs(journal): M4 record and overnight report drafted (figures pending the running chai… |
| `0ee82c5c8` | `e72959b6d` | docs: M4, the quiet-host measurement of the final head; README, ledger, journal and the… |
| `40b782045` | `25224e119` | chore(pre-commit): autofix run |
| `bf05b31e0` | `6b4a9d770` | benchmarks: recompute a reused copy's output agreement for the binary under test |
| `8ebd3bb23` | `73f8a85ee` | docs(journal): unmined tricks, precedents and the wave-3 plan (2026-10-15) |
| `d17cb7462` | `de2c22914` | perf(expr): write an else-if chain into one buffer (EXP-019 cliff) |
| `9c43d8c7f` | `339c42ae4` | perf(imports): compare use segments without copies or substring searchers (EXP-019) |
| `a4b51d529` | `ab342c282` | perf(span, nodes): answer rustc_span and has_leading_attrs from the first child inline … |
| `774559d9e` | `6a0fe4e96` | perf(lists): decide definitive_tactic at the first item that decides it (EXP-019) |
| `f18dac326` | `870472409` | perf(lists): measure an item's last line only for its post-comment (EXP-019) |
| `25b1bf98b` | `daee9b5bf` | perf(macro_args): take a token tree's tokens from the tree instead of lexing its interi… |
| `f81f9ca97` | `6419b3388` | benchmarks: the cliff hunt's scaling fixtures and power-law fit (EXP-019) |
| `26289e34a` | `461d2da15` | docs(journal): EXP-019 record and ledger row; cliffs fit with an additive constant |
| `e289614aa` | `bce3c1448` | docs(journal): handoff: wave 3 (E19-E22) launched from 8ebd3bb |
| `659f08dfd` | `25e906547` | docs(journal): handoff: wave 3 launch time corrected to the commit clock |
| `9a611c25b` | `5d1c709d3` | docs(journal): the external research brief and report, verbatim, with the assessment of… |
| `8df88c325` | `b9a279b42` | perf(lists, overflow): `write_list_into` writes the argument list into the call's buffer |
| `729045701` | `0f83db711` | perf(chains): size the chain's entry lists when they are made |
| `e6fe9c074` | `804c40017` | perf(overflow): move the overflowed last argument into the list instead of copying it |
| `c40a44159` | `5ddfd19e1` | perf(lists): measure an item's last line and the nested-import test where they are read |
| `2a2ae28a9` | `9321a53db` | docs(journal): EXP-020, the transactional sink on write_list, chains and overflow |
| `60de639ef` | `ecf30f970` | Merge branch '20261015-sink' (E20: write_list_into writes the list into the call's buff… |
| `46418c7d3` | `a5947126f` | docs(journal): E20 merged; best-known-good 60de639 at 238,033,187-238,033,298 Ir (-59.5… |
| `cbbf08173` | `12af32d49` | benchmarks: profile.sh --sim, the branch and cache co-metric (EXP-022) |
| `4365e53ad` | `832fd8eea` | cst: finish nodes at their Exit in steps_fused; defer only the root's (EXP-022) |
| `687d6d309` | `3e7abd13c` | docs(journal): EXP-022, the Ir-blind budget (co-metric, splits, kind dispatch, BOLT) |
| `9d7871330` | `275690fb8` | Merge branch '20261015-layout' (E22: profile.sh --sim co-metric; steps_fused finishes n… |
| `fdcbcd518` | `c20dab986` | Merge the integration head (9d78713: E20 sink, E22 layout) into 20261015-cliffs |
| `2f009880e` | `efc206c03` | docs(journal): E22 merged; best-known-good 9d78713 at 237,768,533 Ir (-59.6% from the s… |
| `72df381ca` | `b745cbcd3` | Merge the integration head's journal update (2f00988) into 20261015-cliffs |
| `938c0d1e5` | `ebe180ed1` | docs(journal): EXP-019 merged with the integration head: resolution, figures, gates |
| `05ab7b4da` | `dcbb43d44` | Merge branch '20261015-cliffs' (E19: cliff hunt and repeated-work audit; six exact leve… |
| `e312e3905` | `1fe04fc8d` | docs(journal): E19 merged; best-known-good 05ab7b4 at 232,047,700-232,050,590 Ir (-60.5… |
| `e9959a442` | `760d89dd7` | docs(journal): handoff: E26 launched |
| `92c48b347` | `b07396214` | docs(journal): EXP-021 record and ledger row (resident process, chloro serve) |
| `9135c62a4` | `d4e30695a` | docs(journal): handoff: E21 refuted as stated for a fresh client process; mode stays on… |
| `73abce819` | `8df18d919` | docs(journal): handoff: wave 3 complete, E23 launched |
| `4f1b9d968` | `33866dc12` | docs(journal): E24 memo side array rejected; step-1 memo and block-cache measurements |
| `12bf826f1` | `6642fc5bf` | docs(journal): handoff: E24 rejected, E27 reported, E25 and E28 launched |
| `11034854a` | `5a2a680c1` | benchmarks: profile.sh count independent of where the sample and binary live (EXP-027) |
| `3487091b7` | `dd6bde08f` | benchmarks: profile.sh --per-file, each file in a process of its own (EXP-027) |
| `e615b5ee5` | `bc7bae438` | benchmarks: the registry validation sample and profile.sh --sample-list; protocol rules… |
| `f0ce72ec8` | `b99eda2c4` | chloro-core, bench, profile.sh: the parse-only phase (EXP-027) |
| `96ce5deb3` | `eef66cc20` | benchmarks: profile.sh --shuffle N, the co-metric over random function orders (EXP-027) |
| `a7967218c` | `62e46a20b` | docs(journal): EXP-027 record and ledger row |
| `28534ec15` | `38e4fb1ff` | Merge branch '20261015-harness' (E27: path-independent instruction count, --per-file, t… |
| `19789e1a4` | `a67184d72` | docs(journal): E27 merged; best-known-good 28534ec at 232,051,704 Ir (fixtures) and 305… |
| `a1a7cf3a1` | `c4d041e4c` | cli: CHLORO_TRACE, a timeline of the run per file and worker (EXP-023) |
| `22a4b1425` | `8357c37c7` | benchmarks: schedule/, the four-thread schedule diagnostic (EXP-023) |
| `d962661c1` | `adadc4e43` | docs(journal): EXP-023 record and ledger row (unit queue: the diagnostic, the stop rule… |
| `6fd4c9d9d` | `80ed8ab2d` | Merge branch '20261015-unit-queue' (E23: the four-thread schedule diagnostic and CHLORO… |
| `32b74e5a0` | `0a02b05e9` | docs(journal): E23 merged (diagnostic only, hypothesis refuted); ledger commit 6fd4c9d,… |
| `5ad3793b6` | `a3bd8721f` | docs(journal): handoff: E28 reported (depth-gated memo), E25 reported (arena held for s… |
| `99dc14ad1` | `61d708e27` | Merge origin/20261015-harness (E27: path-independent profile.sh, --per-file, registry s… |
| `fd18d50aa` | `870bd5da3` | context: memoize expression rewrites only from nesting depth 6 (EXP-028) |
| `daf5678cc` | `5008159b1` | docs(journal): EXP-028 record and ledger row (memo store policy) |
| `30f9aaa77` | `388172181` | Merge branch '20261015-memo-policy' (E28: memoize expression rewrites only from nesting… |
| `30370716b` | `5b56e05f7` | docs(journal): E28 merged; best-known-good 30f9aaa at 226,278,649 Ir (fixtures, -61.5% … |
| `ee62a16b9` | `76a355054` | perf(closures): skip a closure body without a block when no one-line rewrite can fit |
| `c915da4aa` | `b7fb26ed1` | perf(expr): skip `if c { a } else { b }` on one line when the branches' bounds exceed t… |
| `900650347` | `9b30d02c4` | perf(width): count the punctuation every rendering of a node writes |
| `772565c81` | `ad42c9432` | docs(journal): EXP-026, a lower bound on one-line width (E26) |
| `830345e23` | `2fefa5a25` | Merge branch '20261015-width' (E26: a lower bound on one-line width; closure-body and o… |
| `750c38cb2` | `35c4a4520` | docs(journal): E26 merged; best-known-good 830345e at 224,591,085 Ir (fixtures, -61.8% … |
| `21042dc7b` | `ed26a34cd` | Merge branch '20261015-harness' (E27's profile.sh: per-file, registry sample, parse pha… |
| `368add03f` | `6ff63fb80` | perf(formatter): scratch arena for module-local intermediates; chains' lists and string… |
| `a61d9757e` | `fc316e3f3` | perf(formatter): overflow's list items in the scratch arena (EXP-025) |
| `6a3df77ea` | `5d3b903b1` | perf(formatter): path segments and a segment's rewrite in the scratch arena (EXP-025) |
| `eaab5305a` | `bc4a7b6f3` | docs(journal): EXP-025 record and ledger row (scratch arena for the formatter's interme… |
| `b2dd41980` | `104493f21` | perf(formatter): scratch arena keeps pointers, not addresses; a formatting test sized f… |
| `224870a84` | `2a5beba61` | perf(formatter): the scratch arena on bumpalo, with a lifetime instead of a live count … |
| `f7d0cf6c0` | `84d66ac71` | docs(journal): EXP-025 soundness and the safe variant (Miri; the arena on bumpalo) |
| `e96b530ed` | `a26b4608d` | Merge branch '20261015-arena' (E25: the formatter's scratch arena on bumpalo for chains… |
| `d2f6fa06c` | `d1cf9cf31` | docs(journal): E25 merged; best-known-good e96b530 at 220,958,649 Ir (fixtures, -62.4% … |
| `03c61bef0` | `4ad3b5477` | docs: M5 quiet-host measurement of the wave-4 head (d2f6fa0): 0.239-0.405 / 0.083-0.173… |
| `204190a6a` | `870708184` | chore(pre-commit): autofix run |
| `2da4c59ae` | `56fbfdaad` | docs(journal): handoff: wave 4 complete (M5), wave 5 launched (E29 split-file tail, E30… |
| `324fb6038` | `e3bf4bda8` | docs(journal): handoff: wave 5 agents stopped by the rate limit on 10-15 and resumed 10… |
| `1b44ee54e` | `600763d61` | perf(parse): rustc_compat inside the tree build (E31 P7 attempt) |
| `eabb60076` | `e1c2eec31` | Revert "perf(parse): rustc_compat inside the tree build (E31 P7 attempt)" |
| `29f8fb566` | `89e4c8873` | perf(parse): own token conversion over rustc_lexer, the parser's input built directly (… |
| `7ce5658ee` | `7ab5f8d0d` | perf(parse): the common tokens lexed by a byte scan, the rest by rustc_lexer (E31 P4) |
| `459cc8281` | `58f91b9d3` | docs(journal): EXP-031, the parse floor second pass: P2 and P4 kept, P7 rejected |
| `2ecc8a3e8` | `de9dcc9dd` | Merge branch '20261015-parse-floor-2' (E31: own token conversion over rustc_lexer with … |
| `0a6080ca3` | `68889e1dc` | parallel: the calling thread formats units beside threads - 1 spawned ones |
| `1c1405e5f` | `241fe3f46` | Revert "parallel: the calling thread formats units beside threads - 1 spawned ones" |
| `fc2902611` | `c611aac0b` | schedule: split_sim.py and collect_ab.sh, the split file's own schedule |
| `a158b1dac` | `3f73396f1` | schedule: split_sim.py's parts table takes lever (a)'s own units out |
| `53d50f4bc` | `4b971f9e0` | docs(journal): EXP-029, the split file's own schedule; ledger row E29 |
| `67853b623` | `47636edd7` | cli: a large file's item threads capped at the cores no worker holds |
| `efd781c56` | `d61d0a7ed` | Revert "cli: a large file's item threads capped at the cores no worker holds" |
| `4dcec09cc` | `8f7389583` | docs(journal): EXP-029 lever (b) measured and reverted; (c), (d) bounded |
| `84824ec5b` | `6eee45612` | schedule: identity_start.sh, output identity against a CLI without --threads |
| `2b1f6cbc8` | `1f105048a` | docs(journal): EXP-029 gates on the head; ledger row E29 completed |
| `ffe31e998` | `1ea27eee8` | docs(journal): EXP-029 summary: the dynamic cap's idle and spread, the kept files |
| `fac58b1a9` | `177950c2b` | docs(journal): EXP-029 lever (b) in a quieter window (load 2.2-2.4) |
| `5dddfc90b` | `21f6f0ecb` | Merge branch '20261015-split-tail' (E29: the split file's own schedule, refuted; split_… |
| `80de6f1b4` | `fb700ada6` | docs: E31 merged; best-known-good 2ecc8a3 at 203,151,226 Ir (fixtures, -65.4% from the … |
| `2c6bc2119` | `9b2a6d205` | width: a closure body that renders a match arm is never one line |
| `454061e17` | `6bd10b0ed` | docs(journal): E30, the width model, measured before it was built |
| `3d471d966` | `94b0c6b2c` | width: a chain wider than chain_width is never one line |
| `d6b4d73a7` | `b8ba4b519` | Revert "width: a chain wider than chain_width is never one line" |
| `30a0656e4` | `97dc47ce7` | docs(journal): E30, the chain-width rule's commit and revert, and the per-file split |
| `5274f9689` | `288b1e863` | width: rustfmt's rewrite_match is at matches.rs:73 |
| `f0b351244` | `dad996083` | docs(journal): E30 record, wording and the documentation fix |
| `3e29e4a08` | `fe49b198e` | docs(journal): E30, the bound scans on the cliff shape |
| `96c9abceb` | `2e1bc8f5a` | Merge branch '20261015-width-model' (E30: a closure body that renders a match arm is ne… |
| `2cb0ba375` | `3c0ae6838` | docs: E30 merged; best-known-good 96c9abc at 202,822,327 Ir (fixtures, -65.5% from the … |
| `9a917a0b9` | `f40c32dc3` | docs: M6 quiet-host measurement of the wave-5 head (2cb0ba3): 0.214-0.345 / 0.077-0.165… |
| `df30b2a30` | `a3f213289` | chore(pre-commit): autofix run |
| `8f8de80df` | `f053efe9b` | docs(journal): reconciliation pass after wave 5 and M6: the 2026-10-17 retrospective on… |
| `1051c1eaa` | `c66077908` | docs(journal): 2026-10-17 handoff for the whole of PR #88: glossary of the codes, goals… |
