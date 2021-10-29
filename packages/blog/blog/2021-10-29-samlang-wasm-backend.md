# WebAssembly Backend for samlang

export const additionalProperties = { ogImage: "/img/2021-10-29-samlang-wasm-backend/wasm.png" };

Since the release of Apple M1 MacBook, a ticking bomb starts: eventually there will be a day that I
cannot run the compiled samlang code anymore, if samlang compiler does not change. samlang must
change to adapt to the new world.

<!--truncate-->

## Background

At that time, samlang emits x86 assembly code. Nine months ago, I made the first move to emit
platform-independent LLVM IR instead. However, it introduces a heavy LLVM dev environment as a cost.
In addition, the GC (garbage collector) still references x86-specific registers to traverse the
stack, which makes the platform-independent drive incomplete. The bomb is still ticking.

I researched different ways to preserve the existing setup and magically achieve platform
independence, but eventually I concluded that the effort is futile. There is one and only way
forward: write my own GC that is truly platform-independent. This move would also unlock the ability
to emit WebAssembly code, which does not have an addressable, explicit stack that's required by the
existing Boehm-Demers-Weiser GC.

## Garbage Collectors and Their Problems

There are two major types of GC out there: mark-and-sweep-based GC and reference-counting-based GC.
Mark-and-sweep-based GC is usually much simpler and does not require type information of the program
at all. However, it requires the ability to access the entire stack from the beginning to the end,
which is impossible under a restrictive environment like WebAssembly where the stack is not
addressable. This leaves us with only reference-counting based GC available for use.

Reference-counting-based GC does not require access to the stack. In fact, we can even keep the old
good `malloc` and `free` intact. This type of GC only requires the language compiler to keep track
of how many times an object allocated in the heap is referenced. It does so by inserting code to
increase and decrease the reference count at appropriate places. For example, the following
TypeScript code is annotated by comments on where to insert code to increase and decrease reference
count:

```typescript
function foo() {
  // Assuming an initial RC of 1 for all objects.
  const a = [1, 3, 4];
  const b = [a, 2]; // incRef(a), since one more reference of a.
  {
    const c = [3, 3];
    // decRef(c), since c falls out of scope.
  }
  // decRef(b), since b falls out of scope.
  return a;
}
```

Increasing reference counting is easy. If your object `o = { a: 1, b: 2 }` is stored in memory as
`[1, 2]`, then you only need to extend to `[referenceCount, 1, 2]` to support reference counting.
Incrementing or decrementing reference counting is as easy as `o[0] += 1` or `o[0] -= 1`.

However, decrementing reference counting has an extra requirement: once the reference count of `o`
drops to zero, the object `o` should be deallocated. In addition, all of the fields referenced in
`o`'s reference counts should also be decremented. The following example illustrates the point:

```typescript
type A = [referenceCount: number, n: number, b: boolean, c: C, d: D];

function decRefA(a: A) {
  a[0] -= 1;
  if (a[0] > 0) return;
  // No decRef(a[1] and decRef(a[2]), since they are not reference types.
  decRefC(a[3]);
  decRefD(a[4]);
  free(a); // Time to deallocate!
}
```

As you can see in the above example, doing reference counting requires precise type information of
all the objects, which seem to be trivial for a statically and strongly typed language like samlang.
However, everything becomes complicated once you have generics and first-class functions...

## Generics Specialization

Generics presents an unavoidable challenge to reference-counting-based GC: some types are actually
not known ahead of time. Consider the following generic type:

```typescript
type Box<T> = [referenceCount: number, value: T];
```

How can we write down the `defRef` function for this type? The following obvious solution does not
work:

```typescript
function decRefBox<T>(b: Box<T>) {
  b[0] -= 1;
  if (b[0] > 0) return;
  decRefT(b[1]); // It does not work. T is not a fixed value at runtime.
  free(a);
}
```

There exists a workaround called _generics specialization_, and it's actually used by many major
languages like C++ and Rust. Generics specialization start from the entry point (which is usually
the `main` function that is not generic), and specializes the generic type into a concrete one based
on its actual usage. For example,

```typescript
type Box<T> = [referenceCount: number, value: T];

function boxCreator<T>(value: T): Box<T> {
  return [1, value];
}

function main() {
  const b1: Box<number> = [1, 1];
  const b2: Box<string> = boxCreator('sdfd');
  const b3: Box<Box<string>> = boxCreator(b2);
}
```

will be specialized into

```typescript
type Box_number = [referenceCount: number, value: number];
type Box_string = [referenceCount: number, value: string];
type Box_Box_string = [referenceCount: number, value: Box_string];

function boxCreator_string(value: string): Box_string {
  return [1, value];
}

function boxCreator_Box_string(value: Box_string): Box_Box_string {
  return [1, value];
}

function main() {
  const b1: Box_number = [1, 1];
  const b2: Box_string = boxCreator_string('sdfd');
  const b3: Box_Box_string = boxCreator_Box_string(b2);
}
```

The exact detail of the implementation is tricky, since recursive types and recursive function calls
need to be dealt with carefully to avoid infinite recursion. Generics specialization does push us
forward a lot to implement reference counting GC. Nevertheless, first-class functions do not fit
into this framework...

## Closure Representation

A first-class function is usually represented as a closure, which contains both the pointer to the
function and the captured variables. For example, the following JS pattern

```typescript
{
  const a = [1, 2, 3];
  const ONE = 1;
  const TWO = 2;
  forEach(a, (v) => console.log(ONE + TWO + v));
}

function forEach(l: number[], f: (n: number) => void) {
  for (const v of l) f(n);
}
```

will be lowered into the following closure form:

```typescript
type Closure<C, F> = { context: C; f: F };

{
  const a = [1, 2, 3];
  const ONE = 1;
  const TWO = 2;
  // Captured variables ONE and TWO are explicitly passed in as context
  const closure = { context: { ONE, TWO }, f: anonymousFun12345 };
  forEach(a, closure);
}

function forEach<C>(l: number[], closure: Closure<C, () => void>) {
  // Pass context as first argument of the function!
  for (const v of l) closure.f(f.context, n);
}

// lambdas are synthesized into a new function!
function anonymousFun12345(
  // Context parameter
  context: { ONE: number; TWO: number },
  // original parameters follow
  v: number
): void {
  const ONE = context.ONE;
  const TWO = context.TWO;
  console.log(ONE + TWO + v);
}
```

You can see that we represent closure as a generic type in the above example, with both the function
type and the context type fully parameterized. However, this setup simply does not work with
recursive data types. Consider the following example of a functional linked list:

```typescript
type List<T> = { type: 'nil' } | { type: 'cons'; value: T; next: List<T> };

const listOfFunctions: List<() => void> = {
  type: 'cons',
  value: () => /* context of type A */ code1,
  next: {
    type: 'cons',
    value: () => /* context of type B */ code2,
    next: {
      type: 'cons',
      value: () => /* context of type C */ code3,
      next: { type: 'nil' },
    },
  },
};
```

In this example, each lambda's type can be easily specialized:
`Closure_A_UnitReturnFunctionType`,`Closure_B_UnitReturnFunctionType`,`Closure_C_UnitReturnFunctionType`,
but the list's type now has a big problem: the elements' types are no longer uniform!

The problem is caused by insisting on specializing everything, including the context parameter that
can't be specialized. We need to remember why we want to specialize type in the first place: to know
what destructors to call. Now in the case of closure, where we know there is only one field (context
field) that might need a destructor, we can simply let the closure record the destructor as well.
Using this idea, the `forEach` example can be rewritten as:

```typescript
type Closure_UnitReturn = [
  referenceCount: number,
  contextDestructor: (context: unknown) => void,
  context: unknown,
  functionToCall: () => void
];

type ContextObj1 = [referenceCount: number, ONE: number, TWO: number];

{
  // ...
  const contextObj1 = [1, ONE, TWO];
  const closure = [1, decRefContextObj1, contextObj1, anonymousFun12345];
  incRef(contextObj1);
  // ...
}

function decRefContextObj1(o: ContextObj1) {
  o[0] -= 1;
  if (o[0] > 0) return;
  o[1](o[2]); // o.contextDestructor(o.context);
  free(o);
}

// ...
```

Now both the generics and closure problem is solved, we can finally use some simple data flow
analysis to find dead variables at the end of function call and insert `decRef` and add `incRef`
whenever a reference-typed value is assigned to an object field.

## WebAssembly Code Generation

Now that I have my own garbage collector, the generated code can finally be platform-independent.
The only thing left to do is to switch from the LLVM infrastructure to the corresponding WebAssembly
one.

I first replaced the C standard library implementation of `malloc` and `free` with
[a more lightweight one](https://github.com/wingo/walloc) that's designed for WebAssembly only. That
library is then compiled to WebAssembly, disassembled into text format, and concatenated with the
rest of the WebAssembly code generated from samlang. Finally, all the WebAssembly text code is
parsed, optimized and emitted as binary through the
[`binaryen.js`](https://github.com/AssemblyScript/binaryen.js) library also used by AssemblyScript,
another language that compiles down to WebAssembly.

## How Everything Fits Together

The codebase, especially the AST, is significantly re-architected to support the new WebAssembly
backend. You can see the compilation pipeline in ASCII art below:

```text
Source-level program
  |
  | Lowering
  |
  \/
High IR
(High-level typed,
with generics and closure)
  |
  | Generics specialization
  |
  \/
High IR
(No more parameterized types)
  |
  | Lowering
  |
  |
  |---------<<-----<<-----------
  \/                           |
Mid IR                         |
(With casts and                |
ref-counting      Optimization |
instructions)                  |
  |                            |
  |--------->>----->>-----------
  |
  | Lowering
  |
  \/
WebAssembly      Standard Library
Text             in WebAssembly
(User code)             |
  |                     |
  |                     |
  -----------------------
  |
  \/
Combined WebAssembly Code
   in Text Format
      |
      |
      |
      \/
Compiled WebAssembly Code
```

## Final Remarks

After six months of hard work and research, this large-scale project is done and released. I am
finally able to remove the heavy LLVM environment from CI and local development, and make everything
runnable with standard Node.JS only. It's a big endeavor worth taking!

![Emitted WebAssembly Binary](/img/2021-10-29-samlang-wasm-backend/wasm.png)
