let a = 3;
let b = 5;

let c = #[a: i32, b: i32]{
    a + b
} -> i32;

console.log(c);



let name = "John";

let greet = #[name: &str]{
    format!("Hello {}", name)
} -> String;

console.log(greet);