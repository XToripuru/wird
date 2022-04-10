let hello = #{
    "Hello".to_string()
} -> String;

console.log(hello);



let len = #{
    "Hello".chars().count()
} -> usize;

console.log(len);