# Bounded Qualification in samlang

export const additionalProperties = ({ ogImage:
"/img/2022-09-05-bounded-qualification/bounded-qualification.png" });

## Introduction

Bounded qualification, more commonly known as bounded generics outside of academia, allows you to
add additional constraints to the generic type parameters. For example, you can have the following
code in TypeScript:

```typescript
interface Comparable<T extends Comparable<T>> {
  compareTo(other: T): number;
}
class BoxedNumber implements Comparable<BoxedNumber> {
  constructor(private readonly n: number) {}
  compareTo(other: BoxedNumber): number {
    return this.n - other.n;
  }
}

function v1IsSmaller<T extends Comparable<T>>(v1: T, v2: T): boolean {
  return v1.compareTo(v2) < 0;
}
v1IsSmaller(new BoxedNumber(1), new BoxedNumber(2));
```

Bounded qualification is usually compared to unbounded parametric polymorphism (commonly known as
unbounded generics). In the above example, if we do not allow bounds on the generic parameter `T`,
then we might either need to do some ugly casts, or we will need to pass into an additional
comparator function.

Bounded qualification does not necessarily make the language more expressive and powerful, because
the same constraints can be achieved by passing additional arguments that can support the necessary
interactions needed on the generic typed values. However, the feature is indeed very handy to reduce
the verboseness of the program, and can allow more efficient execution of the program without the
overhead of constructing extra lambdas.

With all the benefits considered, I started designing the feature near the end of last year in
[this GitHub issue](https://github.com/SamChou19815/samlang/issues/742). After about 10 months of
design, experimentation, and implementation, this is finally ready.

## Design under tight theoretical constraints

At the moment when you try to combine generics with subtyping, it is very easy to create a type
system in which type checking is undecidable. You don't even need a lot of fancy language features
to trigger the undecidability problem: simply support generics, and then allow subtyping almost
everywhere that makes sense. This fact is proved in Benjamin C. Pierce's book _Types and Programming
Languages_, where he shows that System F-sub (System `F<:`) is undecidable, while F-kernel, a
simpler version of the same system but disallowed subtyping in certain places, remains decidable. In
the real world, it was finally proved recently that
[Java's generics are Turing complete](https://arxiv.org/pdf/1605.05274.pdf).

Even we have a magical oracle that can solve all the subtyping problem for us, there is still
another decision we need to make: should we decide subtyping nominally or structurally? Structral
subtyping is indeed more fancy than the nominal ones, since you don't have to explicitly tell the
compiler which interfaces a class implement. However, we are hit by the same undecidability problem.
Even OCaml, the language that places soundness over everything else, still
[has undecidability problem](https://caml.inria.fr/pub/old_caml_site/caml-list/1507.html) because it
allows structural subtyping of modules.

The natural question to ask myself is: in which direction should samlang go? Should it make the
language more expressive at the cost of decidability, or it should aim to be remain completely sound
at the cost of dropping support for most subtyping?

## Unpopular opinion: subtyping is not necessary

We live in a world where some of the most popular statically typed languages support subtyping,
including but not limited to: Java, C++, Python (with types), TypeScript, etc. When you consider
dynamically typed languages, the list goes even longer. If all language feature issues are decided
by a vote by the most commonly used languages, then supporting subtyping is a no-brainer.

Despite the popular vote by top languages, I decide to make the opposite decision: samlang will not
support general-purpose subtyping, but only allow it in very limited places to make bounded
qualification work. Sure, supporting subtyping does imply a lot more work, including but not limited
to variance, subtyping lattice, runtime casting, and much more complicated implicit instantiation.
Nevertheless, my decision is not a result of laziness. Instead, it's a deliberate choice to drop
unnecessary complication and embrace simplicity.

My choice is not alone in the universe. OCaml, which heavily influenced the design of samlang, also
doesn't support arbitrary subtyping. (It does have some support for subtyping, but most of the data
type you will create everyday don't subtype.) Even outside of academia, there are two languages you
can hardly ignore that don't support most of the subtyping: Go and Rust. Rust even writes in its
[docs](https://doc.rust-lang.org/nomicon/subtyping.html) why it doesn't support subtyping where you
hope it will support.

You might wonder: how can you even get away with not recognizing that a value with type `Cat` can be
passed into a function that expects `Animal`? My response is: it's not very commonly required, and
it is required, variant types or bounded qualification can be enough.

Consider a typical codebase. No matter whether it is a product codebase or infrastruture codebase,
it's relatively uncommon that you will need a deep subtyping hierarchy. Most of the time, the
subtyping lattice is very shallow, and the code that requires subtyping more often uses it as a way
to share code. I strongly believe that this is an antipattern. In the industry, React famously
[favors composition over inheritance](https://reactjs.org/docs/composition-vs-inheritance.html). The
typical OOP paradigm can easily lead to complicated override that's hard to figure out what is going
on. Programmer might override a method to just to dig a hole in the framework to complete some task
at the hands, without considering all the long term consequences. A much better solution is to use
the [delegate pattern](https://en.wikipedia.org/wiki/Delegation_pattern), which is natively
supported by Kotlin.

How about some legitimate use cases? For example, we want to create an ordered set backed by
red-black tree, and we want all the elements in the set have a `compareTo` method. In earlier
versions of Java, this is indeed achieved by subtyping: the ordered set will ensure that all
elements in the set implement the `Comparable` interface. Then within the ordered set class, it can
call `compareTo` method as a guide to alter the tree structure. In this case, it's important to note
that even subtyping is not enough. You will need bounded qualification! Earlier versions of Java do
suffer from this problem: you can only express the idea of `List<Object>` or `TreeSet<Comparable>`,
but not `List<String>` or `TreeSet<Integer>` before Java 5.

Fortunately, under the latest version of samlang, such ideas finally have first class support:

```samlang
interface Comparable<T> {
  method compareTo(other: T): int
}
class BoxedInt(val i: int): Comparable<BoxedInt> {
  method compareTo(other: BoxedInt): int = this.i - other.i
}
class Set<T: Comparable<T>> {
  private method lessThan(v1: T, v2: T): int = v1.compareTo(v2)
}
```

Hence, after deliberation, I decided that adding support for general subtyping is not necessarily a
good idea, and it doesn't really fit in samlang's design philosophy to be an expressive but simple
language. For the purpose of implementing bounded qualification, we will only permit subtyping in
one kind of use sites: type arguments, in order to check the type arguments against the declared
bounds.

## Implementation

Despite the deliberate absence of general subtyping in the language, implementing bounded
qualification is still no easy task.

### Conformance

After adding all the necessary syntax support, we start with checking the conformance of interfaces.
An earlier unanswered question arises again: nominal or structural subtyping. I end up choosing
nominal subtyping not only for its simplicity, but also because it makes the contract and purpose of
classes and interfaces explicit. Even with nominal subtyping, you can still easily run into infinite
loops if you are not careful. Consider the following illegal program:

```samlang
interface A : B
interface B : A
class C : B
```

Checking whether class `C` implements all the required functions and methods of `B` requires us to
generate a set of all functions and methods of `A` and `B`, which can lead to infinite cycles if we
forget that the unvalidated type graph can be cyclic.

In addition, interface types can be polymorphic as well, which requires us to properly instantiate
functions and methods when checking conformance. Consider the following interface:

```samlang
interface A<T: A<T>> {
  method m(): T
  function <V> f(): V
}
```

For `class C : A<C>`, we will need to implement `method m(): C` and `function <V> f(): V`.

### Type Inference

How should the subtyping elements in bounded qualification interact with the rest of the type
inference algorithm?

From earlier analysis and my personal belief, arbitrary interactions between the two are off the
table. We already know such arbitrary interaction is the cause of the undecidability of System
`F<:`. I chose to disallow subtyping in all other places, except in type arguments, where we
absolutely need the subtyping check for the feature of bounded qualification to work.

Still, there are open questions remain. How does the presence of bounded type parameters influence
the inference of implicit type arguments? Is type inference still decidable?

To tackle the second question, I decided to drop the Hindley-Milner-based type inference algorithm
that can figure out types globally. Initially, I chose the HM type inference algorithm not because
it can solve some pressing problems, but just because I can and it looks cool. However, over the
years it adds to the maintenance burden and makes the type checker the least touchable part.
Therefore, using the opportunity of implementing bounded qualification, I decided to replace it with
Pierce's idea of [local type inference](https://www.cis.upenn.edu/~bcpierce/papers/lti-toplas.pdf)
(LTI), with some of my extensions for better completeness. The popularity of TypeScript shows that
such limited type inference is good enough for most people, and it is much more controllable and
predictable: adding more type annotations always makes the type checker behave more closely to what
you want.

Even with LTI, we still want to infer type arguments in most cases, so the first question remains
open. I opt to not use the bounds during inference, since it can interfere with the type of the
arguments without proper subtyping support. Even in the absence of any other constraints, I still
choose not to use the bounds, since it's better to rely on explicit user-provided exact return
hints, rather than an inequality constraint. Therefore, the role of bounds in type parameter
signatures is limited solely to conformance checking.

### Code Generation

Most of the complexity lies in the type-checking phase. Only after passing all type-checking rules,
we can enter the code generation phase, where the compiler lives. In the
[last samlang post](/2021/10/29/samlang-wasm-backend), I talked about how I switched to using
generic specialization to generate code that interacts with generics and GC. That infrastructure
provides a solid and easy implementation for the compilation of bounded generics.

Consider the following generic method call:

```samlang
interface Fooable {
  method foo(): unit
}
class C {
  method foo(): unit = {}
}
class A {
  function <T: Fooable> test(t: T): unit = t.foo()
  function main(): unit = A.test(C.init())
}
```

How would you compile such code? Well, all method calls are first lowered into simple function
calls, so we will first have the following intermediate code (written in the syntax of TypeScript):

```typescript
type C = [];
function C$init(): C {
  return [];
}
function C$foo(_this: C): number {
  return 0;
}
function A$test<T>(t: T): number {
  return T$foo(t);
}
function A$main(): number {
  return A$test<C>(C$init());
}
```

Then during generic specialization. The generic call `T$foo` under `T=C` is easily specialized, so
after specialization and dead code elimination, we will have:

```typescript
type C = [];
function C$init(): C {
  return [];
}
function C$foo(_this: C): number {
  return 0;
}
function A$test_C(t: C): number {
  return C$foo(t);
}
function A$main(): number {
  return A$test<C>(C$init());
}
```

It's that simple! However, during the implementation, I did find a use-after-free GC bug since the
last release. Indeed, GC is hard :).

## Next Steps

This is likely the last language feature I plan to implement for a while. After some potential bug
fixes and cleanup, I am planning to rewrite samlang in Rust for fun!
