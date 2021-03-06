# Wird
Wird allows to write Javascript with inlined Rust that will later become Wasm

## Usage
You can inline Rust using `#{...};` syntax, inside these you can define functions, structs, etc.\
Expressions are also allowed by annotating the return type `#{...} -> T;`, they can capture Js variables by annotating which variables are captured along with their type `#[a: A, b: B]{...} -> T;`\
**NOTE:** in order to export function to Js you have to add `#[wasm]` to it and make it `pub`\
Check out `examples`

After writing code you can *compile* it using `wird`:\
`wird expand index.js`

You can also quickly host your files with `wird host`

## Prerequisites
* `cargo` and then you can get the following via `cargo install`
* `wasm-pack`
* `http-server`

## Download
`cargo install wird`

## Work in progress
Although it compiles just fine there are still missing a lot of features, and it shouldn't be used in production yet