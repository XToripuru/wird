#{
    #[wasm]
    pub fn add(x: i32, y: i32) -> i32 {
        x + y
    }
}

let a = 1;
let b = 2;
console.log(add(a, b));