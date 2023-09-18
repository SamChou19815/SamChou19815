# Rewriting samlang in Rust

export const additionalProperties = ({ ogImage:
"/blog/2023-01-08-samlang-in-rust/optimization.png" });

## Motivation

Unlike my previous blog post on [samlang](https://samlang.io)'s
[TypeScript rewrite](/blog/2020/08/30/samlang-ts-rewrite), the Rust rewrite does not have that much
stronger motivation. Of course, Rust is much more performant than TypeScript. However, the scale of
the largest known samlang codebase, which is the integration test suite in the samlang repo, doesn't
justify a rewrite for the sake of performance. Despite that the Rust compiler is rewritten in Rust,
a native compiled language, it also doesn't mean that the build process in Rust is faster. Instead,
due to the complexity of the Rust type system, it's at least 2x slower than the previous TypeScript
setup.

With all the counter-arguments above, it's natural to ask myself: why should I still do it? I found
the answer to that question from Kennedy's speech:

> We choose to go to the Moon in this decade and do the other things, not because they are easy, but
> because they are hard.

I choose to rewrite my language in Rust because it's hard. I would like to take the challenge to
do a rewrite from a language with a world-class garbage collector to a language that I need to prove
the memory-safety to the compiler, to use the greater control that Rust can provide in order to
squeeze the last bits of power from bare metal for better performance, and to start from zero again
to build samlang on a much stronger foundation.

Again, because samlang is not a commercial project and it does not have any stakeholders other than
myself, I do not need any compelling business reason to stall the development of new features for
several months. I will do it because I like to.

## The Grand Migration of samlang Core

Like the previous TypeScript migration, I have enforced a hard limit for myself that all the code
must have exact 100% test coverage. This is a hard requirement, since the project is not a gradual
migration when the previous code can be used as a reference implementation. Instead, it has to start
from scratch, so we must have close to absolute confidence in each step of the journey to catch
issues early. The final result is very rewarding, when I can see a long stream of green tests and
100% coverage report in the end.

![test report](/blog/2023-01-08-samlang-in-rust/beautiful-tests.png)

(Figure 1: Beautiful Test Coverage Report)

The migration itself serves as a perfect opportunity for me to ramp up my proficiency with Rust. I
started the journey by fighting with the borrow checker to keep the existing structure, and ended it
when I used the help of the borrow checker to guide me towards a more efficient solution.

The rewrite follows the composition graph of the project, starting in the order of lexer, parser,
checker, and compiler, to the various components like optimizations and IDE services. Apart from my
initial struggle to figure out the best practices to manage lifetimes, the migration is mostly
smooth, and most of the code structure is kept intact with an almost line-to-line rewrite.
Occasionally, I did find it necessary to dismantle some of the functional-programming style
abstractions, so that I can convince the borrow checker that I'm doing things right.

There is one interesting story of the rewrite. When I started to rewrite the pretty-printer component,
I struggled to get past the borrow checker with the old code structure. It forced me to go back to
the drawing table by re-reading the
[Prettier paper](https://homepages.inf.ed.ac.uk/wadler/papers/prettier/prettier.pdf), which later
makes me realize that my old code is horribly inefficient because I jumped too quickly to the final
code section of the paper. It will be an interesting topic that deserves its own post, so I will
stop the discussion here.

## Performance Optimization

When the core and the CLI of samlang are ported, I did a benchmark test, and found that the new
Rust-based binary is only 5x faster than the old JS-based one. This is a disappointing discovery,
because I was hoping for a speedup of at least 10x, given that the Rust-based JS compiler SWC claims
it is 20x faster than the JS-based Babel.

Therefore, for the first time in my life, I did some profiling to figure out the performance
bottleneck. It turned out that a compilation process is spending about 80% of the time on the
optimization stage, and most of the time is spent on inlining and loop optimizations. Then I managed
to optimize away the need for big clones during loop optimizations, and only cloning small functions
during inlining.

These optimizations turned out to be extremely helpful. In the end, I was able to achieve a 16x
speedup on a single core. I'm also happy that I did something good for the environment, since the
speedup is also likely coming from getting rid of some heavy overhead like Node.JS startup.

![16x speedup](/blog/2023-01-08-samlang-in-rust/optimization.png)

(Figure 2: The last benchmark run before the old TS code was deleted)

## Keep the Demo Alive and Fast

One original reason for the rewrite from Kotlin to TypeScript is for JS-interop. By producing a
JS artifact runnable on the web, I can avoid an extra server and a request roundtrip to run the
samlang compiler remotely. With a Rust rewrite, the ability to compile down to JS is gone, because
the Rust language allows us to control the memory layout much more flexibly than JS (e.g. through
explicit stack allocation). However, the Rust compiler does support compiling down to WebAssembly.

In the old TypeScript codebase, samlang is organized into two big packages:

```text
 ----------------              ---------------
 | samlang-core | ===========> | samlang-cli |
 ----------------              ---------------
```

In the old setup, the `samlang-core` package will provide a platform-independent core library that
operates completely inside RAM, which allows the website code to consume this directly. In the new
setup, the same idea applies, but with a new package:

```text
 ----------------              ---------------
 | samlang-core | ===========> | samlang-cli |
 ----------------              ---------------
       ||
       \/
 ----------------
 | samlang-wasm |
 ----------------
```

In the `samlang-wasm` package, I wrapped some core library functions from `samlang-core` in a way
that allows easy JS-WASM interop. I used `wasm-pack` to generate the JS glue code and added some
glue code by myself, to create a `samlang-demo` package that is synced to my website repo with a
simple well-typed interface:

```typescript
type Range = {
  startLineNumber: number;
  startColumn: number;
  endLineNumber: number;
  endColumn: number;
};
export type CompilationResult = string | { tsCode: string; interpreterResult: string };
export function compile(source: string): Promise<CompilationResult>;
export function queryType(
  source: string,
  line: number,
  column: number,
): Promise<{
  contents: Array<{ language: string; value: string }>;
  range: Range;
} | null>;

// ...
```

Since now the [demo](https://samlang.io/demo) is running on much faster WASM, I was able to afford
running all the compilations and language services _on every keystroke_:

![Demo](/blog/2023-01-08-samlang-in-rust/demo.gif)

(Figure 3: Running the demo on the website)

## Final Thoughts

I have heard about the magical power of Rust for the first time in 2018, in Cornell's programming
language class [CS 4110](https://www.cs.cornell.edu/courses/cs4110/2018fa/). At that time, I had
a failed attempt to rewrite one of my [project](https://github.com/SamChou19815/ten-golang) to Rust,
but it ultimately failed due to my unfamiliarity with the borrow checker.

Nevertheless, I always remind myself that one day I should be proficient in Rust, and maybe in one
day, samlang, my flagship project, should be rewritten in Rust. In fact, the project has been
planned since two years ago, when I first ported one of my simpler compiler
[primitivize](https://github.com/SamChou19815/primitivize) to Rust to test the water.

Today, the project is finally done and deployed, and my project plan spanning multiple years has
been brought to a good closure. But this is not the end of samlang. There are still plenty of
performance optimizations to do and new features to develop.
