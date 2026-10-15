# Document 3: ChatGPT's research report, verbatim

Provenance: the answer to document 1's brief after the owner's retort (document 2), pasted by
the owner into the chloro session on 2026-10-15 and recorded here unedited. It is an
**external research synthesis: its claims and recommendations are pending verification**. The
report says itself that it did not inspect chloro's source tree, profiling traces or benchmark
harness, and that it could not verify the Zig commit's figures. Two places lose mathematical
notation in the paste (the memo-table growth expressions in A2 and the exponent thresholds in F);
they are kept as pasted. The chloro figures it quotes come from the brief, which took them from
this repository's records, except the 405-file directory timings (218.5 / 275.4 / 554.8 ms),
which are the owner's own measurements and are not in this repository.

---

You're right. You gave me a detailed research brief and a bibliography, and I responded with a summary of how I would approach it instead of doing the research. I'm sorry.

Here is the substantive investigation, starting with what the external evidence actually establishes, then connecting the mechanisms to Chloro's architecture, experiments, and engineering trade-offs. I'll distinguish published results from deductions about Chloro, because those are not interchangeable.

Research report: Finding Chloro's next major performance gains

1. Executive synthesis

Chloro has already removed a substantial class of overhead by replacing a general-purpose runtime syntax tree with a flat preorder representation. The next opportunities are therefore less likely to come from repeating that transformation and more likely to come from eliminating repeated work, improving locality in the remaining data structures, and making the formatter's execution more predictable.

My initial ranking is:

1

Dense rewrite memoisation and metadata

High priorityHigh confidence in testability

The flat syntax array makes node-indexed side arrays an unusually natural experiment. Replacing a hash lookup with direct indexing can remove hashing, key construction, and pointer chasing from a frequently executed path. Whether it improves total runtime depends on the memo table's hit rate, entry size, and memory footprint.

2

Transactional output assembly

High priorityMedium confidence in payoff

Chloro's retry-based layout protocol may repeatedly construct strings that are later discarded. A shared output buffer with length checkpoints and truncation could eliminate intermediate allocations and copies. This is promising only if the existing retries really do rebuild substantial output.

3

Binary layout and hardware-counter-guided optimisation

Medium priorityMedium-low confidence in payoff

LLVM's BOLT experience shows that binary layout can improve runtime even when executed instruction counts barely change. Chloro's reported instruction-cache and branch-misprediction counts justify investigation, but not an assumption that either is currently responsible for a large fraction of wall time.

4

Scaling experiments to detect algorithmic cliffs

High priorityHigh confidence in diagnostic value

Carefully designed input families can reveal repeated scans or retry behaviour that ordinary repository benchmarks conceal. An asymptotic improvement can be more valuable than another small constant-factor optimisation.

5

Persistent-process editor latency

High product value potentialLow confidence in current implementation payoff

A daemon can retain process state and caches across saves, changing the economics of item-level caching. But this is an editor-latency project, not automatically a batch-throughput optimisation. Chloro needs an end-to-end measurement before this becomes an established performance win.

These are rankings of research priorities, not predictions of speedup. Dense memoisation is relatively cheap to test; a parser fork or new document-layout architecture is not. The best experiment is often the one that resolves an important uncertainty for the least engineering effort.

2. The most useful quantitative evidence from other projects

Three source findings already help frame the investigation.

LLVM BOLT: 12% faster Clang builds

The LLVM optimisation guide reports a reduction from 202.72 seconds to 180.11 seconds for a Clang build using a BOLT-optimised binary, on top of PGO and LTO. That's approximately 11.2% relative to the reported baseline. Instruction counts remained roughly constant while L1 instruction-cache misses per thousand instructions fell from about 22 to 15.

Interpretation: code placement can matter independently of instruction-count reduction. This does not establish that Chloro's executable has the same opportunity.

Android Rust toolchain: approximately 6.1% incremental gain from BOLT

Google's Android toolchain report gives a 24.7% speedup over its baseline after combining several optimisations, including PGO and BOLT. The reported incremental gain from adding BOLT to the PGO configuration is about 6.1%. The benchmark measures building Rust components of Android images, not formatting source files.

Interpretation: post-link optimisation can add value after PGO, but the measured result belongs to a particular large compiler toolchain and training workload.

Lezer: compact trees and incremental reuse solve different problems

Lezer stores syntax in compact arrays and reuses unchanged tree fragments across edits. Its documentation explicitly notes that small changes can still force substantial reparsing when they change how later input is interpreted.

Interpretation: compact representations are relevant to Chloro's batch parser; incremental reuse is primarily relevant to a persistent editor workflow and requires additional correctness machinery.

The critical evidence gap is that none of these measurements establishes Chloro's remaining wall-time distribution. Chloro's own profiles and controlled experiments must decide which external mechanisms are applicable.

3. A. Compiler design and compact representations

A1. Carbon: what comes after flattening the syntax tree?

Why it matters to Chloro

Carbon's compiler-design work is relevant because its central question is how to design compiler internals around modern hardware rather than around traditional object-oriented compiler abstractions. Chandler Carruth's CppNow talk discusses data-oriented design and the interaction between language design, parsing, and compilation performance.

Chloro has already taken an important step in that direction. A flat preorder syntax array replaces pointer-heavy runtime tree traversal with compact node records and integer indices. That reduces the cost of navigating syntax, but it does not guarantee that every subsequent data structure is equally efficient.

For example, Chloro can still incur pointer chasing in hash maps, repeatedly load node metadata, dispatch through large rewrite functions, or construct temporary strings. A flat tree improves the representation of the syntax, not automatically the representation of every computation performed on it.

How the underlying technique works

Data-oriented compiler design starts with the data a phase consumes and produces, then structures it to make the common operation cheap.

The relevant mechanisms are:

* Index-based ownership: nodes refer to other nodes through integer indices rather than owning nested objects. This can reduce per-node overhead and improve locality.
* Structure of arrays versus array of structures: storing node kinds, flags, parent indices, and ranges in separate arrays can improve access when a pass needs only one or two fields. Keeping fields together can be better when a pass consumes most of a node record. Neither arrangement is universally superior.
* Phase-specific representations: parsing, rewriting, layout, and emission need different information. A representation suitable for one phase can be wasteful for another.
* Explicit lifetime boundaries: scratch data can be discarded wholesale when a file finishes, rather than individually freed.
* Predictable traversal: sequential passes over arrays are easier for hardware prefetching and often easier for compilers to optimise.

The danger is replacing one general-purpose representation with multiple specialised representations that require conversions, duplicated state, and complicated invariants.

What transfers to Chloro

The highest-value question is no longer whether a flat tree beats a pointer-rich tree. Chloro's own results already support that architectural choice.

Instead, ask whether the remaining hot state can be represented more directly:

* Dense rewrite memo tables indexed by node.
* Compact node-kind flags and precomputed classifications.
* Separate arrays for frequently accessed layout metadata.
* Per-file scratch storage with an explicit reset boundary.
* A more linear pipeline between rewriting and printing, where semantics permit it.

Experiment

Choose one frequently accessed structure, preferably rewrite memoisation. Compare its existing representation against a dense indexed implementation without changing the rewrite algorithm.

Measure total elapsed time, instructions, cache misses, allocations, peak RSS, and memory initialisation cost. Test small and large files because dense tables can lose on small inputs when initialisation costs more than the hash lookups they replace.

Stop if: the improvement disappears on the representative corpus, or memory growth outweighs a marginal speed gain.

Engineering cost: low to medium for one isolated side array; high if the experiment becomes a broad redesign of every compiler phase.

A2. Zig: dense metadata and direct indexing

The cited Zig commit is a useful lead, but I could not independently verify the precise before-and-after benchmark and cache-miss figures attributed to it from the retrieved source material. I would therefore treat the reported performance improvement as unverified, not as a quantitative precedent for Chloro.

The underlying design question is nevertheless concrete.

Suppose Chloro memoises rewrite results using a key consisting of a node ID and a rewrite shape. A hash map has to calculate a hash, probe a table, compare keys, and load the associated value. A dense representation may replace some of those operations with direct addressing.

There are several possible layouts:

Layout	Main advantage	Main cost
Hash map keyed by node and shape	Stores only populated entries	Hashing, probing, key storage
One slot per node	Fast direct lookup	Can waste space if only a few nodes are memoised
Dense node × shape table	Direct lookup for every combination	Space grows with both dimensions
Per-node compact shape mask plus values	Avoids a full rectangular table	More complicated lookup and bookkeeping

If there are nodes and possible rewrite shapes, a full table grows as . A per-node slot grows as , but works only if the algorithm can represent the required memoisation state within that slot.

The critical issue is that direct indexing is not free. Clearing an -element table every file may cost more than hashing a small number of actual lookups. A generation counter can sometimes avoid full clearing, but adds state and rollover considerations. If values are large, the table can increase cache pressure even when lookup instructions decrease.

Chloro experiment: instrument memo lookups, hit rates, shape frequencies, and occupied entries. Then compare the current map with a dense table and a sparse-per-node alternative. Keep the rewrite semantics unchanged. Measure both total time and memory on the same corpus.

Recommendation: this is one of the best low-cost experiments because the flat tree gives Chloro a natural integer key. But do not infer a gain from the Zig result until the exact commit, benchmark environment, and changed representation have been verified.

A3. Lezer: compact parsing and incremental reuse

Lezer is designed for editor-oriented parsing. Its documented syntax representation stores selected node types with start/end positions and flat child sequences. It uses compact arrays for groups of small nodes and supports incremental parsing by reusing fragments from previous trees.

Three distinct ideas matter here.

1. Compact syntax storage

Lezer's tree omits some grammar productions from the stored tree when they are not useful to consumers. This reduces memory and tree-construction costs, but makes some forms of analysis less convenient.

Chloro has already made its own representation trade-off. Before copying Lezer's choices, determine whether its current flat representation stores information that formatting rarely uses, or whether its remaining parser cost is predominantly lexer work, parser-state transitions, node construction, or error handling.

2. Incremental parsing

Instead of reparsing an entire file after each edit, an editor parser can reuse unaffected syntax fragments. This can dramatically reduce the amount of work required for small changes.

However, incremental parsing is not equivalent to caching a formatted file. An edit can change the interpretation of subsequent input, invalidate parser context, or affect formatting decisions far beyond the edited region. Lezer explicitly documents this limitation.

For Chloro, incremental parsing would require a way to represent edits, identify reusable syntax regions, invalidate dependent formatting state, and guarantee the same output as a full parse and format.

3. Parser architecture and maintenance

A custom parser fork is a much larger commitment than a dense memo table. It entails grammar maintenance, compatibility with evolving Rust syntax, malformed-input handling, diagnostics, testing, and likely a growing divergence from upstream fixes.

The correct prerequisite is a parser-only profile. Benchmark parsing independently from formatting across:

* Small, medium, and large files.
* Different syntax densities.
* Comments and strings.
* Nested expressions and long chains.
* Macro-heavy and malformed source.

If parsing accounts for 28% of instructions, that establishes a potentially meaningful ceiling for improvements that affect only parsing—not a prediction of achievable speedup. A hypothetical 20% reduction in parser cost would reduce total instruction count by about 5.6%, assuming the profile proportions remain comparable. It would not necessarily reduce wall time by the same amount.

Recommendation: optimise the existing parser's hot operations before considering a fork. Incremental parsing is worth investigating for editor-save latency, but it is not the obvious next step for batch formatting.

4. B. CPU behaviour, PGO, binary layout, and branch avoidance

B1. PGO and BOLT solve different problems

Profile-guided optimisation (PGO) and post-link optimisation (BOLT) are complementary rather than interchangeable.

PGO gathers runtime information during representative executions and uses it while recompiling. It can influence inlining, code generation, branch probabilities, and other compiler decisions.

BOLT works later in the pipeline. It uses profiles of the executable's actual machine-code behaviour to improve binary layout. Depending on configuration, it can reorder functions and basic blocks, separate cold code, and apply machine-code-level optimisations.

The distinction matters because the linker must place functions somewhere even after LTO and PGO. Frequently executed code can be separated by rarely used code, increasing instruction-cache and instruction-TLB pressure.

LLVM's Clang case study is particularly instructive: adding BOLT to PGO+LTO reduced a reported build from 202.72 to 180.11 seconds. The guide reports a decline in L1 instruction-cache misses per thousand instructions from roughly 22 to 15, with executed instruction counts staying approximately constant. It also reports smaller gains when the compiler was used on a different project, with improvements depending on the workload profile.

Google's Android Rust toolchain report describes a similar progression: about 19.8% speedup over its baseline with PGO and earlier optimisations, then 24.7% after adding BOLT. The incremental BOLT gain was approximately 6.1% relative to the PGO stage. The result is specific to building Rust components for Android.

These results establish that binary layout can matter even after PGO. They do not establish the magnitude of Chloro's opportunity.

Why Chloro might benefit

Chloro's reported workload had approximately 5.2 million instruction-cache misses and 4.8 million branch mispredictions over roughly 312.7 million instructions.

Those counts warrant investigation, but absolute counts are insufficient to determine how much time was lost. The cache-miss event may refer to a particular cache level or counter definition; a miss can have different consequences depending on overlap with other execution. Branch misses also vary in penalty depending on the processor and surrounding work.

The next step is to measure:

* Elapsed time and instructions per byte formatted.
* Cycles and instructions per cycle.
* Branches and branch misses, including rates rather than just totals.
* Instruction-cache and instruction-TLB misses, with exact event definitions.
* Front-end stalls where supported.
* Performance on the actual target CPU architecture.

Practical experiment

Build four comparable binaries if the toolchain and build configuration support them:

* Baseline release build.
* PGO build.
* PGO plus LTO build, if separate from the baseline configuration.
* PGO/LTO plus BOLT.

Use a representative training corpus and a disjoint validation corpus. Run each binary on the same input and machine, with identical formatting options. Repeat runs in a controlled order, and verify exact output equality.

If BOLT requires relocation information or a specialised linker configuration, include that engineering cost in the evaluation. Measure binary size and build complexity as well as runtime.

Stop if: BOLT improves counters but does not reliably improve elapsed time on the target workloads, or the gain is too small to justify a more complicated release pipeline.

B2. Branch-avoiding algorithms: when branchless code helps

The cited paper Branch-Avoiding Graph Algorithms demonstrates that branch mispredictions can substantially reduce performance in certain graph algorithms. Its experiments report losses of 30–50% in the studied settings. Those results concern specific algorithms and processors, not formatting workloads.

The principle is straightforward. A modern CPU speculatively executes instructions based on predicted branches. If a branch is difficult to predict, a wrong prediction can require discarding speculative work.

A branchless alternative might use arithmetic, masks, lookup tables, or conditional selection. But eliminating a branch can increase the number of instructions, introduce dependencies, or require extra memory accesses. A predictable branch can be cheaper than a complicated branchless expression.

For Chloro, potential targets include:

* Syntax-kind classification.
* Common layout decisions.
* Fast paths in list formatting.
* String-scanning decisions.
* Frequently executed dispatch in rewrite functions.

The important distinction is between a branch that is frequent and a branch that is frequently mispredicted. A branch taken 99% of the time is often predictable. A branch that depends on irregular syntax patterns may be harder to predict, but the branch itself might not dominate execution.

Experiment: choose one hot function, inspect its generated assembly, and record its branch-miss rate and elapsed time. Compare the original implementation with one alternative. Avoid simultaneously changing the algorithm, data structure, and code layout.

A branchless version should win on the actual workload, not merely have fewer branch instructions.

B3. Hot/cold splitting and function layout

Hot/cold splitting separates common paths from rarely executed paths such as errors, fallback logic, or diagnostics. This can improve instruction-cache density and reduce interference between frequently used functions.

For Chloro, functions such as write_list, chain rewriting, overflow rewriting, and memoisation are candidates only if profiles show they are genuinely hot. The names alone are not evidence.

A practical approach:

* Capture a release-like profile using representative Rust files.
* Inspect hot call paths and binary layout.
* Identify large functions containing both common and uncommon branches.
* Test targeted splitting or BOLT.
* Compare elapsed time, instruction-cache misses, code size, and correctness.

Do not manually split functions merely because they look complicated. The compiler may already perform useful transformations, and additional boundaries can interfere with inlining.

5. C. Formatter architecture, pretty-printing, and transactional output

C1. Three different jobs: layout, line breaking, and emission

Formatter architectures often combine three logically distinct tasks:

* Construct a representation of the desired output, including alternative layouts and break opportunities.
* Decide which layout fits the configured width and other formatting constraints.
* Emit the selected layout into a string or output buffer.

Separating these stages can improve clarity and make it easier to reason about width calculations and layout alternatives. But an intermediate representation also has a cost: it must be constructed, stored, traversed, and eventually printed.

Chloro's architecture already uses rewrite and printing logic with retry behaviour. The question is not whether a document IR is theoretically elegant; it is whether the current retry protocol repeatedly constructs expensive intermediate output.

C2. Ruff: performance through a compatible, integrated implementation

Ruff's formatter was designed as a high-performance alternative to Black, prioritising compatible formatting and integration with Ruff's wider toolchain. Its documentation reports greater than 99.9% line-level agreement on extensive Black-formatted projects such as Django and Zulip. That is a conformance measurement, not a speedup measurement.

The transferable lesson is architectural rather than numerical: a formatter can be designed around performance without inventing a radically different style, and compatibility can be treated as a first-class requirement.

For Chloro, the relevant questions are:

* How much work is done in each formatting pass?
* How often are alternatives reconsidered?
* How much intermediate state is allocated?
* Which operations must be repeated when a line overflows?
* How much output is copied between temporary strings and final output?

Ruff's performance should not be attributed to any single implementation technique without evidence isolating that technique.

C3. Biome: a document IR and a buffer abstraction

Biome's formatter library makes the distinction between formatting representation and output explicit. It defines FormatElement and a Document representation, a Formatter that builds formatting output, and a printer that emits the document. Its best_fitting mechanism can try multiple layout alternatives, and its formatter exposes state snapshots for restoring state when backtracking is needed.

This is particularly relevant to Chloro because it demonstrates that an IR, a printer, and backtracking can coexist. It does not prove that replacing Chloro's current architecture with Biome's would improve performance.

The central trade-off is:

* A document IR can represent layout alternatives cleanly and defer emission until layout decisions are known.
* Direct streaming can avoid storing a large intermediate document.
* A transactional sink can allow direct output while retaining the ability to roll back failed attempts.

Chloro's current architecture may benefit from the third option without adopting a full document IR.

C4. Oppen's pretty-printing algorithm

Swierstra and Chitil's paper, Linear, Bounded, Functional Pretty-Printing, describes functional implementations of Oppen's algorithm with the efficiency of the original imperative approach. Oppen's algorithm aims for linear-time pretty-printing and bounded storage for the relevant streaming model, producing output as soon as enough input has been inspected to determine the layout.

The underlying insight is that a formatter need not always materialise the entire output or repeatedly construct candidate strings. It can represent structural events, track line-width information, and make decisions as the stream progresses.

However, Chloro has a complication: some formatting attempts may be retried. A streaming algorithm that commits output irreversibly cannot directly replace an algorithm that relies on backtracking.

C5. The transactional output sink: the strongest immediate experiment

Consider a current interface conceptually like this:

fn rewrite(node: Node) -> Option<String>

An alternative might write directly into a shared output buffer and return a success/failure result. Before trying an alternative layout, the caller records the current buffer length:

let checkpoint = output.len();
if !try_layout(node, &mut output) {
    output.truncate(checkpoint);
}

This avoids constructing and returning a fresh string for every successful rewrite attempt. If an attempt fails after writing some text, truncation discards the speculative output.

The basic design is simple, but correctness depends on more than the bytes in the buffer.

A rollback must also restore any mutable state that affects subsequent output, such as:

* Current line width.
* Indentation.
* Pending whitespace or line breaks.
* Line-ending state.
* Memoisation results whose values depend on the attempted layout.
* Source-position or diagnostic bookkeeping, if those are updated during emission.

A checkpoint can therefore be a small struct containing the output length and the necessary state snapshot.

The other complication is width accounting. If width is tracked incrementally, truncation must restore the previous width rather than trying to infer it from the retained suffix. If Unicode width or special line-ending rules apply, the width model must exactly match existing semantics.

Experiment design

* Instrument how many temporary strings are created and how many bytes are copied during rewrite and printing.
* Record retry counts and the amount of output discarded by failed attempts.
* Implement a transactional sink behind a narrow interface.
* Test it on the most retry-heavy formatting paths first.
* Compare output against the existing formatter and the rustfmt compatibility suite.
* Benchmark allocation count, bytes allocated, copied bytes, instructions, elapsed time, and peak RSS.

The key diagnostic is whether discarded work is substantial. If retries are rare and intermediate strings are short, the complexity may not pay off.

My assessment: test this before undertaking a broad formatter redesign. It targets a concrete architectural cost and can be introduced incrementally. Do not combine it with a bump arena in the first experiment: if performance changes, you want to know which mechanism caused it.

6. D. Allocation strategies and memory locality

D1. Bump allocation

A bump allocator reserves a region of memory and allocates by advancing a pointer. Instead of invoking general-purpose allocation logic for every object, most allocations become a small pointer update plus any required alignment.

Oxc's allocator documentation describes this model for AST processing: many objects are allocated during parsing, share a common lifetime, and can be released together when the AST is dropped. It also provides allocator-aware containers and strings.

This is attractive when the workload has many short-lived objects with a common lifetime. It is less attractive when objects must be individually freed, survive different phases, or require substantial reallocations.

What could fit Chloro?

Potential candidates include:

* Per-file temporary layout objects.
* Scratch arrays and auxiliary metadata.
* Temporary node classifications.
* Intermediate structures with a clear per-file lifetime.

The fit is less obvious for the final output string, which may grow dynamically and has a different lifetime from most scratch data. A shared transactional output buffer might need ordinary Vec-style capacity management even if other temporary data uses an arena.

Costs and failure modes

An arena can improve allocation throughput while increasing peak memory usage. A large allocation that exceeds the current region may require a new chunk. The allocator may retain unused capacity until the arena is dropped. If a single long-lived object keeps the arena alive, otherwise-dead data can remain resident.

Allocator substitution and arena allocation are different interventions:

* Allocator substitution changes the general-purpose allocator used by allocations.
* Arena allocation changes object lifetime and allocation strategy in selected parts of the program.

A faster allocator does not remove string copying or reduce the number of allocations. An arena might reduce per-allocation overhead but make memory retention worse.

First experiment

Collect allocation counts and allocated bytes by phase. If practical, separate allocation time from deallocation time. Then test a per-file arena for one class of temporary data.

Compare:

* Wall-clock time.
* Total allocation calls.
* Bytes allocated.
* Peak RSS.
* Retained capacity between files.
* One-thread and multi-thread memory use.

Chloro's low measured RSS is already a competitive property. An arena that saves a small amount of time while materially increasing peak memory should not be accepted automatically.

Stopping rule: do not adopt a general arena until a narrowly scoped experiment demonstrates a repeatable benefit. Test output assembly and arena allocation separately.

7. E. Parallelism and persistent editor processes

E1. File-level parallelism versus finer-grained tasks

Chloro already schedules files across workers and parallelises sufficiently large files. These are two different forms of parallelism.

* File-level parallelism distributes independent files across workers.
* Task-level parallelism splits the work within a file into independently executable tasks.
* Work stealing lets idle workers take queued work from busier workers, helping when task sizes are uneven.
* Unit-level scheduling exposes smaller independently formattable units, but requires a clear definition of their dependencies and ordering.

For a formatter, finer-grained parallelism is not free. Formatting decisions can depend on parent layout, width, comments, indentation, and neighbouring syntax. If workers format fragments independently, the parent may need to redo work when their output does not fit.

Splitting tasks also increases scheduling overhead and may worsen locality by distributing related syntax across cores.

Why cores may be idle

Possible causes include uneven file sizes, a few large files dominating the tail, serial phases within each file, input/output overhead, worker startup, or lock contention. An idle core does not by itself prove that the scheduler is inadequate.

Experiment: capture per-file and per-phase timings, worker utilisation, queue depth, and task-duration distributions. Compare the current scheduler against dynamic scheduling using the same files and worker count. Test small-file-heavy and large-file-heavy corpora separately.

Only then consider splitting individual files into finer units.

E2. Persistent processes: a different performance objective

Projects such as eslint_d and prettier_d use background processes to reduce repeated startup costs. eslint_d forwards command-line invocations to a background server and explicitly notes that some editors already cache ESLint instances, in which case its wrapper provides no additional performance benefit.

That qualification is important: a daemon is useful only if it removes work that the client would otherwise repeat.

A persistent Chloro process could retain parsed configuration, caches, and reusable allocations. This is potentially valuable when a developer repeatedly saves the same small group of files.

But it introduces operational requirements:

* Correct invalidation when a file changes.
* Cache keys that include all relevant configuration and formatter versions.
* Handling file replacement, renames, and changes to metadata.
* Process lifecycle and recovery after crashes.
* Memory limits and cache eviction.
* Protocol and socket overhead.
* Ensuring output matches a fresh process exactly.

A persistent process can improve editor responsiveness without improving cold batch throughput at all.

Two scoreboards

Batch throughput	Editor-save latency
Total wall time for a directory	Time from save request to completed formatted output
Files and MB per second	Median, p95 and p99 latency
CPU utilisation and scaling by thread count	Cold request versus warm request
Peak RSS and total allocated bytes	Memory growth over long sessions
Correctness across corpus and options	Output identity with a fresh-process run

For the daemon, run repeated saves on both unchanged and changed files. Measure process startup separately from request latency. Include configuration changes, file renames, and invalidation tests.

Recommendation: prototype a persistent process only after establishing the cold and warm single-file latency baseline. Do not use directory throughput to claim success for an editor-latency optimisation.

8. F. Algorithmic cliffs and scaling

The One Billion Row Challenge illustrates how much performance can come from changing the amount of work, rather than simply making an existing operation cheaper. The fastest submissions combined approaches such as partitioned processing, efficient byte-level parsing, custom data structures, and branch-avoiding inner loops. The reported results are tied to a highly constrained aggregation task, not a general parser or formatter.

For Chloro, the equivalent opportunity would be a syntax shape that triggers disproportionate work.

Examples worth investigating include:

* Long method or field chains.
* Deeply nested expressions.
* Large lists that repeatedly recalculate layout.
* Long comments or strings.
* Macro-heavy files.
* Many nearly overflowing lines.
* Syntax that causes repeated fallback or rewrite attempts.

How to identify a scaling cliff

Generate families of valid Rust inputs with controlled structure. For example, create chains of 100, 200, 400, 800, and 1,600 elements while keeping the syntax shape otherwise consistent.

Measure elapsed time and allocations for each size. Repeat with several different structures and with formatting widths that trigger different layout decisions.

A useful first model is:

T(n) = a + b n^p

where captures fixed overhead, captures a scale factor, and approximates the scaling exponent over the tested range.

* : roughly linear growth.
* : quadratic-like growth over the measured range.
* A changing exponent: possible phase transition, cache effect, or algorithmic behaviour that changes with size.

These are diagnostic approximations, not proof of asymptotic complexity. Small inputs can be dominated by startup and fixed overhead; large inputs can cross cache or allocation thresholds. A single fitted exponent can therefore be misleading.

Experiment: for each syntax family, record time per byte, total allocations, retry count, and bytes copied. Fit the curve before and after an optimisation. A lower constant with an unchanged exponent is still useful, but it is different from eliminating repeated work and changing the scaling behaviour.

The strongest result would be an identified operation whose work grows superlinearly, together with a change that makes the work approximately linear while preserving exact output.

9. G. Research process: autoresearch without benchmark overfitting

Karpathy's autoresearch project uses a constrained loop: change an in-scope implementation, run a fixed-budget experiment, record a metric, and keep or discard the change. Its instructions emphasise a fixed evaluation harness and a machine-readable results log.

That is a useful foundation for Chloro's isolated worktrees and bounded experiments. But systems performance introduces several traps.

Benchmark noise. A short run can be affected by frequency scaling, background processes, cache state, and filesystem activity. A single result is not sufficient to establish a win.

Selection bias. If dozens of variants are tried, some will appear faster by chance. The more experiments are run, the more important independent confirmation becomes.

Benchmark overfitting. An optimisation can exploit one fixture's syntax distribution or one particular file size without helping normal workloads.

Interacting changes. Two individually beneficial changes can interfere. For example, an arena may change allocation patterns enough to alter the apparent benefit of a transactional sink.

Correctness regressions. A speedup is irrelevant if it breaks rustfmt compatibility on a rarely used configuration or malformed source.

Recommended experiment record

Each experiment should record:

Field	What to capture
Hypothesis	The specific bottleneck expected to improve
Change	Exact commit and implementation difference
Baseline	Commit, compiler version, flags, and machine
Workload	Corpus identity, file count, sizes, and configuration
Correctness	Fixture, registry, benchmark, and option-matrix results
Measurements	Median elapsed time, spread, instructions, allocations, RSS
Counter evidence	Relevant event definitions and normalised rates
Decision	Keep, reject, or repeat
Explanation	Why the result supports or falsifies the hypothesis

A failed experiment should still be useful. If a dense table reduces instructions but increases cache misses and total runtime, record that finding; it eliminates a plausible direction under the tested workload.

A practical policy is to require repeated measurements and a clean confirmation run on the baseline and candidate. Use a separate validation corpus for changes that could be workload-specific. When two changes interact, test the combined version only after their individual effects are understood.

The current Chloro setup—isolated worktrees, written hypotheses, and correctness gates—is a good fit for this approach. The missing discipline is to make the evidence threshold as rigorous as the implementation workflow.

10. Comparison matrix: which experiments deserve attention?

The following ratings are qualitative engineering judgements based on Chloro's supplied profile and architecture, not measured speedups.

Direction	Potential upside	Cost	Correctness risk	Memory implications	Evidence strength	First experiment
Dense memoisation	Medium	Low–medium	Low–medium	Could increase RSS	Strong rationale; Chloro gain unmeasured	Compare direct indexing with current hash map
Transactional output sink	Medium–high	Medium	Medium–high	May reduce temporary allocations	Strong architectural rationale; gain unmeasured	Instrument retries and discarded bytes
BOLT / binary layout	Low–medium, possibly higher	Medium–high	Low–medium	Binary may grow	Strong evidence elsewhere; transfer uncertain	Profile and test a BOLT release build
Branchless hot paths	Low–medium	Low–medium	Medium	Usually small	Mechanism established; target-specific	Rewrite one demonstrably mispredicted hot branch
Parser optimisation	Medium–high if parser dominates	Medium–high	High	Depends on representation	Chloro's parser cost is measurable; best target unknown	Parser-only profiles and scaling tests
Per-file bump arena	Low–medium	Medium	Medium	Potentially higher peak RSS	Established technique; Chloro gain unmeasured	Arena-allocate one class of scratch data
Incremental parsing	Potentially high for editor latency	Very high	Very high	May retain old syntax and caches	Established in editor parsers; transfer uncertain	Prototype on a small, well-defined edit workflow
Finer-grained parallelism	Medium for skewed workloads	High	Medium–high	More concurrent scratch state	Plausible; utilisation data needed	Measure task-duration imbalance
Persistent process	Potentially high for warm saves	Medium–high	High for cache invalidation	Persistent growth	Established workflow pattern; Chloro result unmeasured	Compare cold CLI and repeated warm requests
Algorithmic cliff fixes	Potentially very high	Unknown until diagnosed	Medium–high	Depends on solution	No Chloro-specific evidence yet	Synthetic scaling families
Full document-IR redesign	Uncertain	Very high	Very high	Additional representation overhead possible	External architectures documented; no transfer result	Prototype one difficult formatting construct
Automated experiment loop	Indirect but valuable	Medium	Low for tooling; high for weak evaluation	Depends on harness	Strong methodological rationale	Repeatable benchmark and result-validation pipeline

The two highest-value diagnostic investments are instrumentation of retry/output behaviour and scaling tests for syntax-dependent cliffs. The most attractive low-cost implementation experiment is dense memoisation. BOLT is worth investigating because the profile suggests a plausible opportunity, but it needs evidence from the actual release binary.

11. Prioritised experiment roadmap

I would organise the next research cycle into six stages, with each stage informing the next.

01

Establish a trustworthy baseline

Prerequisite

* Freeze the benchmark corpus and configuration matrix.
* Record the exact source commit, compiler version, flags, CPU, and OS.
* Run repeated baseline and candidate measurements on the same machine.
* Separate parser, rewrite, layout, output, and CLI overhead where practical.
* Preserve exact-output comparisons against the existing correctness baseline.

Deliverable: a reproducible baseline with variance estimates and per-phase timings.

02

Run low-cost representation experiments

Test dense memoisation and, separately, a compact metadata layout.

* Record lookup frequency, hit rate, and occupancy.
* Compare hash-map overhead against dense-table initialisation.
* Measure time and memory on both small and large files.

Decision: keep only changes with a repeatable improvement across representative workloads.

03

Investigate output assembly

Instrument allocations, bytes copied, retry counts, and discarded output before implementing a transactional sink.

If discarded work is substantial, implement the sink behind a narrow interface. Keep arena allocation out of this experiment.

Decision: proceed only if the instrumentation identifies meaningful repeated work and the sink preserves all layout semantics.

04

Diagnose CPU execution behaviour

Collect normalised hardware counters and inspect hot assembly. Test BOLT against the existing PGO build, then test individual branch changes independently.

Decision: retain a more complicated build pipeline only if the wall-time improvement is repeatable and worth the maintenance cost.

05

Find scaling cliffs

Generate controlled inputs for chains, nested expressions, large lists, comments, strings, and overflow-heavy layouts.

Fit scaling curves, inspect repeated work, and identify any algorithmic changes that eliminate unnecessary rescans or retries.

Decision: prioritise fixes that remove repeated work over constant-factor changes when the affected workload is relevant.

06

Investigate editor latency and larger architectural changes

Prototype a persistent process with a narrow request protocol and explicit cache invalidation. Separately evaluate whether parser architecture changes are justified by parser-only profiles.

Only after measuring these should Chloro consider a parser fork, incremental parsing, or a broad document-IR redesign.

How to prevent experiments from interfering

Three rules are particularly important:

* Do not combine the transactional sink and arena initially. Both may reduce allocation counts, making it impossible to identify the useful mechanism if tested together.
* Do not use instruction count as the sole selection metric. The BOLT evidence shows why elapsed time and CPU behaviour must be evaluated separately.
* Do not use batch throughput to evaluate daemon success. Measure warm editor requests, tail latency, and cache correctness independently.

For any change that survives its first experiment, perform a clean confirmation run. For potentially workload-specific changes, use a validation corpus that was not used to develop the optimisation.

12. What not to do yet

Several attractive ideas are not justified as immediate implementation projects.

Do not fork the parser just because parsing is 28% of instructions. That figure establishes an opportunity ceiling, not an implementation target. First identify which parser operations are expensive and whether the same costs persist across different syntax distributions.

Do not adopt a full document IR just because Biome uses one. Chloro's retry protocol may permit a cheaper transactional sink. A complete rewrite introduces representation overhead, more complex layout semantics, and a substantial compatibility burden.

Do not replace every hash map with dense arrays. Direct indexing is attractive when keys are dense and lookups frequent. It can waste memory and time when entries are sparse or the table needs frequent clearing.

Do not introduce a general-purpose arena before measuring allocation costs by phase. Chloro has already reduced allocation counts substantially. The next question is which remaining allocations cost enough to justify lifetime changes and extra retained memory.

Do not assume BOLT will deliver the Clang result. Clang's executable and build workload differ from Chloro. Collect profiles from Chloro's own representative workloads and judge the actual wall-time improvement.

Do not pursue finer-grained parallelism without evidence of imbalance. Scheduling overhead and extra shared state can outweigh gains when files are already small and cheap to process.

Do not treat a daemon as a universal speedup. A persistent process is most compelling when repeated requests amortise startup costs or benefit from retained caches. It may do little for a one-off command or an editor that already maintains a long-lived formatter service.

Do not let automated agents optimise a single headline benchmark. Require output identity, configuration coverage, independent reproduction, and a written explanation of the mechanism. Otherwise, the research process can systematically reward overfitting.

13. Source audit and bibliography

The following sources were retrieved during this investigation. I've separated primary technical material from secondary reporting and flagged gaps rather than filling them with assumed results.

Primary technical sources

* Carbon: Modernizing Compiler Design for Carbon's Toolchain — compiler architecture, data-oriented design, and modern hardware. The retrieved material confirmed the talk's scope, but did not independently establish a numerical speedup for Chloro-relevant techniques.
* Zig AST rework, cited commit — intended source for the representation comparison. The exact before-and-after metrics and cache-miss claims were not independently verified here.
* Lezer system guide — compact trees, incremental parsing, contextual tokenisation, and reuse.
* LLVM BOLT: Optimizing Clang — primary practical case study for post-link optimisation.
* Android: Faster Rust Toolchains for Android — quantitative results for codegen-unit changes, section garbage collection, LTO, PGO, and BOLT.
* Branch-Avoiding Graph Algorithms — primary paper on branch avoidance in graph algorithms.
* The Ruff formatter — original formatter design discussion; the retrieved formatter documentation also establishes the compatibility philosophy.
* Biome formatter documentation — format IR, printer, output buffers, and snapshots for restoring state.
* Linear, Bounded, Functional Pretty-Printing — primary paper on functional implementations of Oppen's algorithm.
* oxc_allocator documentation — bump allocation, allocator-aware containers, and common-lifetime AST storage.
* Karpathy's autoresearch experiment instructions — fixed-budget experimentation and explicit keep/discard decisions.

Secondary explanations and workflow references

* InfoQ: One Billion Row Challenge — useful synthesis of parsing, partitioning, memory access, and branch avoidance. The original challenge is the better source for individual implementations and reproducibility.
* eslint_d — persistent-process workflow and its limitations when editors already cache formatter or linter processes.
* TypeScript's Go port — useful context for language-toolchain architecture and performance motivation, but not direct evidence for a Chloro optimisation.
* prettier_d and Black — additional daemon workflow references.
* Autoresearch explanation and Automated research swarms — secondary discussions of autonomous experimentation.
* Sub-millisecond Rust optimisation discussion — anecdotal optimisation examples, not a reliable quantitative baseline for a production formatter.

Quantitative claims that remain unverified or non-transferable

* The precise Zig AST-rework speedup and cache-miss reduction attributed to the cited commit.
* Any numerical speedup attributed to Carbon's architecture that has not been tied to a specific experiment and measurement method.
* A Chloro-specific gain from BOLT, a custom allocator, dense memoisation, or a transactional output sink.
* A Chloro-specific improvement from incremental parsing or persistent-process caching.
* Any conclusion that the reported branch mispredictions or instruction-cache misses explain a particular percentage of Chloro's elapsed time.

The cited Android and LLVM numbers are verifiable reported measurements, but they are measurements of different programs and workloads. The Chloro baseline figures and architecture details in this report come from the research brief you supplied; I have not independently inspected Chloro's source tree, profiling traces, or benchmark harness.

14. Final assessment

The most important question for Chloro is no longer how to apply every optimisation used by modern compilers. It is how to determine which remaining costs are intrinsic to rustfmt-compatible formatting and which are artefacts of the current implementation.

My recommended order is:

* Dense memoisation, because it is a bounded experiment with a clear architectural rationale.
* Output/retry instrumentation followed by a transactional sink if justified, because it can expose and remove repeated work in a central part of the formatter.
* Scaling experiments, because an algorithmic cliff could offer a larger gain than a collection of micro-optimisations.
* Hardware-counter-guided BOLT and targeted branch experiments, because the evidence makes them worth testing but does not establish their payoff for Chloro.
* A separate persistent-process latency investigation, because it may materially improve editor responsiveness even if batch throughput remains unchanged.

The parser, bump arena, and full document-IR redesign remain conditional projects. Their priority should rise only when measurements identify a cost they can actually remove.

One final point: the reported increase from approximately 5 MB/s to 11–12 MB/s, and to about 13 MB/s with PGO, suggests that Chloro has already captured significant gains. At this stage, the best research programme will probably produce a mixture of modest wins, one or two more substantial opportunities, and several well-supported rejections. That is exactly what a mature performance investigation should produce.

The goal is not to keep adding optimisations. It is to build a progressively more accurate model of where Chloro spends its time, then change only the parts of the system for which the evidence supports a change.
