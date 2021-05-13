# Supporting the LLVM Backend for samlang

LLVM is a collection of compiler toolchain that allows you to target any instruction set from any
source-level programming language. Once the source code has been lowered to LLVM IR, the LLVM
toolchain can easily handle the tasks of optimization and emitting assembly code for all the
supported architecture.

I was always planning to migrate the samlang backend to LLVM since Apple announced that future Macs
would be running on ARM chips and I want the ability to run compiled samlang code natively. Such a
plan finally came true this winter when I was stuck in Ithaca having nothing else to do.

<!--truncate-->

## Runtime Library for LLVM

Compiling a language from source-level down to assembly is not all about code transformation. The
source language might have some built-in function calls that must be implemented somewhere. For a
garbage collected language like samlang, you also need to have a garbage collector library

In the past, samlang programs were compiled down to X86 assembly and then linked against an archive
of library object files. Now the library must be compiled to LLVM bitcode file (`.bc`) to work with
LLVM. Fortunately, we can use `clang` to compile the runtime library written in C to LLVM bitcode:

```terminal
clang -emit-llvm library.c -c -o library.bc
```

When the source code has been compiled to LLVM IR, it can be outputted to a file in LLVM text code
format (`.ll`). Then it can be linked with the library with `llvm-link`, compiled to target
platform's object file by `llc`, and finally processed by `gcc`:

```terminal
llvm-link -o linked.bc compiled.bc library.bc
llc -filetype=obj linked.bc
gcc -o program linked.o
```

The entire flow can be described by the following ASCII art diagram:

```text
-----------                 ------------------
| samlang | --(compiler)--> | LLVM Text Code | --
-----------                 ------------------  |
                                                | (llvm-link)
-----------                 ----------------    |   ----------------
| library | ---(clang)----> | LLVM Bitcode | -----> | LLVM Bitcode |
-----------                 ----------------        ----------------
                                                          ||
                                                          || (llc)
                                                          \/
                                                    ---------------
                                                    | Object File |
                                                    ---------------
                                                          ||
                                                          || (gcc)
                                                          \/
                                                    ---------------
                                                    | Executable  |
                                                    ---------------
```

## Power to the Types

### The LLVM Type System

The biggest difference between LLVM and assembly is that LLVM IR is much better typed. In assembly,
all you need to care about is the width of an integer, if you only have integer types in your
language. LLVM types, however, are much more expressive. Even if you restrict yourself to a source
language with only 64-bit integers, you still need to have the following types:

- `i1`, `i64` for booleans and integers
- `i64*` for strings
- `{i64, i1, i64*}*` for structs
- `i64 (i64, i1)*` for function types, if the language support lambdas or OOP features
- `%Foo` for identifier types, if you want to support any recursive types.

It is impossible to figure out the type of an expression by just looking at the syntax. Therefore,
we must retain the type information in IR before emitting any LLVM code.

### The Need for Casts

It might be tempting to say that the task for creating LLVM types is much easier than lowering the
source-level language to IR. If the source language's type system is sufficiently simple, this can
be true. However, when you are lowering the types for a functional language with a powerful and
expressive type system, the claim is far from the truth.

Considering the variant type that you can usually find in a functional programming language.

```ocaml
type list = Nil of unit | Cons of (int, list);
```

At the runtime, a value of the list type would be represented by a struct with two slots, where the
first slot stores the ID of the tag and the second slot stores the attached data. For example, a
`Nil` value might be represented as `[0, 0]`, and a `Cons` value might be represented as
`[1, ptr_to_tuple]`. During pattern matching, we can generate some code that conditions on the first
slot of tag ID to figure out the actual type of the attached data.

Everything discussed above is very straightforward to do in an untyped language, but requires
significant effort to get it right for LLVM. The biggest problem is that LLVM's type system is too
limited to understand that the type of the second slot depends on the integer value of the first
slot. For LLVM, the type of each slot in a struct must be fixed.

To work around the problem, LLVM allows you to insert arbitrary zero-cost casts that can convert
between any two types. However, such casts must be explicit, so it's the compiler's job to determine
when we need the cast.

Generics further complicates the problem. LLVM simply cannot understand generic type parameters, so
we must perform type erasure to convert all the generic types to one fixed type (say `i64*`), and
then insert more casts in case there are any new type incompatibilities after the type erasure.

### Where to Cast

We already know that we need to cast during the creation and pattern matching of variants. However,
the existence of type erasure seems to require the cast to be everywhere. Fortunately, this is not
the case. Actually, we only need to insert casts during the boundary of function call.

To understand how type erasure on function call can create type incompatibility, we can look at the
following example:

```typescript
function <A, B, C>randomFunction(a: A, b: B): C { /* ... */ }

const c: Foo = randomFunction(3, true);
```

After we perform type erasure, the function signature will be:

```typescript
function randomFunction(a: unknown, b: unknown): unknown;
```

Then we can easily see that there are incompatibilities with function arguments and return value, so
we need to check for type equality in those places and insert casts whenever necessary.

You might worry about the type of the function argument being changed might cause some
incompatibilities within the function body. However, it's important to note that the generic types
are changed in a consistent way. In other words, some statements inside the function that are
originally expecting a value of the generic type `A` will also be changed to expect the placeholder
type `unknown` instead. You can see it more clearly from this example:

```typescript
// Original
function <T>identity(v: T): T { return v; }
function <A, B, C>randomFunction(a: A, b: B): C {
  const aa: A = identity(a);
  const bb: B = identity(b);
  // More code
}

// Type Erased
function identity(v: unknown): unknown { return v; }
function <A, B, C>randomFunction(a: unknown, b: unknown): C {
  const aa: unknown = identity(a);
  const bb: unknown = identity(b);
  // More code
}
```

You might also think that I miss the case of the object and struct being generic. Nevertheless, it
is just a special case of being the boundary of the function call. You can consider creating an
object as calling a constructor function and reading a field as calling a getter function.

## Static Single Assignment Form and the LLVM IR

### Phi Functions

A program in the static single assignment (SSA) form ensures that every variable is only assigned
once in one place. In case a variable must be given different values depending on the code branch,
we must use the special `phi` function to _explicitly_ state what's the value in each branch. For
example, the statement `const a = condition ? function1() : function2()` must be translated into
something like

```llvm
  br i1 %condition, label %true_branch label %false_branch
true_branch:
  %t1 = call i64 @function1() nounwind
  br label %end
false_branch:
  %t2 = call i64 @function2() nounwind
  br label %end
end:
  %a = phi i64 [ %t1, %true_branch ], [ %t2, %false_branch ]
```

Programs in SSA form are much easier to reason and analyze than programs with mutable variables,
since it is easy to track the source of every single variable and analyze potential side effects
easily.

You might think a functional programming language gets the SSA form for free, since every variable
is already immutable. However, the transformation is still not trivial, because we still need to
design a good IR so that it's easy to spot where we need the `phi` functions.

### Phi Functions for If-Else Statements

Based on the example above, it might be very tempting to do nothing special in the IR and simply
extract the last statement in both branch of if-else. In case they do assign to the same variable,
we know the variable must be handled by the `phi` function. Nonetheless, this approach is very hacky
and will not scale well when we have more complicated scanarios. For example, consider the partial
redundancy elimination optimization:

```typescript
let v;
if (c) {
  const a = b * c;
  // ...
  v = 1;
} else {
  // ...
  v = 0;
}
const d = b * c;

// b * c is computed twice if you go along the true branch.
// We can eliminate the partial redundancy by:
let d;
if (c) {
  const a = b * c;
  // ...
  v = 1;
  d = a;
} else {
  // ...
  const t = b * c;
  v = 0;
  d = t;
}
```

From this example, you can see that some optimizations might cause a single if-else block to contain
more than one variables that need to be handled by the `phi` function. Then at the point of LLVM
translation, we need to move further up the list of statements in each branch, which can get really
messy.

I end up creating an additional field for the if-else AST called `finalAssignments`. You can think
of it as a list of ternary operations like `let varName = c ? branch1Value : branch2Value`. Using
this setup, the above if-else would be represented by:

```typescript
const ifElse = {
  __type__: 'if-else',
  condition: c,
  trueBranchStatements: ['const a = b * c' /* ... */],
  falseBranchStatements: [, /* ... */ 'const t = b * c'],
  finalAssignments: [
    { name: 'v', branch1Value: 1, branch2Value: 0 },
    { name: 'd', branch1Value: 'a', branch2Value: 't' },
  ],
};
```

### Phi Functions for While Statements

You might wonder why we ever need to emit some white statements for a functional language that does
not have loops. The answer is that we need tail recursion optimization. Consider this recursive
implementation of factorial function with an accumulator:

```samlang
function factorial(n: int, acc: int): int =
  if (n == 0) then acc else factorial(n - 1, n * acc)
```

Such function will first be lowered to the following IR:

```typescript
function factorial(n: int, acc: int): int {
  const t0 = n == 0;
  let t1;
  if (t0) {
    t1 = acc;
  } else {
    const t2 = n - 1;
    const t3 = n * acc;
    t1 = factorial(t2, t3);
  }
  return t1;
}
```

Then the tail recursion optimizer will be able to detect that there is a tail recursive call, and
transform it to the following (assuming we don't worry about SSA form):

```typescript
function factorial(n: int, acc: int): int {
  while (true) {
    const t0 = n == 0;
    var t1;
    if (t0) {
      t1 = acc;
      break;
    } else {
      const t2 = n - 1;
      const t3 = n * acc;
      n = t2;
      acc = t3;
    }
  }
  return t1;
}
```

Such results will be perfectly fine if you don't want SSA form. However, it creates two big
problems:

1. Now the variable `n` and `acc` are assigned multiple times, in potentially multiple branches. Now
   the multi-assignment becomes much harder to analyze.
2. Notice the `break` statement in the true branch but not in the false branch. The false branch has
   some final assignments while the true branch does not. It does not play well with the final
   assignment setup in the previous section.

Unlike if-else, an elegant solution is not obvious, and it took me several iterations to get the
abstraction right. The first breakthrough is the creation of a new single if statement without the
else block.

It's easy to reason about how the break statement interacts with if-else statements. Suppose there
is no break statement in neither or both branch, then we fallback to the good old time when there
are equal number of final assignments. In the other case, then the break statement must be inside
the only if block and there will be no final assignment. To see it more clearly, we can transform
the above function to be:

```typescript
function factorial(n: int, acc: int): int {
  while (true) {
    const t0 = n == 0;
    var t1;
    if (t0) {
      t1 = acc;
      break;
    }
    const t2 = n - 1;
    const t3 = n * acc;
    n = t2;
    acc = t3;
  }
  return t1;
}
```

Then from this program, we can easily see that the function parameters are the ones with multiple
assignments: the initial value from the function argument, and the assignment at the end of the
loop. Therefore, we can create a list of loop variables of the form
`(name, initial value, value from loop)`. To achieve this, we just need to rewrite the function as:

```typescript
function factorial(_param_n: int, _param_acc: int): int {
  let n = _param_n; // initial value
  let acc = _param_acc; // initial value
  while (true) {
    const t0 = n == 0;
    var t1;
    if (t0) {
      t1 = acc;
      break;
    }
    const t2 = n - 1;
    const t3 = n * acc;
    n = t2; // loop value
    acc = t3; // loop value
  }
  return t1;
}
```

Right now the only issue is the value `t1`. It is assigned somewhere in the loop with a break
immediately following it. It's very hacky to treat it as a normal assignment. However, we can choose
to introduce the concept of loop break collector. It's a variable name declared before the loop and
assigned right before the break. Then we can change a break without value to be a break with value.
In JS, it can be written as:

```typescript
function factorial(_param_n: int, _param_acc: int): int {
  let n = _param_n; // initial value
  let acc = _param_acc; // initial value
  let t1;
  while (true) {
    const t0 = n == 0;
    if (t0) {
      // break(acc); statement start
      t1 = acc;
      break;
      // break(acc); statement end
    }
    const t2 = n - 1;
    const t3 = n * acc;
    n = t2; // loop value
    acc = t3; // loop value
  }
  return t1;
}
```

Then during the compilation to LLVM, we can simply collect all the break with values statement to
make `t1` a variable with the `phi` function collecting all the values from the break.

To wrap up, the statements inside the function can be represented as:

```text
WHILE {
  loopVariables: [(n, _param_n, t2), (acc, _param_acc, t3)],
  breakCollector: t0,
  statements: [
    t0 = n == 0,
    if (t0) {
      BREAK(acc);
    }
    t2 = n - 1;
    t3 = n * acc;
  ]
}
```

## Loop Optimizations

Migration to a high-level IR closer to LLVM IR with SSA form makes optimization much easier. In
particular, this release adds a few more loop related optimizations to statically analyze and
optimize complex nested loops.

Consider the following samlang program:

```samlang
class Main {
  function testI(acc: int, i: int): int =
    if (
      i >= (30 + 100 / 100) - (2000 * 2000) / (((10 * 10) * 10) * 4000)
    ) then
      acc
    else
      Main.testI(Main.testJ(acc, 0), i + 1)

  function testJ(acc: int, j: int): int =
    if (j >= 10 + (((100 * 99) * 98) * 97) * 0) then
      acc
    else
      Main.testJ(
        (((acc + 34 * 34) + 4) + 1) + 1231 / 28,
        j + 1
      )

  function main(): unit = println(intToString(Main.testI(0, 0)))

}
```

It will first be lowered to IR and constant folded to be like:

```typescript
function testI(acc: int, i: int): int {
  let t0;
  if (i >= 30) {
    t0 = acc;
  } else {
    const t1 = testJ(acc, 0);
    const t2 = i + 1;
    t0 = testI(t1, t2);
  }
  return t0;
}

function testI(acc: int, j: int): int {
  let t0;
  if (j >= 10) {
    t0 = acc;
  } else {
    const t1 = acc + 1204;
    const t2 = j + 1;
    t0 = testJ(t1, t2);
  }
  return t0;
}

function main(): unit {
  const t0 = testI(0, 0);
  const t1 = intToString(t0);
  println(t1);
}
```

Then it will undergo tail recursion rewrite to be:

```typescript
function testI(_param_acc: int, _param_i: int): int {
  let acc = _param_acc; // initial value
  let i = _param_i; // initial value
  let t0; // break collector
  while (true) {
    if (i >= 30) {
      t0 = acc;
      break;
    } else {
      const t1 = testJ(acc, 0);
      const t2 = i + 1;
      acc = t1; // loop value
      i = t2; // loop value
    }
  }
  return t0;
}

function testJ(_param_acc: int, _param_j: int): int {
  let acc = _param_acc; // initial value
  let j = _param_j; // initial value
  let t0; // break collector
  while (true) {
    if (j >= 10) {
      t0 = acc;
    } else {
      const t1 = acc + 1204;
      const t2 = j + 1;
      acc = t1; // loop value
      j = t2; // loop value
    }
  }
  return t0;
}

function main(): unit {
  const t0 = testI(0, 0);
  const t1 = intToString(t0);
  println(t1);
}
```

Then static analysis over the loop will kick in and figure out that inside the loop in function
`testJ`, we can statically know that `t0 = _param_acc + (10 - _param_j) * 1204`, so the entire
function will be constant propagated and inlined into `testI` to make `testI` become:

```typescript
function testI(_param_acc: int, _param_i: int): int {
  let acc = _param_acc; // initial value
  let i = _param_i; // initial value
  let t0; // break collector
  while (true) {
    if (i >= 30) {
      t0 = acc;
      break;
    } else {
      const t1 = acc + 12040;
      const t2 = i + 1;
      acc = t1; // loop value
      i = t2; // loop value
    }
  }
  return t0;
}
```

After the inlining, the static analysis will figure out that the
`t0 = _param_acc + 12040 * (30 - _param_i)`, which will be constant propagated and inlined into the
main function to make `main` become:

```typescript
function main(): unit {
  const t0 = 361200;
  const t1 = intToString(t0);
  println(t1);
}
```

## Final Words

The above example shows only a fraction of the full power of the compile-time loop analysis and
optimization. Induction variable elimination and strength reduction optimizations are also
implemented.

You can verify by playing on the [demo site](https://samlang.io/demo), or on the command line by
installing the CLI with `yarn add @dev-sam/samlang-cli`.

As a tradition of ending samlang-related blog post, here is a screenshot showing the optimized code
in the example:

![demo site showing highly optimized code](/img/2021-01-24-samlang-llvm-backend/highly-optimized-code.png)
