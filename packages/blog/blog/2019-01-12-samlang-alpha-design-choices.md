---
title: Design Choice of SAMLANG in Alpha
tags: [design-choices]
---

### Background

In the last summer, I developed my first programming language SAMPL. Measured against my technical
skills at that time, it was a clear success. I was able to implement an interpreter and a compiler
for a self-designed functional programming language with only the knowledge to implement an
interpreter for a toy language in Cornell CS 3110. I was particularly proud of the module system
and generics in that language.

<!--truncate-->

However, that language has some fundamental limitations. Type inference isn't great because it
requires type annotation for all functional arguments; the code I used to I solve generic parameter
inference looks long and hacky; more importantly, the error messages are really unhelpful which
makes debugging very painful. What's more, I don't like the name of that language. In the last blog
post, I mentioned that I named it "Sound and Modern Programming Language" just to make it contain
the substring SAM, which seems very unnatural.

Given that giant heap of legacy issues, I decided to start from scratch again to design and
implement a better programming language in my mind, with the additional knowledge in programming
language theory thanks to Cornell CS 4110 and past experience of designing a language. I decided to
name it SAMLANG because I can't think of other names. In this week, I archived the old language's
repo, open sourced the new language's repo on [GitHub](https://github.com/SamChou19815/samlang) and
published its [official documentation](http://samlang-docs.developersam.com). Now it's the time
to discuss some of my design choices.

### Influence of Design Choices and My Justification

In the past fall semester, I worked as a frontend developer in Cornell Design & Tech Initiative. I
introduced Flow to my subteam to help type-check the frontend React and JavaScript code. I enjoyed
using functional programming principle in JavaScript with Flow, and I liked the soundness of Flow's
type system and its elegant syntax. However, there is a major drawback in JavaScript: there are no
extension functions. While it is possible to write classes to have methods, it defeats the principle
of functional programming. Therefore, one of the priorities of the new programming language is to
support the extension function in the same way that Kotlin does.

Since I would start from scratch, I had the liberty to design a completely new type system. The
first decision would be whether the typing rule would be a nominal or structural. I initially
favored structural because Flow's type system is structural and it works great with the React
components typing. Nevertheless, as soon as I dug deeper into the typing rules, I found some
fundamental difficulties related to how extension function works.

For example, we may write this in Kotlin to extend a data class, which is closest to a plain
JavaScript object:

```typescript
data class Student(val name: String, val age: String)
fun Student.getNameWithAge(): String = "$name($age)"
```

If the type system is structural, then the user may choose to write:

```javascript
const info = { name: 'RANDOM_BABY', age: 0 }.getNameWithAge();
```

Then deciding which function to call requires iterating over the entire typing environment, which
is inefficient and slow. It also places some burden on the user to reason which function gets
called. In a nominal type system, the search is confined to a single module and it will simply be a
string search in an immutable map in `O(lg n)` time.

Therefore, I decided to use the nominal typing rule to simplify the implementation. It also makes
type inference a lot easier because the type constraints with objects are more explicit under my
design. In order to simplify the object literal type checking, I decided that the object definition
is only fully visible inside the defining module but opaque outside of the module. In order to make
the typing rule consistent, same applies to variant type. I personally think this is a valid
simplification, since it's also a bad practice to expose the fields publicly in other programming
languages.

### Implementation

The greatest challenge is type inference.
[Cornell's course notes](https://www.cs.cornell.edu/courses/cs4110/2018fa/lectures/slides23.pdf)
say that we should use a set of constraints to gradually solve the type, but the examples given are
quite simple.

With some experimentation and thoughts, I introduced undecided type to represent an expression
without clear type in a limited context and free type to represent a free constraint. Then I used a
union-find data structure to keep track of all the aliasing relation detected by the type-checker
between different temporarily undecided types. Right now in one place, it requires an `O(n)`
iteration over all the typing constraints, which I hope I can optimize it away later.

Nonetheless, it's not all about implementation because no type inference algorithm can infer all
type information because there will always be times when there is not enough context. Here is the
simplest example in JavaScript:

```javascript
function notEnoughContext() {
  const identity = (x) => x;
}
```

Even if we require type annotation for the top-level functions, it still doesn't help in this case.
It's also not very good to make the value `identity` generic in this case, because some of my
planned compilation target (like Java) does not support generic values.

There are two options: make the undecided type `unit` or die with a compile-time error. I chose the
latter. The former option may look fine in this case, but it misses an important mistake. Suppose
the user define a (bad) generic function and use it in this way:

```typescript
function random<T>(): int {
  return 0;
}

function useRandom(): void {
  const a = random();
}
```

The random function clearly has a problem: it defines a generic type that is never used. If we
complain when `random` is called, the user can catch his/her mistake.

### Current Status

Right now I only implemented a type-checker, an interpreter, and a pretty-printer. I will try to
do some IR lowering in the near future to support easy compilation to Java and JavaScript. I'm also
planning to support Kotlin, TypeScript, and Flow as additional compilation targets because the
type-checker can already provide rich type information for those languages to use. Again, I would
welcome your contribution. Let's end the blog post with a code snippet on
[the official docs site](http://samlang-docs.developersam.com):

```samlang
// Hello World
util Main {
    function main(): string = "Hello World"
}
// Pattern Matching
class Option<T>(None(unit), Some(T)) {

    public function <T> getNone(): Option<T> = None(unit)
    public function <T> getSome(d: T): Option<T> = Some(d)

    public method <R> map(f: (T) -> R): Option<R> =
        match (this) {
            | None _ -> None(unit)
            | Some d -> Some(f(d))
        }

}
// First Class Functions and Type Inference
util TypeInference {

    // The following two definitions are equivalent.

    function notAnnotated(): unit =
        val _ = (a, b, c) -> if a(b + 1) then b else c;

    /**
     * I am a block comment.
     * Read the docs to discover how we do the type inference.
     */
    function annotated(): unit =
        val _: ((int) -> bool, int, int) -> int =
            (a: (int) -> bool, b: int, c: int) -> (
                if a(b + 1) then b else c
            );

}
```
