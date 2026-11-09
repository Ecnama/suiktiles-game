# Suiktiles game

A simple [suika game](https://en.wikipedia.org/wiki/Suika_Game) written in Rust with [macroquad](https://macroquad.rs/) and the [rapier](https://rapier.rs/) physics engine.

## Rules

Fill the rectangle with as many balls as possible. Two balls of the same level will merge into a ball of the next level. The game ends when a ball exceeds the height limit for more than 1 second.

You gain points:

- by dropping a ball into the rectangle (1 point)
- by merging two balls (2^level points, where level is the level of the resulting ball)

## Running the game

To build and run the game, you need to have [Cargo](https://doc.rust-lang.org/cargo/getting-started/installation.html) installed. You can then run the following command:

### Native

```bash
cargo run --bin suiktiles-game --release
```

### Web

```bash
cargo run --bin wasm_build
```

You can then serve the `web` folder with a static file server and open `index.html` in your browser. For example, you can use Rust's `basic-http-server` crate:

```bash
cargo install basic-http-server
basic-http-server web/
```

Then open your browser and go to [http://localhost:4000](http://localhost:4000).
