# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.7.3](https://github.com/lmmx/chloro/compare/chloro-core-v0.7.2...chloro-core-v0.7.3) - 2025-12-05

### <!-- 2 -->Bug Fixes

- collect enum variant info with whitespace ([#71](https://github.com/lmmx/chloro/pull/71))
- preserve blank line between comment block and func ([#70](https://github.com/lmmx/chloro/pull/70))

## [0.7.2](https://github.com/lmmx/chloro/compare/chloro-core-v0.7.1...chloro-core-v0.7.2) - 2025-12-05

### <!-- 2 -->Bug Fixes

- process children in order with comments and statements ([#69](https://github.com/lmmx/chloro/pull/69))
- output leading comments before visibility/keywords ([#68](https://github.com/lmmx/chloro/pull/68))
- no duplicate comments where already handled by item preamble ([#67](https://github.com/lmmx/chloro/pull/67))
- collect module comments ([#66](https://github.com/lmmx/chloro/pull/66))
- possible fix to unwanted newline in funcs ([#65](https://github.com/lmmx/chloro/pull/65))
- multiple formats for self-format diagnosed issues ([#64](https://github.com/lmmx/chloro/pull/64))
- internal kinds for use ord ([#63](https://github.com/lmmx/chloro/pull/63))
- avoid mod and use movement ([#61](https://github.com/lmmx/chloro/pull/61))
- stop putting blank lines between mod-mod ([#60](https://github.com/lmmx/chloro/pull/60))
- use statement order fix ([#59](https://github.com/lmmx/chloro/pull/59))

### <!-- 6 -->Testing

- format own code ([#58](https://github.com/lmmx/chloro/pull/58))

### <!-- 8 -->Styling

- clippy

### <!-- 9 -->Other

- sort order rustfmt 2024 ([#62](https://github.com/lmmx/chloro/pull/62))
- delete muchos printing code, use a trait ([#57](https://github.com/lmmx/chloro/pull/57))

## [0.7.1](https://github.com/lmmx/chloro/compare/chloro-core-v0.7.0...chloro-core-v0.7.1) - 2025-12-01

### <!-- 2 -->Bug Fixes

- retain comments and attributes for macro calls ([#50](https://github.com/lmmx/chloro/pull/50))

### <!-- 5 -->Refactor

- incremental approach to expr implementation ([#54](https://github.com/lmmx/chloro/pull/54))

### <!-- 8 -->Styling

- clippy

### <!-- 9 -->Other

- extend exprs ([#56](https://github.com/lmmx/chloro/pull/56))
- self sort ([#52](https://github.com/lmmx/chloro/pull/52))
- imports testing ([#51](https://github.com/lmmx/chloro/pull/51))

## [0.7.0](https://github.com/lmmx/chloro/compare/chloro-core-v0.6.7...chloro-core-v0.7.0) - 2025-11-26

### <!-- 2 -->Bug Fixes

- avoid comment loss in enums and structs ([#40](https://github.com/lmmx/chloro/pull/40))

## [0.6.7](https://github.com/lmmx/chloro/compare/chloro-core-v0.6.6...chloro-core-v0.6.7) - 2025-11-25

### <!-- 2 -->Bug Fixes

- clippy
- avoid comment dropping ([#38](https://github.com/lmmx/chloro/pull/38))

### <!-- 4 -->Documentation

- annotate a little more

### <!-- 5 -->Refactor

- extract common comment processing ([#39](https://github.com/lmmx/chloro/pull/39))

## [0.6.6](https://github.com/lmmx/chloro/compare/chloro-core-v0.6.5...chloro-core-v0.6.6) - 2025-11-25

### <!-- 2 -->Bug Fixes

- comments whitespace handling ([#37](https://github.com/lmmx/chloro/pull/37))

## [0.6.5](https://github.com/lmmx/chloro/compare/chloro-core-v0.6.4...chloro-core-v0.6.5) - 2025-11-25

### <!-- 1 -->Features

- correctly handle nested imports ([#36](https://github.com/lmmx/chloro/pull/36))

## [0.6.4](https://github.com/lmmx/chloro/compare/chloro-core-v0.6.3...chloro-core-v0.6.4) - 2025-11-24

### <!-- 1 -->Features

- improved generic and where handling ([#35](https://github.com/lmmx/chloro/pull/35))

## [0.6.3](https://github.com/lmmx/chloro/compare/chloro-core-v0.6.2...chloro-core-v0.6.3) - 2025-11-24

### <!-- 1 -->Features

- improve struct literal handling ([#34](https://github.com/lmmx/chloro/pull/34))

### <!-- 8 -->Styling

- clippy fix

## [0.6.2](https://github.com/lmmx/chloro/compare/chloro-core-v0.6.1...chloro-core-v0.6.2) - 2025-11-24

### <!-- 2 -->Bug Fixes

- function conformance upgrade ([#33](https://github.com/lmmx/chloro/pull/33))

## [0.6.1](https://github.com/lmmx/chloro/compare/chloro-core-v0.6.0...chloro-core-v0.6.1) - 2025-11-24

### <!-- 2 -->Bug Fixes

- *(clippy)* MSRV updates

### <!-- 9 -->Other

- bump ra_ap_syntax; use blazon for badges; new textum CLI for readme conf ([#22](https://github.com/lmmx/chloro/pull/22))
- r-a MSRV; de-default ctor; split ws crates
- temporarily use git dep for crates

### <!-- 1 -->Features

- trait support ([#10](https://github.com/lmmx/chloro/pull/10))
- capture const and static ([#7](https://github.com/lmmx/chloro/pull/7))

### <!-- 2 -->Bug Fixes

- gitignore the larger ones to allow use
- attribute handling and doc comments preserved ([#11](https://github.com/lmmx/chloro/pull/11))
- clippy
- non-debug import

### <!-- 6 -->Testing

- roundtrip test ([#9](https://github.com/lmmx/chloro/pull/9))

### <!-- 8 -->Styling

- do not use diff line prefix as section marker in diff

### <!-- 9 -->Other

- ra fixtures ([#5](https://github.com/lmmx/chloro/pull/5))

## [0.6.0](https://github.com/lmmx/chloro/compare/chloro-core-v0.5.2...chloro-core-v0.6.0) - 2025-11-21

### <!-- 9 -->Other

- more comments ([#4](https://github.com/lmmx/chloro/pull/4))

## [0.5.2](https://github.com/lmmx/chloro/compare/chloro-core-v0.5.1...chloro-core-v0.5.2) - 2025-11-21

### <!-- 6 -->Testing

- test iterate ([#3](https://github.com/lmmx/chloro/pull/3))

## [0.5.1](https://github.com/lmmx/chloro/compare/chloro-core-v0.5.0...chloro-core-v0.5.1) - 2025-11-21

### <!-- 2 -->Bug Fixes

- clippy
- no hunk headers in stored diffs
- lint

### <!-- 6 -->Testing

- test snapshots ([#1](https://github.com/lmmx/chloro/pull/1))
- comparisons
- snapshots
- use asterism fixtures

## [0.1.0] - 2024-01-01

### Added
- Initial release
