# Document 1: the research brief, as ChatGPT generated it for the owner (2026-10-15)

Provenance: the owner asked ChatGPT (in their own thread, outside this repository) to turn the
sources of [../2026-10-15-unmined-tricks.md](../2026-10-15-unmined-tricks.md) into a research
brief for a separate conversation. This is that message, verbatim as the owner pasted it into
the chloro session; nothing has been edited. It is external text: a prompt, not a finding.

---

I'd frame this as a research brief for another ChatGPT conversation, rather than a list of topics to explain. The goal is to get someone to connect the underlying ideas to Chloro's actual architecture, performance measurements, and ongoing experiments—and to distinguish techniques that could produce a meaningful gain from techniques that merely sound promising.

I've used the sources from your research note as the starting bibliography, including Carbon, Zig, LLVM BOLT, Android's Rust toolchain, Ruff, Biome, Oppen, Lezer, oxc_allocator, and the autoresearch examples.

Here's a copy-pasteable prompt.

Research brief: Finding the next major performance gains in Chloro, a Rust formatter

Act as a compiler-performance researcher and systems engineer. I want a technically deep, source-grounded exploration of the performance techniques below, contextualised specifically for Chloro, an existing Rust formatter that has already undergone substantial optimisation.

Do not give me generic explanations of the technologies. Explain why each is compelling given Chloro's current architecture, measured bottlenecks, previous experiments, and performance goals. Help me understand the underlying ideas well enough to judge whether an experiment is worth running, what its likely payoff is, and what evidence would confirm or falsify it.

I am interested in learning and making good research decisions, not prescribing an implementation. Do not assume that any particular technique should be adopted.

1. Context: what Chloro is and where it stands

Chloro is a Rust source formatter aiming for rustfmt-compatible output.

Its recent optimisation work has achieved:

* Full agreement with rustfmt across 1,220 rust-analyzer fixtures and the tested option matrix.
* Agreement across 20,540 registry-corpus files, including the tested configurations.
* Agreement on 16 benchmark workloads containing roughly 1,000 Rust files.
* A flat, preorder syntax representation replacing Rowan's red/green tree at runtime.
* Runtime dependencies reduced from 37 to 16.
* Fixture allocations reduced by approximately 43%.
* A measured single-core throughput increase from roughly 5 MB/s to 11–12 MB/s, with PGO reaching approximately 13 MB/s on the measured workloads.
* A content/configuration cache, an opt-in item-level cache, parallel formatting for large files, and improved CLI file scheduling.
* A collection of targeted scanner, list-gap, and string-assembly fast paths.
* An experimental PGO workflow that has been locally validated but still needs release-level validation.

The current feature branch measured approximately 218.5 ms for a 405-file ruff directory workload, versus 275.4 ms for v0.7.0 and 554.8 ms for v0.8.0. Those are three-run measurements from separate sessions, so treat the comparison as indicative rather than a perfectly controlled benchmark. The historical releases also differ significantly in conformance.

On dedicated crate benchmarks, Chloro's peak RSS has ranged from 4–14 MiB at one thread and 5–31 MiB at four threads, compared with roughly 43–85 MiB for rustfmt. These are workload-dependent figures, not a universal memory ratio.

Profiling and performance observations

Earlier profiles suggested approximately:

* Formatter logic: 40% of instructions.
* Parser and lexer: 28%.
* Allocator: 15%.
* Core/std: 8%.
* Flat-tree operations: 5%.
* libc copies: 3%.

A later investigation found that allocation blocks had fallen substantially, but allocator activity still represented a measurable part of execution time.

Branch mispredictions and instruction-cache misses appear worth investigating. The research notes estimate approximately 4.8 million mispredicted branches and 5.2 million instruction-cache misses in one 312.7-million-instruction workload. The exact contribution of these events to elapsed time has not been established with sufficient certainty; validate the assumptions rather than treating them as known wall-time fractions.

PGO substantially reduced instruction count, but its wall-time improvement was smaller and workload-dependent. This motivates investigating execution stalls, code layout, memory access, and parallel utilisation rather than optimising solely for instruction count.

Existing architecture and constraints

* The syntax tree is now represented as a flat preorder array with integer node indices.
* Rewrite memoisation currently uses a hash map keyed by node and shape. Because node IDs index the flat array, a dense side array might be feasible.
* Layout and printing have retry behaviour involving an Option<String>-style protocol. A transactional output sink using length checkpoints and truncation might reduce intermediate string allocations.
* An opt-in item-level cache can avoid reformatting unchanged units, but its cold pass costs roughly 10–15% extra and its process-level benefit is diluted by fixed overhead.
* A persistent formatter process could retain caches across editor saves, but no successful implementation or end-to-end latency result has yet been established.
* The parser is a substantial remaining cost, and a parser fork would be a large maintenance commitment.
* A per-file bump arena, shared output sink, and several width/layout optimisations have been proposed, but overlapping changes should be measured separately.
* Correctness is non-negotiable: exact output agreement, tested configuration coverage, deterministic behaviour, and safe fallback paths matter more than an attractive speedup on a narrow benchmark.

2. What I want to understand

For every topic below, answer four questions:

1. Why is this interesting for Chloro specifically? Connect the idea to its current architecture and measured bottlenecks.
2. How does it work under the hood? Explain the important mechanisms, trade-offs, and prerequisites, not just the high-level concept.
3. What might transfer, and what might not? Distinguish results demonstrated in the source project from plausible hypotheses about Chloro.
4. How would we test it? Propose a concrete experiment, relevant metrics, correctness gates, likely failure modes, and a stopping rule.

Use quantitative results from the cited work where available, but never imply that another project's speedup predicts the same gain for Chloro. Identify any source claims you cannot independently verify.

3. Research topics and source-specific questions

A. Compiler design and compact representations

Carbon's modernising-compiler-design work

Source: https://schedule.cppnow.org/session/2023/modernizing-compiler-design-for-carbons-toolchain/

Chloro has already replaced a general-purpose tree representation with a flat syntax array. That makes compiler architecture research particularly relevant: the project has already demonstrated that representation choices can remove overhead across many code paths.

Explain the principles behind modern compiler representations, arena or index-based structures, locality, ownership, and phase boundaries. Which principles are relevant to Chloro's next stage, now that the major tree-representation change is complete? What lessons should we draw without simply copying another compiler's architecture?

Zig AST rework

Source: https://git.jakstys.lt/motiejus/zig/src/commit/4dca99d3f6b732c415d270f0c97def144ed6d3b7

The research notes attribute fewer cache misses and a substantial wall-time improvement to this AST rework. Investigate what changed, how the result was measured, and why it worked.

Chloro already has flat node indices, so focus on whether there are analogous opportunities in memo tables, side arrays, node metadata, and frequently accessed flags. Explain the trade-off between dense arrays and hash maps, including memory initialisation, sparsity, and cache locality.

Lezer's parser architecture

Source: https://lezer.codemirror.net/docs/guide

Chloro's parser and lexer account for a meaningful fraction of execution. Explain which parser-design ideas from Lezer are transferable to a batch Rust formatter and which depend on Lezer's incremental, editor-oriented use case.

In particular, explore compact syntax representations, reuse, incremental processing, and the costs of maintaining a custom parser. How can we establish whether parser architecture is actually the next bottleneck before considering a fork?

B. CPU behaviour, profile-guided optimisation, and binary layout

LLVM BOLT on Clang

Source: https://llvm.googlesource.com/llvm-project/+/refs/heads/main/bolt/docs/OptimizingClang.md

Android's Rust toolchain optimisation

Source: https://android-developers.googleblog.com/2023/12/faster-rust-toolchains-for-android.html

rustc BOLT tracking

Source: https://github.com/rust-lang/rust/issues/103595

Chloro has already experimented with LTO and PGO. The remaining hypothesis is that reducing instruction count alone is insufficient because execution is affected by branch behaviour, instruction-cache locality, code layout, and stalls.

Explain PGO versus post-link optimisation such as BOLT, the information each uses, what changes in the binary, and why a compiler or formatter might benefit. Examine the cited quantitative results and their experimental conditions.

Then contextualise the ideas for Chloro: a relatively small Rust executable, a parser with event dispatch, and hot formatting functions such as write_list, overflow rewriting, chain rewriting, and memoisation.

Discuss hot/cold splitting, branchless syntax-kind predicates, function layout, and post-link optimisation. Explain what hardware counters can establish and what they cannot. Propose a benchmark protocol that distinguishes a real wall-time improvement from a better instruction count or a misleading counter change.

Branch-avoiding algorithms

Source: https://arxiv.org/pdf/1411.1460

Explain when replacing branches with tables, arithmetic, or branchless operations helps—and when it hurts because of extra instructions, dependency chains, or memory accesses.

Apply this to syntax-kind predicates and layout decisions. What evidence should be required before rewriting a branch-heavy hot path?

C. Formatter layout, output assembly, and avoiding repeated work

Ruff's formatter architecture

Source: https://astral.sh/blog/the-ruff-formatter

Biome's formatter library

Source: https://docs.rs/biome_formatter/

Oppen's pretty-printing algorithm, via Swierstra and Chitil

Source: https://www.cs.kent.ac.uk/pubs/2009/2847/content.pdf

Google's rfmt research

Source: https://research.google/pubs/pub44667/

Chloro's formatting and printing logic accounts for a large portion of execution. It has also identified a potentially important opportunity to reduce intermediate strings through a shared transactional output sink.

Explain the distinction between constructing a layout representation, deciding line breaks, and emitting output. Compare document-based pretty-printing approaches, streaming printers, and direct output sinks.

The crucial Chloro-specific complication is retry behaviour: a formatting attempt may need to roll back output and try another layout. Investigate whether a sink with length checkpoints, truncation, and incremental width tracking can preserve those semantics without rebuilding intermediate strings.

Explain the possible role of layout IRs, document combinators, output buffering, width calculations, and allocation patterns. Which architectural properties make these techniques effective, and which would require a more fundamental rewrite?

The objective is not to copy Ruff or Biome. It is to understand which design choices plausibly explain their performance and what can be borrowed without sacrificing Chloro's compatibility requirements.

D. Allocation strategies and memory locality

oxc_allocator

Source: https://docs.rs/crate/oxc_allocator/0.143.0/source/README.md

Chloro has already reduced allocations and runtime dependencies, but allocator activity remains measurable. A per-file bump arena with grow-in-place is one proposed experiment.

Explain bump allocation, arena lifetimes, bulk reclamation, growable buffers, and the situations where arenas reduce overhead. Cover the costs too: retained capacity, peak RSS, fragmentation-like waste, large allocations, and lifetime constraints.

Relate this to Chloro's formatting pipeline and its need to manage syntax nodes, temporary strings, and layout state. Explain how to determine whether allocations are still expensive enough to justify an arena, rather than optimising an already-solved problem.

Also explain why allocator substitution and arena allocation are not equivalent: changing the allocator may improve allocation speed but increase memory usage or fail to improve wall time.

E. Parallelism and persistent editor processes

TypeScript's Go port

Source: https://www.heise.de/en/background/Web-development-The-new-TypeScript-compiler-in-Go-10357839.html?seite=2

eslint_d

Source: https://github.com/dpinol/eslint_d.js/blob/master/README.md

prettier_d

Source: https://github.com/josephfrazier/prettier_d

Black's daemon mode

Project: https://github.com/psf/black

Chloro already parallelises large files and schedules files across worker threads. Profiling suggests that some multi-file workloads do not keep every core busy for the full run.

Explain the difference between file-level parallelism, task-level parallelism, work stealing, and a queue of independently formattable units. What limits finer-grained parallelism in a formatter? Consider dependencies, ordering, shared state, memory locality, scheduling overhead, and deterministic output.

Separately, explain the persistent-process model for editor saves. Chloro can retain caches and avoid repeated startup, but it must remain correct when files, configuration, and binaries change. Explain cache invalidation, stat-based validation, process lifecycle, socket overhead, and memory growth.

Use the daemon projects to ground the discussion in real workflows. Distinguish their published latency results from what has actually been measured for Chloro.

Propose two separate scoreboards: batch throughput and editor-save latency. Include median and p99 latency, cold and warm operation, memory growth, and output identity.

F. Algorithmic cliffs and scaling

The One Billion Row Challenge

Source: https://www.infoq.com/news/2024/01/1brc-fast-java-processing

Chloro's next major win might come from an algorithmic cliff rather than a general micro-optimisation. For example, particular syntax shapes may trigger repeated scans, repeated layout attempts, or unexpectedly expensive handling of long chains, nested expressions, comments, or macros.

Explain how to detect such cliffs using controlled synthetic inputs at increasing sizes and shapes. Discuss power-law exponents, constant factors, input distributions, and how to distinguish genuinely superlinear behaviour from fixed startup overhead or cache effects.

Propose an experiment that varies both file size and syntax shape, fits scaling curves, and identifies when an optimisation changes the asymptotic behaviour rather than merely reducing a constant factor.

G. Research process and evidence quality

Autoresearch

Source: https://datasciencedojo.com/blog/karpathy-autoresearch-explained/

Automated research swarms

Source: https://rekursiv.ai/blog/autoautoresearch/

Sub-millisecond Rust optimisation discussion

Source: https://redlib.hbubli.cc/r/rust/comments/1hrgir5/solving_aoc_2024_in_under_1ms

Chloro is being optimised through independent agents working in isolated Git worktrees, each with a bounded experiment, a written record, and a correctness gate.

Explain which parts of autoresearch-style loops work well for systems optimisation and which are dangerous. Cover noisy benchmarks, hypothesis quality, negative results, independent reproduction, interacting changes, experiment budgets, and selection bias.

Suggest how to structure experiment records so the project learns even when an agent fails to produce a speedup. Explain how to avoid rewarding benchmark overfitting, excessive code churn, or improvements that disappear under a controlled rerun.

4. How to research and present the findings

Research the linked sources directly. Follow primary sources, implementation notes, papers, and relevant commits where possible. Use secondary sources for context, not as substitutes for the underlying evidence.

For each topic:

* Start with a short explanation of why it is relevant to Chloro.
* Explain the mechanism in enough detail for a systems engineer to reason about it.
* Summarise the source project's actual measured result, if available.
* State what is known, what is inferred, and what remains uncertain.
* Connect the idea to Chloro's existing data structures and performance profile.
* Propose one or more falsifiable experiments with explicit measurements and correctness requirements.
* Identify likely engineering cost, maintenance burden, and the conditions under which the idea should be rejected.

Avoid repeating the same general performance advice in every section. Cross-reference related ideas and explain interactions—for example, why a transactional sink and a bump arena should initially be tested separately, or why better task scheduling and persistent-process latency need different benchmarks.

Do not assume that every topic deserves implementation. A negative result that eliminates an attractive hypothesis is useful.

5. Final deliverable

Produce:

1. An executive synthesis identifying the three to five most promising directions for Chloro, with reasons and confidence levels.
2. A deep-dive section for every topic above, including source-grounded explanations and Chloro-specific implications.
3. A comparison matrix with expected upside, implementation cost, correctness risk, memory implications, evidence strength, and first experiment.
4. A prioritised experiment roadmap that accounts for interactions and avoids testing overlapping changes simultaneously.
5. A source list distinguishing primary sources, secondary explanations, quantitative claims, and unverified claims.
6. A section on what not to do yet: attractive ideas whose evidence is too weak, whose costs are too high, or whose potential gain is already addressed by existing work.

The goal is not to predict a headline speedup. It is to develop a defensible understanding of where Chloro's remaining performance costs come from, which external ideas genuinely transfer, and how to find out efficiently.

Why I structured it this way

A few choices here are deliberate:

* Carbon, Zig, and Lezer are grouped around representation and parser architecture, because Chloro has already made a major structural change. The question is what is still worth borrowing after that work—not whether flat arrays are good in the abstract.
* BOLT, Android, and branch-avoiding algorithms are grouped around CPU execution. They address the gap between instruction-count improvement and actual elapsed-time improvement.
* Ruff, Biome, Oppen, and rfmt are grouped around the central formatter problem: how layout decisions and output assembly interact. This is where a promising low-level optimisation can easily turn into a much larger architectural project.
* oxc_allocator gets its own section because memory reduction, allocation throughput, and wall-clock performance are related but distinct objectives.
* The TypeScript port and daemon tools are separated conceptually into batch parallelism and interactive latency. They solve different problems and should not share one headline performance metric.
* The autoresearch sources inform the research methodology itself, so the result should improve the quality of your next experiments as well as your understanding of individual techniques.

One especially important instruction is the requirement to separate published measurements from hypotheses about Chloro. A paper showing a 20% gain in another system is evidence that a mechanism can matter, not evidence that Chloro has 20% available to recover.

The result you want from that separate chat is not simply a larger list of optimisation ideas. It is a map of the remaining design space, grounded in the work you've already done, with enough technical depth to help you decide what deserves another four-hour experiment—and what doesn't.
