# Wird
Wird allows to write Javascript with inlined Rust that will later become Wasm

## Usage
You can inline Rust using `#{...};` syntax, inside these you can define functions, structs, etc.
Expressions are also allowed by annotating the return type `#{...} -> T;`, they can capture Js variables by annotating which variables are captured along with their type `#[a: A, b: B]{...} -> T;`
Check out `examples`

After writing code you can *compile* it using `wird`:
`wird expand index.js`

You can also quickly host your files with `wird host`

## Prerequisites
* `cargo` and then you can get the following via `cargo install`
* `wasm-pack`
* `http-server`

## Download
`cargo install wird`

## Work in progress
This is still *very* incomplete, and shouldn't be used in production yet