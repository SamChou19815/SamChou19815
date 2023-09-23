# Performance Optimization on the samlang Compiler

export const additionalProperties = ({ ogImage:
"/blog/2023-09-23-samlang-perf-opt/comparison.png" });

With a long-planned Rust rewrite of samlang finally done at the start of 2023, I decided to focus this year on fixing a lot of rough edges of the language, including syntax, semantics, and IDE services. Nine months later, I managed to tick quite a few items in the continuously growing [checklist](https://github.com/SamChou19815/samlang/issues/921).

However, clearing a large number of small items is never as exciting as a big number change. In the oxidizing Rust world, there is probably no other topic more exciting than massive performance improvement. Despite increasing complexity in the language, I managed to achieve about 40% compilation time reduction on a growing integration test suite. This blog post will describe some of the techniques.

![Benchmark Comparison](/blog/2023-09-23-samlang-perf-opt/comparison.png)

(Figure 1: Benchmark Comparison. Top: samlang v0.8.0. Bottom: samlang v0.9.1)

## Benchmark Technique

I ensure that the benchmark actually measures the compiler performance, instead of heavily affected by start-up times. It doesn't run the program 10000 times, but instead sets a special environment variable to instruct the compiler to repeat 10000 times.

However, it's worth noting that the current benchmark is not scientific at all. It's only a measure of time of running compiler end-to-end on the samlang integration test 10000 times. I have made no attempt to make the integration test an accurate approximation of real world programs.

Feel free to take the result with a grain of salt, but with margins this large, the performance gain is definitely real.

## String Interning and Small String Inlining (-20%)

String interning is always a contentious strategy for performance optimization. It certainly has the potential to help performance, especially when strings are commonly shared. However, it also has some serious downsides. Frequent string pool existence check in several threads might create a lot of competition for locks. Given this fact, doing string interning for all strings in a program can easily lead to worse performance, and I don't remember any language that makes interned strings the default.

However, in the compiler settings, benefits can outweight costs. A long of symbols are likely to be used at least twice. For instance, variables are used once on the definition site, and once or more in use sites in most cases. Struct and field names are probably used more often. By performing string interning and returning an opaque id, we can make copying significant cheaper by avoiding reference counting, and making comparison much cheaper since it's just a simple integer comparison, which helps performance of symbol lookup table.

Using an opaque id for strings everywhere introduces new problems as well. It causes more indirections when we want to inspect the contents of strings. Fortunately, it is relatively uncommon that we care about the actual content of identifiers. Apart from pretty printing and error messages, all we care about is the identity of symbol rather than its content. Hopefully your program doesn't contain a billion of type errors, so performance hit on error message printing is acceptable. Since pretty printing is much cheaper than type checking and compilation, that tradeoff is also worth to take.

What about garbage collection? By using an easily copiable opaque id everywhere, we lose the ability to deallocate the string once it is unused. For compilation, this is less concerning. Once we get there, we already stopped caring about most of the identity of strings, so they can be all dropped all at once. It will indeed become a big issue for a long running language server running on a large monorepo, where unused identifiers will eventually eat up all the available memories. I don't have a planet scale monorepo of samlang code for me to actually run into this problem, but it is still interesting to solve. In the end, I did implement an incremental garbage collector in the userland for interning strings. I use the fact that I only intern program symbols, so we only need to scan all the ASTs. Garbage collection will be run after certain number of rechecks.

String interning is implemented in [PR#918](https://github.com/SamChou19815/samlang/pull/918), and actual garbage collection is done in [PR#935](https://github.com/SamChou19815/samlang/pull/935). It ends up with around 10% compilation time reduction.

The optimization above completely ignores the potential performance regression in multi-thread settings. While samlang is not running anything in parallel yet, I don't want to actively doing things to prevent that. One way of preventing a lot of contention for global lock is by reducing the pressue on the global string interning pool.

In programming languages, most of the strings are quite small. You are probably not writing 42-character variables all the time. In [PR#1002](https://github.com/SamChou19815/samlang/pull/1002), I introduced small string optimizations for strings less than 8 bytes and got a 10% compilation time reduction. I might revisit this later to expand the length to 15 bytes.

## Avoiding Unnecessary Computation and Allocation (-30%)

I took a lot of shortcuts for convenience to finish the Rust rewrite as soon as possible. Not a lot of care has been put into writing the most performant code. Unsurprisingly, there are a lot of low hanging fruits to pick.

I previously convert the source code string into a `char` vector during the lexing stage. It was done just to convince the Rust standard library that the lexed tozens are still in valid UTF-8 forms. It turns out that it is quite an expensive way to ensure correctness. After all, we are only lexing ASCII keywords and operators. In [PR#943](https://github.com/SamChou19815/samlang/pull/943), I started to unsafely operate on raw bytes directly. Combined with other small optimizations, it brings down the total compilation time by a staggering 20%.

During compilation, I used to have a single intermediate representation for programs before and after generics specialization. This forces me to use a vector type to keep track of generic type arguments for functions, even after generics specialization is done. A lot of empty vectors are unnecessarily allocated as a result. Simply by having two different specialized intermediate representations in [PR#1007](https://github.com/SamChou19815/samlang/pull/1007), we can cut the compilation time by another 15%.

There are probably still quite a lot of stupid inefficiencies like this...

## Future Plans

This is certainly not the end of performance optimization of samlang. The next obvious thing to do is to parallelize parsing, type checking and compilation. However, I do hope to expand the number of integration tests first to have some more realistic benchmarks.

I plan to do the first 100 LeetCode problems in samlang and make them integration tests to battle test the standard library correctness, compiler performance, and compiled code performance. It will also force me to improve the ergonomics of the language. The first few things that come to my mind is arbitrary-depth pattern matching, mutable data structure support and a standard library that contains `HashMap` and `TreeMap`.

Try it out in the [demo page](https://samlang.io/demo)!
