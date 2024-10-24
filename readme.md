# fyfth

A stack-based Forth-like programming language for use with the [Bevy game engine](https://github.com/bevyengine/bevy).

## Syntax
fyfth is stack-based and uses [reverse Polish notation](https://en.wikipedia.org/wiki/Reverse_Polish_notation) (RPN). In RPN, a term like `1 + 2` would instead be written as `1 2 +`. The advantage of this style of notation is that it is much easier for a computer to reason about. For example, there is no operator precedence: `1 + 2 * 3` is not associative, i.e. `(1 + 2) * 3 != 1 + (2 * 3)`, and so operator precedence needs to decide which of the two orders of operation is correct. In RPN, however, we always evaluate from left to right. So `1 2 + 3 *` would be `(1 + 2) * 3 = 9` and `1 2 3 * +` would be `1 + (2 * 3) = 7`. If we want the addition to be evaluated first, we write `+` left of `*` and if we want the multiplication to be evaluated first, we do the opposite.

> [!NOTE]
> You can try out the above calculations for yourself in fyfth. Simply clone the repository and run `cargo run --example simple_example --features=example_features` (if you're on Linux using wayland, you may also want to use `--features=example_features,bevy_wayland` to enable Bevy's `wayland` feature).
> A scene with five cubes and an egui terminal should pop up. In the terminal's text field enter `1 2 add 3 mul print` and `1 2 3 mul add print` respectively.

See below for a full list of fyfth commands.


## fyfth Commands
### Core Built-Ins
 * `iter` turns the stack into an iterator and pushes it onto the stack; complementary to `push`
 * `push` takes an iterator and pushes its contents onto the stack; complementary to `iter`
 * `macro` starts recording a macro
 * `;` ends recording a macro
 * `queue` takes an iterator and inserts it at the front of the queue
 * `dup` duplicates the top element of the stack and pushes the duplicate on the stack as well
 * `swap` swaps the places of the last two elements of the stack
    - `a b swap` -> `b a`
 * `swap_n` consumes a number `n` from the top of the stack and then swaps now last element of the stack with the element `n` spots from the end
    - `a b c 2 swap_n` -> `c b a`
 * `rotr` consumes a number `n` from the top of the stack and then rotates the last `n` elements of the stack one (1) position to the right, looping around the last element's position
    - `a b c d 3 rotr` -> `a d b c`
 * `rotl` consumes a number `n` from the top of the stack and then rotates the last `n` elements of the stack one (1) position to the left, looping around `n`th last element of the stack to the last position of the stack
    - `a b c d 3 rotl` -> `a c d b`

### Core Language Extension
 * `get` gets a named parameter from a value
    - `vec2(3, 4) x get` -> `3`
 * `set` sets a named parameter of a struct to the given value
    - `vec2(3, 4) x 7 set` -> `vec2(7, 4)`
 * `add` adds two values
    - `1 2 add` -> `3`
 * `sub` subtracts a value from another
    - `1 2 sub` -> `-1`
 * `mul` multiplies two values
    - `1 2 add` -> `2`
 * `div` divides two values
    - `1 2 div` -> `0.5`
 * `print` prints the last value on the stack
    - `3 print` -> prints out `3`
 * `store` consumes a literal (string) from the top of the stack and then stores the next value under that name in that variable map
    - `3.141 pi store` -> now there is a variable `pi` with the value `3.141`
 * `load` consumes a literal (string) from the top of the stack and pushes the variable by that name onto the stack
    - `pi load` -> `3.141`
 * `print_vars` prints out all the current variables. This includes all saved macros.
 * `geq` consumes the top two elements off the top of the stack and returns true if the left one is greater than or equal to the right
    - `4 3 geq` -> `true`
 * `leq` consumes the top two elements off the top of the stack and returns true if the left one is less than or equal to the right
    - `4 3 leq` -> `false`
 * `eq` consumes the top two elements off the top of the stack and returns true if they are equal
    - `uwu owo eq` -> `false`
 * `eqq` non-broadcasting version of `eq`, see the Broadcasting section
    - `[1 3] [1 4] eqq` -> `false` where as `[1 3] [1 4] eq` -> `[true false]`
 * `not` inverts a boolean value
 * `entities` returns an iterator of all entities in the scene
 * `name` consumes an entity value off the top of the stack and returns its name or `nil` if it does not have a name component
 * `pop` pops the top-most element off the stack and drops it
 * `index` indexes into an iterator
    - `[1 2 3] 1 index` -> `2`
 * `enum` enumerates an iterator or a number
    - `[a b c d] enum` -> `[0 1 2 3]` and `4 enum` -> `[0 1 2 3]`
 * `type` gives the type of the top item of the stack as a literal
    - `3.141 type` -> `"num"`
 * `append` appends a value to an iterator
    - `[1 2 3] 4 append` -> `[1 2 3 4]`
 * `extend` extends an iterator with another iterator
    - `[1 2 3] [4 5 6] extend` -> `[1 2 3 4 5 6]`
 * `reverse` reverses an iterator
    - `[1 2 3] reverse` -> `[3 2 1]`
 * `filter` consumes `val cond` off the top of the stack and pushes val onto the stack if and only if `cond` is true.
    - `a true filter` -> `a`, `b false filter` -> ` `. This is particularly useful with broadcasting: `[a b c] [true false true] filter` -> `[a c]`
 * `select` consumes `lhs rhs cond` off the top of the stack and
    - `true a b select` -> `a`, `false a b select` -> `b`. This is particularly useful with broadcasting: `[true false true] [a b c] [1 2 3] select` -> `[a 2 c]`
 * `mod` consumes `lhs rhs` off the top of the stack and pushes `lhs % rhs` back onto it. If `lhs` is not a number, it instead pushes `nil`.
    - `7 3 mod` -> `1`
 * `vec2` constructs a `vec2` using the top two values on the stack
    - `1 2 vec2` -> `vec2(1, 2)`
 * `vec3` constructs a `vec3` using the top three values on the stack
    - `1 2 3 vec3` -> `vec3(1, 2, 3)`
 * `quat` constructs a `quat` using the top four values on the stack (note that the quaternion gets normalized automatically to be valid)
    - `0 0 0 1 quat` -> `quat(0, 0, 0, 1)`
 * `fuzzy` consumes two literals (strings) `haystack needle` off the top of the stack and returns a boolean to indicate if `haystack` fuzzily matches `needle`
    - `GlobalTransform glbtrans fuzzy` -> `true`
 * `sin` computes the sine of a number
    - `0 sin` -> `0`
* `cos` computes the cosine of a number
* `tan` computes the tangent of a number
* `atan` computes the arctan of a number
* `atan2` computes the arctan of a fraction
    - `lhs rhs atan2` produces the same as `lhs.atan2(rhs)` in Rust

### Requires Feature: `regex`
 * `regex` consumes two literals (strings) `haystack reg` off the top of the stack and returns a boolean to indicate if `haystack` matches the regular expression `reg`
    - `"tim.apple@apple.com" "\\w+\\.\\w+@\\w+\\.\\w{2,3}" regex` -> `true`

### Requires Feature: `focus`
 * `focus` consumes an entity off the top of the stack and highlights it in the scene
 * `unfocus` consumes an entity off the top of the stack and removes its highlight
 * `focused` returns an iterator of all currently highlighted entities


## fyfth Prefixes
fyfth allows for single-character prefixes to speed scripting. For example, `*foo` prefixes the word `foo` with `*`. Prefixes can also be followed by words encapsulated by quotes, so `$"foo bar baz"` prefixes the word `foo bar baz` with `$`.

### Core Language Prefixes
 * `*` loads the variable with the name following the prefix (`*word` expands to `word load`)
    - `*pi` -> `3.141` if we have previously stored the value.
 * `$` runs a macro stored in memory (`$word` expands to `word load queue`)
    - `100 enum 1 add $fizzbuzz` runs the `fizzbuzz` macro from the standard prelude on `[1 2 3... 100]`
 * `@` fuzzily searches for an entity of that name and selects the first match
    - if there's an entity "My Camera", then `@mycam` will load it (if `mycam` does not match any other entity's name)


## Broadcasting
fyfth takes inspiration from how [numpy](https://numpy.org/) uses [broadcasting](https://numpy.org/doc/stable/user/basics.broadcasting.html) to more easily apply operations. We misuse the term here to mean the entire process of how fyfth takes simpler commands and applies them to iterators automatically.

As an example, consider the `add` command. It is defined on `num` types, so that e.g. `1 2 add` produces `3`. However, the broadcast behavior of `add` is
```rust
[FyfthBroadcastBehavior::MayIter, FyfthBroadcastBehavior::MayIter]
```
meaning that it may apply the operation to the individual elements of an iterator for both arguments. Thus we can also use `[1 2] [3 4] add` to get `[3 6]`. Moreover, we can use the broadcasting idea of numpy to combine an iterator with a non iterator. In this case, we can also do `[1 2] 3 add` to get `[4 5]`, or `1 [2 3] add` to get `[3 4]`. Essentially, what fyfth does in these situations is that it broadcasts the scalar value into an iterator of the same length as the other. So `[1 2] 3 add` turns into `[1 2] [3 3] add` and `1 [2 3] add` into `[1 1] [2 3] add`.

> [!NOTE]
> This behavior currently has the unintended side effect of making some commands return empty iterators when you might expect them to not return anything. For example, `some_entity focus` leaves an empty stack, where as `entities focus` leaves `[]` as `focus` goes over the iterator produced by `entities` and consumes its elements but not the iterator itself.

## License
The fyfth programming language and all code within this repository is dual-licensed under either:
 * MIT License [LICENSE-MIT](LICENSE-MIT)
 * Apache License, Version 2.0 [LICENSE-APACHE](LICENSE-APACHE)
