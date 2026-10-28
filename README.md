<p align="center"><img src="assets/logo.svg" width="120" alt="unitconv logo"></p>

# unitconv

unitconv is a universal unit converter for the terminal.
It also has a scientific calculator.
You can use the keyboard or the mouse.

```text
 unitconv   Common   Engineering   Science   Computing   Calculator
┌ Value ──────────────────────────────────────────────────────────────────── Square inch (in²) ┐
│12.5█                                                                                         │
└──────────────────────────────────────────────────────────────────────────────────────────────┘
┌ Category ──────┐┌ From ──────────────────┐┌ Result ──────────────────────────────────────────┐
│Length          ││Square meter            ││        0.0080645 m²      Square meter            │
│Area            ││Square kilometer        ││        8.0645e-9 km²     Square kilometer        │
│Volume          ││Square centimeter       ││           80.645 cm²     Square centimeter       │
│Mass            ││Square millimeter       ││           8064.5 mm²     Square millimeter       │
│Time            ││Hectare                 ││        8.0645e-7 ha      Hectare                 │
│Speed           ││Are                     ││        8.0645e-5 a       Are                     │
│Temperature     ││Square inch             ││             12.5 in²     Square inch             │
│                ││Square foot             ││  0.0868055555556 ft²     Square foot             │
│                ││Square yard             ││  0.0096450617284 yd²     Square yard             │
│                ││Acre                    ││ 1.99278134884e-6 ac      Acre                    │
│                ││Square mile             ││ 3.11372085757e-9 mi²     Square mile             │
│                ││                        ││                                                  │
```

## Features

- More than 30 categories in 4 tabs: Common, Engineering, Science, and Computing.
- One input value shows the result in all the units of the category.
- The value input accepts expressions, for example `3*12.5` or `2pi`.
- A scientific calculator tab with a keypad that you can click.
- One file contains all the unit data. To add a unit, add one line.

## Install

You must have the Rust toolchain (`cargo`).

```sh
cargo install --git https://github.com/polymood/unitconv
```

To build from the source code:

```sh
git clone https://github.com/polymood/unitconv
cd unitconv
cargo run --release
```

## Use

| Key | Action |
| --- | --- |
| Left, Right | Go to the previous or the next tab |
| Tab | Move the focus between the category list and the unit list |
| Up, Down | Select a category or a unit |
| Characters | Type the value or the expression |
| Backspace | Delete the last character |
| Delete | Clear the input |
| Enter | Calculate the expression (calculator tab) |
| Esc, Ctrl+C | Quit |

With the mouse:

- Click a tab, a category, or a unit to select it.
- Click a result to make it the new input value and unit.
- Turn the mouse wheel to scroll a list.
- Click the keys of the calculator keypad.

An empty input has the value 1.

## Calculator

| Item | Syntax |
| --- | --- |
| Operators | `+ - * / % ^ !` and parentheses |
| Implicit multiplication | `2pi`, `3(1+2)` |
| Constants | `pi`, `e`, `tau`, `ans` (the last result) |
| Functions | `sin cos tan asin acos atan sinh cosh tanh sqrt cbrt ln log log2 exp abs floor ceil round` |

The DEG key sets the angle unit to degrees or radians.
The trigonometric functions use this angle unit.
After a result, type a number to start a new calculation.
Type an operator to continue the calculation with the result.

## Add a unit

All the unit data is in `src/units.rs`.
The first unit of each category is the base unit.
The factor of a unit is the value of 1 unit in the base unit.

1. Open `src/units.rs`.
2. Find the category.
3. Add one line to the list of units:

   ```rust
   u("Furlong", "fur", 201.168),
   ```

4. Run `cargo test`.

For units that do not have a simple factor, use these functions:

- `affine(name, symbol, factor, offset)`: base = value × factor + offset (temperature scales).
- `inverse(name, symbol, k)`: base = k / value (fuel consumption).

## Add a category

1. Write a `const` block in `src/units.rs`:

   ```rust
   const LUMINOUS_FLUX: Category = Category {
       name: "Luminous flux",
       units: &[u("Lumen", "lm", 1.0), u("Kilolumen", "klm", 1e3)],
   };
   ```

2. Add the name of the block to a group in `GROUPS`.
3. Run `cargo test`.

To add a tab, add a new `Group` to `GROUPS`.

## Test

```sh
cargo test
```

The test `round_trip_all_units` converts each unit to the base unit and back.
Thus, an incorrect line in `src/units.rs` causes a test failure.

## License

MIT. Refer to the [LICENSE](LICENSE) file.
