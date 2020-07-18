---
title: How to Implement Autocomplete
tags: [design-choices]
---

Implement autocomplete in 79 lines of code. Actually, it's not that easy.

Without the infrastructure discussed in this post, that code snippet mentioned here is useless.

I will walk through my journey to implement autocomplete in this blog post, using my programming
language [SAMLANG](https://samlang.developersam.com) as an example.

<!--truncate-->

### Background

First, let's understand what is the autocomplete that I'm talking about. Here are some examples:

![Autocomplete for SAMLANG](/img/2020-01-09-implement-autocomplete/autocomplete-samlang.png)
Autocomplete for SAMLANG (IDE: VSCode)

![Autocomplete for TypeScript](/img/2020-01-09-implement-autocomplete/autocomplete-ts.png)
Autocomplete for TypeScript (IDE: VSCode)

![Autocomplete for Kotlin](/img/2020-01-09-implement-autocomplete/autocomplete-kotlin.png)
Autocomplete for Kotlin (IDE: IntelliJ IDEA)

Autocompletion is a useful service that can usually be well implemented for statically typed
programming languages. The most common form is dot completion: you press `.` after an object
expression, and a list of available fields and methods for that object will popup. It is possible
to build the same feature for dynamic programming languages, but the lack of type information can
decrease the quality of completion results.

### Compiler 101

_Feel free to skip this section if you already know what are ASTs and compiler stages._

Usually, we represent a program as a tree structure instead of a simple string in memory. For
example, this simple TypeScript hello-world program

```typescript
console.log('Hello World!');
```

can be represented as the following tree:

```text

                         /-- Function Call --- \
                        /                       \
                       /                         \
                  ModuleMember                    \
                   /        \                      \
                  /          \                      \
                 /            \                      \
                /              \                      \
               /                \                      \
              /                  \                      \
             /                    \                      \
ModuleReference("console")    memberName="log"   Literal("Hello World")
```

These kind of trees are called abstract syntax trees (AST). Compilers and any other language
services produce ASTs in two or three stages. They first turn a string like
`"console.log('Hello World!');"` into a stream of tokens, where each token represents an indivisible
unit. For example, the above program might be turned into

```text
Identifier("console")
DOT
Identifier("log")
LEFT_PARENTHESIS
STRING("Hello World!")
RIGHT_PARENTHESIS
SEMICOLON
```

This stage is called _lexing_. Then the compiler will organize this stream into a tree structure in
a _parsing_ stage.

It might be useful to attach some location information to each node in the tree. For example, if you
are a compiler author and your job is to point out some type errors in the program, you will need
this location information to help users find the type errors. This location information can be
produced at parsing stage. The above program can be represented by this location-attached AST:

```text

                         /-- Function Call --- \
                        /    0:0-0:26           \
                       /                         \
                  ModuleMember 0:0-0:11           \
                   /        \                      \
                  /          \                      \
                 /            \                      \
                /              \                      \
               /                \                      \
              /                  \                      \
             /                    \                      \
ModuleReference("console")    memberName="log"   Literal("Hello World")
0:0-0:7                        0:8-0:11              0:12-0:26
```

For statically typed programming languages, we want to check whether the program is well-typed. In
other words, each expression or statement has to have a pre-determined type that makes sense. For
this reason, we can also attach type to each node in the AST. Hence, the above program can be
represented by this type-attached AST:

```text

                         /-- Function Call --- \
                        /    0:0-0:26, void     \
                       /                         \
                  ModuleMember 0:0-0:11           \
                   /        \  (string) -> void    \
                  /          \                      \
                 /            \                      \
                /              \                      \
               /                \                      \
              /                  \                      \
             /                    \                      \
ModuleReference("console")    memberName="log"   Literal("Hello World")
0:0-0:7, Module("console")     0:8-0:11              0:12-0:26, string
```

Analyzing types for expressions and statements is the job of _type checking_ stage.

### Type Query is Easy

Autocomplete is not trivial to implement. You don't see all VSCode extensions for a certain
programming language providing autocomplete services. To understand what it is hard, let's first see
why some other feature is easy.

An example of an easy feature is type query. When a user hovers over an expression, he or she might
expect IDE to show some information about this expression. Usually, type information is shown to the
user. Here are some examples of type query:

![Type Query for SAMLANG](/img/2020-01-09-implement-autocomplete/type-query-samlang.png)
Type Query for SAMLANG (IDE: VSCode)

![Type Query for Kotlin](/img/2020-01-09-implement-autocomplete/type-query-kotlin.png)
Type Query for Kotlin (IDE: IntelliJ IDEA)

Once we have a typed AST with locations information, implementing type query is easy. To initiate a
type query, the IDE will send the location of the cursor to the language service. The language
service can find the smallest possible expression that contains the cursor location, and directly
reads the type of the expression. The idea can be expressed by the following pseudocode:

```typescript
function typeQuery(cursorLocation: Location): Type {
  // Smallest range that contains `cursorLocation`.
  const range: Range = getBestRangeByLocation(cursorLocation);
  // Keep track of range to expression mapping elsewhere.
  const expression: Expression = getExpressionByRange(range);
  // Assuming we have typed AST.
  return expression.getType();
}
```

### Autocomplete is Hard

After you have read the type query implementation above, you might think autocompletion is easy,
because we can implement `autocomplete` by `typeQuery`. Namely, we look at the expression before the
`.`, read its type, and find a list of fields and methods available to that type.

However, it is tricker than you expect. The first issue that we might not even have an AST at the
point of type checking. To understand why this is a problem, let's look back into the compiler
stages.

A naive implementation strategy for these stages can be represented by this ASCII art diagram:

```text
Lexing --------> (when program is bad) ---> Throw exception and die
  |                                           /\
  | (Program is still good after lexing)      |
  |                                           |
  \/                                          |
Parsing -------> (when program is bad) --->----
  |                                           |
  | (program is still good after parsing)     |
  |                                           |
  \/                                          |
Type Checking -> (when program is bad) --->----
  |
  | (Program is good after type checking)
  |
  \/
Further Services
```

When we are writing the above hello-world program, the program looks like this at the point when we
need autocompletion:

```text
console.
```

You will see that this program does not parse. Therefore, the compiler will throw and die during
parsing stage and we won't even have an AST. Even if we did some magic to get a valid AST that
parses (for example, by removing the dot), we can still end up getting a program that does not type
check. For example, consider this TypeScript program:

```typescript
type ABC = { foo: string };
const bar: string = { foo: '' }; /* press dot here to autocomplete. */
```

When we press dot after `new ABC("")`, we would expect field `foo` to popup. However, the current
program does not type check since `ABC` is not assignable to `String`. Thus, type checking stage
will throw and we end up having only an untyped AST that we cannot extract type information from.

### A Clever But Flaky Hack

Can we hack? Yes, we can!

I already revealed a hack in the previous section, but let's repeat it again. Usually, the program
fails to parse after pressing a DOT, so we can simply remove the DOT and parse to get an AST.
Although the type checking problem still exists, sometimes we are lucky:

```typescript
function run() {
  // "foo" is assignable to `string` here, so we can autocomplete!
  const bar: string = 'foo'; /* press dot here to autocomplete. */
}
```

The above example shows that when the object expression's type is the same as the type after
autocompletion is done, we can use the simple strategy to autocomplete.

The idea can be represented by this pseudocode:

```typescript
function autoComplete(dotLocation: Location, programWithDot: string): CompletionItem[] {
  const programStringThatParses: string = removeDot(programWithDot, dotLocation);
  const parsedProgram: Program = parse(programStringThatParses);
  const checkedProgram: Program | null = typeCheck(parsedProgram);
  if (checkedProgram === null) {
    return []; // give up
  }
  const queriedType: Type = typeQuery(dotLocation, checkedProgram);
  return findFieldAndMethods(queriedType);
}
```

However, the solution is very flaky. Some random results might popup when we press dot in a random
position. Also, it's sad to have a solution that only works in limited situations.

### The Real Solution

#### The Grand Strategy

You can observe that we are limited by the design decision that we immediate throw an exception and
die when we encounter _any_ problem in the program. This strategy is very easy to follow, but
becomes a bottleneck for autocomplete. Therefore, we will first remove these bad designs. After
that, we will figure out how we can tweak our parser and type checker so that we can work with an
imperfect AST.

#### Recovery Type Checking

Although parsing precedes type checking in compiler stages, it's actually easier to fix the type
checker first. Here is a quick observation:

> When we encounter a type error, the error is usually local.

For example, the error in the first line shouldn't affect type checking the second line:

```typescript
const a: number = 'foo'; // oops, 'foo' is not assignable to `number`.
const b: number = a + 3;
```

If we do not want to do type inference, we can simply use the annotated fact that `a` has type
`number`. Then we are happy to add two numbers together in the second line. In the end, we will have
a two-statement AST with a broken first statement and a well-typed second statement.

To help autocomplete more, we should not return a completed untyped first line like
`const a: ??? = 'foo'`, but instead `const a: number = ('foo' : string)`. Although the AST's type
is inconsistent with each other, it's better than having no types.

Now we have to consider how we collect type errors. In a naive approach, collecting type checker
error is easy: simply catch the exception, print it and die. Now it's slight more involves.

If we are programming in a non-functional programming language, we can use a mutable list to keep
track of all type errors:

```kotlin
class TypeChecker(private val errorCollector: ErrorCollector) {
  // ...
  fun typeCheck(literal: Literal, context: Context): Literal {
    // ...
    if (notWellTyped) {
      errorCollector.add(UnexpectedTypeError(/* some error information */))
      return dummyLiteralThatHasExpectedType
    }
    // ...
  }
  // ...
}
```

In a functional programming language, we can use a monad whose `flat_map` function has type:
`expr -> (expr, error list)`.

Supporting type inference is trickier, since errors can come from more sources other than the main
type checker. For example, the type constraint solver can also generate type errors. Therefore, the
error collector needs to be passed to more classes and we need to be careful to avoid throwing
errors while the type inference engine is still exploring different possibilities. In addition, at
the end of type checking, we must make the AST fully typed instead of leaving out some unresolved
types during type checking.

#### Recovery Parsing

It's harder to do recovery parsing, since parsing errors can be fatal. For example, how can you
parse this nonsensical program with recovery:

```text
I am not a program. HAHA :)
```

Fortunately, parser generators support recovery parsing natively nowadays. I use
[ANTLR4](https://github.com/antlr/antlr4) for SAMLANG, and it defaults to parsing with recovery.
However, you need to be careful. ANTLR4 can generate a parse tree that can be accepted by its
generated visitor interface, but it can return `null` when it cannot deal with a really broken node.
Therefore, you need to handle `null` safety everywhere.

I implemented my language with [Kotlin](https://kotlinlang.org/) and it has nullable types builtin.
Thus, with nullable annotation to almost any parse tree node, we can generate an AST with best
effort, even if the program does not parse. Depending on a situation, when we got `null` for an
expected subexpression, we can either skip the expression or stick in some default value instead.
The final code looks like this:

![Give-up Example for Parsing](/img/2020-01-09-implement-autocomplete/recovery-parsing-give-up.png)
Give-up Example for Parsing

![Dummy Value Example for Parsing](/img/2020-01-09-implement-autocomplete/recovery-parsing-dummy.png)
Dummy Value Example for Parsing

#### Autocomplete with Imperfect AST

Now that we have parsing and type checking with recovery, we still have a typed AST with locations
even if we have partial expressions like `console.`. With the new infrastructure, the partial
expression `console.` will be parsed into

```text
     ModuleMember
      /     \
     /       \
console  [INCOMPLETE]
```

We have a placeholder at the position after the dot, but we don't care since it is exactly the stuff
we are supposed to fill in.

The expression `console.`'s type depends on the context, but we still know the type of `console` is
a module and thus we can fetch the module's type definition to find a list of members.

#### Optional: Incremental Parsing and Type Checking for Performance

If you implement the above strategy in a naive way, autocompletion will be extremely expensive. You
have to re-parse and recheck every single file whenever the user presses a dot. If you have a
codebase of a million lines of code, the IDE will freeze. Therefore, we must have incremental
parsing and type checking for performance.

We can do incremental parsing and checking down to expression levels, but it's usually sufficient
to figure out the files that need to be rechecked, since a single source file is not and should not
be terribly long.

Doing incremental parsing is easy: IDEs can tell us which files are changed and we can simply
re-parse those files. Incremental type checking is slightly more involved. We cannot just recheck
changed files. Consider these two TypeScript files:

```typescript
// a.ts
export const a: number = 1;

// b.ts
import { a } from './a';
const b: number = a + 1;
```

If we changed `a`'s type in `a.ts` from `number` to `boolean`, it will introduce a type error in
both `a.ts` and `b.ts`. The fundamental reason is that `b` uses an exported value in `a`. In other
words, `b` depends on `a`. When `a` changes, potentially all files that depend on `a` (known as
reverse dependencies) must be rechecked.

Therefore, we have to maintain a reverse dependency table and incrementally update it based on
re-parsing results. Then we use the table to find all the files that have to be rechecked.

#### Final Autocomplete Code

With all the infrastructure setup, autocomplete is actually not very hard to implement. It only
takes me 79 lines of code to get it done:
[link](https://github.com/SamChou19815/samlang/blob/9212e81fb6678b48a5949325265ec10a72caebf3/src/main/kotlin/samlang/lsp/LanguageServerServices.kt#L18-L96).

### My Journey

I organized this blog post in the approximate order I implemented various stages. I first came up
with the clever hack while I was interning at Facebook. The code is still
[there](https://github.com/facebook/pyre-check/blob/365a908c9fac91daab69ae05876a69c2404882e1/server/autoComplete.ml#L1-L213),
but it was disabled by default and gated behind a flag, because it is unstable with the reasons I
mentioned above.

I started the SAMLANG project a year ago. I implemented the type checker with type inference,
cleaned up the code and added module system in the summer, and finally introduced incremental
checking and various IDE feature support in this winter. I wanted autocomplete for my language for
a long time, now it's finally there!

### Amazing Results

![Autocomplete for SAMLANG](/img/2020-01-09-implement-autocomplete/autocomplete-samlang.png)
![Autocomplete for SAMLANG 2](/img/2020-01-09-implement-autocomplete/autocomplete-samlang-2.png)
![Autocomplete for SAMLANG 3](/img/2020-01-09-implement-autocomplete/autocomplete-samlang-3.png)
![Autocomplete for SAMLANG 4](/img/2020-01-09-implement-autocomplete/autocomplete-samlang-4.png)
