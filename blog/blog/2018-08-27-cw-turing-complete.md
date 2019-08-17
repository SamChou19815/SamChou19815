---
title: Critter World is Turing Complete - A Not-So-Rigorous Proof
tags: [math]
---

Critter World is Cornell's CS 2112's Final Project. It is a simulated hexagon world where
critters, controled by programs, can move, eat, mate, bud, attack, etc. The programming language
is very primitive and not Turing complete, but it's still useful and expressive enough to cleverly
control one step of a critter move. You can see the spec publicly
[here](http://www.cs.cornell.edu/courses/cs2112/2018fa/project/project.pdf).

<!--truncate-->

Although the critter language is not Turing complete, a running critter world simulation, under some
circumstances, can be used to simulate a Turing machine, thus making it Turing complete.

### Assumptions and Modifications

For the convenience of proof, we will make some assumptions:

- Critters have infinite amount of energy.
- Some tasks can be done immediately. e.g. two consecutive turn around, turn back, etc.
- Critters are allowed to perform multiple actions at once.

Note that these assumptions are not really needed. If these assumptions are not true, we can simply
use some of critter's memory to remember the things they need to do.

We need these modifications to make it Turing Complete:

- The world is a rectangular grid with width 2 that extends infinitely in one direction.
- The input of a Turing machine is given as a line of food in the first line that represents the
  tape, and a second line of empty tile with one critter on it.
- Some useless actions of critters (in this settings e.g. bud) are interpreted as a yes halt, some
  as a no halt, some simply as a halt. The rest actions are useful.

### Turing Machine Simulation

A Turing Machine is specified by an alphabet, a state set, and transition rules.

In a Turing machine, the alphabet is a set of all characters that can initially appear on the tape.
In critter world, the alphabet is a set of all possible food valeus that can initially appear on
the second line.

The state set can be represented by the finite amount of critter memory that you specify during
critter creation. Note that we can also just use them as variables conveniently while still
representing a Turing machine. It is explained in section 2.1 in Cornell's
[Turing Machine Handout](http://www.cs.cornell.edu/courses/cs4820/2018sp/handouts/turingm.pdf).

The transition rules are simply a critter program which is just a big chunk of if-else block that
encode transition rules.

Hence, critter world is Turing complete.

### A Working Example

I will demonstrate a very simple example, adding 1 to a binary number, ignoring overflow problems.
We assume the number is given in little-endian format. (i.e. 2 is given as 01 instead of 10.)

Here is what you will do in Turing Machine pseudocode, which is very straight-forward:

```javascript
let carry = 1; // Initially carry is 1 to represent +1

function transition() {
  if (tapeContent == 0 && carry == 0) {
    setTapeContent(0);
    carry = 0;
  } else if (tapeContent == 0 && carry == 1) {
    setTapeContent(1);
    carry = 0;
  } else if (tapeContent == 1 && carry == 0) {
    setTapeContent(1);
    carry = 0;
  } else {
    setTapeContent(0);
    carry = 1;
  }
  moveForward();
}
```

However, critter world does not understand the JavaScript above. Since it's very tedious to dealing
with those low-level critter world details (e.g. cannot do multiple actions in one step), I created
a tool to try to compile a higher level non-Turing-complete language to a low-level language like
critter language. The language disallows recursion and aggressively inlines variables and functions
to put everything into a set of global variables and a single giant expression. The tool is called
[primitivize](https://github.com/SamChou19815/primitivize) and is open sourced on GitHub.

Here is the code that will be compiled to a critter language:

```kotlin
// Variables: TM Constructs
var needHalt = 0
// 0 ==> not in stage, 1 ==> right, 2 ==> serve/eat, 3 ==> left, 4 ==> move
var setTapeContentAndMoveStage = 0
var valueToSet = 0
var moveDir = 0

// Variables: Problem Specific Constructs

var dir = 1
var carry = 1

// Functions: TM Constructs
fun doesNeedHalt(): bool = needHalt != 0
fun getTapeContent(): int = nearby(dir)

fun setTapeContentAndMove(
  v: int, d: int, haltNow: bool,
  beforeSetTapeContent: void
): void =
  if haltNow then (
    needHalt = 1
  ) else if (setTapeContentAndMoveStage == 0) then (
    setTapeContentAndMoveStage = 1;
    valueToSet = v; moveDir = d
  ) else if (setTapeContentAndMoveStage == 1) then (
    setTapeContentAndMoveStage = 2;
    dir = 0;
    right()
  ) else if (setTapeContentAndMoveStage == 2 && valueToSet == 0 && ahead(1) == 0) then (
    setTapeContentAndMoveStage = 3;
    beforeSetTapeContent;
    waitFor()
  ) else if (setTapeContentAndMoveStage == 2 && valueToSet == 1 && ahead(1) == 0) then (
    setTapeContentAndMoveStage = 3;
    beforeSetTapeContent;
    serve(1)
  ) else if (setTapeContentAndMoveStage == 2 && valueToSet == 0 && ahead(1) == 0-2) then (
    setTapeContentAndMoveStage = 3;
    beforeSetTapeContent;
    eat()
  ) else if (setTapeContentAndMoveStage == 2 && valueToSet == 1 && ahead(1) == 0-2) then (
    setTapeContentAndMoveStage = 3;
    beforeSetTapeContent;
    waitFor()
  ) else if (setTapeContentAndMoveStage == 3) then (
    setTapeContentAndMoveStage = 4;
    dir = 1;
    left()
  ) else if (setTapeContentAndMoveStage == 4 && moveDir == 0 - 1) then (
    setTapeContentAndMoveStage = 0;
    backward()
  ) else if (setTapeContentAndMoveStage == 4 && moveDir == 0) then (
    setTapeContentAndMoveStage = 0;
    waitFor()
  ) else if (setTapeContentAndMoveStage == 4 && moveDir == 1) then (
    setTapeContentAndMoveStage = 0;
    forward()
  ) else waitFor() // bad case


// Functions: Problem Specific Constructs

fun main(): void =
  if doesNeedHalt() then (
    waitFor()
  ) else if (ahead(1) == 0-1) then (
    needHalt = 1
  ) else if getTapeContent() == 0 && carry == 0 then (
    setTapeContentAndMove(0, 1, false, carry = 0)
  ) else if getTapeContent() == 0 && carry == 1 then (
    setTapeContentAndMove(1, 1, false, carry = 0)
  ) else if getTapeContent() == 0-2 && carry == 0 then (
    setTapeContentAndMove(1, 1, false, carry = 0)
  ) else if getTapeContent() == 0-2 && carry == 1 then (
    setTapeContentAndMove(0, 1, false, carry = 1)
  ) else waitFor()
```

The code already looks nasty somewhere, but the compiled code's readability is comparable to
assembly. Here is a random segment of it:

```text
...
{nearby[mem[13]] = 0 and mem[14] = 1} and {{mem[10] = 2 and mem[11] = 0} and ahead[1] = 0 - 2} -->
  mem[10] := 3
  mem[14] := 0
  eat
  ;
{nearby[mem[13]] = 0 and mem[14] = 1} and {{mem[10] = 2 and mem[11] = 1} and ahead[1] = 0 - 2} -->
  mem[10] := 3
  mem[14] := 0
  wait
  ;
{nearby[mem[13]] = 0 and mem[14] = 1} and mem[10] = 3 -->
  mem[10] := 4
  mem[13] := 1
  left
  ;
...
```

The simulation appears to be pretty good!

(In the demo, the initial tape content is 1111101 and the expected output should be 0000011.)

![Simulation](/img/2018-08-27-cw-turing-complete/simulation.gif)
