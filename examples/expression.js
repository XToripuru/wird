let hello = #{
    String::from("Hello")
} -> String;

console.log(hello);



let len = #{
    "Hello".chars().count()
} -> usize;

console.log(len);