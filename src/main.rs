use std::path::*;
use std::fs::*;

fn main() {

    let args: Vec<String> = std::env::args().collect();

    if args.len() >= 3 {
        if args[1] == "expand" {
            let path = PathBuf::from(&*args[2]);
            let code = read_to_string(&*args[2]).unwrap();
            let pkg = if args.len() >= 4 {
                &*args[3]
            } else {
                "wird"
            };

            codegen(code, path.file_name().unwrap().to_str().unwrap(), pkg);
        }
    }
    if args.len() == 2 {
        if args[1] == "host" {
            let _child = std::process::Command::new("cmd.exe")
            .arg("/C").arg("http-server exp").spawn().unwrap();
        }
    }
    
}

fn codegen(code: String, name: &str, pkg: &str) {
    let out = PathBuf::from("./exp");
    if !out.exists() {
        let src = PathBuf::from("./exp/src");
        create_dir_all(&src).unwrap();
        write("./exp/Cargo.toml", format!("[package]
name = \"{}\"
version = \"0.0.0\"
edition = \"2018\"

[lib]
crate-type = [\"cdylib\", \"rlib\"]

[dependencies]
wasm-bindgen = \"0.2\"", pkg)).unwrap();
    }

    std::fs::copy("./index.html", "./exp/index.html").unwrap();

    let mut javascript = String::new();
    let mut rust = String::new();
    let mut fns = vec![];

    for gen in generate(code) {
        match gen {
            Codegen::Javascript { code } => {
                javascript.push_str(&*code);
            }
            Codegen::Static { code } => {
                let tokens: Vec<&str> = code.split(" ").collect();
                for k in 0..tokens.len() {
                    if tokens[k].trim() == "#[wasm]" {
                        for p in (k+1).. {
                            if tokens[p] == "fn" && tokens[p-1] == "pub" {
                                let name = tokens[p+1].split("(").next().unwrap();
                                fns.push(name.into());
                                break;
                            }
                        }
                    }
                }
                rust.push_str(&*code);
            }
            Codegen::Expression { capture, expr, ret, n } => {
                fns.push(format!("V_A{}", n));
                if let Some(capture) = capture {
                    rust.push_str(
                        &*format!("#[wasm] pub fn V_A{}({}) -> {} {{ {} }}",
                        n,
                        capture.into_iter().map(|(n, t)| format!("{}: {}", n, t)).collect::<Vec<String>>().join(", "),
                        ret,
                        expr
                    ));
                } else {
                    rust.push_str(
                        &*format!("#[wasm] pub fn V_A{}() -> {} {{ {} }}",
                        n,
                        ret,
                        expr
                    ));
                }
                
            }
        }
    }

    write(format!("./exp/{}", name), format!("import init from './pkg/{}.js';
import {{ {} }} from './pkg/{}.js';

function run() {{
{}
}}

init().then(run);", pkg, fns.join(", "), pkg, javascript)).unwrap();

    write("./exp/src/lib.rs", format!("use wasm_bindgen::prelude::wasm_bindgen as wasm;\r\n{}", rust)).unwrap();

    std::thread::sleep(std::time::Duration::from_millis(1000));
    
    let child = std::process::Command::new("cmd.exe")
    .stdout(std::process::Stdio::piped())
    .stderr(std::process::Stdio::piped())
    .arg("/C").arg("wasm-pack build exp --target web").spawn().unwrap();

    let output = child.wait_with_output().unwrap();
    let out = String::from_utf8(output.stderr).unwrap();
    let out: Vec<&str> = out.split("\n").collect();
    if out[out.len()-2].contains("ready") {
        println!("Built successfully!");
    } else {
        println!("Error occured during compilation");
        println!("{}", out.join("\n"));
    }
}

fn generate(mut code: String) -> Vec<Codegen> {

    let ch: Vec<char> = code.chars().collect();
    let s: Vec<bool> = {
        let mut r = Vec::with_capacity(ch.len());
        let mut init = None;
        for k in 0..ch.len() {
            if let Some(a) = init {
                if a == ch[k] {
                    for b in 0.. {
                        if ch[k-1-b] != '\\' {
                            if b%2 == 0 {
                                init = None;
                            }
                            break;
                        }
                    }
                }
            } else if (ch[k] == '"' || ch[k] == '\'' || ch[k] == '`') && (k == 0 || ch[k-1] != '\\') {
                init = Some(ch[k]);
            }
            r.push(init.is_some());
        }
        r
    };

    let mut gens = vec![];
    let mut nexpr = 0;
    let mut drains = vec![];
    
    let mut k = 0;
    while k < ch.len() {

        if !s[k] && ch[k] == '#' && (ch[k+1] == '{' || ch[k+1] == '[') {

            let mut capture = None;

            let mut n = k;

            // Check if capture is present and parse it
            if ch[k+1] == '[' {
                let mut b: i32 = 1;
                for mut p in (n+2).. {

                    b += if !s[p] && ch[p] == '[' {1}
                    else if !s[p] && ch[p] == ']' {-1}
                    else {0};

                    if b == 0 {
                        let cap = {
                            let mut cap: String = code[n..p+1].into();
                            cap = cap[2..(cap.len()-1)].into();
                            cap
                        };

                        let mut captures: Vec<(String, String)> = vec![];
                        
                        let mut last = 0;
                        let mut b = 0;
                        for (k, c) in cap.chars().enumerate() {
                            match c {
                                '(' => b += 1,
                                ')' => b -= 1,
                                ',' if b == 0 => {
                                    let mut tokens = cap[last..k].split(":");
                                    captures.push((tokens.next().unwrap().trim().into(), tokens.next().unwrap().trim().into()));
                                    last = k + 1;
                                }
                                _ => {}
                            }
                        }
                        let mut tokens = cap[last..].split(":");
                        captures.push((tokens.next().unwrap().trim().into(), tokens.next().unwrap().trim().into()));

                        capture = Some(captures);
                        n = p;
                        break;
                    }
                }
            }

            let mut b: i32 = 1;
            for mut p in (n+2).. {

                b += if !s[p] && ch[p] == '{' {1}
                else if !s[p] && ch[p] == '}' {-1}
                else {0};

                if b == 0 {
                    let mut block = {
                        let mut block: String = code[n..p+1].into();
                        block = block[2..(block.len()-1)].into();
                        block
                    };
                    
                    let mut ret = None;

                    if ch[p+1] == ';' {
                        p += 1;
                    } else {
                        for a in (p+1).. {
                            if !s[a] && ch[a] == ';' {
                                ret = Some({
                                    let mut ret: String = code[(p+1)..a].into();
                                    ret = ret.trim()[2..].trim().into();
                                    ret
                                });
                                p = a;
                                break;
                            }
                        }
                    }

                    if let Some(ret) = ret {
                        block = block.trim().into();
                        let name = format!("V_A{}", nexpr);
                        //println!("[expression] [{}] [{}] -> [{}]", name, block, ret);

                        gens.push(Codegen::Expression { capture: capture.clone(), expr: block, ret, n: nexpr });
                        if let Some(capture) = capture {
                            drains.push(
                                (k..(p+1),
                                Some(
                                    format!("{}({});",
                                    name,
                                    capture.into_iter().map(|(n, _)| n).collect::<Vec<String>>().join(", ")
                                )
                            )));
                        } else {
                            drains.push((k..(p+1), Some(format!("{}();", name))));
                        }
                        nexpr += 1;
                    } else {
                        //println!("[static] [{}]", block);
                        gens.push(Codegen::Static { code: block });
                        drains.push((k..(p+1), None));
                    }

                    k = p;
                    break;
                }
            }
        }

        k += 1;
    }

    let mut roff = 0;
    for (d, rep) in drains {
        let (a, b) = (d.start, d.end);
        if let Some(rep) = &rep {
            code.replace_range((a-roff)..(b-roff), rep);
        } else {
            code.drain((a-roff)..(b-roff));
        }
        roff += b - a;
    }

    //println!("[pure] {}", code);
    gens.push(Codegen::Javascript { code });

    gens
}

enum Codegen {
    Javascript {
        code: String
    },
    Static {
        code: String
    },
    Expression {
        capture: Option<Vec<(String, String)>>,
        expr: String,
        ret: String,
        n: usize,
    }
}





























