---
title: Design Choice of SAMPL - Written After the First Alpha Release
tags: [design-choices]
---

### Beginning

Starting from May 21, after I finished the algo final at Cornell, I started to develop my own
programming language [SAMPL](https://github.com/SamChou19815/SAMPL). I decide to design a new
language for a while, because I was frustrated by the ugliness of OCaml's namespace but miss its
nice functional features. The exact name of the language was not chosen with much deliberation: I
just want the name to contain a substring SAM.

<!--truncate-->

As soon as I started to design the language, I realized that I need to clearly specify the scope of
this project. In other words, I need to answer following questions before even writing a single line
of code:

- Do I implement an interpreter or a compiler for this language? Or both?
- If I decide to implement a compiler, what's the target code? JVM or native?
- How should the syntax of the language look like?
- What are the supported advanced features? Generics? Module system? Type inference?

I don't have the exact answer when I started, because I want to play with the technology I will use.
I chose [Kotlin](https://kotlinlang.org/) as the implementation language due to its elegance in both
OOP and FP. I chose [ANTLR](http://www.antlr.org/) as the parser generator because its grammar
specification file's syntax was very easy to understand and work with. Also, ANTLR has good IDE
support.

### Design Choices

#### Syntax Design

Without much thought, I started the language with an OCaml-like syntax, because according to my
imagination it should be semantically similar to OCaml. Therefore, if you read the early history of
the grammar specification, you can see a list of familiar keywords: `module`, `type`, `let`,
`match`, `with`, etc.

During the early stages, the exact syntax of the language frequently changes. I'm a newbie of
language design, so I was unaware of many tricky issues of lexing and parsing. For example, I chose
`>>` as the shift-right operator at first, but when I read ANTLR's tutorial on context-sensitive
lexing, I found that things can get bad when we have this type `List<List<T>>`: the final `>>` will
be understood as an operator and a legal program will be incorrectly rejected by the parser. I want
to support generics well, and I don't want to introduce context-sensitive parsing to overly
complicate this project, so I later changed the operator to Kotlin's `shr`.

Starting from this moment, the syntax of the language becomes more and more familiar to Kotlin, and
the syntax design goal has become: to have a Kotlin-like syntax but an OCaml-like semantics. Later,
`module` is renamed to `class`; type declaration is done in a way similar to Kotlin's constructor;
function declaration uses commas to separate arguments; function application needs parenthesis;
generics use `<` and `>` instead of OCaml's `'a`; semicolon is used instead of `in`. The design
principle is similar to Facebook's Reason: write OCaml code in JS way.

#### Type Inference and Type Checking

It wasn’t hard to realize that if we don’t require type inference on functions, then type inference
was almost trivial to implement: a simple environment model will do the job. In addition, we have a
good reason to explicitly specify types of functions at the top level.

```ocaml
let confusing_function g x y = g (g (x,y))
```

```ocaml
let confusing_function (g: ('a _ 'b -> 'a _ 'b)) (x: 'a) (y: 'b) : 'a \* 'b =
g (g (x,y))
```

If you take a look at the code below, you may find the first one very confusing. Can you tell what
is the type of `g` without thinking too much? Do you want to run a type inference algorithm in your
head each time you read someone else's not-type-annotated code?

With this justification, I decide that all functions defined in class level must be type annotated.
Since it's still in prototype and I want to simplify the design, types of arguments for local
functions must also be specified. Now, I can do type inference and type checking simply with the
environment model.

The environment model itself was not hard to deal with, but as I was gradually implementing the
type-checker, I decided that I wanted to support generics, classes, and `private`/`public` modifier
all at once. Classes and access modifiers were relatively easy to implement, although they need some
careful thought.

Generics are notoriously hard to deal with because you need to keep track of an additional list of
information for type, which may be empty, a concrete definition like `List<String>` or just a
parameter like `Set<T>`. To make it worse, I decided that the target code should be able to run on
the type-erased JVM. I know at least on Java and Kotlin, polymorphic values are not allowed. In
TypeScript, the missing generics type will be silently treated as `any`. For example:

```typescript
function test() {
  const anyList = []; // type: any[]
  const stringList1: string[] = []; // type: string[]
  const stringList2 = new Array<string>(); // type: string[]
}
```

This is a huge departure from the OCaml semantics, which permits the polymorphic value:

```ocaml
type 'a option = None | Some of 'a
let empty : 'a option = None
```

In the end, I decided to forbidden polymorphic type, although I thought it was possible to simulate
that by manually doing type erasure. The goal is to simplify the design in the first prototype. In
this way, each expression has a type that is **fixed at any moment**! However, since `throw` is an
expression in my language, we have to introduce this ugly syntax:

`if true then "hi" else throw<String> "bad"`

I hope I can improve this when I'm ready to introduce subtyping and use Kotlin's idea of the
[Nothing type](https://kotlinlang.org/docs/reference/exceptions.html) to prettify the syntax.

#### Code Generation and Compilation

After I finished type checking, I moved to the actual compiler part. At this stage the syntax of the
language is almost fixed: it looks pretty similar to Kotlin. Initially, I thought it may be a good
idea to make it compile to Java code, but a transpilation to Kotlin code seems more elegant:

- Keyword `when` in Kotlin corresponds closely to pattern matching.
- Inline function `run` in Kotlin can be used to implement nested let expression.
- Lambdas in Kotlin are almost the same as functions in my language, except that I had to hack the
  currying feature by myself.
- Data class and sealed class in Kotlin can be used to implement struct and variant easily.

With such compelling reasons, I decide to change the target code to Kotlin. It also has a good
reason: the generated code can work pretty well with a Kotlin code base.

Kotlin supports both OOP and FP well, so should I use visitor pattern or simply pattern matching?
Although I dislike visitor pattern for its verbose syntax, I eventually chose it because pattern
matching can easily produce a giant method of several hundred lines with ugly indentation limited by
the 100-characters limit.

This step is quite straightforward thanks to the excellent FP support in Kotlin.

#### Runtime Injection and Interpretation

After the compiler part is done, the language can be considered as finished (if no bugs exist).
However, it just can't do anything useful. I/O is not part of the language grammar. Without I/O,
although the language has various advanced features, it's NOT Turing complete because it can't even
read in a Turing Machine specification. Thus, runtime is clearly necessary.

It turns out that runtime injection can be done elegantly by reflection. I decide to use the
annotation based scanning to automatically find functions that are declared to be a runtime
function. For the compiler part, I can simply emit an import line with the fully qualified name for
the function's class. For the interpreter part, it can be invoked by reflection, although it is
costly.

Since the type system is sound (if the provided runtime library does not use any unsound features on
JVM platform), we do not need type information for interpretation. Therefore, the interpretation
part is quite easy and I finished it in two nights.

### End

It's almost the end of this blog post, but it's only the beginning of this young language. The test
coverage right now is very poor and I expect to see a lot of bugs. Before I'm able to fix these
bugs, I don't plan to add any complex new features.

In the next few days, I will try to write a Turing Machine simulator in this language to prove it's
Turing complete and use that complex program to test various part of the language design.

There will be no big structural change in near future (which roughly translates to "this year"), so
the design will be stable for a while. Therefore, I think it is now possible to openly invite you to
participate in this project and learn some nice programming language thing. Hope to see you on
[forks](https://github.com/SamChou19815/SAMPL/network),
[pull requests](https://github.com/SamChou19815/SAMPL/pulls) or even
[contributors](https://github.com/SamChou19815/SAMPL/graphs/contributors)!
