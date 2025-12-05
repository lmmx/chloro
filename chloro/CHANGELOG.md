# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.7.3](https://github.com/lmmx/chloro/compare/chloro-v0.7.2...chloro-v0.7.3) - 2025-12-05

### <!-- 2 -->Bug Fixes

- collect enum variant info with whitespace ([#71](https://github.com/lmmx/chloro/pull/71))
- preserve blank line between comment block and func ([#70](https://github.com/lmmx/chloro/pull/70))

### <!-- 4 -->Documentation

- version

## [0.7.2](https://github.com/lmmx/chloro/compare/chloro-v0.7.1...chloro-v0.7.2) - 2025-12-05

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

### <!-- 9 -->Other

- sort order rustfmt 2024 ([#62](https://github.com/lmmx/chloro/pull/62))

## [0.7.1](https://github.com/lmmx/chloro/compare/chloro-v0.7.0...chloro-v0.7.1) - 2025-12-01

### <!-- 2 -->Bug Fixes

- retain comments and attributes for macro calls ([#50](https://github.com/lmmx/chloro/pull/50))

### <!-- 9 -->Other

- extend exprs ([#56](https://github.com/lmmx/chloro/pull/56))
- self sort ([#52](https://github.com/lmmx/chloro/pull/52))
- imports testing ([#51](https://github.com/lmmx/chloro/pull/51))

## [0.7.0](https://github.com/lmmx/chloro/compare/chloro-v0.6.7...chloro-v0.7.0) - 2025-11-26

### <!-- 2 -->Bug Fixes

- avoid comment loss in enums and structs ([#40](https://github.com/lmmx/chloro/pull/40))

## [0.6.7](https://github.com/lmmx/chloro/compare/chloro-v0.6.6...chloro-v0.6.7) - 2025-11-25

### <!-- 2 -->Bug Fixes

- avoid comment dropping ([#38](https://github.com/lmmx/chloro/pull/38))

## [0.6.6](https://github.com/lmmx/chloro/compare/chloro-v0.6.5...chloro-v0.6.6) - 2025-11-25

### <!-- 2 -->Bug Fixes

- comments whitespace handling ([#37](https://github.com/lmmx/chloro/pull/37))
- correct newline

### <!-- 9 -->Other

- exclude glob for diff cat

## [0.6.5](https://github.com/lmmx/chloro/compare/chloro-v0.6.4...chloro-v0.6.5) - 2025-11-25

### <!-- 1 -->Features

- correctly handle nested imports ([#36](https://github.com/lmmx/chloro/pull/36))

## [0.6.4](https://github.com/lmmx/chloro/compare/chloro-v0.6.3...chloro-v0.6.4) - 2025-11-24

### <!-- 1 -->Features

- improved generic and where handling ([#35](https://github.com/lmmx/chloro/pull/35))

## [0.6.3](https://github.com/lmmx/chloro/compare/chloro-v0.6.2...chloro-v0.6.3) - 2025-11-24

### <!-- 1 -->Features

- improve struct literal handling ([#34](https://github.com/lmmx/chloro/pull/34))

## [0.6.2](https://github.com/lmmx/chloro/compare/chloro-v0.6.1...chloro-v0.6.2) - 2025-11-24

### <!-- 2 -->Bug Fixes

- function conformance upgrade ([#33](https://github.com/lmmx/chloro/pull/33))

## [0.6.1](https://github.com/lmmx/chloro/compare/chloro-v0.6.0...chloro-v0.6.1) - 2025-11-24

### <!-- 9 -->Other

- bump ra_ap_syntax; use blazon for badges; new textum CLI for readme conf ([#22](https://github.com/lmmx/chloro/pull/22))
- r-a MSRV; de-default ctor; split ws crates

### <!-- 4 -->Documentation

- oops
- link to diffs
- *(conf)* align the top 5s
- smaller table
- smaller conf
- markdown conf report
- no score
- explain conformance test leaderboard
- use conf table in README

### <!-- 5 -->Refactor

- tidy up conformance leaderboard

## [0.6.0](https://github.com/lmmx/chloro/compare/chloro-v0.5.2...chloro-v0.6.0) - 2025-11-21

### <!-- 9 -->Other

- updated the following local packages: chloro-core

## [0.5.2](https://github.com/lmmx/chloro/compare/chloro-v0.5.1...chloro-v0.5.2) - 2025-11-21

### <!-- 4 -->Documentation

- not absolved :(

## [0.5.1](https://github.com/lmmx/chloro/compare/chloro-v0.5.0...chloro-v0.5.1) - 2025-11-21

### <!-- 4 -->Documentation

- released at 0.5

## [0.1.0] - 2024-01-01

### Added
- Initial release
