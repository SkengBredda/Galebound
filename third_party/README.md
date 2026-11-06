# third_party

`miniquad/` is miniquad 0.4.8 (MIT / Apache-2.0, https://github.com/not-fl3/miniquad),
the window and input library macroquad uses, with two small Galebound changes so
game controllers work on Android:

- `src/native/android/keycodes.rs`: gamepad buttons (A, B, X, Y, L1, R1, L2, R2,
  L3, R3, Start, Select, Mode) are passed to the game as F13-F25 instead of being
  dropped as unknown keys.
- `java/MainActivity.java`: analog sticks, hats and triggers are turned into D-pad
  and L2 / R2 key presses.

The workspace `Cargo.toml` uses it through `[patch.crates-io]`.
