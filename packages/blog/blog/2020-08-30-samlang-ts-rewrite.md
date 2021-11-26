# Rewriting samlang in TypeScript

A lot has changed since my [last blog post](/2020/05/17/samlang-in-browser) on my effort to make
samlang run in browsers. The samlang demo site has been merged into the samlang documentation site
as a [demo tab](https://samlang.developersam.com/demo). In addition, samlang can now emit optimized
JavaScript code as its second target language. However, the most exciting changes come from a
complete overhaul of the codebase: _rewriting samlang in TypeScript_.

## Engineering problems with Kotlin

Kotlin is a great language. It has a powerful static type system that can catch a lot of issues
during compile time. For example, the biggest advantage of Kotlin over Java is its compile-time null
safety. Kotlin also has a sweet syntax. It offers extension and inline functions that allow you to
write code in a functional way without worrying about the extra cost. The same features also allow
you to write DSL-like code without using a new language.

However, Kotlin is not perfect, especially on the ecosystem and tooling side.

Since the Herculean effort to [make samlang run in browsers](/2020/05/17/samlang-in-browser), the CI
running time of samlang project has skyrocketed to 8 minutes. It is very difficult to cut down the
CI time with the Kotlin build infrastructure. Kotlin officially supports Maven, Gradle, and Ant
build system, but only Gradle has the best support. Although Gradle does get the best support, the
introduction of multiplatform build has significantly slowed down the build due to its large
overhead. In addition, the multiplatform support for JavaScript target is still in the experimental
stage so undocumented behavior and bugs happen all the time, and it imposes a huge engineering
burden to maintain it when there is a Kotlin version update.

The multiplatform setup also introduces parser maintenance problems. Instead of the idealized
picture painted by Kotlin team where I can write the code once and the Kotlin compiler can correctly
produce JVM and JS code for me effortlessly, the reality is that I have to write the parser adapter
code twice. Sadly, this is the
[least painful hack](/2020/05/17/samlang-in-browser#stage-2-fight-the-parser) I can found among
several other choices.

Tooling is also a big concern for Kotlin ecosystem. Kotlin is invented by JetBrains. JetBrains want
you to do everything in their IDEs, so that CLI tooling support for Kotlin is very limited. For
example, the only code coverage tool that can work with Kotlin is JaCoCo, and it seems to have some
[issues that requires hacks](https://tech.wayfair.com/2018/02/kotlin-and-jacoco-will-they-blend/).
The best known Kotlin formatter and linter [ktlint](https://github.com/pinterest/ktlint) has a huge
overhead when it's run over Gradle, and it sometimes
[conflicts with unconfigurable IDE settings](https://github.com/pinterest/ktlint/issues/527) until
recently.

## Reasons for a TypeScript rewrite

Despite Kotlin's numerous advantages, I still decided to switch to TypeScript. At the programming
language level, TypeScript is less safe and less elegant than Kotlin. It inherits a lot of serious
issues from JavaScript, and its evolution speed is bounded above by the evolution speed of
JavaScript. However, if safety and elegance of programming languages are the only concern, then I
should probably rewrite samlang in [Coq](https://coq.inria.fr/) or
[Haskell](https://www.haskell.org/) instead. Considering language ecosystem, tooling, and future
goals of samlang, I would argue that TypeScript is the best language for powering samlang.

The problems mentioned in the above section can be largely mitigated after switching to TypeScript.
TypeScript still has null safety, type inference, and pattern matching via disjointed tagged union,
so we don't lose a lot of type safety after switching. TypeScript shares a large ecosystem with
JavaScript and NodeJS, so we have a lot of tools working with TypeScript out of box: ESLint for
linting, Prettier for formatting, Jest for testing, and Istanbul for test coverage analysis.
TypeScript is also not opinionated in the build system: it's CLI can already compile an entire
directory so additional tooling is usually not needed.

The flexibility of the NodeJS ecosystem means that there are more opportunities to optimize the CI
run times. Unlike the monolithic Gradle build job that has a high initial booting cost, the NodeJS
ecosystem usually writes build steps as NPM scripts. Therefore, the underlying build steps are
usually transparent, and it's easy to break a single monolithic CI job into several smaller ones for
parallel runs.

Switching to the NodeJS ecosystem also unlocks a future opportunity for samlang to be extensible. We
can easily use the dynamic nature of JavaScript to make samlang dynamically configurable via
plugins. We can also dynamically inject standard library into samlang using NodeJS's `require`,
which can unblock samlang's potential to be a flexible Turing-complete language for build systems.

A final important and usually overlooked reason for TypeScript is its flat learning curve. It's not
a problem for me but a potential problem to attract external contributors. There are a lot of
engineers who already know JavaScript/TypeScript thanks to the popularity of web development, but
the number of engineers who know Kotlin is relatively small. If I limit the sample size to the
people I know, then TypeScript to Kotlin ratio is almost 50:1.

## A successful gradual migration to a TypeScript codebase

There isn't an automatic Kotlin to TypeScript converter; therefore, the migration has to be done
manually. I started the migration with the creation of
[GitHub issue 19](https://github.com/SamChou19815/samlang/issues/19) on June 28, 2020, and finished
the complete migration on August 12, 2020. The migration was not an easy undertaking. I think it is
valuable to document some of the highlights.

![100-todo-item migration issue](/img/2020-08-30-samlang-ts-rewrite/migration-todo-issue.png)

(Figure 1: 100-todo-item migration issue)

Although the majority of the code can be migrated to TypeScript quite brainlessly if you know both
languages well, some can be tricky. The biggest obstacle is the lack of universal `equals()` and
`hashCode()` methods in TypeScript. To overcome the issue, I have to create customized `HashMap` and
`HashSet` libraries for TypeScript so that efficiency can be maintained. Even for those brainless
migration work that is basically changing the syntax to TypeScript, it's still a non-negligible
chance that during the migration something is slightly changed due to variable renaming.

### Now unit testing is my best friend

To ensure correctness and build confidence for the migration, unit testing is a focus from day one.
Instead of aiming for a high test coverage percentage like 90%, I setup the CI to enforce 100% test
coverage all the time. Counterintuitively, this extremely strict setup actually helps the
productivity of the migration. Unlike web apps where UI testing can be very time consuming,
samlang's codebase mainly consists of pure functions that can be run in isolation. Therefore, it's
very easy to deterministicaly test every line of code. The 100% test coverage goal helps ensure that
every branch and every function are covered by tests, so I can be certain that every branch does the
correct things (at least according to the tests), and that every branch is _actually useful_. In
fact, the difficulty of attaining 100% test coverage helps to identify some unreachable branches in
some instances.

![test report](/img/2020-08-30-samlang-ts-rewrite/beautiful-tests.png)

(Figure 2: Beautiful Test Coverage Report)

### Improving while migrating

The migration is not only about moving the codebase to a new language. Instead, another goal is to
create a much better foundation for future engineering. Thus, I constantly check whether the
original design decision for a file makes sense before converting that to TypeScript. In case when
the original design doesn't make sense, I first refactored the old Kotlin code to make it more
sensible and then started the migration. This approach prevents the potential
"garbage-in-garbage-out" problem during migration. The biggest win from this refactor-then-migrate
approach is a complete overhaul of samlang's IR (intermediate representation).

Uses of intermediate representations are very common in compilers. They are the intermediate
language between the source-level language and target language (e.g. assembly). The intermediate
layer helps the compiler to modularize the implementation by avoiding doing too much work at once.

```text
-----------      -------      -------      ------------
| samlang | ---> | HIR | ---> | MIR | ---> | Assembly |
-----------      -------      -------      ------------
```

In the past, samlang had two layers of IR between the source-level language and assembly, which I
called HIR (high-level IR) and MIR (mid-level IR). The HIR had a much older history than MIR, and
was poorly designed. Back in the old days when samlang compiler emits Java and TypeScript code
instead of X86 assembly, I needed an IR that closely resembles both Java and TypeScript, so that
compiling from HIR to _readable_ Java and TS is as easy as pretty-printing. Such a weird design
decision caused some extremely ugly design decisions. For example, to ensure that we never return a
`unit` value when compiling to Java code, the IR compiler needs a lot of special-casing in all
possible IR nodes, which can easily miss some edge cases. In addition, in order to compile to a
typed language, it limits the ability of the compiler to turn the source-level language to a more
primitive language closer to assembly. Therefore, even after compiling to HIR, there is still a lot
of tricky lowering work that needs to be done at the HIR to MIR lowering stage.

The problem persisted even after support for emitting Java and TS code is dropped, simply because
everything seems to work and I don't want to deal with a potentially breaking change by changing the
AST. The TypeScript migration offers a perfect opportunity to revisit this design decision, because
I really don't want to replace ugly code with ugly code, and then write ugly tests to ensure 100%
test coverage.

The refactor starts with a goal to move most of the heavy lowering work to the source-level to HIR
lowering stage, in order to see whether we have a chance to get rid of MIR completely. Although it
turns out that the attempt to eliminate MIR failed, a cleaner separation of concern between HIR and
MIR becomes clear:

> HIR should be as close to assembly as possible while maintaining the ability to compiling to a
> high level language. MIR's role is to close the final gap.

It turns out that the refactor results not only in a cleaner design, but also a more comprehensible
codebase, which in turns helps to identify some sketchy code that turns out to incorrectly dropped
side effects of original source-level code. Similarly, unsound behavior of CSE optimization
([common subexpression elimination](https://en.wikipedia.org/wiki/Common_subexpression_elimination))
has been identified by tests and fixed.

### Faster feedback cycle

The TypeScript migration started with the latest technology in the NodeJS ecosystem. Dependencies
are managed by Yarn v2, which eliminates `node_modules` and replaces them with zip archives and
pnp-runtime. Using Yarn v2, we can retrieve zips from CI cache extremely fast and consistently keep
the `yarn install` command under 10 seconds.

Instead of the do-it-all Gradle used for building, testing, and linting, NodeJS ecosystem has
separate tools for each of the tasks above. In addition, a lot of tools can work on TypeScript
directly with the help of Babel, testing doesn't need to depend on compiling TS to JS. Therefore,
it's much easier to split the CI jobs into different independent workflows that can be run in
parallel.

![Splitted CI jobs](/img/2020-08-30-samlang-ts-rewrite/ci-jobs.png)

(Figure 3: Splitted CI jobs)

The end result is a 50% in combined CI run time and 85% reduction in run time when running jobs in
parallel. Now, most jobs can complete in 30 seconds, while the longest testing job takes at most 1
minute. With such a large speed improvement, I'm able to disable pushing to master directly and wait
for all CI jobs to complete without impacting productivity.

### Help from external maintainers

With the transition to TypeScript, samlang codebase becomes much more beginner-friendly given a more
familiar language to most people. During the migration, the amazing
[Megan Yin](https://meganyin.com/) ([GitHub](https://github.com/meganyin13)) implements a
[source-level interpreter](https://github.com/SamChou19815/samlang/issues/28) to help doing
integration tests on compiler correctness, and
[a HIR to JS compiler](https://github.com/SamChou19815/samlang/issues/44) to bring back the ability
to emit JavaScript code. If you want your name to appear on a future blog post, you can take a look
at [this issue](https://github.com/SamChou19815/samlang/issues/82) asking for help on LSP.

## samlang 0.3.0 release

The hard work of the migration results in a much better codebase that allows me to much faster in
the future. Since now the samlang codebase is completely in TypeScript, distribution of the work
becomes much easier.

Let's end the article with a screenshot of the new demo site powered by samlang in TS:

![new samlang demo site](/img/2020-08-30-samlang-ts-rewrite/new-demo.png)
